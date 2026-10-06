struct Noisy(&'static str);
impl Drop for Noisy { fn drop(&mut self) { println!("drop {}", self.0); } }
fn make() -> Noisy { let _t = Noisy("temp"); Noisy("made") }
fn main() {
    let _a = Noisy("a");
    {
        let _b = Noisy("b");
        let _c = Noisy("c");
        println!("inner end");
    }
    let d = Noisy("d");
    drop(d);
    let m = make();
    println!("got {}", m.0);
    let v = vec![Noisy("v1"), Noisy("v2")];
    println!("vec built");
    drop(v);
    let boxed: Box<Noisy> = Box::new(Noisy("boxed"));
    let _moved = boxed;
    let mut slot = Some(Noisy("slot"));
    slot = None;
    println!("after slot {}", slot.is_none());
    let _shadow = Noisy("first");
    let _shadow = Noisy("second");
    println!("main end");
}
