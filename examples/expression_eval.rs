enum Expression {
    Literal(i32),
    Add(i32, i32),
    Subtract(i32, i32),
    Multiply(i32, i32),
}

fn main() {
    let expr1 = Expression::Literal(42);
    let expr2 = Expression::Add(10, 20);
    let expr3 = Expression::Multiply(5, 6);

    let result1 = eval(expr1);
    let result2 = eval(expr2);
    let result3 = eval(expr3);

    println!("Result 1: {}", result1);
    println!("Result 2: {}", result2);
    println!("Result 3: {}", result3);
}

fn eval(expr: Expression) -> i32 {
    match expr {
        Expression::Literal(n) => n,
        Expression::Add(a, b) => a + b,
        Expression::Subtract(a, b) => a - b,
        Expression::Multiply(a, b) => a * b,
    }
}
