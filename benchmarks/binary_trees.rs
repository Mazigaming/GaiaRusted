// Allocation-heavy: build and walk many binary trees (benchmarks game).
struct Tree { left: Option<Box<Tree>>, right: Option<Box<Tree>> }
fn build(depth: u32) -> Box<Tree> {
    if depth == 0 { Box::new(Tree { left: None, right: None }) }
    else { Box::new(Tree { left: Some(build(depth - 1)), right: Some(build(depth - 1)) }) }
}
fn check(t: &Tree) -> u32 {
    1 + t.left.as_ref().map_or(0, |l| check(l)) + t.right.as_ref().map_or(0, |r| check(r))
}
fn main() {
    let max_depth = 16;
    let long_lived = build(max_depth);
    let mut depth = 4;
    while depth <= max_depth {
        let iterations = 1 << (max_depth - depth + 4);
        let mut total = 0;
        for _ in 0..iterations { total += check(&build(depth)); }
        println!("{} trees of depth {} check {}", iterations, depth, total);
        depth += 2;
    }
    println!("long lived tree check {}", check(&long_lived));
}
