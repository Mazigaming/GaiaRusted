// Graph search with a queue: shortest paths through a generated maze.
use std::collections::VecDeque;
fn main() {
    let (w, h) = (2000usize, 2000usize);
    let mut seed: u32 = 7;
    let mut wall = vec![false; w * h];
    for cell in wall.iter_mut() {
        seed ^= seed << 13; seed ^= seed >> 17; seed ^= seed << 5;
        *cell = seed % 100 < 28;
    }
    wall[0] = false;
    let mut dist = vec![u32::MAX; w * h];
    let mut queue = VecDeque::new();
    dist[0] = 0;
    queue.push_back(0usize);
    while let Some(at) = queue.pop_front() {
        let (x, y) = (at % w, at / w);
        let d = dist[at] + 1;
        let mut visit = |next: usize| { if !wall[next] && dist[next] == u32::MAX { dist[next] = d; queue.push_back(next); } };
        if x > 0 { visit(at - 1); }
        if x + 1 < w { visit(at + 1); }
        if y > 0 { visit(at - w); }
        if y + 1 < h { visit(at + w); }
    }
    let reached = dist.iter().filter(|&&d| d != u32::MAX).count();
    let farthest = dist.iter().filter(|&&d| d != u32::MAX).max().unwrap();
    println!("reached {} farthest {}", reached, farthest);
}
