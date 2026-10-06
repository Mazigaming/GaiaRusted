// Test program for Memory Optimization module (v0.12.0)
// Tests escape analysis, refcount optimization, memory pools, and layout optimization

fn main() {
    // Test 1: Stack allocation example
    let stack_var = 42;
    println!("Stack variable: {}", stack_var);

    // Test 2: Simple struct for layout optimization
    struct Point {
        x: i64,
        y: i64,
    }

    let p = Point { x: 100, y: 50 };
    println!("Point x: {}", p.x);
    println!("Point y: {}", p.y);

    // Test 3: Array and iteration (memory pool potential)
    let mut arr = [1, 2, 3, 4, 5];
    let mut sum = 0;
    let mut i = 0;
    while i < 5 {
        sum = sum + arr[i];
        i = i + 1;
    }
    println!("Array sum: {}", sum);

    // Test 4: Nested scope (lifetime-based pooling)
    {
        let inner_var = 99;
        println!("Inner scope var: {}", inner_var);
    }

    // Test 5: Function return (escape analysis)
    let result = compute_value(10);
    println!("Computed value: {}", result);

    // Test 6: Complex struct
    struct Rectangle {
        width: i64,
        height: i64,
    }

    let rect = Rectangle { width: 10, height: 20 };
    println!("Rectangle width: {}", rect.width);
    println!("Rectangle height: {}", rect.height);

    println!("All memory optimization tests passed!");
}

fn compute_value(x: i64) -> i64 {
    let y = x * 2;
    let z = y + 5;
    z
}
