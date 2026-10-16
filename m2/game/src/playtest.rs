// Search bot that plays levels through the real simulation to prove they can be finished.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Beam search over short input macros. The bot forks the live `World` at every step, so it
//! exercises the same physics, hazards, doors and switches the player meets. Hostile enemies are
//! removed so a result speaks for the platforming; a level passes when the exit is reached.

use crate::ents::Kind;
use crate::world::{input::*, Mode, World};
use std::collections::HashSet;

/// One input macro: hold `mask` for `ticks` ticks, releasing jump for the first tick so each jump
/// press is a fresh edge.
#[derive(Clone, Copy)]
struct Act {
    mask: u32,
    ticks: u8,
}

const WALK: [Act; 9] = [
    Act {
        mask: RIGHT,
        ticks: 8,
    },
    Act {
        mask: RIGHT | JUMP,
        ticks: 6,
    },
    Act {
        mask: RIGHT | JUMP,
        ticks: 11,
    },
    Act {
        mask: RIGHT | JUMP,
        ticks: 18,
    },
    Act {
        mask: LEFT,
        ticks: 8,
    },
    Act {
        mask: LEFT | JUMP,
        ticks: 16,
    },
    Act {
        mask: JUMP,
        ticks: 16,
    },
    Act { mask: 0, ticks: 6 },
    Act {
        mask: POGO,
        ticks: 1,
    },
];

/// Extra macros for levels with ladders and lifts.
const CLIMB: [Act; 6] = [
    Act {
        mask: UP,
        ticks: 10,
    },
    Act {
        mask: DOWN,
        ticks: 10,
    },
    Act {
        mask: UP | RIGHT,
        ticks: 8,
    },
    Act {
        mask: DOWN | RIGHT,
        ticks: 8,
    },
    Act {
        mask: UP | LEFT,
        ticks: 8,
    },
    Act {
        mask: DOWN | LEFT,
        ticks: 8,
    },
];

/// Extra macros for levels where fizz is part of the puzzle.
const SHOOT: [Act; 3] = [
    Act {
        mask: UP | FIRE,
        ticks: 3,
    },
    Act {
        mask: FIRE,
        ticks: 3,
    },
    Act { mask: 0, ticks: 4 },
];

/// A spot Ben must stand on, in tile coordinates (his feet at `y`, within half a tile of `x`, so a
/// key placed there is always picked up); the last one is normally the exit, which also ends the
/// search once it is touched.
#[derive(Clone, Copy)]
pub struct Waypoint {
    pub x: f64,
    pub y: f64,
    /// When set, the waypoint is reached once that switch channel is off (a gate has opened),
    /// wherever Ben is; `x` and `y` then say where he should be while he works on it.
    pub gate: Option<u8>,
}

impl Waypoint {
    pub fn stand(x: f64, y: f64) -> Self {
        Waypoint { x, y, gate: None }
    }

    pub fn gate(channel: u8, x: f64, y: f64) -> Self {
        Waypoint {
            x,
            y,
            gate: Some(channel),
        }
    }
}

pub struct Options {
    pub beam: usize,
    /// Most states kept per tile column, height band and ground/air state.
    pub per_bucket: usize,
    pub max_ticks: u32,
    pub climb: bool,
    /// Add fizz shots (straight up, and sideways) to the moves, for levels with mirrors and switches.
    pub fire: bool,
    /// Allow the pogo toggle. Turn it off to prove a route needs nothing but running and jumping.
    pub pogo: bool,
    /// Keep enemies (the default removes everything hostile).
    pub keep_enemies: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            beam: 120,
            per_bucket: 4,
            max_ticks: 6000,
            climb: false,
            fire: false,
            pogo: true,
            keep_enemies: false,
        }
    }
}

#[derive(Debug)]
pub struct Outcome {
    pub finished: bool,
    pub ticks: u32,
    /// Index of the furthest waypoint reached, and the furthest grounded position seen.
    pub waypoint: usize,
    pub best_x: f64,
    pub best_y: f64,
    /// Where the best few states ended up (`x`, `y`, `grounded`), for diagnosing a stall.
    pub top: Vec<(f64, f64, bool)>,
    /// Which keys the best state holds (red, blue, green).
    pub keys: [bool; 3],
}

struct Node {
    w: World,
    wp: usize,
}

fn reached(w: &World, p: Waypoint) -> bool {
    if let Some(ch) = p.gate {
        return !w.map.switch(ch);
    }
    w.p.b.on_ground && (w.p.b.centre_x() - p.x).abs() < 0.45 && (w.p.b.y - p.y).abs() < 0.3
}

