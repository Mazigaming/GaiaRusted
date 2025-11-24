enum Opt {
    Some(i32),
    None,
}

fn main() {
    let opt1 = Opt::Some(42);
    let opt2 = Opt::None;
    let opt3 = Opt::Some(100);

    process_option(opt1);
    process_option(opt2);
    process_option(opt3);
}

fn process_option(opt: Opt) {
    match opt {
        Opt::Some(n) => {
            println!("Got value: {}", n);
            println!("Doubled: {}", n + n);
        }
        Opt::None => {
            println!("Got nothing");
        }
    }
}
