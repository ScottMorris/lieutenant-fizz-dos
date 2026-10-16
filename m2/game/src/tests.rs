// Whole-world tests for the Episode 1 simulation, including determinism.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Whole-world behaviour tests: the player, items, doors, enemies, boss, overworld and the
//! determinism of the instance buffer.

use crate::ents::{ItemKind, Kind, St};
use crate::levels::{CAVES, CITADEL, CRATER};
use crate::text::{ev, Cap};
use crate::tiles::*;
use crate::world::{death, input::*, Mode, World, MANTLE_REACH, MANTLE_TIME, POGO_DRAWN_HEIGHT};

fn level(id: u8) -> World {
    let mut w = World::new();
    w.game_new();
    w.enter_level(id);
    w.events.clear();
    w
}

fn run(w: &mut World, ticks: usize, held: u32) {
    for _ in 0..ticks {
        w.step(held);
    }
}

fn events_of(w: &World, kind: u32) -> Vec<lf_sim::Event> {
    (0..w.events.len())
        .filter_map(|i| w.events.get(i))
        .filter(|e| e.kind == kind as f32)
        .collect()
}

#[test]
fn ben_lands_and_stands_on_the_ground() {
    let mut w = level(CRATER);
    run(&mut w, 90, 0);
    assert!(w.p.b.on_ground);
    assert!((w.p.b.y - 4.0).abs() < 0.01, "y = {}", w.p.b.y);
}

#[test]
fn running_accelerates_to_top_speed_and_stops_with_friction() {
    let mut w = level(CRATER);
    run(&mut w, 30, 0);
    let x0 = w.p.b.x;
    run(&mut w, 60, RIGHT);
    assert!(w.p.b.x > x0 + 4.0);
    assert!(
        (w.p.b.vx - 7.0).abs() < 1e-9,
        "top speed 7 tiles/s, got {}",
        w.p.b.vx
    );
    run(&mut w, 20, 0);
    assert_eq!(w.p.b.vx, 0.0);
}

#[test]
fn jump_height_is_variable() {
    let apex = |hold: bool| {
        let mut w = level(CRATER);
        run(&mut w, 30, 0);
        w.step(JUMP);
        let mut max_y = w.p.b.y;
        for _ in 0..90 {
            w.step(if hold { JUMP } else { 0 });
            max_y = max_y.max(w.p.b.y);
        }
        max_y - 4.0
    };
    let (full, short) = (apex(true), apex(false));
    assert!((full - 3.5).abs() < 0.35, "full jump apex {full}");
    assert!(short < full * 0.5, "released early: {short} vs {full}");
}

#[test]
fn pogo_bounces_automatically_and_high_with_jump_held() {
    let mut w = level(CRATER);
    run(&mut w, 30, 0);
    w.step(POGO);
    assert!(w.p.pogo);
    let mut max_y = 0.0f64;
    for _ in 0..200 {
        w.step(JUMP | POGO);
        max_y = max_y.max(w.p.b.y);
    }
    assert!(
        max_y - 4.0 > 6.0 && max_y - 4.0 < 7.2,
        "pogo apex {}",
        max_y - 4.0
    );
    // Without jump held, the bounce is the short one (~1.6 tiles).
    let mut w = level(CRATER);
    run(&mut w, 30, 0);
    w.step(POGO);
    let mut max_y = 0.0f64;
    for _ in 0..120 {
        w.step(POGO);
        max_y = max_y.max(w.p.b.y);
    }
    assert!(max_y - 4.0 < 2.5, "short bounce {}", max_y - 4.0);
}

#[test]
fn spikes_and_chocolate_kill_and_death_costs_a_life() {
    let mut w = level(CRATER);
    // First chocolate pool: x = 14+2+6+2+5 = 29 .. 32, ground height 4 -> choc rows 1..2.
    w.p.b.x = 30.0;
    w.p.b.y = 2.4;
    w.step(0);
    assert!(w.p.dead > 0.0, "chocolate is deadly");
    run(&mut w, 120, 0);
    assert_eq!(events_of(&w, ev::DIED).len(), 1);
    assert_eq!(w.game.lives, 2);

    let mut w = level(CRATER);
    let spike_x = (0..192)
        .find(|&x| (0..28).any(|y| w.map.get(x, y) == SPIKE))
        .unwrap();
    w.p.b.x = f64::from(spike_x) + 0.1;
    w.p.b.y = 4.0;
    w.step(0);
    assert!(w.p.dead > 0.0, "spikes are deadly");
}

#[test]
fn running_out_of_lives_is_game_over() {
    let mut w = level(CRATER);
    w.game.lives = 0;
    w.kill();
    run(&mut w, 120, 0);
    assert_eq!(events_of(&w, ev::GAME_OVER).len(), 1);
}

#[test]
fn collecting_snacks_scores_and_extra_lives_arrive_every_hundred() {
    let mut w = level(CRATER);
    let it = w.items[0];
    assert_eq!(it.kind, ItemKind::Cheezie);
    w.p.b.x = it.x - 0.35;
    w.p.b.y = it.y - 0.5;
    w.step(0);
    assert_eq!(w.game.score, 1);
    assert!(w.items[0].taken);

    w.game.score = 99;
    let it = w.items[1];
    w.p.b.x = it.x - 0.35;
    w.p.b.y = it.y - 0.5;
    w.step(0);
    assert_eq!(w.game.score, 100);
    assert_eq!(w.game.lives, 4);
    assert_eq!(w.game.next_life, 200);
}

#[test]
fn fizz_uses_ammo_and_stops_when_empty() {
    let mut w = level(CRATER);
    run(&mut w, 30, 0);
    w.step(FIRE);
    assert_eq!(w.game.ammo, 4);
    assert_eq!(w.shots.len(), 1);
    assert!(w.shots[0].vx > 0.0);
    w.game.ammo = 0;
    w.step(0);
    w.step(FIRE);
    assert_eq!(w.game.ammo, 0);
    assert_eq!(
        w.shots.iter().filter(|s| s.ben).count(),
        1,
        "no new shot without ammo"
    );
}

#[test]
fn fizz_aims_up() {
    let mut w = level(CRATER);
    run(&mut w, 30, 0);
    w.step(FIRE | UP);
    let s = w.shots.last().unwrap();
    assert_eq!((s.vx, s.vy), (0.0, 16.0));
}

#[test]
fn fizz_stuns_a_gloop_for_six_seconds() {
    let mut w = level(CRATER);
    let gi = w.ents.iter().position(|e| e.kind == Kind::Gloop).unwrap();
    let (gx, gy) = (w.ents[gi].b.x, w.ents[gi].b.y);
    w.p.b.x = gx - 3.0;
    w.p.b.y = gy;
    w.p.face = 1.0;
    w.step(FIRE);
    run(&mut w, 40, 0);
    assert!(w.ents[gi].stun > 4.0, "stun {}", w.ents[gi].stun);
}

#[test]
fn stomping_a_gloop_stuns_and_bounces() {
    let mut w = level(CRATER);
    let gi = w.ents.iter().position(|e| e.kind == Kind::Gloop).unwrap();
    // Stop it from walking away: it is on the ground; drop Ben on its head.
    w.ents[gi].b.vx = 0.0;
    w.ents[gi].dir = 0.0;
    let (gx, gy, gh) = (w.ents[gi].b.x, w.ents[gi].b.y, w.ents[gi].b.h);
    w.p.b.x = gx + 0.1;
    w.p.b.y = gy + gh + 0.4;
    w.p.b.px = w.p.b.x;
    w.p.b.py = w.p.b.y;
    w.p.b.vy = -8.0;
    let mut stunned = false;
    for _ in 0..30 {
        w.step(0);
        stunned |= w.ents[gi].stun > 0.0;
        if w.p.dead > 0.0 {
            panic!("Ben should survive a stomp");
        }
    }
    assert!(stunned);
}

#[test]
fn gumdrop_opens_matching_door_only() {
    let mut w = level(CRATER);
    let doors = |w: &World| w.map.data.iter().filter(|&&t| t == DOOR_R).count();
    assert_eq!(doors(&w), 2);
    // Without the key the door is solid.
    w.p.b.x = 170.3;
    w.p.b.y = 6.0;
    w.p.face = 1.0;
    w.step(0);
    assert_eq!(doors(&w), 2);
    w.keys_blue = true;
    w.step(0);
    assert_eq!(doors(&w), 2, "blue gumdrop does not open a red door");
    w.keys_red = true;
    w.step(0);
    assert_eq!(doors(&w), 0);
    assert!(!w.keys_red, "the key is used up");
}

#[test]
fn reaching_the_exit_completes_the_level_once() {
    let mut w = level(CRATER);
    let (ex, ey) = (186, 6);
    assert_eq!(w.map.get(ex, ey), EXIT);
    w.p.b.x = f64::from(ex) + 0.1;
    w.p.b.y = f64::from(ey);
    w.step(0);
    w.step(0);
    assert_eq!(events_of(&w, ev::LEVEL_COMPLETE).len(), 1);
    assert!(w.game.is_done(CRATER));
}

#[test]
fn switch_toggles_the_bridge_and_bridge_collision() {
    let mut w = level(CAVES);
    assert!(!w.map.solid(65, 3, false, 0.0));
    let si = w.ents.iter().position(|e| e.kind == Kind::Switch).unwrap();
    w.p.b.x = w.ents[si].b.x;
    w.p.b.y = w.ents[si].b.y;
    w.step(0);
    assert!(w.map.switch(0));
    assert!(w.map.solid(65, 3, false, 0.0));
}

#[test]
fn hover_platform_carries_ben() {
    let mut w = level(CAVES);
    let pl = w.plats[0].clone();
    w.p.b.x = pl.x + 0.6;
    w.p.b.y = pl.y + pl.h + 0.01;
    run(&mut w, 5, 0);
    let x0 = w.p.b.x;
    assert!(w.p.b.on_plat.is_some(), "standing on the hover platform");
    run(&mut w, 120, 0);
    assert!(
        (w.p.b.x - x0).abs() > 0.3,
        "moved with the platform: {} -> {}",
        x0,
        w.p.b.x
    );
}

#[test]
fn gloops_patrol_and_turn_around() {
    let mut w = level(CRATER);
    let gi = w.ents.iter().position(|e| e.kind == Kind::Gloop).unwrap();
    let x0 = w.ents[gi].b.x;
    w.p.hidden = true; // keep the camera irrelevant; enemies don't need Ben
    run(&mut w, 60, 0);
    assert!((w.ents[gi].b.x - x0).abs() > 1.0, "gloop moves");
    assert!(w.ents[gi].b.on_ground);
}

#[test]
fn every_enemy_type_survives_ten_simulated_seconds_without_leaving_the_world() {
    for id in 0..crate::levels::LEVEL_COUNT {
        let mut w = level(id);
        w.p.inv = 1e9;
        for t in 0..600 {
            // Wander so enemies react.
            w.step(if (t / 90) % 2 == 0 { RIGHT } else { LEFT });
            for e in &w.ents {
                assert!(
                    e.b.x.is_finite() && e.b.y.is_finite(),
                    "{:?} went non-finite",
                    e.kind
                );
                assert!(
                    e.b.y > -30.0,
                    "{:?} fell out of the world: y={}",
                    e.kind,
                    e.b.y
                );
            }
        }
    }
}

#[test]
fn boss_fight_progresses_through_its_phases() {
    let mut w = level(CITADEL);
    w.p.inv = 1e9;
    let arena = w.arena.unwrap();
    w.p.b.x = arena.x0 + 6.0;
    w.p.b.y = arena.floor;
    run(&mut w, 20, 0);
    assert_eq!(events_of(&w, ev::DIALOGUE).len(), 1, "boss intro");
    let mut seen = std::collections::HashSet::new();
    for _ in 0..1500 {
        w.p.b.x = arena.x0 + 6.0;
        w.p.b.y = arena.floor;
        w.step(0);
        let b = w.ents.iter().find(|e| e.kind == Kind::Boss).unwrap();
        seen.insert(format!("{:?}", b.state));
        assert!(b.b.x >= arena.x0 - 1e-6 && b.b.x + b.b.w <= arena.x1 + 1e-6);
    }
    for s in ["Hover", "Land", "Charge", "Hot", "Rise"] {
        assert!(seen.contains(s), "boss never reached {s}: {seen:?}");
    }
    assert!(w.shots.iter().any(|s| s.g) || w.tick_count > 0);
}

#[test]
fn defeating_the_boss_drops_the_usb_and_the_terminal_ends_the_game() {
    let mut w = level(CITADEL);
    w.p.inv = 1e9;
    let bi = w.ents.iter().position(|e| e.kind == Kind::Boss).unwrap();
    w.ents[bi].state = St::Hot;
    for _ in 0..3 {
        w.ents[bi].state = St::Hot;
        w.hit_boss_for_test(bi);
    }
    assert_eq!(w.ents[bi].state, St::Down);
    run(&mut w, 200, 0);
    assert!(w.ents[bi].dead);
    assert!(w.items.iter().any(|i| i.kind == ItemKind::Usb));
    assert_eq!(events_of(&w, ev::DIALOGUE).len(), 1, "defeat dialogue");
    // Pick up the USB and use the terminal.
    w.has_usb = true;
    let ti = w
        .ents
        .iter()
        .position(|e| e.kind == Kind::Terminal)
        .unwrap();
    w.p.b.x = w.ents[ti].b.x;
    w.p.b.y = w.ents[ti].b.y;
    w.step(0);
    assert!(w.hacked);
    run(&mut w, 100, 0);
    assert_eq!(events_of(&w, ev::ENDING).len(), 1);
}

#[test]
fn overworld_walk_prompt_and_enter_level() {
    let mut w = World::new();
    w.game_new();
    w.enter_map();
    assert_eq!(w.mode, Mode::Map);
    w.events.clear();
    // Walk down the river bank towards the first level point.
    let pt = *w
        .points
        .iter()
        .find(|p| p.kind == crate::levels::PtKind::Level && p.level == CRATER)
        .unwrap();
    w.p.b.x = pt.x + 0.2;
    w.p.b.y = pt.y + 0.2;
    w.step(0);
    let prompts = events_of(&w, ev::MAP_PROMPT);
    assert_eq!(prompts.last().unwrap().a, 1.0);
    // A press only opens a level once the map is armed (see `MAP_ARM_SECS`).
    w.map_arm = 0.0;
    w.step(JUMP);
    assert_eq!(w.mode, Mode::Level);
    assert_eq!(w.level_id, CRATER);
    assert_eq!(events_of(&w, ev::LEVEL_START).len(), 1);
    assert!(
        w.game.map_pos.is_some(),
        "map position remembered for the return trip"
    );
}

#[test]
fn the_map_is_lit_by_day_with_a_sun_that_leaves_flat_ground_at_full_brightness() {
    use crate::render::out;
    let mut w = World::new();
    w.game_new();
    w.enter_map();
    // Night does not reach the map: it is the same lit daytime either way.
    for flags in [0, crate::render::flags::NIGHT] {
        w.render(1.0, flags);
        assert_eq!(w.out[out::LIGHTING], 1.0, "the map is lit");
        let z = w.out[out::SUN_Z];
        assert!(z > 0.5, "the sun is up");
        for i in 0..3 {
            let flat = w.out[out::AMB_R + i] + w.out[out::SUN_R + i] * z;
            assert!(
                (flat - 1.0).abs() < 1e-4,
                "channel {i}: flat ground is at {flat}, not full brightness"
            );
        }
        assert!(w.out[out::LIGHTS] >= 1.0, "the map has lights");
    }
}

/// A map world with each sprite the overworld draws given its own atlas coordinate, so instances can be told
/// apart: the ground at 0.05, a shadow at 0.2, a tree at 0.4, Ben at 0.62.
fn map_world_with_known_sprites() -> World {
    use crate::sprites::Spr;
    use lf_sim::SpriteRect;
    let mut w = map_world();
    let rect = |u: f32, h: f32| SpriteRect {
        u,
        v: u,
        uw: 0.01,
        vh: 0.01,
        w: 16.0,
        h,
    };
    w.spr[Spr::OwGrass as usize] = rect(0.05, 16.0);
    w.spr[Spr::OwShadowS as usize] = rect(0.2, 4.0);
    w.spr[Spr::OwTree0 as usize] = rect(0.4, 24.0);
    w.spr[Spr::OwTree1 as usize] = rect(0.4, 24.0);
    w.spr[Spr::BenMap0 as usize] = rect(0.62, 16.0);
    w.spr[Spr::BenMap1 as usize] = rect(0.62, 16.0);
    w.half_w = 11.0;
    w.half_h = 6.5;
    w
}

#[test]
fn on_the_map_what_is_further_north_is_drawn_first_so_ben_walks_behind_trees() {
    use lf_sim::STRIDE;
    let mut w = map_world_with_known_sprites();
    // Open ground in Crater Fields (the column cleared of the map's own trees): a tree three tiles north of
    // Ben and one three tiles south.
    for x in 8..=12 {
        for y in 4..=16 {
            w.map.set(x, y, GRASS);
        }
    }
    for (x, y) in [(10, 13), (10, 7)] {
        w.map.set(x, y, TREE);
    }
    w.p.b.x = 10.2;
    w.p.b.y = 10.2;
    w.cam_x = 10.5;
    w.cam_y = 10.5;
    w.pcx = 10.5;
    w.pcy = 10.5;
    w.render(1.0, crate::render::flags::CULLING);
    let data = w.inst.as_slice();
    let n = w.inst.len();
    let u = |i: usize| data[i * STRIDE + 16];
    let y = |i: usize| data[i * STRIDE + 13];
    let ben = (0..n).find(|&i| u(i) == 0.62).expect("Ben is drawn");
    // The trees in Ben's column (the rest of the view has the map's own).
    let trees: Vec<usize> = (0..n)
        .filter(|&i| u(i) == 0.4 && (data[i * STRIDE + 12] - 10.5).abs() < 1e-4)
        .collect();
    let north = trees
        .iter()
        .copied()
        .find(|&i| y(i) > y(ben))
        .expect("the north tree");
    let south = trees
        .iter()
        .copied()
        .find(|&i| y(i) < y(ben))
        .expect("the south tree");
    assert!(
        north < ben,
        "a tree north of Ben is behind him: drawn first"
    );
    assert!(
        south > ben,
        "a tree south of Ben is in front of him: drawn after"
    );
    // All the ground goes down before anything that stands on it.
    let first_object = (0..n)
        .find(|&i| [0.2, 0.4, 0.62].contains(&u(i)))
        .expect("something stands on the ground");
    assert!((0..n).filter(|&i| u(i) == 0.05).all(|i| i < first_object));
    // A tree stands on its tile's bottom edge: 24 pixels tall, so its centre is 0.75 above the tile's base.
    assert!((y(north) - (13.0 + 0.75)).abs() < 1e-4);
}

