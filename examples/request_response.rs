enum Request {
    GetUser(i32),
    CreateUser(String),
    DeleteUser(i32),
}

enum Response {
    Success(String),
    Error(String),
    NotFound,
}

fn handle_request(req: Request) -> Response {
    match req {
        Request::GetUser(id) => {
            if id > 0 {
                Response::Success("User found")
            } else {
                Response::NotFound
            }
        }
        Request::CreateUser(name) => {
            Response::Success("User created")
        }
        Request::DeleteUser(id) => {
            if id > 0 {
                Response::Success("User deleted")
            } else {
                Response::Error("Invalid ID")
            }
        }
    }
}

fn print_response(resp: Response) {
    match resp {
        Response::Success(msg) => println!("Success: {}", msg),
        Response::Error(msg) => println!("Error: {}", msg),
        Response::NotFound => println!("Resource not found"),
    }
}

fn main() {
    let req1 = Request::GetUser(1);
    let req2 = Request::CreateUser("Alice");
    let req3 = Request::DeleteUser(2);

    let resp1 = handle_request(req1);
    let resp2 = handle_request(req2);
    let resp3 = handle_request(req3);

    print_response(resp1);
    print_response(resp2);
    print_response(resp3);
}