fn dist(w: &World, p: Waypoint) -> f64 {
    (w.p.b.centre_x() - p.x).abs() + 1.5 * (w.p.b.y - p.y).abs()
}

type Key = (i32, i32, i8, i8, bool, bool, usize, u32, i32, u8);

fn key(n: &Node) -> Key {
    let b = &n.w.p.b;
    // Switches, ammo and mirror angles are part of the state: two Bens in the same spot are not the
    // same if one has already swung a mirror round.
    let swivels =
        n.w.ents
            .iter()
            .filter(|e| e.kind == Kind::Swivel)
            .fold(0u8, |a, e| (a << 1) | u8::from(e.dir > 0.0));
    (
        (b.x * 3.0).round() as i32,
        (b.y * 3.0).round() as i32,
        b.vx.round() as i8,
        (b.vy / 4.0).round() as i8,
        b.on_ground,
        n.w.p.pogo,
        n.wp,
        n.w.map.switches,
        n.w.game.ammo,
        swivels,
    )
}

fn play(w: &mut World, a: Act) -> bool {
    for t in 0..a.ticks {
        let held = if t == 0 { a.mask & !JUMP } else { a.mask };
        w.step(held);
        if w.p.dead > 0.0 || w.mode != Mode::Level {
            return false;
        }
        if w.won {
            return true;
        }
    }
    true
}

/// Searches for an input sequence that visits every waypoint in order and wins the level.
pub fn solve(mut start: World, route: &[Waypoint], o: &Options) -> Outcome {
    if !o.keep_enemies {
        start.ents.retain(|e| {
            matches!(
                e.kind,
                Kind::Switch
                    | Kind::Terminal
                    | Kind::Cage
                    | Kind::Mirror
                    | Kind::Swivel
                    | Kind::CrystalSwitch
            )
        });
    }
    let acts: Vec<Act> = WALK
        .iter()
        .chain(if o.climb { CLIMB.iter() } else { [].iter() })
        .chain(if o.fire { SHOOT.iter() } else { [].iter() })
        .filter(|a| o.pogo || a.mask != POGO)
        .copied()
        .collect();
    let mut beam = vec![Node { w: start, wp: 0 }];
    let mut best = Outcome {
        finished: false,
        ticks: 0,
        waypoint: 0,
        best_x: 0.0,
        best_y: 0.0,
        top: Vec::new(),
        keys: [false; 3],
    };
    while !beam.is_empty() && beam[0].w.tick_count < o.max_ticks {
        let mut next: Vec<Node> = Vec::new();
        let mut seen = HashSet::new();
        for n in &beam {
            for a in &acts {
                let mut w = n.w.fork();
                let alive = play(&mut w, *a);
                if w.won {
                    // Touching the exit only counts once every waypoint before it has been reached,
                    // so a route's optional-looking stops (a key, a switch, the secret room) cannot be
                    // skipped by a path that happens to win without them.
                    let mut reached_to = n.wp;
                    while reached_to < route.len() && reached(&w, route[reached_to]) {
                        reached_to += 1;
                    }
                    if reached_to + 1 >= route.len() {
                        best.finished = true;
                        best.ticks = w.tick_count;
                        return best;
                    }
                    continue;
                }
                if !alive {
                    continue;
                }
                let mut wp = n.wp;
                while wp < route.len() && reached(&w, route[wp]) {
                    wp += 1;
                }
                if w.p.b.on_ground && w.p.b.x > best.best_x {
                    best.best_x = w.p.b.x;
                    best.best_y = w.p.b.y;
                }
                let child = Node { w, wp };
                if seen.insert(key(&child)) {
                    next.push(child);
                }
            }
        }
        next.sort_by(|a, b| {
            b.wp.cmp(&a.wp).then_with(|| {
                let da = route.get(a.wp).map_or(0.0, |p| dist(&a.w, *p));
                let db = route.get(b.wp).map_or(0.0, |p| dist(&b.w, *p));
                da.total_cmp(&db)
            })
        });
        // Keep the beam spread out: at most a few states per tile column, height band and ground/air state, so
        // states already committed to a doomed fall cannot crowd out ones waiting to jump.
        let mut per_bucket = std::collections::HashMap::new();
        next.retain(|n| {
            let bucket = (
                n.w.p.b.x.floor() as i32,
                (n.w.p.b.y / 3.0).floor() as i32,
                n.w.p.b.on_ground,
                n.wp,
            );
            let c = per_bucket.entry(bucket).or_insert(0usize);
            *c += 1;
            *c <= o.per_bucket
        });
        next.truncate(o.beam);
        if let Some(n) = next.first() {
            best.waypoint = n.wp;
            best.ticks = n.w.tick_count;
            best.keys = [n.w.keys_red, n.w.keys_blue, n.w.keys_green];
            best.top = next
                .iter()
                .take(6)
                .map(|n| (n.w.p.b.x, n.w.p.b.y, n.w.p.b.on_ground))
                .collect();
        }
        beam = next;
    }
    best
}