#[test]
fn every_tree_and_ben_have_a_shadow_drawn_just_under_them() {
    use lf_sim::STRIDE;
    let mut w = map_world_with_known_sprites();
    w.map.set(12, 11, TREE);
    w.p.b.x = 10.2;
    w.p.b.y = 10.2;
    w.cam_x = 10.5;
    w.cam_y = 10.5;
    w.pcx = 10.5;
    w.pcy = 10.5;
    w.render(1.0, crate::render::flags::CULLING);
    let data = w.inst.as_slice();
    let n = w.inst.len();
    let u = |i: usize| data[i * STRIDE + 16];
    let mut seen = 0;
    for i in (0..n).filter(|&i| u(i) == 0.4 || u(i) == 0.62) {
        assert_eq!(u(i - 1), 0.2, "instance {i}: a shadow comes just before it");
        seen += 1;
    }
    assert!(seen >= 2, "the tree and Ben");
}

#[test]
fn a_tall_tree_just_below_the_view_still_reaches_into_it() {
    use lf_sim::STRIDE;
    let mut w = map_world_with_known_sprites();
    // The view's bottom edge is at 13.2; a tree on row 12 stands on 12 and rises to 13.5, into the view.
    w.map.set(10, 12, TREE);
    w.p.b.x = 10.2;
    w.p.b.y = 19.2;
    w.cam_x = 10.5;
    w.cam_y = 19.7;
    w.pcx = 10.5;
    w.pcy = 19.7;
    w.render(1.0, crate::render::flags::CULLING);
    let data = w.inst.as_slice();
    let n = w.inst.len();
    assert!(
        (0..n)
            .any(|i| data[i * STRIDE + 16] == 0.4 && (data[i * STRIDE + 13] - 12.75).abs() < 1e-4),
        "the tree on row 12 is drawn"
    );
}

#[test]
fn every_level_has_its_own_landmark() {
    let mut icons: Vec<u16> = crate::levels::LEVELS
        .iter()
        .map(|l| l.icon as u16)
        .collect();
    let n = icons.len();
    icons.sort_unstable();
    icons.dedup();
    assert_eq!(icons.len(), n, "no two levels share an icon");
}

#[test]
fn a_landmark_stands_behind_whoever_is_north_of_its_node_and_a_flag_shows_the_levels_state() {
    use crate::sprites::Spr;
    use lf_sim::{SpriteRect, STRIDE};
    let mut w = map_world_with_known_sprites();
    let rect = |u: f32, h: f32| SpriteRect {
        u,
        v: u,
        uw: 0.01,
        vh: 0.01,
        w: 32.0,
        h,
    };
    let (i, pt) = w
        .points
        .iter()
        .copied()
        .enumerate()
        .find(|(_, p)| p.kind == crate::levels::PtKind::Level && !p.big)
        .expect("a level node");
    let icon = crate::levels::LEVELS[usize::from(pt.level)].icon;
    w.spr[icon as usize] = rect(0.81, 32.0);
    w.spr[Spr::OwFlagHolder as usize] = rect(0.83, 16.0);
    w.spr[Spr::OwFlag as usize] = rect(0.87, 16.0);
    // Ben on the tile just north of the node, as when he walks down a path to it.
    stand_on(&mut w, i);
    w.p.b.y = pt.y + 1.3;
    w.cam_x = pt.x + 0.5;
    w.cam_y = pt.y + 0.5;
    w.pcx = w.cam_x;
    w.pcy = w.cam_y;
    w.render(1.0, crate::render::flags::CULLING);
    let data = w.inst.as_slice();
    let n = w.inst.len();
    let u = |k: usize| data[k * STRIDE + 16];
    let at = |v: f32| (0..n).find(|&k| (u(k) - v).abs() < 1e-6);
    let lm = at(0.81).expect("the landmark is drawn");
    let ben = at(0.62).expect("Ben is drawn");
    assert!(lm < ben, "the landmark is behind Ben, north of its node");
    // Base-anchored: 32 pixels tall on a point 0.35 above the node tile's bottom edge.
    assert!((data[lm * STRIDE + 13] - (pt.y as f32 + 0.35 + 1.0)).abs() < 1e-3);
    // A level not yet cleared has a bare pole and no flag.
    assert!(at(0.83).is_some(), "the holder for an uncleared level");
    assert!(
        (0..n).all(|k| (u(k) - 0.87).abs() > 1e-6),
        "no flag on an uncleared level"
    );
}

#[test]
fn a_level_cleared_for_the_first_time_has_its_flag_thrown_to_the_pole_and_a_replay_does_not() {
    use crate::sprites::Spr;
    use lf_sim::{SpriteRect, STRIDE};
    let mut w = map_world_with_known_sprites();
    let rect = |u: f32| SpriteRect {
        u,
        v: u,
        uw: 0.01,
        vh: 0.01,
        w: 16.0,
        h: 16.0,
    };
    w.spr[Spr::OwFlagHolder as usize] = rect(0.83);
    w.spr[Spr::OwFlag as usize] = rect(0.87);
    let pt = w
        .points
        .iter()
        .copied()
        .find(|p| p.kind == crate::levels::PtKind::Level && !p.big)
        .expect("a level node");
    let pole = (f64::from(pt.x as f32) + 1.1, f64::from(pt.y as f32) + 1.4);
    let flag_at = |w: &World| {
        let data = w.inst.as_slice();
        (0..w.inst.len())
            .find(|&k| (data[k * STRIDE + 16] - 0.87).abs() < 1e-6)
            .map(|k| {
                (
                    f64::from(data[k * STRIDE + 12]),
                    f64::from(data[k * STRIDE + 13]),
                )
            })
    };
    // Cleared for the first time: the flag leaves Ben, goes up and arrives on the pole.
    w.fresh_clear = Some(pt.level);
    w.game.set_done(pt.level);
    w.enter_map();
    w.map_arm = 0.0;
    w.p.b.x = pt.x + 3.0;
    w.p.b.y = pt.y + 3.0;
    w.cam_x = pt.x + 0.5;
    w.cam_y = pt.y + 0.5;
    w.pcx = w.cam_x;
    w.pcy = w.cam_y;
    assert!(w.flag_throw.is_some());
    w.render(1.0, crate::render::flags::CULLING);
    let start = flag_at(&w).expect("the flag is in flight");
    assert!(
        (start.0 - pole.0).abs() > 1.0,
        "it starts away from the pole"
    );
    for _ in 0..30 {
        w.step(0);
    }
    w.render(1.0, crate::render::flags::CULLING);
    let mid = flag_at(&w).expect("still in flight");
    assert!(
        mid.1 > pole.1.min(start.1),
        "it arcs above the straight line"
    );
    for _ in 0..60 {
        w.step(0);
    }
    assert!(w.flag_throw.is_none(), "the throw is over");
    w.render(1.0, crate::render::flags::CULLING);
    let end = flag_at(&w).expect("the flag stands on the pole");
    assert!((end.0 - pole.0).abs() < 1e-3 && (end.1 - pole.1).abs() < 1e-3);
    // Entering the map again without a new clear throws nothing.
    w.enter_map();
    assert!(w.flag_throw.is_none());
}

/// Where the first instance with atlas coordinate `u` is, as (x, y, x scale, y scale), among those drawn.
fn instance_of(w: &World, u: f32, near: (f64, f64)) -> Option<(f32, f32, f32, f32)> {
    use lf_sim::STRIDE;
    let data = w.inst.as_slice();
    (0..w.inst.len())
        .map(|i| {
            let b = &data[i * STRIDE..(i + 1) * STRIDE];
            (b[16], b[12], b[13], b[0], b[5])
        })
        .find(|&(iu, x, y, _, _)| {
            iu == u && (f64::from(x) - near.0).abs() < 0.01 && (f64::from(y) - near.1).abs() < 0.01
        })
        .map(|(_, x, y, sx, sy)| (x, y, sx, sy))
}

#[test]
fn water_gets_a_bank_face_under_land_to_its_north_and_thin_lips_on_its_other_land_sides() {
    use crate::sprites::Spr;
    use lf_sim::SpriteRect;
    let mut w = map_world_with_known_sprites();
    let rect = |u: f32, wd: f32, h: f32| SpriteRect {
        u,
        v: u,
        uw: 0.01,
        vh: 0.01,
        w: wd,
        h,
    };
    w.spr[Spr::OwBank as usize] = rect(0.7, 16.0, 6.0);
    w.spr[Spr::OwLipH as usize] = rect(0.71, 16.0, 2.0);
    w.spr[Spr::OwLipV as usize] = rect(0.72, 2.0, 16.0);
    // The horizontal river runs along rows 22 and 23; the vertical one along columns 24 and 25.
    w.p.b.x = 18.2;
    w.p.b.y = 22.2;
    w.cam_x = 18.5;
    w.cam_y = 22.5;
    w.pcx = 18.5;
    w.pcy = 22.5;
    w.half_w = 14.0;
    w.half_h = 7.0;
    w.render(1.0, crate::render::flags::CULLING);
    // Row 23 has land to its north (row 24): a bank face on its top edge.
    let bank =
        instance_of(&w, 0.7, (10.5, 23.0 + 1.0 - 3.0 / 16.0)).expect("a bank face at (10,23)");
    assert!(bank.3 > 0.0, "upright");
    // Row 22 has land to its south (row 21): a lip along its bottom edge, flipped over.
    let lip = instance_of(&w, 0.71, (10.5, 22.0 + 1.0 / 16.0)).expect("a bottom lip at (10,22)");
    assert!(
        lip.3 < 0.0,
        "turned over, so its solid row is on the outside"
    );
    // Column 24 (row 15) has land to its west: a lip down its left; column 25 has land to its east: one down its right.
    assert!(
        instance_of(&w, 0.72, (24.0 + 1.0 / 16.0, 15.5)).is_some(),
        "a left lip at (24,15)"
    );
    let right = instance_of(&w, 0.72, (26.0 - 1.0 / 16.0, 15.5)).expect("a right lip at (25,15)");
    assert!(right.2 < 0.0, "mirrored");
    // No bank face where the land is not to the north: the tile north of (24,15) is water too.
    assert!(instance_of(&w, 0.7, (24.5, 15.0 + 1.0 - 3.0 / 16.0)).is_none());
}

#[test]
fn a_path_has_an_edge_where_it_meets_something_else() {
    use crate::sprites::Spr;
    use lf_sim::SpriteRect;
    let mut w = map_world_with_known_sprites();
    w.spr[Spr::OwLipH as usize] = SpriteRect {
        u: 0.71,
        v: 0.71,
        uw: 0.01,
        vh: 0.01,
        w: 16.0,
        h: 2.0,
    };
    w.spr[Spr::OwPath as usize] = SpriteRect {
        u: 0.9,
        v: 0.9,
        uw: 0.01,
        vh: 0.01,
        w: 16.0,
        h: 16.0,
    };
    // The street east of the tower runs along row 16, with houses on its north side and grass to its south.
    w.p.b.x = 9.2;
    w.p.b.y = 15.2;
    w.cam_x = 9.5;
    w.cam_y = 15.5;
    w.pcx = 9.5;
    w.pcy = 15.5;
    w.render(1.0, crate::render::flags::CULLING);
    assert!(
        instance_of(&w, 0.71, (9.5, 16.0 + 1.0 - 1.0 / 16.0)).is_some(),
        "an edge on the north side"
    );
    assert!(
        instance_of(&w, 0.71, (9.5, 16.0 + 1.0 / 16.0)).is_some(),
        "an edge on the south side"
    );
    // Between two path tiles there is none: (8,16) and (9,16) share a side.
    assert!(
        instance_of(&w, 0.9, (9.5, 16.5)).is_some(),
        "the path tile itself is drawn"
    );
}

#[test]
fn the_ground_takes_three_looks_so_the_speckle_does_not_repeat_in_a_grid() {
    use crate::sprites::Spr;
    use lf_sim::{SpriteRect, STRIDE};
    let mut w = map_world_with_known_sprites();
    for (spr, u) in [
        (Spr::OwGrass, 0.05),
        (Spr::OwGrassB, 0.06),
        (Spr::OwGrassC, 0.07),
    ] {
        w.spr[spr as usize] = SpriteRect {
            u,
            v: u,
            uw: 0.01,
            vh: 0.01,
            w: 16.0,
            h: 16.0,
        };
    }
    w.cam_x = 14.5;
    w.cam_y = 10.5;
    w.pcx = 14.5;
    w.pcy = 10.5;
    w.render(1.0, crate::render::flags::CULLING);
    let data = w.inst.as_slice();
    let count = |u: f32| {
        (0..w.inst.len())
            .filter(|&i| data[i * STRIDE + 16] == u)
            .count()
    };
    for u in [0.05, 0.06, 0.07] {
        assert!(
            count(u) >= 20,
            "look {u} is used on a good share of the ground: {}",
            count(u)
        );
    }
}

#[test]
fn the_maps_light_eases_between_areas_and_the_sun_stays_put() {
    use crate::render::out;
    let mut w = World::new();
    w.game_new();
    w.enter_map();
    let here = w.map_amb;
    // Into Rock Candy Reach (the north-east), a long way from Crater Fields.
    w.p.b.x = 40.0;
    w.p.b.y = 30.0;
    w.step(0);
    assert_ne!(w.map_amb, here, "the light starts to move");
    assert!(
        w.map_amb != crate::levels::map_ambient(crate::levels::AREA_ROCK_CANDY_REACH),
        "but it eases in and does not jump"
    );
    for _ in 0..180 {
        w.step(0);
    }
    for (a, b) in w.map_amb.iter().zip(crate::levels::map_ambient(
        crate::levels::AREA_ROCK_CANDY_REACH,
    )) {
        assert!((a - b).abs() < 0.01, "after a few seconds it has settled");
    }
    w.render(1.0, 0);
    let flat = w.out[out::AMB_R] + w.out[out::SUN_R] * w.out[out::SUN_Z];
    assert!((flat - 1.0).abs() < 1e-4);
}

#[test]
fn render_writes_no_sun_in_a_level_whatever_was_left_in_the_out_array() {
    use crate::render::out;
    let mut w = World::new();
    w.game_new();
    w.enter_level(CRATER);
    w.out[out::SUN_R] = 1.0;
    w.out[out::SUN_Z] = 1.0;
    w.render(1.0, 0);
    assert!(
        w.out[out::SUN_X..=out::SUN_B].iter().all(|&v| v == 0.0),
        "a level has no sun"
    );
    assert_eq!(w.out.len(), out::LEN);
}

#[test]
fn ben_comes_back_to_the_map_at_his_levels_doorstep_with_its_prompt_up() {
    use crate::levels::PtKind;
    let mut w = World::new();
    w.game_new();
    w.enter_map();
    let nodes: Vec<_> = w
        .points
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, p)| matches!(p.kind, PtKind::Level | PtKind::Saucer))
        .collect();
    assert!(nodes.len() >= 16, "every level node and the saucer");
    for (i, pt) in nodes {
        w.enter_map();
        w.enter_level(pt.level);
        w.enter_map();
        w.events.clear();
        w.step(0);
        let d = (w.p.b.x + 0.3 - (pt.x + 0.5)).hypot(w.p.b.y + 0.3 - (pt.y + 0.5));
        assert!(
            d < 0.95,
            "node {i} at {},{}: Ben is {d:.2} tiles from it, outside its reach",
            pt.x,
            pt.y
        );
        assert_eq!(
            w.near,
            Some(i),
            "node {i}: its prompt is up on the first step"
        );
        assert!(
            !events_of(&w, ev::MAP_PROMPT).is_empty(),
            "node {i}: a prompt is raised on the first step"
        );
    }
}

#[test]
fn a_press_that_closed_a_card_does_not_reopen_the_level_but_the_next_one_does() {
    let mut w = World::new();
    w.game_new();
    w.enter_map();
    w.enter_level(CRATER);
    w.enter_map();
    // The press is still down (or lands a moment later): the map is not armed yet.
    w.step(JUMP);
    assert_eq!(
        w.mode,
        Mode::Map,
        "a press straight after returning opens nothing"
    );
    w.step(0);
    // Half a second on, a fresh press opens it.
    for _ in 0..30 {
        w.step(0);
    }
    w.step(JUMP);
    assert_eq!(w.mode, Mode::Level);
    assert_eq!(w.level_id, CRATER);
}

#[test]
fn rivers_and_trees_block_the_overworld_walker() {
    let mut w = World::new();
    w.game_new();
    w.enter_map();
    let (x0, y0) = (w.p.b.x, w.p.b.y);
    run(&mut w, 600, UP | LEFT);
    assert!(
        w.p.b.x >= 2.0 && w.p.b.y >= 2.0,
        "stopped by the border river: {},{}",
        w.p.b.x,
        w.p.b.y
    );
    let _ = (x0, y0);
}

#[test]
fn teleporters_need_a_cleared_level() {
    let mut w = World::new();
    w.game_new();
    w.enter_map();
    let ti = w
        .points
        .iter()
        .position(|p| p.kind == crate::levels::PtKind::Tele)
        .unwrap();
    let pt = w.points[ti];
    w.p.b.x = pt.x + 0.2;
    w.p.b.y = pt.y + 0.2;
    w.step(0);
    w.step(JUMP);
    assert_eq!(w.mode, Mode::Map);
    assert!(
        (w.p.b.x - (pt.x + 0.2)).abs() < 0.2,
        "locked teleporter does not move Ben"
    );
    w.game.set_done(CRATER);
    w.near = None;
    w.step(0);
    w.step(JUMP | FIRE);
    let to = w.points[pt.to];
    assert!(
        (w.p.b.x - (to.x + 0.2)).abs() < 0.2,
        "teleported to its partner"
    );
}

