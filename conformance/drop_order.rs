// Destructors: when they run, in what order, and that each runs exactly once.
struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Pair {
    first: Noisy,
    second: Noisy,
}

struct Wrapper {
    label: &'static str,
    inner: Noisy,
}

impl Drop for Wrapper {
    fn drop(&mut self) {
        println!("drop wrapper {}", self.label);
    }
}

fn consume(n: Noisy) {
    println!("consuming {}", n.0);
}

fn pass_through(n: Noisy) -> Noisy {
    n
}

fn make(name: &'static str) -> Noisy {
    Noisy(name)
}

fn early(flag: bool) -> u32 {
    let _a = Noisy("early-a");
    if flag {
        let _b = Noisy("early-b");
        return 1;
    }
    let _c = Noisy("early-c");
    2
}

fn maybe(flag: bool) -> Option<Noisy> {
    let n = Noisy("maybe");
    if flag { Some(n) } else { None }
}

fn main() {
    println!("-- scope order");
    {
        let _x = Noisy("x");
        let _y = Noisy("y");
        println!("end of block");
    }

    println!("-- moves");
    let a = Noisy("a");
    consume(a);
    let b = pass_through(Noisy("b"));
    let c = b;
    println!("moved b to c: {}", c.0);

    println!("-- temporaries");
    make("temp");
    println!("after temp statement");
    let len = make("temp-len").0.len();
    println!("len {}", len);
    let _ = make("underscore");
    println!("after underscore");

    println!("-- reassignment");
    let mut slot = Noisy("old");
    slot = Noisy("new");
    println!("reassigned to {}", slot.0);

    println!("-- struct fields");
    {
        let _p = Pair { first: Noisy("first"), second: Noisy("second") };
        let _w = Wrapper { label: "w", inner: Noisy("inner") };
    }

    println!("-- early return");
    println!("{}", early(true));
    println!("{}", early(false));

    println!("-- conditional move");
    for flag in [true, false] {
        let n = Noisy("cond");
        if flag {
            consume(n);
        }
        println!("end of iteration");
    }

    println!("-- options");
    let some = maybe(true);
    let none = maybe(false);
    println!("{} {}", some.is_some(), none.is_none());
    if let Some(n) = some {
        println!("unwrapped {}", n.0);
    }

    println!("-- collections");
    {
        let mut v = vec![Noisy("v0"), Noisy("v1"), Noisy("v2")];
        let popped = v.pop();
        println!("popped {}", popped.is_some());
        v.remove(0);
        println!("removed");
        v.push(Noisy("v3"));
        println!("leaving vec scope");
    }
    {
        let boxed = Box::new(Noisy("boxed"));
        println!("in box: {}", boxed.0);
    }
    {
        let v = vec![Noisy("iter0"), Noisy("iter1"), Noisy("iter2")];
        for n in v {
            if n.0 == "iter1" {
                break;
            }
            println!("visited {}", n.0);
        }
        println!("after loop");
    }

    println!("-- explicit drop and forget");
    let d = Noisy("explicit");
    drop(d);
    println!("after drop");
    let f = Noisy("forgotten");
    std::mem::forget(f);

    println!("-- loop with break");
    let mut i = 0;
    loop {
        let _each = Noisy("each");
        i += 1;
        if i == 2 {
            break;
        }
    }

    println!("-- replace");
    let mut holder = Some(Noisy("held"));
    let taken = holder.take();
    println!("taken {}", taken.is_some());
    holder = Some(Noisy("refilled"));
    println!("end of main");
}