/// Starts a fresh game on `id` and searches `route`.
pub fn play_level(id: u8, route: &[Waypoint], o: &Options) -> Outcome {
    let mut w = World::new();
    w.game_new();
    w.enter_level(id);
    solve(w, route, o)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::levels::{build_level, METEOR_MESA};
    use crate::tiles::EXIT;

    /// Where the exit tile sits, as a waypoint standing in front of it.
    pub fn exit_of(id: u8) -> Waypoint {
        let l = build_level(id);
        for x in 0..l.map.w {
            for y in 0..l.map.h {
                if l.map.get(x, y) == EXIT {
                    return Waypoint::stand(f64::from(x) + 0.5, f64::from(y));
                }
            }
        }
        panic!("level {id} has no exit");
    }

    fn wp(x: f64, y: f64) -> Waypoint {
        Waypoint::stand(x, y)
    }

    #[test]
    fn crater_fields_can_be_finished_via_the_red_key() {
        let r = play_level(
            crate::levels::CRATER,
            &[wp(132.5, 13.0), exit_of(crate::levels::CRATER)],
            &Options::default(),
        );
        assert!(
            r.finished,
            "bot stalled at x={:.1} y={:.1} wp {} after {} ticks",
            r.best_x, r.best_y, r.waypoint, r.ticks
        );
    }

    #[test]
    fn crystal_caves_can_be_finished_via_the_switch_and_blue_key() {
        let r = play_level(
            crate::levels::CAVES,
            &[
                wp(57.5, 4.0),
                wp(101.5, 10.0),
                exit_of(crate::levels::CAVES),
            ],
            &Options::default(),
        );
        assert!(
            r.finished,
            "bot stalled at x={:.1} y={:.1} wp {} after {} ticks",
            r.best_x, r.best_y, r.waypoint, r.ticks
        );
    }

    #[test]
    fn zarg_lookout_can_be_finished_via_the_roof_key() {
        use crate::levels::ZARG_LOOKOUT;
        // Standing spots up the tower (floor `k` stands on row 3 + 9k), then the key, then the exit.
        let route = [
            wp(18.5, 12.0), // ladder, ground to floor 1
            wp(23.5, 12.0), // foot of the stairs
            wp(34.5, 21.0), // floor 2 pocket
            wp(36.5, 30.0), // ladder, floor 2 to 3
            wp(15.5, 30.0), // foot of the stairs
            wp(2.5, 39.0),  // floor 4 pocket
            wp(3.5, 48.0),  // ladder, floor 4 to 5
            wp(22.5, 57.0), // floor 6, off the lift
            wp(25.5, 57.0), // foot of the stairs
            wp(36.5, 66.0), // floor 7 pocket
            wp(37.5, 75.0), // roof
            wp(19.5, 75.0), // the key
            // And back down, floor by floor.
            wp(37.5, 75.0),
            wp(36.5, 66.0),
            wp(25.5, 57.0),
            wp(12.0, 48.0), // dropped down the lift shaft
            wp(3.5, 48.0),
            wp(2.5, 39.0),
            wp(15.5, 30.0),
            wp(36.5, 30.0),
            wp(34.5, 21.0),
            wp(23.5, 12.0),
            wp(18.5, 12.0),
            wp(18.5, 3.0),
            exit_of(ZARG_LOOKOUT),
        ];
        let o = Options {
            climb: true,
            max_ticks: 14_000,
            ..Options::default()
        };
        let r = play_level(ZARG_LOOKOUT, &route, &o);
        assert!(
            r.finished,
            "bot stalled at wp {} after {} ticks; best states {:?}",
            r.waypoint, r.ticks, r.top
        );
    }

    #[test]
    fn marshmallow_meadows_can_be_finished_via_the_blue_key() {
        use crate::levels::MARSHMALLOW_MEADOWS;
        let route = [wp(59.5, 15.0), exit_of(MARSHMALLOW_MEADOWS)];
        let r = play_level(MARSHMALLOW_MEADOWS, &route, &Options::default());
        assert!(
            r.finished,
            "bot stalled at wp {} after {} ticks; best states {:?}",
            r.waypoint, r.ticks, r.top
        );
    }

    #[test]
    fn the_saucer_can_be_walked_through_to_the_bridge_and_back_out() {
        use crate::levels::SAUCER;
        let route = [wp(50.5, 9.0), exit_of(SAUCER)];
        let o = Options {
            climb: true,
            max_ticks: 9_000,
            ..Options::default()
        };
        let r = play_level(SAUCER, &route, &o);
        assert!(
            r.finished,
            "bot stalled at wp {} after {} ticks; best states {:?}",
            r.waypoint, r.ticks, r.top
        );
    }

    #[test]
    fn whisper_hollow_can_be_finished_by_climbing() {
        use crate::levels::WHISPER_HOLLOW;
        let route = [exit_of(WHISPER_HOLLOW)];
        let o = Options {
            climb: true,
            beam: 600,
            max_ticks: 12_000,
            ..Options::default()
        };
        let r = play_level(WHISPER_HOLLOW, &route, &o);
        assert!(
            r.finished,
            "bot stalled at wp {} after {} ticks, best {:.1},{:.1}; best states {:?}",
            r.waypoint, r.ticks, r.best_x, r.best_y, r.top
        );
    }

    #[test]
    fn whisper_hollow_cannot_be_finished_without_climbing() {
        use crate::levels::WHISPER_HOLLOW;
        let route = [exit_of(WHISPER_HOLLOW)];
        let o = Options {
            max_ticks: 3_000,
            ..Options::default()
        };
        assert!(!play_level(WHISPER_HOLLOW, &route, &o).finished);
    }

    #[test]
    fn bonbon_playhouse_can_be_finished_via_both_keys() {
        use crate::levels::BONBON_PLAYHOUSE;
        let route = [wp(50.5, 10.0), wp(108.5, 14.0), exit_of(BONBON_PLAYHOUSE)];
        // Pulling up onto ledges and kicking off walls reach heights the distance heuristic likes
        // better than the walk to the door, so keep a wider beam than the default.
        let o = Options {
            beam: 400,
            ..Options::default()
        };
        let r = play_level(BONBON_PLAYHOUSE, &route, &o);
        assert!(
            r.finished,
            "bot stalled at wp {} after {} ticks; best states {:?}",
            r.waypoint, r.ticks, r.top
        );
    }

    #[test]
    fn fudge_bog_can_be_finished_via_the_red_key() {
        use crate::levels::FUDGE_BOG;
        let route = [wp(139.5, 15.0), exit_of(FUDGE_BOG)];
        let r = play_level(FUDGE_BOG, &route, &Options::default());
        assert!(
            r.finished,
            "bot stalled at wp {} after {} ticks; best states {:?}",
            r.waypoint, r.ticks, r.top
        );
    }

    #[test]
    fn mirror_shafts_can_be_finished_by_bouncing_fizz_off_the_mirrors() {
        use crate::levels::MIRROR_SHAFTS;
        let route = [
            wp(18.5, 21.0),
            Waypoint::gate(1, 18.5, 21.0),
            wp(10.5, 54.0),
            Waypoint::gate(2, 10.5, 54.0),
            wp(18.5, 78.0),
            Waypoint::gate(3, 18.5, 78.0),
            exit_of(MIRROR_SHAFTS),
        ];
        let o = Options {
            fire: true,
            // Nothing in the shaft should need the pogo: three-row steps and ledges under each gate.
            pogo: false,
            max_ticks: 16_000,
            ..Options::default()
        };
        let r = play_level(MIRROR_SHAFTS, &route, &o);
        assert!(
            r.finished,
            "bot stalled at wp {} after {} ticks; best states {:?}",
            r.waypoint, r.ticks, r.top
        );
    }

    #[test]
    fn sugar_glass_gallery_can_be_finished_and_its_mural_room_reached() {
        use crate::levels::SUGAR_GLASS_GALLERY;
        let route = [
            wp(46.5, 10.0),  // red key
            wp(100.5, 14.0), // blue key
            wp(130.5, 10.0), // the mural alcove
            exit_of(SUGAR_GLASS_GALLERY),
        ];
        let r = play_level(SUGAR_GLASS_GALLERY, &route, &Options::default());
        assert!(
            r.finished,
            "bot stalled at wp {} after {} ticks; best states {:?}",
            r.waypoint, r.ticks, r.top
        );
    }

    #[test]
    fn frosting_flats_can_be_finished_via_the_red_key() {
        use crate::levels::FROSTING_FLATS;
        let route = [wp(131.5, 12.0), exit_of(FROSTING_FLATS)];
        let r = play_level(FROSTING_FLATS, &route, &Options::default());
        assert!(
            r.finished,
            "bot stalled at wp {} after {} ticks; best states {:?}",
            r.waypoint, r.ticks, r.top
        );
    }

    #[test]
    fn frosting_spire_can_be_finished_with_all_three_keys() {
        use crate::levels::FROSTING_SPIRE;
        // Floor `k` stands on row 3 + 9k. Ladders alternate sides (left for even connectors,
        // right for odd ones); connectors 2, 6 and 9 are lifts.
        let route = [
            wp(6.5, 12.0),   // up: floor 1
            wp(38.5, 21.0),  // floor 2
            wp(21.5, 30.0),  // floor 3, by lift
            wp(14.5, 30.0),  // the red key
            wp(38.5, 39.0),  // floor 4
            wp(6.5, 48.0),   // floor 5
            wp(38.5, 57.0),  // floor 6, past the red door
            wp(21.5, 66.0),  // floor 7, by lift
            wp(14.5, 66.0),  // the blue key
            wp(38.5, 75.0),  // floor 8
            wp(6.5, 84.0),   // floor 9, past the blue door
            wp(21.5, 93.0),  // floor 10, by lift
            wp(6.5, 102.0),  // the roof
            wp(30.5, 102.0), // the green key
            // And down again.
            wp(6.5, 93.0),
            wp(16.0, 84.0), // dropped down the lift shaft
            wp(6.5, 84.0),
            wp(6.5, 75.0),
            wp(38.5, 75.0),
            wp(38.5, 66.0),
            wp(28.0, 57.0), // dropped down the lift shaft
            wp(38.5, 57.0),
            wp(38.5, 48.0),
            wp(6.5, 48.0),
            wp(6.5, 39.0),
            wp(38.5, 39.0),
            wp(38.5, 30.0),
            wp(28.0, 21.0), // dropped down the lift shaft
            wp(38.5, 21.0),
            wp(38.5, 12.0),
            wp(6.5, 12.0),
            wp(6.5, 3.0),
            exit_of(FROSTING_SPIRE),
        ];
        let o = Options {
            climb: true,
            max_ticks: 40_000,
            ..Options::default()
        };
        let r = play_level(FROSTING_SPIRE, &route, &o);
        assert!(
            r.finished,
            "bot stalled at wp {} after {} ticks; keys {:?}; best states {:?}",
            r.waypoint, r.ticks, r.keys, r.top
        );
    }

    #[test]
    fn cocoa_foundry_can_be_finished_via_the_blue_key() {
        use crate::levels::COCOA_FOUNDRY;
        let route = [wp(93.5, 12.0), exit_of(COCOA_FOUNDRY)];
        let r = play_level(COCOA_FOUNDRY, &route, &Options::default());
        assert!(
            r.finished,
            "bot stalled at wp {} after {} ticks; keys {:?}; best states {:?}",
            r.waypoint, r.ticks, r.keys, r.top
        );
    }

    #[test]
    fn gumdrop_isle_can_be_finished() {
        use crate::levels::GUMDROP_ISLE;
        let route = [exit_of(GUMDROP_ISLE)];
        let r = play_level(GUMDROP_ISLE, &route, &Options::default());
        assert!(
            r.finished,
            "bot stalled at wp {} after {} ticks; best states {:?}",
            r.waypoint, r.ticks, r.top
        );
    }

    #[test]
    fn meteor_mesa_can_be_finished() {
        let r = play_level(METEOR_MESA, &[exit_of(METEOR_MESA)], &Options::default());
        assert!(
            r.finished,
            "bot stalled at x={:.1} y={:.1} after {} ticks",
            r.best_x, r.best_y, r.ticks
        );
    }

    #[test]
    fn a_win_does_not_count_while_a_waypoint_before_the_exit_is_unreached() {
        // The exit is easy to reach, but the first waypoint is in mid-air above the start where
        // nobody can stand: the bot may touch the exit and still must not be called finished.
        let route = [wp(12.5, 25.0), exit_of(METEOR_MESA)];
        let o = Options {
            max_ticks: 2500,
            ..Options::default()
        };
        let r = play_level(METEOR_MESA, &route, &o);
        assert!(!r.finished, "a skipped waypoint must not pass");
    }
}