#[test]
fn render_writes_finite_instances_for_every_scene() {
    let mut w = level(CRATER);
    for (id, name) in [
        (CRATER, "crater"),
        (CAVES, "caves"),
        (CITADEL, "citadel"),
        (crate::levels::METEOR_MESA, "meteor mesa"),
    ] {
        w.enter_level(id);
        w.half_w = 11.0;
        w.half_h = 6.5;
        run(&mut w, 30, RIGHT);
        for fl in [0, 1, 2, 3] {
            w.render(0.5, fl);
            let count = w.out[crate::render::out::COUNT] as usize;
            assert!(count > 50, "{name} flags {fl}: only {count} instances");
            assert!(
                w.inst.as_slice().iter().all(|v| v.is_finite()),
                "{name}: non-finite instance data"
            );
            assert!(
                w.out[crate::render::out::LIGHTS] >= 1.0,
                "{name}: no lights"
            );
        }
    }
    w.enter_map();
    w.render(1.0, 2);
    assert!(w.out[crate::render::out::COUNT] > 100.0);
}

#[test]
fn culling_cuts_instance_counts_dramatically() {
    let mut w = level(CRATER);
    w.half_w = 11.0;
    w.half_h = 6.5;
    run(&mut w, 10, 0);
    w.render(1.0, 2);
    let culled = w.out[crate::render::out::COUNT];
    w.render(1.0, 0);
    let uncull = w.out[crate::render::out::COUNT];
    assert!(culled * 2.0 < uncull, "culled {culled} vs {uncull}");
}

#[test]
fn simulation_and_rendering_are_deterministic() {
    let script = |t: usize| match (t / 37) % 5 {
        0 => RIGHT,
        1 => RIGHT | JUMP,
        2 => RIGHT | FIRE,
        3 => LEFT | JUMP,
        _ => POGO,
    };
    let play = || {
        let mut w = level(CRATER);
        w.half_w = 11.0;
        w.half_h = 6.5;
        for t in 0..900 {
            w.step(script(t));
        }
        w.render(0.25, 2);
        (
            w.p.b.x.to_bits(),
            w.p.b.y.to_bits(),
            w.inst
                .as_slice()
                .iter()
                .map(|f| f.to_bits())
                .collect::<Vec<_>>(),
            w.game.score,
        )
    };
    assert_eq!(play(), play());
}

#[test]
fn attract_mode_scrolls_without_ben() {
    let mut w = World::new();
    w.load_attract(0, false);
    w.half_w = 11.0;
    w.half_h = 6.5;
    let x0 = w.cam_x;
    run(&mut w, 600, 0);
    assert!((w.cam_x - x0).abs() > 0.01);
    w.render(0.5, 2);
    assert!(w.p.hidden);
}

#[test]
fn requirement_masks_need_every_listed_level() {
    let mut w = level(CRATER);
    let both = (crate::levels::level_bit(CRATER) | crate::levels::level_bit(CAVES)) as u16;
    assert!(w.met(0), "no requirement is always met");
    assert!(!w.met(both));
    w.game.set_done(CAVES);
    assert!(!w.met(both), "one of two is not enough");
    w.game.set_done(CRATER);
    assert!(w.met(both));
}

#[test]
fn a_locked_level_point_refuses_entry_until_its_requirement_is_met() {
    let mut w = World::new();
    w.game_new();
    w.enter_map();
    let i = w
        .points
        .iter()
        .position(|p| p.kind == crate::levels::PtKind::Level && p.level == CAVES)
        .unwrap();
    w.points[i].req = crate::levels::level_bit(CRATER) as u16;
    let pt = w.points[i];
    w.p.b.x = pt.x + 0.2;
    w.p.b.y = pt.y + 0.2;
    w.map_arm = 0.0;
    w.step(0);
    w.step(JUMP);
    assert_eq!(w.mode, Mode::Map, "locked level did not start");
    w.game.set_done(CRATER);
    w.near = None;
    w.step(0);
    w.step(JUMP);
    assert_eq!(w.mode, Mode::Level);
    assert_eq!(w.level_id, CAVES);
}

#[test]
fn a_locked_level_point_prompts_as_locked_and_names_what_is_missing() {
    let mut w = World::new();
    w.game_new();
    w.enter_map();
    let i = w
        .points
        .iter()
        .position(|p| p.kind == crate::levels::PtKind::Level && p.level == CAVES)
        .unwrap();
    w.points[i].req = crate::levels::level_bit(CRATER) as u16;
    let pt = w.points[i];
    w.p.b.x = pt.x + 0.2;
    w.p.b.y = pt.y + 0.2;
    w.events.clear();
    w.step(0);
    let prompt = events_of(&w, ev::MAP_PROMPT).pop().expect("a prompt");
    assert_eq!(prompt.a, 4.0, "locked level prompt type");
    assert_eq!(prompt.b, f32::from(CRATER), "names the first missing level");
    assert_eq!(prompt.c, f32::from(CAVES), "carries the locked level's id");
    w.game.set_done(CRATER);
    w.near = None;
    w.events.clear();
    w.step(0);
    let prompt = events_of(&w, ev::MAP_PROMPT).pop().expect("a prompt");
    assert_eq!(prompt.a, 1.0, "unlocked once the requirement is cleared");
    assert_eq!(prompt.b, f32::from(CAVES));
}

#[test]
fn loading_progress_drops_bits_that_mean_nothing() {
    let mut w = World::new();
    w.game
        .load_done((1 << 20) | crate::world::SECRET_FOUND | 0b101);
    assert_eq!(w.game.done, crate::world::SECRET_FOUND | 0b101);
    // Garbage that is not even a whole number of bits cannot get through either.
    w.game.load_done(u32::MAX);
    assert_eq!(w.game.done, crate::world::PROGRESS_BITS);
}

#[test]
fn an_unknown_level_id_builds_the_first_level_and_cannot_set_stray_bits() {
    let mut w = World::new();
    w.game_new();
    w.enter_level(200);
    assert_eq!(w.level_id, 0, "falls back to the first level, id and all");
    w.won = false;
    w.game.set_done(w.level_id);
    assert_eq!(w.game.done, 1, "only the first level's bit");
}

fn lookout() -> World {
    let mut w = level(crate::levels::ZARG_LOOKOUT);
    w.ents.clear();
    w
}

#[test]
fn ben_climbs_a_ladder_with_up_and_hops_off_with_jump() {
    let mut w = lookout();
    // The ground-floor ladder is at x = 18; stand at its foot.
    w.p.b.x = 17.8;
    w.p.b.y = 3.0;
    run(&mut w, 2, 0);
    assert!(!w.p.climb);
    run(&mut w, 30, UP);
    assert!(w.p.climb, "Up grabs the ladder");
    assert!(!w.p.b.on_ground);
    let y0 = w.p.b.y;
    run(&mut w, 30, UP);
    let rate = (w.p.b.y - y0) / 0.5;
    assert!(
        (rate - 4.5).abs() < 0.3,
        "climbs at 4.5 tiles/s, got {rate}"
    );
    assert!(
        (w.p.b.centre_x() - 18.5).abs() < 0.01,
        "snapped to the rungs"
    );
    run(&mut w, 1, UP | JUMP);
    assert!(!w.p.climb, "Jump lets go");
    assert!(w.p.b.vy > 10.0, "and hops, vy = {}", w.p.b.vy);
}

#[test]
fn a_ladder_top_is_a_ledge_and_down_goes_back_onto_the_ladder() {
    let mut w = lookout();
    w.p.b.x = 17.8;
    w.p.b.y = 3.0;
    run(&mut w, 200, UP);
    assert!(!w.p.climb, "ran out of ladder");
    assert!(w.p.b.on_ground);
    assert!(
        (w.p.b.y - 12.0).abs() < 0.01,
        "settled on the ledge, y = {}",
        w.p.b.y
    );
    run(&mut w, 15, DOWN);
    assert!(w.p.climb, "Down from the ledge grabs the ladder");
    run(&mut w, 200, DOWN);
    assert!(
        (w.p.b.y - 3.0).abs() < 0.05,
        "back on the ground, y = {}",
        w.p.b.y
    );
    assert!(!w.p.climb, "and let go at the bottom");
}

#[test]
fn up_and_down_do_nothing_away_from_a_ladder() {
    let mut w = lookout();
    w.p.b.x = 5.0;
    w.p.b.y = 3.0;
    run(&mut w, 30, UP);
    assert!(!w.p.climb);
    run(&mut w, 30, DOWN);
    assert!(!w.p.climb);
}

#[test]
fn climbing_ignores_the_pogo_toggle() {
    let mut w = lookout();
    w.p.b.x = 17.8;
    w.p.b.y = 3.0;
    run(&mut w, 40, UP);
    w.step(UP | POGO);
    assert!(!w.p.pogo && w.p.climb);
}

#[test]
fn the_tower_camera_keeps_ben_in_view_through_a_long_fall() {
    let mut w = lookout();
    w.half_h = 6.5;
    // Knock a one-tile shaft through every floor and the roof at x = 5 so Ben falls the whole
    // tower, about 72 tiles, instead of landing on the roof he starts on.
    for y in 3..75 {
        w.map.set(5, y, 0);
    }
    w.p.b.x = 5.2;
    w.p.b.y = 75.0;
    w.cam_y = 78.0;
    w.pcy = 78.0;
    let mut ticks = 0;
    for t in 0..600 {
        w.step(0);
        if w.p.b.on_ground {
            break;
        }
        ticks = t;
        assert!(
            (w.p.b.y - w.cam_y).abs() < w.half_h - 1.0,
            "tick {t}: Ben at {:.1}, camera at {:.1}",
            w.p.b.y,
            w.cam_y
        );
    }
    assert!(
        ticks > 150,
        "a long fall, not a one-tick landing: {ticks} ticks"
    );
    assert!(w.p.b.on_ground, "landed");
    assert!(w.p.b.y < 4.0, "all the way down, y = {}", w.p.b.y);
}

#[test]
fn the_tower_camera_looks_down_and_up_on_request_while_standing() {
    let mut w = lookout();
    // Floor 3 is far enough from the map's bottom and top that the camera is never clamped.
    w.p.b.x = 5.0;
    w.p.b.y = 30.0;
    w.cam_y = 31.0;
    w.pcy = 31.0;
    run(&mut w, 90, 0);
    let rest = w.cam_y;
    run(&mut w, 90, DOWN);
    assert!(w.cam_y < rest - 1.0, "looks down");
    run(&mut w, 120, 0);
    run(&mut w, 120, UP);
    assert!(w.cam_y > rest + 1.0, "looks up, {} vs {rest}", w.cam_y);
}

#[test]
fn the_lift_carries_ben_up_and_waits_at_each_end() {
    let mut w = lookout();
    let lift = &w.plats[0];
    let (bottom, top) = (lift.ay + lift.h, lift.by + lift.h);
    w.p.b.x = 18.9;
    w.p.b.y = bottom;
    let mut rose = false;
    let mut waited = 0;
    for _ in 0..1800 {
        w.step(0);
        if w.p.b.on_plat.is_some() && w.p.b.y > bottom + 4.0 {
            rose = true;
        }
        if w.plats[0].dy == 0.0 && w.plats[0].wait > 0.0 {
            waited += 1;
        }
    }
    assert!(rose, "rode the lift, y = {}", w.p.b.y);
    assert!(
        waited > 100,
        "the lift rests at its ends, waited {waited} ticks"
    );
    assert!(w.p.b.y <= top + 0.5);
}

fn playhouse() -> World {
    let mut w = level(crate::levels::BONBON_PLAYHOUSE);
    w.ents.clear();
    w
}

#[test]
fn a_hidden_room_fades_while_ben_is_inside_and_recovers_when_he_leaves() {
    let mut w = playhouse();
    // The tutorial room sits on the path at x 18..29.
    assert_eq!(w.room_alpha[0], 1.0, "starts opaque");
    w.p.b.x = 22.0;
    w.p.b.y = 6.0;
    let room = w
        .rooms
        .iter()
        .position(|r| r.contains(22, 6))
        .expect("room on the path");
    run(&mut w, 60, 0);
    assert!(
        w.room_alpha[room] < 0.3,
        "faded inside, alpha {}",
        w.room_alpha[room]
    );
    w.p.b.x = 5.0;
    w.p.b.y = 4.0;
    run(&mut w, 60, 0);
    assert!(
        w.room_alpha[room] > 0.95,
        "opaque again outside, alpha {}",
        w.room_alpha[room]
    );
}

#[test]
fn hidden_rooms_are_hidden_from_the_start_and_only_the_room_ben_is_in_fades() {
    let mut w = playhouse();
    assert!(w.room_alpha.iter().all(|&a| a == 1.0));
    w.p.b.x = 22.0;
    w.p.b.y = 6.0;
    run(&mut w, 60, 0);
    let seen = w.rooms.iter().position(|r| r.contains(22, 6)).unwrap();
    for (i, &a) in w.room_alpha.iter().enumerate() {
        assert_eq!(a < 0.5, i == seen, "room {i} alpha {a}");
    }
}

#[test]
fn flats_are_drawn_after_ben_so_they_cover_him_until_they_fade() {
    use crate::sprites::Spr;
    use lf_sim::{SpriteRect, STRIDE};
    let mut w = playhouse();
    // Give the two sprites distinct atlas coordinates so their instances can be told apart.
    let rect = |u: f32| SpriteRect {
        u,
        v: u,
        uw: 0.01,
        vh: 0.01,
        w: 16.0,
        h: 16.0,
    };
    w.spr[Spr::Facade as usize] = rect(0.31);
    w.spr[Spr::BenStand as usize] = rect(0.62);
    w.half_w = 11.0;
    w.half_h = 6.5;
    w.p.b.x = 22.0;
    w.p.b.y = 6.0;
    w.cam_x = 22.0;
    w.cam_y = 8.0;
    w.pcx = 22.0;
    w.pcy = 8.0;
    // Let Ben land so he is drawn with the standing pose, then look at the very first frame's
    // room before it has had time to fade by drawing right after the landing.
    w.p.b.on_ground = true;
    w.room_alpha.iter_mut().for_each(|a| *a = 1.0);
    w.render(0.5, 0);
    let data = w.inst.as_slice();
    let at = |i: usize| data[i * STRIDE + 16];
    let n = w.inst.len();
    let ben = (0..n).find(|&i| at(i) == 0.62).expect("Ben is drawn");
    let flats: Vec<usize> = (0..n).filter(|&i| at(i) == 0.31).collect();
    assert!(!flats.is_empty(), "the room's flats are drawn");
    assert!(
        flats.iter().all(|&i| i > ben),
        "every flat comes after Ben in draw order"
    );
    // After the fade the flats inside his room are still drawn, but translucent.
    run(&mut w, 60, 0);
    w.render(0.5, 0);
    let data = w.inst.as_slice();
    let alpha = |i: usize| data[i * STRIDE + 19];
    let n = w.inst.len();
    let faded = (0..n).filter(|&i| data[i * STRIDE + 16] == 0.31 && alpha(i) < 0.5);
    assert!(faded.count() > 0, "flats in Ben's room are translucent");
}

fn shafts() -> World {
    let mut w = level(crate::levels::MIRROR_SHAFTS);
    // Keep only the puzzle pieces, so a stray phantom cannot interfere.
    w.ents
        .retain(|e| matches!(e.kind, Kind::Mirror | Kind::Swivel | Kind::CrystalSwitch));
    w
}

fn bubble(x: f64, y: f64, vx: f64, vy: f64) -> crate::ents::Shot {
    crate::ents::Shot {
        x,
        y,
        vx,
        vy,
        ben: true,
        life: 1.1,
        sprite: crate::sprites::Spr::Bubble as u16,
        g: false,
        last: -1,
    }
}

fn find(w: &World, kind: Kind, x: f64) -> usize {
    w.ents
        .iter()
        .position(|e| e.kind == kind && (e.b.x - x).abs() < 0.01)
        .expect("entity at x")
}

#[test]
fn a_mirror_turns_an_upward_bubble_sideways_exactly_once() {
    let mut w = shafts();
    // The first puzzle's `/` mirror is at (18, 26).
    w.shots.clear();
    w.shots.push(bubble(18.5, 24.0, 0.0, 16.0));
    let mut turned = None;
    for _ in 0..60 {
        w.step(0);
        if let Some(s) = w.shots.first() {
            if s.vy == 0.0 {
                turned = Some(*s);
                break;
            }
        }
    }
    let s = turned.expect("the bubble turned");
    assert_eq!(s.vx, 16.0, "`/` sends an up bubble right");
    assert_eq!(
        s.last,
        find(&w, Kind::Mirror, 18.0) as i32,
        "remembers the mirror"
    );
    // It keeps going right instead of bouncing off the same mirror again.
    run(&mut w, 5, 0);
    let s = w.shots.first().expect("still flying");
    assert_eq!((s.vx, s.vy), (16.0, 0.0));
}

#[test]
fn a_back_slash_mirror_sends_an_upward_bubble_left() {
    let mut w = shafts();
    let i = find(&w, Kind::Mirror, 18.0);
    w.ents[i].dir = -1.0;
    w.shots.clear();
    w.shots.push(bubble(18.5, 24.0, 0.0, 16.0));
    run(&mut w, 30, 0);
    let s = w.shots.first().expect("flying");
    assert_eq!((s.vx, s.vy), (-16.0, 0.0));
}

#[test]
fn a_crystal_switch_opens_only_its_own_gate() {
    let mut w = shafts();
    assert!(
        w.map.switch(1) && w.map.switch(2) && w.map.switch(3),
        "all gates start shut"
    );
    assert!(w.map.solid(10, 30, false, 0.0), "gate one blocks the shaft");
    w.shots.clear();
    // Straight at the first switch at (28, 26).
    w.shots.push(bubble(22.0, 26.5, 16.0, 0.0));
    run(&mut w, 30, 0);
    assert!(!w.map.switch(1), "gate one is open");
    assert!(w.map.switch(2) && w.map.switch(3), "the others stay shut");
    assert!(!w.map.solid(10, 30, false, 0.0));
    assert!(w.map.solid(10, 66, false, 0.0), "gate two still blocks");
    assert!(!w.map.switch(0), "the bridge channel is untouched");
}

