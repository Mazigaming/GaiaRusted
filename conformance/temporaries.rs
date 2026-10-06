// When temporaries are dropped: at the end of the statement that made
// them, in reverse order, unless a `let` binds a reference to them.
struct Noisy(&'static str);

impl Noisy {
    fn name(&self) -> &str {
        self.0
    }
}

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn make(name: &'static str) -> Noisy {
    println!("make {}", name);
    Noisy(name)
}

fn shout(text: &str) -> String {
    text.to_uppercase()
}

fn main() {
    println!("{} {}", make("a").name(), make("b").name());
    println!("after print");

    let length = make("c").name().len();
    println!("length {}", length);

    let kept = &make("d");
    println!("kept {}", kept.name());

    let text = String::from("  padded  ");
    println!("[{}] [{}]", String::from("  inner ").trim(), shout(text.trim()));
    assert_eq!(String::from(" x ").trim(), "x");

    match make("e").name() {
        "e" => println!("matched e"),
        _ => println!("no match"),
    }
    println!("after match");

    let message = format!("{}-{}", make("f").name(), make("g").name());
    println!("{}", message);

    if make("h").name() == "h" {
        println!("in if");
    }
    println!("end");
}
