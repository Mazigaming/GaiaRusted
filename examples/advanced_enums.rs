enum Res {
    Ok(i32),
    Err(i32),
}

enum Status {
    Active,
    Inactive,
    Pending,
}

enum Action {
    Click(i32, i32),
    Drag(i32, i32, i32, i32),
    Type(i32),
}

fn main() {
    let res = Res::Ok(200);
    let err = Res::Err(404);
    let status = Status::Active;
    let action = Action::Click(10, 20);

    match res {
        Res::Ok(code) => println!("Success: {}", code),
        Res::Err(code) => println!("Error: {}", code),
    }

    match action {
        Action::Click(x, y) => {
            println!("Clicked at ({}, {})", x, y);
        }
        Action::Drag(x1, y1, x2, y2) => {
            println!("Dragged from ({}, {}) to ({}, {})", x1, y1, x2, y2);
        }
        Action::Type(code) => {
            println!("Typed: {}", code);
        }
    }

    match status {
        Status::Active => println!("System is active"),
        Status::Inactive => println!("System is inactive"),
        Status::Pending => println!("System is pending"),
    }
}