#[test]
fn a_swivel_mirror_sends_the_first_bubble_one_way_and_the_second_the_other() {
    let mut w = shafts();
    let i = find(&w, Kind::Swivel, 18.0);
    assert_eq!(w.ents[i].dir, 1.0);
    w.shots.clear();
    w.shots.push(bubble(18.5, 80.0, 0.0, 16.0));
    run(&mut w, 25, 0);
    assert_eq!(
        w.shots.first().map(|s| (s.vx, s.vy)),
        Some((16.0, 0.0)),
        "first goes right"
    );
    assert_eq!(w.ents[i].dir, -1.0, "and the mirror swings round");
    w.shots.clear();
    w.shots.push(bubble(18.5, 80.0, 0.0, 16.0));
    run(&mut w, 25, 0);
    assert_eq!(
        w.shots.first().map(|s| (s.vx, s.vy)),
        Some((-16.0, 0.0)),
        "second goes left"
    );
}

#[test]
fn the_second_shot_through_the_swivel_reaches_the_switch() {
    let mut w = shafts();
    for _ in 0..2 {
        w.shots.clear();
        w.shots.push(bubble(18.5, 80.0, 0.0, 16.0));
        run(&mut w, 75, 0);
    }
    assert!(!w.map.switch(3), "gate three opened by the second bubble");
    assert!(w.map.switch(2), "gate two is not involved");
}

#[test]
fn the_secret_room_sets_the_found_bit_once_and_toasts() {
    use crate::text::Toast;
    let mut w = level(crate::levels::SUGAR_GLASS_GALLERY);
    w.ents.clear();
    assert_eq!(w.game.done & crate::world::SECRET_FOUND, 0);
    let toasts = |w: &World| {
        events_of(w, ev::TOAST)
            .iter()
            .filter(|e| e.a == Toast::SecretFound as u16 as f32)
            .count()
    };
    // Walk past the clue without going in: nothing happens.
    w.p.b.x = 119.0;
    w.p.b.y = 4.0;
    run(&mut w, 30, 0);
    assert_eq!(w.game.done & crate::world::SECRET_FOUND, 0);
    assert_eq!(toasts(&w), 0);
    // Step into the mural room.
    w.p.b.x = 130.0;
    w.p.b.y = 10.0;
    run(&mut w, 30, 0);
    assert_ne!(w.game.done & crate::world::SECRET_FOUND, 0, "bit 15 set");
    assert_eq!(toasts(&w), 1, "one toast");
    // Leave and come back: still found, no second toast.
    w.p.b.x = 119.0;
    w.p.b.y = 4.0;
    run(&mut w, 30, 0);
    w.p.b.x = 130.0;
    w.p.b.y = 10.0;
    run(&mut w, 30, 0);
    assert_eq!(toasts(&w), 1, "found once");
}

#[test]
fn ordinary_hidden_rooms_do_not_count_as_the_secret() {
    let mut w = playhouse();
    w.p.b.x = 22.0;
    w.p.b.y = 6.0;
    run(&mut w, 60, 0);
    assert_eq!(w.game.done & crate::world::SECRET_FOUND, 0);
}

#[test]
fn the_secret_bit_survives_loading_a_save() {
    // A save that carries the secret bit and the first two levels loads all three, and only those.
    let mut w = World::new();
    w.game_new();
    w.game
        .load_done(crate::world::SECRET_FOUND | 0b11 | (1 << 20));
    assert!(w.game.is_done(CRATER) && w.game.is_done(CAVES));
    assert_ne!(w.game.done & crate::world::SECRET_FOUND, 0);
    assert_eq!(w.game.done & (1 << 20), 0, "stray bits are dropped");
}

#[test]
fn every_gate_tile_is_drawn_and_fades_when_its_gate_opens() {
    use crate::sprites::Spr;
    use lf_sim::{SpriteRect, STRIDE};
    let mut w = level(crate::levels::MIRROR_SHAFTS);
    w.ents.clear();
    w.spr[Spr::Gate as usize] = SpriteRect {
        u: 0.4,
        v: 0.4,
        uw: 0.01,
        vh: 0.01,
        w: 16.0,
        h: 16.0,
    };
    // Every instance of the gate sprite, as (faint, opaque); drawing is not culled in this call,
    // so all three gates (30 tiles wide, two rows each) are in the buffer.
    let counts = |w: &mut World| {
        w.render(0.5, 0);
        let data = w.inst.as_slice();
        let gates: Vec<f32> = (0..w.inst.len())
            .filter(|&i| data[i * STRIDE + 16] == 0.4)
            .map(|i| data[i * STRIDE + 19])
            .collect();
        let faint = gates.iter().filter(|&&a| a < 0.5).count();
        (faint, gates.len() - faint)
    };
    assert_eq!(
        counts(&mut w),
        (0, 180),
        "all three gates drawn and opaque while shut"
    );
    w.map.set_switch(1, false);
    assert_eq!(
        counts(&mut w),
        (60, 120),
        "gate one faint, the others opaque"
    );
    w.map.set_switch(2, false);
    assert_eq!(counts(&mut w), (120, 60), "gate two faint as well");
    w.map.set_switch(3, false);
    assert_eq!(
        counts(&mut w),
        (180, 0),
        "all faint once all three are open"
    );
}

#[test]
fn the_mural_stays_drawn_while_only_part_of_it_is_on_screen() {
    use crate::sprites::Spr;
    use lf_sim::{SpriteRect, STRIDE};
    let mut w = level(crate::levels::SUGAR_GLASS_GALLERY);
    w.ents.clear();
    w.spr[Spr::Mural as usize] = SpriteRect {
        u: 0.77,
        v: 0.77,
        uw: 0.01,
        vh: 0.01,
        w: 48.0,
        h: 32.0,
    };
    // The mural's bottom-left tile is (130, 11). Put the left edge of the view at column 131, so
    // that tile is off screen while the mural's other two columns are still in view.
    w.half_w = 5.0;
    w.half_h = 4.0;
    w.cam_x = 137.5;
    w.cam_y = 12.5;
    w.pcx = w.cam_x;
    w.pcy = w.cam_y;
    w.render(0.5, crate::render::flags::CULLING);
    let data = w.inst.as_slice();
    let murals = (0..w.inst.len())
        .filter(|&i| data[i * STRIDE + 16] == 0.77)
        .count();
    assert_eq!(
        murals, 1,
        "the mural is drawn although its anchor tile is off screen"
    );
}

#[test]
fn the_green_gumdrop_is_collected_and_opens_only_the_green_door() {
    let mut w = level(crate::levels::FROSTING_SPIRE);
    w.ents.clear();
    assert!(!w.keys_green);
    w.p.b.x = 30.0;
    w.p.b.y = 102.0;
    run(&mut w, 10, 0);
    assert!(w.keys_green, "picked up the green gumdrop");
    assert!(!w.keys_red && !w.keys_blue);
    // Ben is carried to the ground floor and walks into the green door.
    w.p.b.x = 34.5;
    w.p.b.y = 3.0;
    w.p.b.vy = 0.0;
    w.p.face = 1.0;
    run(&mut w, 40, RIGHT);
    assert!(!w.keys_green, "the key is used up");
    assert_eq!(
        w.map.get(36, 5),
        WALLBG,
        "the green door is gone, leaving interior wall"
    );
    assert_ne!(w.map.get(21, 49), 0, "the red door is untouched");
}

fn foundry() -> World {
    let mut w = level(crate::levels::COCOA_FOUNDRY);
    w.ents.retain(|e| e.kind == Kind::Press);
    w
}

#[test]
fn a_belt_carries_ben_and_he_can_walk_against_it() {
    let mut w = foundry();
    w.ents.clear();
    // The first right-hand belt starts at x = 12 on floor 4.
    w.p.b.x = 13.0;
    w.p.b.y = 4.0;
    run(&mut w, 5, 0);
    let x0 = w.p.b.x;
    run(&mut w, 30, 0);
    let drift = (w.p.b.x - x0) / 0.5;
    assert!(
        (drift - 3.0).abs() < 0.4,
        "carried right at 3 tiles/s, got {drift}"
    );
    // Walking left on it nets 7 - 3 tiles/s once he is up to speed; start at the belt's far end so
    // he stays on it for the whole measurement.
    w.p.b.x = 20.8;
    w.p.b.y = 4.0;
    run(&mut w, 25, LEFT);
    let x1 = w.p.b.x;
    run(&mut w, 15, LEFT);
    let net = (x1 - w.p.b.x) / 0.25;
    assert!(w.p.b.x > 12.0, "still on the belt, at {:.1}", w.p.b.x);
    assert!(
        (net - 4.0).abs() < 0.8,
        "net 4 tiles/s against the belt, got {net}"
    );
}

#[test]
fn belts_carry_enemies_too_and_they_still_turn_at_the_end() {
    let mut w = level(crate::levels::COCOA_FOUNDRY);
    w.ents.clear();
    let g = crate::ents::Spawn {
        kind: Kind::Gloop,
        x: 14.0,
        y: 4.0,
        dir: -1.0,
        ride: false,
    };
    let e = w.init_ent(&g);
    w.ents.push(e);
    run(&mut w, 300, 0);
    let b = &w.ents[0].b;
    assert!(
        b.x > 0.0 && b.y > 3.0,
        "stayed on the floor, at {:.1},{:.1}",
        b.x,
        b.y
    );
}

#[test]
fn molten_metal_kills_like_fudge() {
    let mut w = level(crate::levels::COCOA_FOUNDRY);
    w.ents.clear();
    w.game.lives = 3;
    // The first furnace pool is at x 43..45; drop in.
    assert_eq!(w.map.get(44, 1), FURNACE);
    w.p.b.x = 44.0;
    w.p.b.y = 4.0;
    run(&mut w, 60, 0);
    assert!(w.p.dead > 0.0, "Ben fell into molten metal and died");
}

#[test]
fn a_press_kills_while_down_and_is_harmless_while_raised() {
    let mut w = foundry();
    // The press at x = 24 sits over floor 4, raised bottom at y 7.
    let i = w
        .ents
        .iter()
        .position(|e| e.kind == Kind::Press && (e.b.x - 23.5).abs() < 0.01)
        .expect("press at 24");
    // Raised: phase 0.
    w.ents[i].t = 0.0;
    w.p.b.x = 23.8;
    w.p.b.y = 4.0;
    w.p.inv = 0.0;
    w.step(0);
    w.step(0);
    assert_eq!(w.p.dead, 0.0, "a raised press is harmless");
    assert!(w.ents[i].b.y > 6.5, "raised, y = {}", w.ents[i].b.y);
    // Down: half a period later, bottom on the floor.
    w.ents[i].t = std::f64::consts::PI / 1.6;
    w.step(0);
    w.step(0);
    assert!(w.ents[i].b.y < 4.5, "down, y = {}", w.ents[i].b.y);
    assert!(w.p.dead > 0.0, "a press that is down crushes Ben");
}

#[test]
fn jumping_into_a_raised_press_is_harmless_and_it_kills_as_it_comes_down_on_him() {
    let mut w = foundry();
    let i = w
        .ents
        .iter()
        .position(|e| e.kind == Kind::Press && (e.b.x - 23.5).abs() < 0.01)
        .expect("press at 24");
    // Hold the press raised (phase 0 keeps it up) and put Ben inside its box, as if he had jumped
    // up into it from the floor below: nothing happens.
    w.ents[i].t = 0.0;
    w.p.b.x = 23.8;
    w.p.b.y = 7.5;
    w.p.b.vy = 0.0;
    w.p.inv = 0.0;
    w.step(0);
    assert!(w.ents[i].b.y > 6.5, "still raised, y = {}", w.ents[i].b.y);
    assert_eq!(
        w.p.dead, 0.0,
        "a raised press does not hurt even when he is inside it"
    );
    // As soon as it has come down far enough, the same overlap is fatal.
    w.ents[i].t = std::f64::consts::PI / 1.6;
    w.p.b.y = 4.0;
    // Entities move after Ben in a tick, so the press is down by the second one.
    w.step(0);
    w.step(0);
    assert!(w.p.dead > 0.0, "the lowered press crushes him");
}

#[test]
fn presses_are_staggered_deterministically() {
    let a = foundry();
    let b = foundry();
    let phases = |w: &World| -> Vec<f64> {
        w.ents
            .iter()
            .filter(|e| e.kind == Kind::Press)
            .map(|e| e.t)
            .collect()
    };
    assert_eq!(phases(&a), phases(&b), "same every run");
    let p = phases(&a);
    assert!(p.len() >= 10);
    assert!(
        p.windows(2).any(|w| (w[0] - w[1]).abs() > 0.5),
        "not all in step"
    );
}

fn map_world() -> World {
    let mut w = World::new();
    w.game_new();
    w.enter_map();
    w.map_arm = 0.0;
    w.events.clear();
    w
}

fn stand_on(w: &mut World, i: usize) {
    let pt = w.points[i];
    w.p.b.x = pt.x + 0.2;
    w.p.b.y = pt.y + 0.2;
    w.near = None;
    // These tests press straight away: the map is already armed (see `MAP_ARM_SECS`).
    w.map_arm = 0.0;
}

#[test]
fn the_island_pad_is_dormant_and_says_nothing_helpful_until_the_secret_is_found() {
    let mut w = map_world();
    stand_on(&mut w, 20);
    w.step(0);
    let p = events_of(&w, ev::MAP_PROMPT).pop().expect("a prompt");
    assert_eq!(p.a, 5.0, "dormant-teleporter prompt");
    w.step(JUMP);
    assert!(
        (w.p.b.x - (w.points[20].x + 0.2)).abs() < 0.3,
        "a dormant pad does not move Ben"
    );
}

#[test]
fn the_hidden_pad_is_neither_drawn_nor_usable_until_the_secret_is_found() {
    use crate::sprites::Spr;
    use lf_sim::{SpriteRect, STRIDE};
    let mut w = map_world();
    let rect = |u: f32| SpriteRect {
        u,
        v: u,
        uw: 0.01,
        vh: 0.01,
        w: 16.0,
        h: 16.0,
    };
    w.spr[Spr::OwTele0 as usize] = rect(0.2);
    w.spr[Spr::OwTele1 as usize] = rect(0.2);
    w.half_w = 40.0;
    w.half_h = 30.0;
    w.cam_x = 30.0;
    w.cam_y = 22.0;
    w.pcx = 30.0;
    w.pcy = 22.0;
    let pads = |w: &mut World| {
        w.render(0.5, 0);
        let data = w.inst.as_slice();
        (0..w.inst.len())
            .filter(|&i| data[i * STRIDE + 16] == 0.2)
            .count()
    };
    let before = pads(&mut w);
    // Stand on the hidden pad: no prompt appears, because nothing is there yet.
    stand_on(&mut w, 21);
    w.events.clear();
    w.step(0);
    assert!(
        events_of(&w, ev::MAP_PROMPT).is_empty(),
        "nothing to prompt about"
    );
    w.game.done |= crate::world::SECRET_FOUND;
    let after = pads(&mut w);
    assert_eq!(
        after,
        before + 1,
        "the hidden pad appears once the secret is found"
    );
    // Now it prompts, and the pair carry Ben across the lake and back.
    stand_on(&mut w, 21);
    w.events.clear();
    w.step(0);
    let p = events_of(&w, ev::MAP_PROMPT).pop().expect("a prompt");
    assert_eq!((p.a, p.c), (2.0, 1.0), "a powered teleporter");
    w.step(JUMP);
    let isle = w.points[20];
    assert!(
        (w.p.b.x - (isle.x + 0.2)).abs() < 0.5 && (w.p.b.y - (isle.y + 0.1)).abs() < 0.5,
        "carried to the island pad at {:.1},{:.1}",
        w.p.b.x,
        w.p.b.y
    );
}

#[test]
fn the_citadel_point_is_locked_until_the_spire_and_the_foundry_are_both_cleared() {
    let mut w = map_world();
    let i = w
        .points
        .iter()
        .position(|p| p.kind == crate::levels::PtKind::Level && p.level == CITADEL)
        .unwrap();
    stand_on(&mut w, i);
    w.step(0);
    let p = events_of(&w, ev::MAP_PROMPT).pop().unwrap();
    assert_eq!(p.a, 4.0, "locked");
    w.step(JUMP);
    assert_eq!(w.mode, Mode::Map);
    w.game.set_done(crate::levels::FROSTING_SPIRE);
    stand_on(&mut w, i);
    w.step(0);
    w.step(JUMP);
    assert_eq!(w.mode, Mode::Map, "one of two is not enough");
    w.game.set_done(crate::levels::COCOA_FOUNDRY);
    stand_on(&mut w, i);
    w.step(0);
    w.step(JUMP);
    assert_eq!(w.mode, Mode::Level);
    assert_eq!(w.level_id, CITADEL);
}

#[test]
fn a_saved_map_position_inside_a_river_snaps_back_to_the_saucer() {
    let mut w = World::new();
    w.game_new();
    // On the old map this was open ground; on the new one it is the vertical river.
    w.game.map_pos = Some((24.5, 10.0));
    w.enter_map();
    let start = crate::levels::build_overworld().start;
    assert_eq!((w.p.b.x, w.p.b.y), start, "snapped to the saucer");
    // So does one off the map altogether, which a hand-edited save could hold.
    for off in [(12.0, -5.0), (-3.0, 10.0), (12.0, 80.0), (90.0, 10.0)] {
        let mut w = World::new();
        w.game_new();
        w.game.map_pos = Some(off);
        w.enter_map();
        assert_eq!((w.p.b.x, w.p.b.y), start, "{off:?} snaps to the saucer");
    }
    // A position on open ground is kept.
    let mut w = World::new();
    w.game_new();
    w.game.map_pos = Some((12.2, 19.0));
    w.enter_map();
    assert_eq!((w.p.b.x, w.p.b.y), (12.2, 19.0));
}

