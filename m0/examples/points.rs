// Prints the overworld's level markers and where the player starts, to find the positions the scripted map walk steers to.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use lf_episode_1::world::World;
fn main() {
    let mut w = World::new();
    w.game_new();
    w.enter_map();
    for p in w.points.iter().take(40) {
        println!("{:?} level {} at ({:.1}, {:.1}) req {}", p.kind as u8, p.level, p.x, p.y, p.req);
    }
    println!("player at ({:.1}, {:.1})", w.p.b.x, w.p.b.y);
}
