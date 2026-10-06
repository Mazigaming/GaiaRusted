// Catching panics: payloads, hooks, nesting, resuming, and threads that
// panic, reported through `join`.
use std::any::Any;
use std::panic;
use std::thread;

fn checked_tenfold(n: i32) -> i32 {
    if n > 2 {
        panic!("too big: {}", n);
    }
    if n < 0 {
        panic!("negative");
    }
    n * 10
}

fn describe(payload: &Box<dyn Any + Send>) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        format!("str {}", text)
    } else if let Some(text) = payload.downcast_ref::<String>() {
        format!("string {}", text)
    } else if let Some(code) = payload.downcast_ref::<i32>() {
        format!("code {}", code)
    } else {
        "unknown".to_string()
    }
}

fn main() {
    panic::set_hook(Box::new(|info| {
        let message = info.payload().downcast_ref::<&str>().copied();
        let message = message.or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()));
        println!("hook: {}", message.unwrap_or("?"));
    }));
    for n in [1, 3, -1] {
        match panic::catch_unwind(|| checked_tenfold(n)) {
            Ok(value) => println!("ok {}", value),
            Err(payload) => println!("caught {}", describe(&payload)),
        }
    }

    let mut counter = 0;
    let outcome = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        counter += 1;
        let inner = panic::catch_unwind(|| -> i32 { panic::panic_any(42) });
        counter += 10;
        if let Err(payload) = inner {
            println!("inner {}", describe(&payload));
        }
        let empty: Vec<i32> = Vec::new();
        empty[counter as usize]
    }));
    println!("counter {} {}", counter, outcome.is_err());
    println!("{}", panic::catch_unwind(|| None::<i32>.unwrap()).is_err());

    let _ = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let handles: Vec<_> = (0..4)
        .map(|i| thread::spawn(move || if i % 2 == 1 { panic!("thread {}", i) } else { i * 100 }))
        .collect();
    for handle in handles {
        match handle.join() {
            Ok(value) => println!("joined {}", value),
            Err(payload) => println!("thread failed: {}", describe(&payload)),
        }
    }

    let total: u64 = (0..1000u64)
        .map(|i| panic::catch_unwind(move || if i % 7 == 0 { panic!("multiple of seven") } else { i }).unwrap_or(0))
        .sum();
    println!("{}", total);
    let resumed = panic::catch_unwind(|| {
        panic::resume_unwind(Box::new(String::from("resumed")));
    });
    println!("{} {}", describe(&resumed.unwrap_err()), thread::current().name().unwrap_or("none"));
}