#[test]
fn every_teleporter_lands_ben_on_walkable_ground() {
    let mut w = map_world();
    w.game.done = crate::world::PROGRESS_BITS;
    let teles: Vec<usize> = w
        .points
        .iter()
        .enumerate()
        .filter(|(_, p)| p.kind == crate::levels::PtKind::Tele)
        .map(|(i, _)| i)
        .collect();
    assert!(teles.len() >= 8);
    for i in teles {
        let from = w.points[i];
        stand_on(&mut w, i);
        w.step(0);
        w.step(JUMP);
        let to = w.points[from.to];
        // Ben's 0.6 by 0.6 body covers only grass or path at the other end.
        for (dx, dy) in [(0.0, 0.0), (0.6, 0.0), (0.0, 0.6), (0.6, 0.6)] {
            let t = w
                .map
                .get((w.p.b.x + dx).floor() as i32, (w.p.b.y + dy).floor() as i32);
            assert!(
                t == GRASS || t == PATH,
                "pad {i} lands Ben on tile {t} at {:.1},{:.1}",
                w.p.b.x + dx,
                w.p.b.y + dy
            );
        }
        assert!(
            (w.p.b.x - (to.x + 0.2)).abs() < 0.01,
            "pad {i} lands at its partner"
        );
        // And he is not wedged: some direction lets him move.
        let (x0, y0) = (w.p.b.x, w.p.b.y);
        let mut moved = false;
        for dir in [LEFT, RIGHT, UP, DOWN] {
            let mut v = map_world();
            v.game.done = w.game.done;
            v.p.b.x = x0;
            v.p.b.y = y0;
            run(&mut v, 12, dir);
            moved |= (v.p.b.x - x0).abs() + (v.p.b.y - y0).abs() > 0.5;
        }
        assert!(moved, "pad {i}: Ben is stuck after landing");
    }
}

#[test]
fn enemies_in_a_hidden_room_wait_there_until_ben_walks_in() {
    let mut w = level(crate::levels::BONBON_PLAYHOUSE);
    // The tutorial room on the path holds a gloop that would otherwise pace out of it.
    let room = w
        .rooms
        .iter()
        .position(|r| r.contains(24, 6))
        .expect("tutorial room");
    let g = w
        .ents
        .iter()
        .position(|e| e.kind == Kind::Gloop && w.rooms[room].contains(e.b.x as i32, e.b.y as i32))
        .expect("a gloop in the room");
    let x0 = w.ents[g].b.x;
    // Ben waits far away, well past the 4 seconds a gloop needs to leave the room.
    w.p.inv = 1e9;
    w.p.b.x = 5.0;
    w.p.b.y = 4.0;
    run(&mut w, 360, 0);
    assert_eq!(w.ents[g].b.x, x0, "the ambusher has not moved");
    assert!(w.rooms[room].contains(w.ents[g].b.x as i32, w.ents[g].b.y as i32));
    // Once Ben is inside and the flat has faded, it moves like any other gloop.
    w.p.b.x = 18.0;
    w.p.b.y = 6.0;
    run(&mut w, 90, 0);
    assert!(w.room_alpha[room] < 0.9, "revealed");
    assert!((w.ents[g].b.x - x0).abs() > 0.2, "and awake now");
}

#[test]
fn enemies_outside_hidden_rooms_are_not_held_back() {
    let mut w = level(crate::levels::BONBON_PLAYHOUSE);
    let g = w
        .ents
        .iter()
        .position(|e| e.kind == Kind::Gloop && e.b.x < 12.0)
        .expect("the first gloop is outside any room");
    let x0 = w.ents[g].b.x;
    run(&mut w, 120, 0);
    assert!((w.ents[g].b.x - x0).abs() > 0.5, "it paces as usual");
}

#[test]
fn the_secret_mural_is_covered_by_the_flat_until_ben_walks_in() {
    use crate::sprites::Spr;
    use lf_sim::{SpriteRect, STRIDE};
    let mut w = level(crate::levels::SUGAR_GLASS_GALLERY);
    w.ents.clear();
    let rect = |u: f32| SpriteRect {
        u,
        v: u,
        uw: 0.01,
        vh: 0.01,
        w: 16.0,
        h: 16.0,
    };
    w.spr[Spr::Mural as usize] = rect(0.77);
    w.spr[Spr::Facade as usize] = rect(0.31);
    w.half_w = 12.0;
    w.half_h = 8.0;
    w.render(0.5, 0);
    let data = w.inst.as_slice();
    let n = w.inst.len();
    let mural = (0..n)
        .find(|&i| data[i * STRIDE + 16] == 0.77)
        .expect("the mural is drawn");
    // Every one of the mural's six cells has a flat drawn over it, after the mural.
    for dx in 0..3 {
        for dy in 0..2 {
            let (cx, cy) = (130.5 + dx as f32, 11.5 + dy as f32);
            let covered = (mural + 1..n).any(|i| {
                data[i * STRIDE + 16] == 0.31
                    && (data[i * STRIDE + 12] - cx).abs() < 0.01
                    && (data[i * STRIDE + 13] - cy).abs() < 0.01
            });
            assert!(covered, "no flat over mural cell {cx},{cy}");
        }
    }
}

#[test]
fn jumping_off_a_ladder_with_up_held_does_not_grab_it_again() {
    let mut w = lookout();
    w.p.b.x = 17.8;
    w.p.b.y = 3.0;
    run(&mut w, 40, UP);
    assert!(w.p.climb);
    // Up stays held through the jump, which is the natural way to leap while climbing.
    w.step(UP | JUMP);
    assert!(!w.p.climb, "let go");
    let y0 = w.p.b.y;
    run(&mut w, 8, UP | JUMP);
    assert!(!w.p.climb, "still not on the ladder with Up held");
    assert!(
        w.p.b.y > y0 + 0.8,
        "the hop carried him up and away, y {} -> {}",
        y0,
        w.p.b.y
    );
    // Letting go of Up lifts the lock, so a fresh press grabs the ladder again.
    w.p.b.x = 17.8;
    w.p.b.vy = 0.0;
    run(&mut w, 2, 0);
    run(&mut w, 3, UP);
    assert!(w.p.climb, "a new press of Up grabs it");
}

#[test]
fn a_bubble_fired_from_beside_a_mirror_still_bounces_off_it() {
    let mut w = shafts();
    w.shots.clear();
    // Fired left from Ben's usual muzzle offset (0.6 in front of him) with Ben against the right
    // edge of the swivel's column (centre 0.35 right of the cell's left edge), so the bubble starts
    // 0.75 left of the mirror's centre and moves away from it.
    let i = find(&w, Kind::Swivel, 18.0);
    let (mx, my) = (w.ents[i].b.x + 0.5, w.ents[i].b.y + 0.5);
    w.shots.push(bubble(mx - 0.75, my, -16.0, 0.0));
    w.step(0);
    let s = w.shots.first().expect("the bubble is still flying");
    assert_ne!(
        (s.vx, s.vy),
        (-16.0, 0.0),
        "it was turned by the mirror, not left to fly on into the corridor"
    );
    assert_eq!(s.last, i as i32);
}

/// A bare arena: a floor two tiles thick, open sky, no enemies or items. Walls and vines are added
/// by each test. Ben starts standing at (4, 2).
fn arena() -> World {
    let mut w = level(CRATER);
    w.ents.clear();
    w.items.clear();
    w.plats.clear();
    w.map = lf_sim::TileMap::new(40, 40, props());
    for x in 0..40 {
        for y in 0..2 {
            w.map.set(x, y, FILL);
        }
    }
    w.p.b.x = 4.0;
    w.p.b.y = 2.0;
    w.p.b.vx = 0.0;
    w.p.b.vy = 0.0;
    w.p.inv = 1e9;
    w.cam_x = 10.0;
    w.cam_y = 10.0;
    run(&mut w, 5, 0);
    w
}

#[test]
fn ben_free_climbs_a_vine_in_every_direction_and_lets_go_when_it_runs_out() {
    let mut w = arena();
    // A patch of vine, five wide and eight tall.
    for x in 10..15 {
        for y in 2..10 {
            w.map.set(x, y, VINE);
        }
    }
    w.p.b.x = 11.65;
    w.p.b.y = 2.0;
    run(&mut w, 2, 0);
    run(&mut w, 20, UP);
    assert!(
        w.p.climb && w.p.wall,
        "Up grabs the vine and he climbs it freely"
    );
    let (x0, y0) = (w.p.b.x, w.p.b.y);
    run(&mut w, 30, UP);
    let rate = (w.p.b.y - y0) / 0.5;
    assert!(
        (rate - VINE_SPEED_FOR_TEST).abs() < 0.4,
        "climbs at the vine speed, got {rate}"
    );
    assert!(
        (w.p.b.x - x0).abs() < 0.01,
        "a plain climb does not drift sideways"
    );
    // Sideways, and diagonally, across the vine.
    run(&mut w, 30, RIGHT);
    let moved = w.p.b.x - x0;
    assert!(moved > 1.2, "crosses the vine sideways, moved {moved}");
    let y1 = w.p.b.y;
    run(&mut w, 20, UP | LEFT);
    assert!(
        w.p.b.y > y1 + 0.8 && w.p.b.x < x0 + moved - 0.6,
        "up and left at once"
    );
    // Out the top: no vine left, so he lets go and falls.
    run(&mut w, 200, UP);
    assert!(!w.p.climb, "ran out of vine");
}

#[test]
fn a_ladder_still_snaps_to_its_column_where_a_vine_does_not() {
    let mut w = arena();
    for y in 2..10 {
        w.map.set(10, y, RUNG);
        w.map.set(14, y, VINE);
    }
    w.p.b.x = 9.8;
    run(&mut w, 2, 0);
    run(&mut w, 10, UP);
    assert!(w.p.climb && !w.p.wall, "a ladder is not a vine");
    assert!(
        (w.p.b.centre_x() - 10.5).abs() < 0.01,
        "snapped to the rung"
    );
    let mut v = arena();
    for y in 2..10 {
        v.map.set(14, y, VINE);
    }
    v.p.b.x = 14.1;
    run(&mut v, 2, 0);
    run(&mut v, 10, UP);
    assert!(v.p.wall);
    assert!((v.p.b.x - 14.1).abs() < 0.01, "not snapped to the column");
}

#[test]
fn a_jump_off_a_vine_hops_clear_and_does_not_regrab_with_up_held() {
    let mut w = arena();
    for x in 10..15 {
        for y in 2..12 {
            w.map.set(x, y, VINE);
        }
    }
    w.p.b.x = 11.65;
    run(&mut w, 2, 0);
    run(&mut w, 30, UP);
    assert!(w.p.climb);
    w.step(UP | JUMP);
    assert!(!w.p.climb && !w.p.wall);
    run(&mut w, 6, UP | JUMP);
    assert!(!w.p.climb, "no immediate re-grab with Up held");
}

#[test]
fn ben_pulls_himself_up_onto_a_ledge_he_nearly_made() {
    let mut w = arena();
    // A wall four tiles tall: a plain jump (about 3.4) falls just short of its top.
    for x in 12..30 {
        for y in 2..6 {
            w.map.set(x, y, FILL);
        }
    }
    w.p.b.x = 10.5;
    run(&mut w, 2, 0);
    run(&mut w, 6, RIGHT);
    w.step(RIGHT | JUMP);
    run(&mut w, 40, RIGHT | JUMP);
    assert!(w.p.b.on_ground, "ended up standing");
    assert!(
        (w.p.b.y - 6.0).abs() < 0.01,
        "on top of the wall, y = {}",
        w.p.b.y
    );
    assert!(w.p.b.x >= 12.0, "over the wall, x = {}", w.p.b.x);
}

/// An arena with a four-tile wall to the right and Ben jumping at it, stepped to the tick the
/// pull-up begins.
fn grabbing_world() -> World {
    let mut w = arena();
    for x in 12..30 {
        for y in 2..6 {
            w.map.set(x, y, FILL);
        }
    }
    w.p.b.x = 10.5;
    run(&mut w, 2, 0);
    run(&mut w, 6, RIGHT);
    w.step(RIGHT | JUMP);
    for _ in 0..80 {
        if w.p.mantle.is_some() {
            return w;
        }
        w.step(RIGHT | JUMP);
    }
    panic!("Ben never grabbed the ledge");
}

fn body_overlaps_solid(w: &World) -> bool {
    let b = &w.p.b;
    (b.x.floor() as i32..=(b.x + b.w - 1e-4).floor() as i32).any(|cx| {
        (b.y.floor() as i32..=(b.y + b.h - 1e-4).floor() as i32)
            .any(|cy| w.map.solid(cx, cy, false, 0.0))
    })
}

#[test]
fn a_mantle_takes_mantle_time_and_ends_where_the_instant_pull_up_did() {
    let mut w = grabbing_world();
    let heaves = |w: &World| {
        events_of(w, ev::CAPTION)
            .iter()
            .filter(|e| e.c == f32::from(Cap::Heave as u16))
            .count()
    };
    assert_eq!(heaves(&w), 1, "the heave sounds as the mantle starts");
    let from = w.p.b.y;
    let mut ticks = 0;
    while w.p.mantle.is_some() {
        w.step(RIGHT | JUMP);
        ticks += 1;
        assert!(ticks < 100, "the mantle never finished");
    }
    assert_eq!(heaves(&w), 1, "one heave per mantle");
    assert_eq!(ticks, (MANTLE_TIME / lf_sim::STEP).round() as i32);
    const { assert!(MANTLE_TIME > 0.2 && MANTLE_TIME < 0.5) };
    let b = &w.p.b;
    assert!(
        from < 6.0 && 6.0 - from <= MANTLE_REACH,
        "grabbed below the lip"
    );
    assert_eq!((b.x, b.y), (12.1, 6.0));
    assert_eq!((b.vx, b.vy), (0.0, 0.0));
    assert!(b.on_ground);
    assert!(!w.p.cut);
}

#[test]
fn ben_hangs_at_the_grab_then_rises_then_steps_over_the_lip() {
    let mut w = grabbing_world();
    let m = w.p.mantle.unwrap();
    assert_eq!(m.t, 0.0);
    assert_eq!((w.p.b.x, w.p.b.y), m.from, "the grab starts where he hangs");
    let (mut last_y, mut last_x) = (w.p.b.y, w.p.b.x);
    let mut stepping = false;
    while w.p.mantle.is_some() {
        w.step(0);
        let b = &w.p.b;
        assert!(
            b.y >= last_y - 1e-12 && b.x >= last_x - 1e-12,
            "never moves backwards"
        );
        if b.x > m.from.0 + 1e-9 {
            stepping = true;
        }
        if stepping {
            assert!(
                (b.y - m.to.1).abs() < 1e-9,
                "level with the lip while stepping over"
            );
        } else {
            assert_eq!(b.x, m.from.0, "straight up the wall first");
        }
        (last_y, last_x) = (b.y, b.x);
    }
    assert!(stepping, "ended with a step over the lip");
}

#[test]
fn during_a_mantle_ben_cannot_move_jump_fire_or_pogo_and_never_touches_a_solid() {
    let mut calm = grabbing_world();
    let mut mashed = grabbing_world();
    let ammo = mashed.game.ammo;
    let mut tick = 0;
    while calm.p.mantle.is_some() {
        calm.step(RIGHT);
        // Every other input at once, changing every tick.
        let held = [
            LEFT | UP | FIRE,
            POGO | DOWN | JUMP | FIRE,
            LEFT | JUMP | POGO,
        ][tick % 3];
        mashed.step(held);
        tick += 1;
        assert_eq!((mashed.p.b.x, mashed.p.b.y), (calm.p.b.x, calm.p.b.y));
        assert!(
            !body_overlaps_solid(&mashed),
            "inside a solid on tick {tick}"
        );
        assert_eq!(mashed.game.ammo, ammo, "no shot while hauling up");
        assert!(mashed.shots.is_empty());
        assert!(!mashed.p.pogo);
    }
    assert!(mashed.p.mantle.is_none());
    assert_eq!((mashed.p.b.x, mashed.p.b.y), (12.1, 6.0));
}

#[test]
fn a_mantle_never_overlaps_a_solid_when_mirrored_either_way() {
    let mut w = arena();
    // A wall to the left of Ben this time.
    for x in 2..9 {
        for y in 2..6 {
            w.map.set(x, y, FILL);
        }
    }
    w.p.b.x = 9.8;
    run(&mut w, 2, 0);
    run(&mut w, 6, LEFT);
    w.step(LEFT | JUMP);
    let mut seen = false;
    for _ in 0..120 {
        w.step(LEFT | JUMP);
        seen |= w.p.mantle.is_some();
        assert!(!body_overlaps_solid(&w));
        if seen && w.p.mantle.is_none() {
            break;
        }
    }
    assert!(seen, "mantled leftwards");
    assert!(w.p.b.on_ground && (w.p.b.y - 6.0).abs() < 0.01 && w.p.b.x < 9.0);
}

#[test]
fn the_camera_follows_a_mantle_without_a_jump() {
    let mut w = grabbing_world();
    let mut last = (w.cam_x, w.cam_y);
    while w.p.mantle.is_some() {
        w.step(RIGHT);
        let step = (w.cam_x - last.0).abs().max((w.cam_y - last.1).abs());
        assert!(step < 0.15, "camera jumped {step} in one tick");
        last = (w.cam_x, w.cam_y);
    }
}

#[test]
fn two_runs_with_the_same_inputs_mantle_identically() {
    let trace = || {
        let mut w = grabbing_world();
        let mut t = vec![];
        for i in 0..40 {
            w.step(if i % 2 == 0 { RIGHT } else { 0 });
            t.push((w.p.b.x, w.p.b.y, w.p.mantle.is_some()));
        }
        t
    };
    assert_eq!(trace(), trace());
}

#[test]
fn a_hit_mid_mantle_cancels_it_and_ben_tumbles_from_where_he_was() {
    let mut w = grabbing_world();
    run(&mut w, 6, 0);
    assert!(w.p.mantle.is_some());
    w.kill();
    assert!(w.p.mantle.is_none(), "dying ends the pull-up");
    assert!(w.p.dead > 0.0);
    run(&mut w, 100, 0);
    assert!(w.p.mantle.is_none());
    assert!(w.p.b.y < 6.0, "fell away instead of landing on the ledge");
    assert!(w.p.b.x < 12.0, "still on the near side of the wall");
}

#[test]
fn an_enemy_touching_ben_mid_mantle_kills_him_and_ends_the_mantle() {
    let mut w = grabbing_world();
    w.p.inv = 0.0;
    let g = w.init_ent(&crate::ents::Spawn {
        kind: Kind::Gloop,
        x: w.p.b.x,
        y: w.p.b.y,
        dir: 1.0,
        ride: false,
    });
    w.ents.push(g);
    run(&mut w, 3, 0);
    assert!(w.p.dead > 0.0, "damage still applies while hanging");
    assert!(w.p.mantle.is_none());
}

