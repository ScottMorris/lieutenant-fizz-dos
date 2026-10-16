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
