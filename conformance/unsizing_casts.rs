// Casts that unsize, as coercions do: arrays to slices and values to trait
// objects, number literals taking their default type on the way.
use std::fmt::Display;
fn main() {
    let e = &[] as &[u8];
    let s = &[1, 2, 3] as &[i32];
    let d = &5 as &dyn Display;
    let d2: &dyn Display = &2.5;
    let p = s.as_ptr() as *const u8;
    println!("{} {:?} {} {} {}", e.len(), s, d, d2, p.is_null());
}