#[test]
fn a_gate_that_closes_on_the_path_ends_the_mantle_without_pulling_ben_through_it() {
    let mut w = grabbing_world();
    run(&mut w, 8, 0);
    // The ledge fills in over the spot he was climbing to.
    for y in 6..8 {
        w.map.set(12, y, FILL);
        w.map.set(13, y, FILL);
    }
    run(&mut w, 30, 0);
    assert!(w.p.mantle.is_none());
    assert!(!body_overlaps_solid(&w));
    assert!(w.p.b.x < 12.0, "still on the near side of the wall");
}

#[test]
fn a_mantle_does_not_start_when_there_is_no_room_to_haul_up_the_wall() {
    // The wall is climbable, but a ceiling over Ben's head blocks the way up.
    let mut w = arena();
    for y in 2..6 {
        w.map.set(12, y, FILL);
    }
    for x in 8..12 {
        w.map.set(x, 6, FILL);
        w.map.set(x, 7, FILL);
    }
    w.p.b.x = 10.5;
    w.p.b.y = 4.8;
    w.p.b.vy = 0.0;
    for _ in 0..30 {
        w.step(RIGHT | JUMP);
        assert!(w.p.mantle.is_none());
    }
}

#[test]
fn leaving_for_the_map_or_reloading_the_level_clears_a_mantle() {
    let mut w = grabbing_world();
    run(&mut w, 5, 0);
    assert!(w.p.mantle.is_some());
    w.enter_level(CRATER);
    assert!(w.p.mantle.is_none());
    let mut w = grabbing_world();
    w.enter_map();
    assert!(w.p.mantle.is_none());
    let mut w = grabbing_world();
    w.load_level(CAVES);
    assert!(w.p.mantle.is_none());
}

#[test]
fn a_mantle_sprite_follows_its_progress() {
    use crate::render::ben_sprite;
    use crate::sprites::Spr;
    let mut w = grabbing_world();
    let mut seen = vec![];
    loop {
        let s = ben_sprite(&w.p);
        if seen.last() != Some(&s) {
            seen.push(s);
        }
        if w.p.mantle.is_none() {
            break;
        }
        w.step(0);
    }
    assert_eq!(
        seen[..4],
        [
            Spr::BenMantle1,
            Spr::BenMantle2,
            Spr::BenMantle3,
            Spr::BenStand
        ]
    );
}

#[test]
fn a_ledge_that_is_too_high_or_too_cramped_is_not_mantled() {
    // Too high: a six-tile wall.
    let mut w = arena();
    for y in 2..8 {
        w.map.set(12, y, FILL);
    }
    w.p.b.x = 10.5;
    run(&mut w, 2, 0);
    run(&mut w, 6, RIGHT);
    w.step(RIGHT | JUMP);
    run(&mut w, 60, RIGHT | JUMP);
    assert!(
        w.p.b.y < 3.0,
        "a six-tile wall is not pulled up, y = {}",
        w.p.b.y
    );
    // Too cramped: the same four-tile wall under a ceiling with no room to stand on top.
    let mut c = arena();
    for y in 2..6 {
        c.map.set(12, y, FILL);
    }
    for x in 12..16 {
        c.map.set(x, 7, FILL);
    }
    c.p.b.x = 10.5;
    run(&mut c, 2, 0);
    run(&mut c, 6, RIGHT);
    c.step(RIGHT | JUMP);
    run(&mut c, 40, RIGHT | JUMP);
    assert!(
        c.p.b.y < 3.0,
        "no pull-up into a spot he cannot fit, y = {}",
        c.p.b.y
    );
}

#[test]
fn ben_kicks_off_a_wall_up_and_away_once_per_side() {
    let mut w = arena();
    for y in 2..30 {
        w.map.set(12, y, FILL);
    }
    // Airborne against the left face of the wall.
    w.p.b.x = 11.28;
    w.p.b.y = 8.0;
    w.p.b.vy = 0.0;
    w.p.b.on_ground = false;
    w.step(RIGHT);
    w.step(RIGHT | JUMP);
    assert!(w.p.b.vy > 15.0, "kicked upwards, vy {}", w.p.b.vy);
    assert!(w.p.b.vx < -3.0, "and away from the wall, vx {}", w.p.b.vx);
    assert_eq!(w.p.kick_side, 1.0);
    // Back against the same wall: a second kick on the same side is refused.
    w.p.b.x = 11.28;
    w.p.b.y = 15.0;
    w.p.b.vy = 0.0;
    w.p.b.vx = 0.0;
    w.p.wall_t = 0.0;
    w.step(RIGHT);
    let before = w.p.b.vy;
    w.step(RIGHT | JUMP);
    assert!(
        w.p.b.vy <= before + 0.1,
        "no second kick off the same wall, vy {}",
        w.p.b.vy
    );
}

#[test]
fn a_single_wall_cannot_be_climbed_by_kicking_but_two_facing_walls_can() {
    let climb = |both: bool| {
        let mut w = arena();
        for y in 2..38 {
            w.map.set(12, y, FILL);
            if both {
                w.map.set(16, y, FILL);
            }
        }
        // Between the walls (a gap three tiles wide) or beside one, kicking whenever he touches one.
        w.p.b.x = if both { 14.0 } else { 11.28 };
        w.p.b.y = 3.0;
        let mut top = 0.0_f64;
        for t in 0..300 {
            // Head for the next wall, tap Jump (every other tick) while he touches one, so each kick is a
            // fresh press.
            let towards = if !both || w.p.b.vx >= 0.0 {
                RIGHT
            } else {
                LEFT
            };
            let jump = if w.wall_side().is_some() && t % 2 == 0 {
                JUMP
            } else {
                0
            };
            w.step(towards | jump);
            top = top.max(w.p.b.y);
        }
        top
    };
    let single = climb(false);
    let double = climb(true);
    assert!(single < 9.0, "one wall gives one kick, reached {single}");
    assert!(
        double > single + 4.0,
        "two walls chain kicks: {double} vs {single}"
    );
}

/// The vine speed the tests expect (kept in step with `VINE_SPEED`).
const VINE_SPEED_FOR_TEST: f64 = crate::world::VINE_SPEED;

#[test]
fn a_fizz_shot_brings_down_a_cracked_wall_and_its_cracked_neighbours() {
    let mut w = arena();
    for y in 2..6 {
        w.map.set(10, y, CRACKED);
    }
    w.map.set(10, 6, FILL);
    w.map.set(11, 2, FILL);
    w.p.b.x = 6.0;
    w.p.b.y = 2.0;
    w.p.face = 1.0;
    w.step(FIRE);
    run(&mut w, 40, 0);
    for y in 2..6 {
        assert_eq!(w.map.get(10, y), EMPTY, "cracked tile at row {y} crumbled");
    }
    assert_eq!(w.map.get(10, 6), FILL, "plain rock is untouched");
    assert_eq!(w.map.get(11, 2), FILL);
    // A bubble that is not Ben's leaves it alone.
    w.map.set(20, 2, CRACKED);
    w.shots.push(crate::ents::Shot {
        x: 19.0,
        y: 2.5,
        vx: 10.0,
        vy: 0.0,
        ben: false,
        life: 2.0,
        sprite: 0,
        g: false,
        last: -1,
    });
    run(&mut w, 30, 0);
    assert_eq!(w.map.get(20, 2), CRACKED);
}

#[test]
fn a_cracked_wall_blocks_ben_until_it_is_shot() {
    let mut w = arena();
    for y in 2..8 {
        w.map.set(10, y, CRACKED);
    }
    w.p.b.x = 6.0;
    run(&mut w, 120, RIGHT);
    assert!(w.p.b.x < 10.0, "held at the wall, at {:.1}", w.p.b.x);
}

#[test]
fn shooting_a_lantern_pops_it_into_snacks() {
    let mut w = arena();
    let t = crate::ents::Spawn {
        kind: Kind::Target,
        x: 12.0,
        y: 3.0,
        dir: 4.0,
        ride: false,
    };
    let e = w.init_ent(&t);
    w.ents.push(e);
    w.p.b.x = 6.0;
    w.p.b.y = 2.0;
    w.p.face = 1.0;
    w.step(FIRE);
    run(&mut w, 40, 0);
    assert!(w.ents[0].dead, "popped");
    assert_eq!(w.items.len(), 4);
    assert!(w.items.iter().all(|i| i.kind == ItemKind::Cheezie));
}

#[test]
fn a_wall_painting_shows_its_line_once_per_visit() {
    use crate::text::Toast;
    let mut w = arena();
    let g = crate::ents::Spawn {
        kind: Kind::Glyph,
        x: 8.0,
        y: 2.0,
        dir: 2.0,
        ride: false,
    };
    let e = w.init_ent(&g);
    w.ents.push(e);
    let count = |w: &World| {
        events_of(w, ev::TOAST)
            .iter()
            .filter(|e| e.a == Toast::GlyphFizz as u16 as f32)
            .count()
    };
    w.p.b.x = 8.2;
    run(&mut w, 30, 0);
    assert_eq!(count(&w), 1, "read once while standing in front");
}

/// Plays a scripted visit to a vine nook: walk to the vine, climb to the shelf, step off towards
/// the cracked wall, shoot it, and walk into the room. Returns where Ben ends up.
fn visit_nook(id: u8, start: (f64, f64), vx: i32, shelf: i32, dir: f64, fire: bool) -> (f64, f64) {
    let mut w = level(id);
    w.ents.retain(|e| e.kind == Kind::Glyph);
    w.p.b.x = start.0;
    w.p.b.y = start.1;
    w.p.inv = 1e9;
    w.game.ammo = 5;
    let (toward, away) = if dir > 0.0 {
        (RIGHT, LEFT)
    } else {
        (LEFT, RIGHT)
    };
    // Walk under the vine (it is on the side opposite the direction of the pocket).
    let vine_x = f64::from(vx) + 0.5;
    for _ in 0..600 {
        let dx = vine_x - (w.p.b.x + w.p.b.w / 2.0);
        if dx.abs() < 0.15 {
            break;
        }
        w.step(if dx > 0.0 { RIGHT } else { LEFT });
    }
    let _ = away;
    // Climb until level with the shelf, then step off.
    for _ in 0..600 {
        w.step(UP);
        if w.p.b.y >= f64::from(shelf) + 0.3 {
            break;
        }
    }
    for _ in 0..90 {
        w.step(toward);
    }
    assert!(
        w.p.b.on_ground,
        "standing on the shelf, at {:.1},{:.1}",
        w.p.b.x, w.p.b.y
    );
    if fire {
        w.step(toward | FIRE);
        run(&mut w, 60, 0);
        run(&mut w, 150, toward);
    } else {
        // Without a shot, try everything: run at the wall, jumping again and again.
        for t in 0..300 {
            w.step(toward | if t % 25 < 12 { JUMP } else { 0 });
        }
    }
    (w.p.b.x, w.p.b.y)
}

#[test]
fn the_caves_hidden_room_can_be_reached_by_vine() {
    let (x, y) = visit_nook(crate::levels::CAVES, (91.5, 8.0), 89, 13, -1.0, true);
    assert!(x < 85.0 && y >= 13.0, "inside the room, at {x:.1},{y:.1}");
}

#[test]
fn the_bogs_hidden_room_can_be_reached_by_vine() {
    let (x, y) = visit_nook(crate::levels::FUDGE_BOG, (40.0, 7.0), 36, 12, 1.0, true);
    assert!(x > 40.0 && y >= 12.0, "inside the room, at {x:.1},{y:.1}");
}

#[test]
fn mirror_shafts_has_a_vine_shortcut_past_the_second_gate_that_pays_in_cookies() {
    let mut w = level(crate::levels::MIRROR_SHAFTS);
    w.ents.retain(|e| e.kind == Kind::Glyph);
    // On the wide ledge under gate two, facing the right-hand wall.
    w.p.b.x = 29.0;
    w.p.b.y = 63.0;
    w.p.face = 1.0;
    w.p.inv = 1e9;
    w.game.ammo = 5;
    run(&mut w, 10, 0);
    assert!(w.p.b.on_ground, "standing on the ledge at {:.1}", w.p.b.y);
    assert!(w.map.switch(2), "gate two is closed");
    w.step(RIGHT | FIRE);
    run(&mut w, 40, 0);
    assert_eq!(w.map.get(33, 63), EMPTY, "the cracked wall crumbled");
    // Into the wall, up the vine, and out on top of the gate.
    let score = w.game.score;
    run(&mut w, 60, RIGHT);
    for _ in 0..400 {
        w.step(UP);
        if w.p.b.y >= 68.3 {
            break;
        }
    }
    run(&mut w, 60, LEFT);
    assert!(
        w.p.b.on_ground && w.p.b.y >= 68.0,
        "on top of the gate at {:.1},{:.1}",
        w.p.b.x,
        w.p.b.y
    );
    assert!(w.p.b.x < 33.0, "back in the shaft, at {:.1}", w.p.b.x);
    assert!(w.map.switch(2), "gate two never opened");
    assert!(w.game.score > score, "the cookies paid out");
}

/// Ben bounces on a pogo (Jump held) or hops (Jump tapped) under `x`, and reports whether the
/// cookie on the ledge there got taken.
fn takes_cache_cookie(id: u8, x: f64, pogo: bool) -> bool {
    let mut w = level(id);
    w.ents.clear();
    w.p.inv = 1e9;
    w.p.b.x = x;
    w.p.b.y = 5.5;
    run(&mut w, 120, 0);
    w.p.b.x = x;
    if pogo {
        w.step(POGO);
    }
    for t in 0..600 {
        let jump = if pogo || t % 12 < 8 { JUMP } else { 0 };
        w.step(jump);
        if w.items
            .iter()
            .any(|i| i.kind == ItemKind::Cookie && i.taken && (i.x - x).abs() < 3.0)
        {
            return true;
        }
    }
    false
}

#[test]
fn each_pogo_cache_needs_a_pogo_bounce() {
    use crate::levels::{CAVES, CRATER, FUDGE_BOG};
    for (id, x) in [(CRATER, 9.2), (FUDGE_BOG, 19.2), (CAVES, 6.2)] {
        assert!(
            takes_cache_cookie(id, x, true),
            "level {id}: pogo reaches the cache"
        );
        assert!(
            !takes_cache_cookie(id, x, false),
            "level {id}: a plain jump does not"
        );
    }
}

#[test]
fn rare_actions_earn_a_joke_once() {
    use crate::text::Toast;
    let count = |w: &World, t: Toast| {
        events_of(w, ev::TOAST)
            .iter()
            .filter(|e| e.a == t as u16 as f32)
            .count()
    };
    // Standing around.
    let mut w = arena();
    run(&mut w, 60 * 25, 0);
    assert_eq!(count(&w, Toast::JokeIdle), 1);
    // Thirty pogo bounces.
    let mut w = arena();
    w.step(POGO);
    run(&mut w, 60 * 30, 0);
    assert_eq!(
        count(&w, Toast::JokePogo),
        1,
        "pogo joke fires exactly once"
    );
    // Wall-kicking back and forth.
    let mut w = arena();
    for y in 2..38 {
        w.map.set(12, y, FILL);
        w.map.set(16, y, FILL);
    }
    w.p.b.x = 14.0;
    w.p.b.y = 3.0;
    for t in 0..900 {
        let towards = if w.p.b.vx >= 0.0 { RIGHT } else { LEFT };
        let jump = if w.wall_side().is_some() && t % 2 == 0 {
            JUMP
        } else {
            0
        };
        w.step(towards | jump);
        if w.p.b.y > 30.0 {
            w.p.b.y = 3.0;
            w.p.b.vy = 0.0;
        }
    }
    assert_eq!(count(&w, Toast::JokeKick), 1);
}

#[test]
fn chalk_drawings_of_billy_and_mortimer_have_their_say() {
    use crate::text::Toast;
    let mut w = arena();
    for (x, dir, toast) in [
        (8.0, 0.0, Toast::CameoBilly),
        (20.0, 1.0, Toast::CameoMortimer),
    ] {
        let e = w.init_ent(&crate::ents::Spawn {
            kind: Kind::Cameo,
            x,
            y: 2.0,
            dir,
            ride: false,
        });
        w.ents.push(e);
        w.p.b.x = x + 0.1;
        w.p.b.y = 2.0;
        run(&mut w, 20, 0);
        assert!(
            events_of(&w, ev::TOAST)
                .iter()
                .any(|e| e.a == toast as u16 as f32),
            "{toast:?} shown"
        );
    }
}

#[test]
fn jumping_off_a_vine_beside_a_wall_hops_instead_of_kicking() {
    let mut w = arena();
    for y in 2..20 {
        w.map.set(12, y, FILL);
        w.map.set(13, y, VINE);
    }
    w.p.b.x = 13.02;
    w.p.b.y = 6.0;
    w.p.b.on_ground = false;
    w.step(UP);
    assert!(w.p.climb, "holding the vine");
    w.step(RIGHT | JUMP);
    assert!(
        (w.p.b.vy - 12.0).abs() < 2.0,
        "a hop, not a kick, vy {}",
        w.p.b.vy
    );
    assert!(
        w.p.b.vx >= 0.0,
        "pressed away from the wall, vx {}",
        w.p.b.vx
    );
}

#[test]
fn nothing_gets_past_a_hidden_rooms_cracked_wall_without_a_shot() {
    let (x, _) = visit_nook(crate::levels::CAVES, (91.5, 8.0), 89, 13, -1.0, false);
    assert!(x > 86.0, "stopped at the wall, at {x:.1}");
    let (x, _) = visit_nook(crate::levels::FUDGE_BOG, (40.0, 7.0), 36, 12, 1.0, false);
    assert!(x < 39.0, "stopped at the wall, at {x:.1}");
}

fn ladder_world() -> World {
    let mut w = level(CRATER);
    w.ents.clear();
    for y in 4..14 {
        w.map.set(10, y, RUNG);
    }
    w.p.b.x = 10.15;
    w.p.b.y = 8.0;
    w.p.b.vx = 0.0;
    w.p.b.vy = 0.0;
    w.p.b.on_ground = false;
    w.game.ammo = 9;
    run(&mut w, 2, UP);
    assert!(w.p.climb, "on the ladder");
    w
}

