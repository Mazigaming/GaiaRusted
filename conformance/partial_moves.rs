// Moving one field out of a variable or a temporary: the other fields are
// still dropped, when their owner's scope ends, and the moved one is not.
struct Noisy(&'static str);
impl Drop for Noisy {
    fn drop(&mut self) { println!("  drop {}", self.0); }
}
struct Person { name: Noisy, pet: Noisy, home: (Noisy, Noisy) }

fn person(tag: &'static str) -> Person {
    let _ = tag;
    Person { name: Noisy("name"), pet: Noisy("pet"), home: (Noisy("street"), Noisy("city")) }
}

fn take(n: Noisy) { println!("  took {}", n.0); }

fn main() {
    println!("variable field");
    {
        let p = person("a");
        let name = p.name;
        println!("  moved {}", name.0);
    }
    println!("nested field");
    {
        let p = person("b");
        take(p.home.1);
        println!("  end of block");
    }
    println!("conditional");
    for round in 0..2 {
        let p = person("c");
        if round == 0 {
            take(p.pet);
        }
        println!("  round {} ends", round);
    }
    println!("refill");
    {
        let mut p = person("d");
        take(p.name);
        p.name = Noisy("new name");
        p.pet = Noisy("new pet");
        println!("  refilled");
    }
    println!("temporary");
    {
        let city = person("e").home.1;
        println!("  have {}", city.0);
    }
    println!("tuple");
    {
        let t = (Noisy("left"), Noisy("right"), 5);
        let right = t.1;
        println!("  {} {}", right.0, t.2);
    }
    println!("loop reuse");
    let mut keep = Vec::new();
    for _ in 0..2 {
        let p = person("f");
        keep.push(p.pet);
    }
    println!("  kept {}", keep.len());
    drop(keep);
    println!("done");
}
