// Test program for Profiling & Diagnostics module (v0.12.0)
// Tests compiler diagnostics, error messages, and performance tracking

fn main() {
    // Test 1: Basic arithmetic (coverage tracking)
    let a = 10;
    let b = 20;
    let c = a + b;
    println!("Addition: {} + {} = {}", a, b, c);

    // Test 2: Conditional branches (coverage tracking)
    if a < b {
        println!("a is less than b");
    } else {
        println!("a is not less than b");
    }

    // Test 3: Loop with branches (path tracking)
    let mut i = 0;
    while i < 3 {
        if i == 0 {
            println!("First iteration");
        } else if i == 1 {
            println!("Second iteration");
        } else {
            println!("Third iteration");
        }
        i = i + 1;
    }

    // Test 4: Function calls (regression detection)
    let result1 = calculate_something(5);
    let result2 = calculate_something(10);
    let result3 = calculate_something(15);
    println!("Results: {} {} {}", result1, result2, result3);

    // Test 5: Multiple paths through function
    for_loop_test(3);

    // Test 6: Struct method calls
    struct Counter {
        value: i64,
    }

    let mut counter = Counter { value: 0 };
    let mut j = 0;
    while j < 5 {
        counter.value = counter.value + 1;
        j = j + 1;
    }
    println!("Counter final value: {}", counter.value);

    // Test 7: Nested function calls
    let nested = nested_calculation(2, 3);
    println!("Nested calculation result: {}", nested);

    println!("All profiling and diagnostics tests passed!");
}

fn calculate_something(x: i64) -> i64 {
    x * x
}

fn for_loop_test(count: i64) {
    let mut total = 0;
    let mut idx = 0;
    while idx < count {
        total = total + idx;
        idx = idx + 1;
    }
    println!("Loop total: {}", total);
}

fn nested_calculation(a: i64, b: i64) -> i64 {
    let intermediate = a + b;
    let final_result = intermediate * 2;
    final_result
}