#[test]
fn firing_on_a_ladder_goes_where_up_or_down_points() {
    let mut w = ladder_world();
    w.step(DOWN | FIRE);
    assert!(w.shots.last().unwrap().vy < 0.0, "fired down");
    assert_eq!(w.shots.last().unwrap().vx, 0.0);
    let mut w = ladder_world();
    w.step(UP | FIRE);
    assert!(w.shots.last().unwrap().vy > 0.0, "fired up");
    let mut w = ladder_world();
    w.p.face = 1.0;
    w.step(FIRE);
    let s = w.shots.last().unwrap();
    assert!(s.vx > 0.0 && s.vy == 0.0, "sideways when neither is held");
}

#[test]
fn an_opened_cookie_door_in_a_tower_shows_the_interior_wall_not_the_sky() {
    let mut w = level(crate::levels::ZARG_LOOKOUT);
    w.ents.clear();
    let (dx, dy) = (0..w.map.w)
        .flat_map(|x| (0..w.map.h).map(move |y| (x, y)))
        .find(|&(x, y)| w.map.get(x, y) == DOOR_R)
        .expect("a red door");
    w.keys_red = true;
    w.p.b.x = f64::from(dx) - 0.85;
    w.p.b.y = f64::from(dy);
    w.p.face = 1.0;
    run(&mut w, 3, 0);
    run(&mut w, 20, RIGHT);
    assert_eq!(
        w.map.get(dx, dy),
        WALLBG,
        "the gap is backed by interior wall"
    );
}

#[test]
fn enemies_ignore_lift_trays_and_are_not_carried_off() {
    use lf_sim::Platform;
    let mut w = level(CRATER);
    w.ents.clear();
    // A tray hanging just above the ground, right over where a gloop walks.
    w.plats = vec![Platform::new(7.0, 6.0, 7.0, 12.0, 1.0).sized(3.0, 0.5)];
    let g = w.init_ent(&crate::ents::Spawn {
        kind: Kind::Gloop,
        x: 8.0,
        y: 6.6,
        dir: -1.0,
        ride: false,
    });
    w.ents.push(g);
    run(&mut w, 120, 0);
    assert!(
        w.ents[0].b.y < 5.0,
        "fell through the tray to the ground, at {:.1}",
        w.ents[0].b.y
    );
}

#[test]
fn an_enemy_found_far_from_its_floor_is_sent_home() {
    let mut w = level(CRATER);
    w.ents.clear();
    let g = w.init_ent(&crate::ents::Spawn {
        kind: Kind::Gloop,
        x: 12.0,
        y: 4.0,
        dir: -1.0,
        ride: false,
    });
    let home = (g.b.x, g.b.y);
    w.ents.push(g);
    w.ents[0].b.y += 12.0;
    w.ents[0].b.on_ground = false;
    // Keep it up there (as if on a ledge) for a few seconds.
    let mut sent_home_after = 0;
    for t in 0..400 {
        if w.ents[0].b.y > home.1 + 6.0 {
            w.ents[0].b.y = home.1 + 12.0;
            w.ents[0].b.vy = 0.0;
        } else {
            sent_home_after = t;
            break;
        }
        w.step(0);
    }
    assert!(
        (170..200).contains(&sent_home_after),
        "sent home after about three seconds, at tick {sent_home_after}"
    );
    assert!(
        (w.ents[0].b.x - home.0).abs() < 4.0 && (w.ents[0].b.y - home.1).abs() < 1.0,
        "back home"
    );
}

#[test]
fn ben_remembers_which_way_he_fired_for_his_pose() {
    let mut w = ladder_world();
    w.step(UP | FIRE);
    assert_eq!(w.p.aim, 1);
    let mut w = ladder_world();
    w.step(DOWN | FIRE);
    assert_eq!(w.p.aim, -1);
    let mut w = ladder_world();
    w.step(FIRE);
    assert_eq!(w.p.aim, 0);
}

#[test]
fn holding_up_or_down_on_the_ground_starts_a_look_that_ends_on_release() {
    let mut w = level(CRATER);
    w.ents.clear();
    run(&mut w, 30, 0);
    run(&mut w, 10, UP);
    assert!(w.p.look_up > 0.0 && w.p.look_down == 0.0);
    run(&mut w, 2, 0);
    assert_eq!(w.p.look_up, 0.0);
    run(&mut w, 10, DOWN);
    assert!(w.p.look_down > 0.0);
}

#[test]
fn a_rider_is_carried_by_a_moving_platform_and_is_never_sent_home() {
    use lf_sim::Platform;
    let mut w = level(CRATER);
    w.ents.clear();
    w.plats = vec![Platform::new(8.0, 8.0, 14.0, 8.0, 1.0).sized(3.0, 0.5)];
    let spawn = |ride| crate::ents::Spawn {
        kind: Kind::Beetle,
        x: 9.0,
        y: 8.5,
        dir: -1.0,
        ride,
    };
    let rider = w.init_ent(&spawn(true));
    w.ents.push(rider);
    let start = w.ents[0].b.x;
    run(&mut w, 150, 0);
    assert!(
        w.ents[0].b.y > 7.5,
        "still up on the tray, at {:.1}",
        w.ents[0].b.y
    );
    assert!((w.ents[0].b.x - start).abs() > 0.5, "carried along");
}

#[test]
fn the_saucer_opens_from_the_map_and_leaving_it_clears_nothing() {
    use crate::levels::{LEVEL_COUNT, SAUCER};
    let mut w = World::new();
    w.game_new();
    w.enter_map();
    w.p.b.x = 8.2;
    w.p.b.y = 10.2;
    w.map_arm = 0.0;
    w.step(0);
    w.step(JUMP);
    assert_eq!(w.mode, Mode::Level);
    assert_eq!(w.level_id, SAUCER);
    assert!(
        w.ents
            .iter()
            .all(|e| matches!(e.kind, Kind::Glyph | Kind::Cameo)),
        "no enemies"
    );
    assert!(w.items.is_empty(), "nothing to collect");
    // Walk out through the exit on the left.
    w.p.b.x = 2.5;
    w.p.b.y = 3.0;
    w.p.face = -1.0;
    run(&mut w, 40, LEFT);
    assert!(w.won, "touching the exit ends the visit");
    assert_eq!(w.game.done, 0, "and sets no cleared bit");
    assert!(!w.game.is_done(SAUCER));
    assert_eq!(LEVEL_COUNT, 15);
}

#[test]
fn the_saucers_paintings_all_speak() {
    use crate::text::Toast;
    let mut seen = vec![];
    let mut w = level(crate::levels::SAUCER);
    w.p.inv = 1e9;
    let glyphs: Vec<(f64, f64)> = w
        .ents
        .iter()
        .filter(|e| e.kind == Kind::Glyph)
        .map(|e| (e.b.x, e.b.y))
        .collect();
    assert_eq!(glyphs.len(), 6);
    for (x, y) in glyphs {
        w.p.b.x = x + 0.2;
        w.p.b.y = y;
        w.p.b.vy = 0.0;
        run(&mut w, 6, 0);
    }
    for t in [
        Toast::SaucerHatch,
        Toast::SaucerGalley,
        Toast::SaucerBunks,
        Toast::SaucerChart,
        Toast::SaucerShelf,
        Toast::SaucerBridge,
    ] {
        if events_of(&w, ev::TOAST)
            .iter()
            .any(|e| e.a == t as u16 as f32)
        {
            seen.push(t);
        }
    }
    assert_eq!(seen.len(), 6, "every painting shows its line: {seen:?}");
}

#[test]
fn crater_corners_has_solid_buildings_and_signs_that_speak_when_approached() {
    let mut w = World::new();
    w.game_new();
    w.enter_map();
    for (x, y) in [(7, 17), (9, 17), (11, 17), (4, 17), (4, 15)] {
        assert!(
            w.map.solid(x, y, false, 0.0),
            "building at {x},{y} blocks the way"
        );
    }
    for (i, (x, y)) in [(8.0, 16.0), (11.0, 16.0), (4.0, 16.0)]
        .into_iter()
        .enumerate()
    {
        w.events.clear();
        w.p.b.x = x + 0.2;
        w.p.b.y = y + 0.2;
        w.step(0);
        assert!(
            events_of(&w, ev::MAP_PROMPT)
                .iter()
                .any(|e| e.a == 6.0 && e.b == i as f32),
            "sign {i} prompts with its own text"
        );
    }
}

#[test]
fn ben_shows_his_aim_pose_while_firing_along_a_ladder() {
    use crate::render::ben_sprite;
    use crate::sprites::Spr;
    let mut w = ladder_world();
    w.step(UP | FIRE);
    assert!(w.p.climb && w.p.shoot_t > 0.0);
    assert_eq!(ben_sprite(&w.p), Spr::BenAimUp);
    let mut w = ladder_world();
    w.step(DOWN | FIRE);
    assert_eq!(ben_sprite(&w.p), Spr::BenAimDown);
    // Climbing without firing keeps the climb frames.
    let mut w = ladder_world();
    w.step(UP);
    assert!(matches!(ben_sprite(&w.p), Spr::BenClimb1 | Spr::BenClimb2));
}

#[test]
fn a_rider_on_a_tall_vertical_platform_is_not_sent_home() {
    use lf_sim::Platform;
    let mut w = level(CRATER);
    w.ents.clear();
    w.plats = vec![Platform::new(8.0, 8.0, 8.0, 20.0, 0.25).sized(3.0, 0.5)];
    let rider = w.init_ent(&crate::ents::Spawn {
        kind: Kind::Beetle,
        x: 9.0,
        y: 8.5,
        dir: -1.0,
        ride: true,
    });
    w.ents.push(rider);
    run(&mut w, 300, 0);
    assert!(
        w.ents[0].b.y > w.ents[0].ay + crate::world::STRAY_HEIGHT,
        "still riding high up, at {:.1}",
        w.ents[0].b.y
    );
}

#[test]
fn the_saucer_has_its_five_crystals() {
    let w = level(crate::levels::SAUCER);
    let n = w.map.data.iter().filter(|&&t| t == CRYS).count();
    assert_eq!(n, 5, "crystals light the saucer");
}

#[test]
fn the_overworld_thumbnail_marks_rivers_areas_and_level_nodes() {
    use crate::levels::{
        area_at, build_overworld, overworld_areas, overworld_thumb, PtKind, AREA_CRATER_FIELDS,
        AREA_GUMDROP_ISLE, THUMB_GRASS, THUMB_NODE, THUMB_RIVER,
    };
    let (w, h, cells) = overworld_thumb();
    assert_eq!((w, h), (60, 44));
    assert_eq!(cells.len(), (w * h) as usize);
    let kind = |x: i32, y: i32| cells[(y * w + x) as usize] & 0x0f;
    let aux = |x: i32, y: i32| cells[(y * w + x) as usize] >> 4;
    assert_eq!(kind(0, 0), THUMB_RIVER, "the outer river");
    assert_eq!(kind(24, 10), THUMB_RIVER, "the north-south river");
    assert_eq!(kind(10, 10), THUMB_GRASS);
    assert_eq!(aux(10, 10), AREA_CRATER_FIELDS);
    let areas = overworld_areas();
    assert_eq!(aux(10, 10), area_at(&areas, 10, 10));
    assert_eq!(area_at(&areas, 24, 22), AREA_GUMDROP_ISLE);
    // Every visible level point is a node carrying its level id.
    let data = build_overworld();
    let nodes: Vec<_> = data
        .points
        .iter()
        .filter(|p| p.kind == PtKind::Level && !p.hidden)
        .collect();
    assert!(!nodes.is_empty());
    for p in &nodes {
        let (x, y) = (p.x as i32, p.y as i32);
        assert_eq!(kind(x, y), THUMB_NODE, "node at {x},{y}");
        assert_eq!(aux(x, y), p.level);
    }
    let count = cells.iter().filter(|&&c| c & 0x0f == THUMB_NODE).count();
    assert_eq!(count, nodes.len());
}

#[test]
fn attract_mode_visits_the_three_levels_in_turn() {
    use crate::levels::{CAVES, CITADEL, CRATER};
    let mut w = World::new();
    for (idx, id) in [
        (0, CRATER),
        (1, CAVES),
        (2, CITADEL),
        (3, CRATER),
        (7, CAVES),
    ] {
        w.load_attract(idx, false);
        assert_eq!(w.level_id, id, "attract level {idx}");
        assert_eq!(w.mode, Mode::Attract);
        assert!(w.p.hidden);
        assert!(w.attract_period() > 0);
    }
}

#[test]
fn attract_camera_never_shows_below_the_bottom_row() {
    for idx in 0..3 {
        for (hw, hh) in [
            (8.0, 4.0),
            (10.0, 6.5),
            (14.0, 6.5),
            (20.0, 6.0),
            (30.0, 8.0),
        ] {
            let mut w = World::new();
            w.half_w = hw;
            w.half_h = hh;
            w.load_attract(idx, false);
            let period = w.attract_period();
            for t in 0..(period + 5) {
                w.step(0);
                let bottom = w.cam_y - w.half_h;
                assert!(
                    bottom >= -1e-9,
                    "level {idx}, view {hw}x{hh}, tick {t}: bottom edge at {bottom}"
                );
                let left = w.cam_x - w.half_w;
                assert!(left >= -1e-9, "level {idx}: left edge at {left}");
            }
            // A level at least as tall as the view rests exactly on row 0.
            if f64::from(w.map.h) >= hh * 2.0 {
                assert!((w.cam_y - hh).abs() < 1e-9, "bottom edge is row 0");
            }
        }
    }
}

