static mut COUNTER: u32 = 0;
fn bump() -> u32 { unsafe { COUNTER += 1; COUNTER } }
unsafe fn read_at(p: *const i32, i: usize) -> i32 { *p.add(i) }
fn main() {
    let mut x = 10;
    let p = &mut x as *mut i32;
    unsafe { *p += 5; }
    println!("{}", x);
    let arr = [1, 2, 3, 4];
    println!("{}", unsafe { read_at(arr.as_ptr(), 2) });
    bump(); bump();
    println!("{}", bump());
    let v = vec![10u8, 20, 30];
    let s = unsafe { std::slice::from_raw_parts(v.as_ptr().add(1), 2) };
    println!("{:?}", s);
    let boxed = Box::new(77);
    let raw = Box::into_raw(boxed);
    let back = unsafe { Box::from_raw(raw) };
    println!("{}", back);
    let a: u32 = unsafe { std::mem::transmute(1.0f32) };
    println!("{:#x}", a);
    println!("{}", p.is_null());
}