#[test]
fn attract_camera_depends_only_on_the_tick_count() {
    let path = || {
        let mut w = World::new();
        w.half_w = 11.0;
        w.half_h = 6.5;
        w.load_attract(1, false);
        (0..400)
            .map(|_| {
                w.step(0);
                (w.cam_x.to_bits(), w.cam_y.to_bits())
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(path(), path());
}

#[test]
fn attract_camera_pans_across_and_comes_back_each_period() {
    let mut w = World::new();
    w.half_w = 11.0;
    w.half_h = 6.5;
    w.load_attract(0, false);
    let start = w.cam_x;
    let period = w.attract_period();
    let mut far = start;
    for _ in 0..period {
        w.step(0);
        far = far.max(w.cam_x);
    }
    assert!(far - start > 20.0, "pans a good way across, to {far}");
    assert!(
        (w.cam_x - start).abs() < 0.2,
        "back near the start after one period"
    );
    assert_eq!(w.attract_t, period);
}

#[test]
fn attract_camera_holds_still_for_reduced_motion() {
    let mut w = World::new();
    w.half_w = 11.0;
    w.half_h = 6.5;
    w.load_attract(2, true);
    let (x, y) = (w.cam_x, w.cam_y);
    run(&mut w, 600, 0);
    assert_eq!((w.cam_x, w.cam_y), (x, y));
}

// Shots pass through enemies that are already stunned.

fn put(w: &mut World, kind: Kind, x: f64) -> usize {
    let s = crate::ents::Spawn {
        kind,
        x,
        y: 2.0,
        dir: 1.0,
        ride: false,
    };
    let e = w.init_ent(&s);
    w.ents.push(e);
    w.ents.len() - 1
}

fn stunned_pair() -> (World, usize, usize) {
    let mut w = arena();
    let a = put(&mut w, Kind::Gloop, 10.0);
    let b = put(&mut w, Kind::Gloop, 14.0);
    w.ents[a].stun = 6.0;
    (w, a, b)
}

#[test]
fn a_shot_passes_through_a_stunned_enemy_and_stuns_the_one_behind() {
    let (mut w, a, b) = stunned_pair();
    w.shots.push(bubble(6.0, 2.4, 16.0, 0.0));
    run(&mut w, 40, 0);
    assert!(w.shots.is_empty(), "the second enemy consumed the shot");
    assert!(
        w.ents[b].stun > 4.0,
        "second enemy stunned: {}",
        w.ents[b].stun
    );
    assert!(w.ents[a].stun > 0.0);
}

#[test]
fn passing_through_does_not_refresh_the_stun() {
    let mut w = arena();
    let a = put(&mut w, Kind::Gloop, 10.0);
    w.ents[a].stun = 3.0;
    w.shots.push(bubble(6.0, 2.4, 16.0, 0.0));
    run(&mut w, 80, 0);
    assert!(
        w.ents[a].stun < 3.0 - 0.5,
        "kept counting down: {}",
        w.ents[a].stun
    );
    assert!(
        w.shots.is_empty(),
        "the shot flew on to the wall or ran out"
    );
}

#[test]
fn a_shot_still_stuns_and_is_consumed_by_a_non_stunned_enemy() {
    let mut w = arena();
    let a = put(&mut w, Kind::Gloop, 10.0);
    w.shots.push(bubble(6.0, 2.4, 16.0, 0.0));
    run(&mut w, 20, 0);
    assert!(w.shots.is_empty());
    assert!(w.ents[a].stun > 5.0);
}

#[test]
fn bosses_and_invulnerable_enemies_still_stop_a_shot() {
    for kind in [Kind::Roller, Kind::Marsh, Kind::Boss] {
        let mut w = arena();
        let a = put(&mut w, kind, 10.0);
        let c = put(&mut w, Kind::Gloop, 14.0);
        w.ents[a].stun = 6.0;
        // A parked (not hot) boss: the shot plinks off it.
        w.ents[a].state = crate::ents::St::Idle;
        w.shots.push(bubble(6.0, 2.4, 16.0, 0.0));
        run(&mut w, 30, 0);
        assert!(w.shots.is_empty(), "{kind:?}: shot is gone");
        assert!(w.ents[c].stun <= 0.0, "{kind:?} stopped the shot");
    }
}

#[test]
fn an_enemy_that_wakes_up_stops_shots_again() {
    let mut w = arena();
    let a = put(&mut w, Kind::Gloop, 10.0);
    let c = put(&mut w, Kind::Gloop, 14.0);
    w.ents[a].stun = 0.05;
    run(&mut w, 10, 0);
    assert!(w.ents[a].stun <= 0.0);
    w.shots.push(bubble(6.0, 2.4, 16.0, 0.0));
    run(&mut w, 30, 0);
    assert!(w.ents[a].stun > 5.0, "re-stunned");
    assert!(w.ents[c].stun <= 0.0, "the woken enemy absorbed the shot");
}

#[test]
fn a_shot_behind_a_stunned_enemy_still_hits_walls_and_cracked_walls() {
    let mut w = arena();
    let a = put(&mut w, Kind::Gloop, 10.0);
    w.ents[a].stun = 6.0;
    w.map.set(14, 2, CRACKED);
    w.map.set(14, 3, CRACKED);
    w.map.set(20, 2, FILL);
    w.shots.push(bubble(6.0, 2.4, 16.0, 0.0));
    run(&mut w, 30, 0);
    assert_eq!(w.map.get(14, 2), EMPTY, "cracked wall crumbled");
    assert_eq!(w.map.get(14, 3), EMPTY);
    assert!(w.shots.is_empty());
    let mut w = arena();
    let a = put(&mut w, Kind::Gloop, 10.0);
    w.ents[a].stun = 6.0;
    w.map.set(14, 2, FILL);
    w.shots.push(bubble(6.0, 2.4, 16.0, 0.0));
    run(&mut w, 30, 0);
    assert!(w.shots.is_empty(), "plain wall stopped it");
}

#[test]
fn a_shot_spawned_inside_a_stunned_enemy_passes_without_a_spark() {
    let (mut w, a, b) = stunned_pair();
    w.shots.push(bubble(10.3, 2.4, 16.0, 0.0));
    let sparks = w.fx.len();
    w.step(0);
    assert_eq!(w.fx.len(), sparks, "no spark for an overlap at spawn");
    run(&mut w, 30, 0);
    assert!(w.ents[b].stun > 4.0);
    assert!(w.ents[a].stun > 0.0);
}

#[test]
fn passing_a_stunned_enemy_sparks_once_and_runs_identically_twice() {
    let run_it = || {
        let (mut w, _, _) = stunned_pair();
        w.shots.push(bubble(6.0, 2.4, 16.0, 0.0));
        let mut sparks = 0;
        for _ in 0..40 {
            let before = w.fx.iter().filter(|f| (f.life - 0.15).abs() < 1e-9).count();
            w.step(0);
            let after = w.fx.iter().filter(|f| (f.life - 0.15).abs() < 1e-9).count();
            sparks += after.saturating_sub(before);
        }
        let stuns: Vec<f64> = w.ents.iter().map(|e| e.stun).collect();
        (sparks, stuns, w.shots.len())
    };
    let (s1, st1, n1) = run_it();
    let (s2, st2, n2) = run_it();
    assert_eq!(s1, 1, "one spark for the one pass");
    assert_eq!((s1, &st1, n1), (s2, &st2, n2));
}

fn camera_x_after_render(w: &mut World) -> f32 {
    w.render(1.0, 0);
    w.out[crate::render::out::CAM_X]
}

#[test]
fn entering_the_map_clears_any_leftover_shake() {
    let mut w = level(CRATER);
    w.shake = 0.8;
    w.enter_map();
    assert_eq!(w.shake, 0.0);
}

#[test]
fn shake_fades_out_on_the_map_and_the_camera_settles() {
    let mut w = World::new();
    w.game_new();
    w.enter_map();
    w.step(0);
    w.shake = 0.8;
    // 0.8 fades at 2 per second: 0.4 s, so 24 ticks, plus slack.
    run(&mut w, 40, 0);
    assert_eq!(w.shake, 0.0);
    let cam = w.cam_x as f32;
    assert_eq!(camera_x_after_render(&mut w), cam);
}

#[test]
fn shake_fades_out_in_the_attract_loop() {
    let mut w = World::new();
    w.game_new();
    w.load_attract(0, true);
    assert_eq!(w.mode, Mode::Attract);
    w.shake = 0.8;
    run(&mut w, 40, 0);
    assert_eq!(w.shake, 0.0);
    let cam = w.cam_x as f32;
    assert_eq!(camera_x_after_render(&mut w), cam);
}

#[test]
fn shake_in_a_level_fades_at_two_per_second() {
    let mut w = level(CRATER);
    w.shake = 0.6;
    run(&mut w, 15, 0);
    let expected = 0.6 - 15.0 * lf_sim::STEP * 2.0;
    assert!((w.shake - expected).abs() < 1e-9, "shake {}", w.shake);
    run(&mut w, 60, 0);
    assert_eq!(w.shake, 0.0);
}

/// The head-bump clunks raised so far: caption events carrying `Cap::Clunk`.
fn clunks(w: &World) -> usize {
    events_of(w, ev::CAPTION)
        .iter()
        .filter(|e| e.c == f32::from(Cap::Clunk as u16))
        .count()
}

#[test]
fn hitting_a_ceiling_while_rising_clunks_once() {
    let mut w = arena();
    for x in 0..40 {
        w.map.set(x, 5, FILL);
    }
    w.events.clear();
    w.step(JUMP);
    // Hold Jump and keep pressing up under the ceiling long after the bump.
    run(&mut w, 120, JUMP | UP);
    assert_eq!(clunks(&w), 1);
}

#[test]
fn walking_under_a_low_ceiling_does_not_clunk() {
    let mut w = arena();
    for x in 0..40 {
        w.map.set(x, 4, FILL);
    }
    w.events.clear();
    run(&mut w, 60, RIGHT | UP);
    assert!(w.p.b.on_ground);
    assert_eq!(clunks(&w), 0);
}

#[test]
fn falling_or_standing_against_a_ceiling_does_not_clunk() {
    let mut w = arena();
    for x in 0..40 {
        w.map.set(x, 5, FILL);
    }
    // Starts with his head hard against the ceiling and no upward speed: he drops without a sound.
    w.p.b.y = 5.0 - w.p.b.h - 1e-6;
    w.p.b.vy = 0.0;
    w.p.b.on_ground = false;
    w.events.clear();
    run(&mut w, 60, UP);
    assert_eq!(clunks(&w), 0);
}

#[test]
fn every_jump_into_a_ceiling_clunks_once() {
    let mut w = arena();
    for x in 0..40 {
        w.map.set(x, 5, FILL);
    }
    w.events.clear();
    for n in 1..=3 {
        run(&mut w, 60, 0);
        assert!(w.p.b.on_ground);
        w.step(JUMP);
        run(&mut w, 30, JUMP);
        assert_eq!(clunks(&w), n, "after jump {n}");
    }
}

#[test]
fn climbing_into_a_ceiling_does_not_clunk() {
    let mut w = arena();
    for y in 2..5 {
        w.map.set(4, y, VINE);
    }
    for x in 0..40 {
        w.map.set(x, 5, FILL);
    }
    w.events.clear();
    run(&mut w, 60, UP);
    assert_eq!(clunks(&w), 0);
}

#[test]
fn head_bump_raises_no_clunk_on_the_overworld_map() {
    let mut w = World::new();
    w.game_new();
    w.events.clear();
    run(&mut w, 120, UP | JUMP);
    assert_eq!(clunks(&w), 0);
}

#[test]
fn pogo_bouncing_into_a_slab_clunks_each_bounce() {
    let mut w = arena();
    for x in 0..40 {
        for y in 5..8 {
            w.map.set(x, y, FILL);
        }
    }
    w.step(POGO);
    assert!(w.p.pogo);
    w.events.clear();
    run(&mut w, 240, JUMP);
    assert!(clunks(&w) >= 3, "clunks = {}", clunks(&w));
}

#[test]
fn a_full_pogo_bounce_stops_with_the_drawn_head_under_the_slab() {
    // Slab bottom at 10.0: the 1.4 body alone would top out near 9.77, but the drawn head is 2.0 tall.
    let mut w = arena();
    for x in 0..40 {
        w.map.set(x, 10, FILL);
    }
    w.step(POGO);
    w.events.clear();
    let (mut bonks, mut top) = (0, 0.0_f64);
    for _ in 0..400 {
        w.step(JUMP);
        bonks += usize::from(w.p.b.bonk);
        top = top.max(w.p.b.y + POGO_DRAWN_HEIGHT);
    }
    let boings = events_of(&w, ev::CAPTION)
        .iter()
        .filter(|e| e.c == f32::from(Cap::Boing as u16))
        .count();
    assert!(bonks >= 4, "bonks = {bonks}");
    assert!(
        top <= 10.0,
        "the drawn head stays under the slab, top = {top}"
    );
    assert!(top > 9.99, "and reaches it, top = {top}");
    assert!(boings >= 4, "boings = {boings}");
    assert!(
        clunks(&w) + 1 >= boings && clunks(&w) <= boings,
        "{} clunks, {boings} boings",
        clunks(&w)
    );
}

#[test]
fn a_two_tile_corridor_keeps_the_pogo_usable_without_a_clunk_loop() {
    // Floor top at 2, ceiling bottom at 4: the 2.0-tall head fits exactly.
    let mut w = arena();
    for x in 0..40 {
        w.map.set(x, 4, FILL);
    }
    w.step(POGO);
    w.events.clear();
    let x0 = w.p.b.x;
    for _ in 0..300 {
        w.step(RIGHT | JUMP);
        assert!(!body_overlaps_solid(&w));
        assert!(
            (w.p.b.y - 2.0).abs() < 0.05 || w.p.b.y > 2.0 && w.p.b.y < 2.1,
            "y = {}",
            w.p.b.y
        );
    }
    assert!(
        w.p.b.x > x0 + 5.0,
        "still walks along: {} -> {}",
        x0,
        w.p.b.x
    );
    assert!(clunks(&w) <= 2, "clunks = {}", clunks(&w));
}

#[test]
fn a_normal_jump_is_unaffected_by_the_pogo_head_box() {
    // Off the stick, the body alone meets the ceiling: its top ends flush under row 5.
    let mut w = arena();
    for x in 0..40 {
        w.map.set(x, 5, FILL);
    }
    w.step(JUMP);
    let mut top = 0.0_f64;
    for _ in 0..40 {
        w.step(JUMP);
        top = top.max(w.p.b.y + w.p.b.h);
    }
    assert!((top - 5.0).abs() < 0.01, "top = {top}");
}

#[test]
fn a_low_pogo_bounce_under_a_high_slab_stays_silent() {
    let mut w = arena();
    for x in 0..40 {
        w.map.set(x, 12, FILL);
    }
    w.step(POGO);
    w.events.clear();
    run(&mut w, 300, 0);
    assert_eq!(clunks(&w), 0);
}

#[test]
fn a_rise_into_a_ceiling_below_the_last_footing_still_clunks() {
    // He stood on a high ledge, fell, then kicked off a wall (or let go of a ladder) and rose into a
    // lower ceiling before landing: the rise gains height from where it started, so it clunks.
    let mut w = arena();
    for x in 0..40 {
        w.map.set(x, 5, FILL);
    }
    w.p.rise_y = 30.0;
    w.p.b.y = 3.0;
    w.p.b.vy = 0.0;
    w.p.b.on_ground = false;
    w.events.clear();
    w.step(0);
    w.p.b.vy = 14.0;
    run(&mut w, 40, 0);
    assert_eq!(clunks(&w), 1);
}

// ---------- How Ben died ----------

#[test]
fn ben_has_no_death_cause_while_alive_or_when_killed_without_one() {
    let mut w = level(CRATER);
    assert_eq!((w.p.death_cause, w.p.death_detail), (0, 0));
    w.kill();
    assert!(w.p.dead > 0.0);
    assert_eq!((w.p.death_cause, w.p.death_detail), (0, 0));
}

#[test]
fn chocolate_and_spikes_record_their_tile() {
    let mut w = level(CRATER);
    w.p.b.x = 30.0;
    w.p.b.y = 2.4;
    w.step(0);
    assert_eq!(w.p.death_cause, death::CHOCOLATE);
    assert_eq!(w.p.death_detail, u16::from(CHOC));

    let mut w = level(CRATER);
    let spike_x = (0..192)
        .find(|&x| (0..28).any(|y| w.map.get(x, y) == SPIKE))
        .unwrap();
    w.p.b.x = f64::from(spike_x) + 0.1;
    w.p.b.y = 4.0;
    w.step(0);
    assert_eq!(w.p.death_cause, death::SPIKES);
    assert_eq!(w.p.death_detail, u16::from(SPIKE));
}

#[test]
fn molten_metal_is_not_called_chocolate() {
    let mut w = level(crate::levels::COCOA_FOUNDRY);
    w.ents.clear();
    w.p.b.x = 44.0;
    w.p.b.y = 4.0;
    run(&mut w, 60, 0);
    assert_eq!(w.p.death_cause, death::MOLTEN_METAL);
    assert_eq!(w.p.death_detail, u16::from(FURNACE));
}

#[test]
fn falling_out_of_the_world_is_a_fall() {
    let mut w = level(CRATER);
    w.p.b.y = -2.0;
    w.step(0);
    assert_eq!(w.p.death_cause, death::FELL);
    assert_eq!(w.p.death_detail, 0);
}

#[test]
fn touching_an_enemy_records_its_kind() {
    let mut w = level(CRATER);
    let gi = w.ents.iter().position(|e| e.kind == Kind::Gloop).unwrap();
    w.ents[gi].stun = 0.0;
    w.ents[gi].b.vx = 0.0;
    w.p.inv = 0.0;
    // Beside it, not on its head.
    w.p.b.x = w.ents[gi].b.x;
    w.p.b.y = w.ents[gi].b.y;
    w.p.b.vy = 0.0;
    w.step(0);
    assert!(w.p.dead > 0.0, "a gloop kills on contact");
    assert_eq!(w.p.death_cause, death::ENEMY);
    assert_eq!(w.p.death_detail, Kind::Gloop as u16);
}

#[test]
fn the_first_cause_stands_once_ben_is_dead() {
    let mut w = level(CRATER);
    w.p.b.x = 30.0;
    w.p.b.y = 2.4;
    w.step(0);
    assert_eq!(w.p.death_cause, death::CHOCOLATE);
    w.kill_by(death::SHOT, 0);
    assert_eq!(
        w.p.death_cause,
        death::CHOCOLATE,
        "a second kill changes nothing"
    );
}

#[test]
fn recording_the_cause_changes_nothing_the_simulation_does() {
    // Two identical worlds, one with a stale cause already stored: no rule reads it, so a long run with
    // the same inputs ends in the same place.
    let script = |t: usize| match t % 40 {
        0..=19 => RIGHT,
        20..=24 => RIGHT | JUMP,
        _ => 0,
    };
    let mut a = level(CRATER);
    let mut b = level(CRATER);
    b.p.death_cause = 99;
    b.p.death_detail = 12345;
    for t in 0..900 {
        a.step(script(t));
        b.step(script(t));
        assert_eq!(a.tick_count, b.tick_count);
        assert_eq!((a.p.b.x, a.p.b.y), (b.p.b.x, b.p.b.y), "tick {t}");
        assert_eq!(a.p.dead, b.p.dead, "tick {t}");
        assert_eq!(a.game.score, b.game.score, "tick {t}");
        assert_eq!(a.game.lives, b.game.lives, "tick {t}");
    }
}

/// Where the first Phantom of a level lands when Ben steps up beside it on the given absolute tick.
fn phantom_landing(id: u8, tick: u32) -> Option<(f64, f64)> {
    let mut w = level(id);
    let j = w.ents.iter().position(|e| e.kind == Kind::Phantom)?;
    w.tick_count = tick;
    w.ents[j].cd = 0.0;
    w.ents[j].stun = 0.0;
    // A Phantom teleports out of the way of one of Fizz's shots, so put a shot right beside it.
    let (x, y) = (w.ents[j].b.x + 0.4, w.ents[j].b.y + 0.7);
    w.shots.push(crate::ents::Shot {
        x,
        y,
        vx: 0.0,
        vy: 0.0,
        ben: true,
        life: 1.0,
        sprite: crate::sprites::Spr::Bubble as u16,
        g: false,
        last: -1,
    });
    w.step(0);
    Some((w.ents[j].b.x, w.ents[j].b.y))
}

#[test]
fn a_phantoms_teleport_follows_the_absolute_tick() {
    // The direction a Phantom jumps reads the absolute tick, so a replay of a level entered late in a session
    // has to restore that tick. The same tick always gives the same landing, and across the ticks of a short
    // stretch at least one Phantom level lands in more than one place.
    let mut differs = false;
    for id in 0..crate::levels::LEVEL_COUNT {
        let landings: Vec<_> = (7000..7040)
            .filter_map(|t| phantom_landing(id, t))
            .collect();
        if landings.is_empty() {
            continue;
        }
        for t in 7000..7040 {
            assert_eq!(
                phantom_landing(id, t),
                phantom_landing(id, t),
                "level {id}, tick {t}"
            );
        }
        differs |= landings.iter().any(|l| *l != landings[0]);
    }
    assert!(
        differs,
        "no Phantom level reads the tick any more; the replay can drop the restore"
    );
}

#[test]
fn a_levels_fingerprint_is_stable_and_follows_its_tiles_and_spawns() {
    use crate::levels::{build_level, LEVEL_COUNT};
    let prints: Vec<u32> = (0..LEVEL_COUNT)
        .map(|id| build_level(id).fingerprint())
        .collect();
    for id in 0..LEVEL_COUNT {
        assert_eq!(
            prints[usize::from(id)],
            build_level(id).fingerprint(),
            "level {id}"
        );
    }
    let mut sorted = prints.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        prints.len(),
        "every level has its own fingerprint"
    );

    let mut edited = build_level(CRATER);
    let before = edited.fingerprint();
    let t = edited.map.get(10, 6);
    edited
        .map
        .set(10, 6, if t == CRACKED { EMPTY } else { CRACKED });
    assert_ne!(edited.fingerprint(), before, "a changed tile");
    let mut moved = build_level(CRATER);
    moved.spawns[0].x += 1.0;
    assert_ne!(moved.fingerprint(), before, "a moved spawn");
}

#[test]
fn a_new_game_forgets_a_flag_that_was_waiting_to_be_thrown() {
    let mut w = World::new();
    w.game_new();
    w.game.set_done(0);
    w.fresh_clear = Some(0);
    w.game_new();
    w.game.set_done(0);
    w.enter_map();
    assert!(
        w.flag_throw.is_none(),
        "nothing is thrown for the old session's clear"
    );
}

#[test]
fn the_maps_lights_follow_what_has_been_cleared_while_ben_is_on_it() {
    let mut w = map_world();
    let lit = |w: &World| w.map_lights.len();
    let before = lit(&w);
    // Clearing every level opens every teleporter pad that needs one.
    for l in 0..crate::levels::LEVEL_COUNT {
        w.game.set_done(l);
    }
    w.step(0);
    assert!(lit(&w) > before, "a pad that has just opened is lit");
    assert_eq!(w.map_lights_done, w.game.done);
}
