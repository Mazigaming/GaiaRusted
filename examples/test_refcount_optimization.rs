// Test program for Reference Counting Optimization (v0.12.0)
// Tests scenarios where refcount operations can be optimized

fn main() {
    // Test 1: Simple value (no refcount needed)
    let val1 = 42;
    println!("Value 1: {}", val1);

    // Test 2: Multiple references
    let val2 = 100;
    use_twice(val2, val2);

    // Test 3: Move semantics potential
    let container = Box {
        data: 50,
    };
    println!("Container: {}", container.data);

    // Test 4: Sequential operations
    let mut sum = 0;
    sum = sum + 10;
    sum = sum + 20;
    sum = sum + 30;
    println!("Sum: {}", sum);

    // Test 5: Loop with operations
    let mut accumulator = 0;
    let mut i = 0;
    while i < 5 {
        accumulator = accumulator + i;
        i = i + 1;
    }
    println!("Accumulator: {}", accumulator);

    // Test 6: Function chaining
    let start = 5;
    let step1 = increment(start);
    let step2 = increment(step1);
    let step3 = increment(step2);
    println!("Chained result: {}", step3);

    // Test 7: Conditional operations
    let x = 10;
    if x > 5 {
        let y = x + 5;
        println!("X+5: {}", y);
    }

    // Test 8: Struct field increments
    struct Counter {
        count: i64,
    }

    let mut counter = Counter { count: 0 };
    let mut j = 0;
    while j < 3 {
        counter.count = counter.count + 1;
        j = j + 1;
    }
    println!("Counter: {}", counter.count);

    // Test 9: Array operations
    let mut array = [1, 2, 3];
    let mut k = 0;
    while k < 3 {
        array[k] = array[k] * 2;
        k = k + 1;
    }
    println!("Array[0]: {}", array[0]);
    println!("Array[1]: {}", array[1]);
    println!("Array[2]: {}", array[2]);

    // Test 10: Nested scope cleanup
    {
        let temp = 25;
        println!("Temp: {}", temp);
    }

    // Test 11: Return value optimization
    let result = compute_chain(3);
    println!("Compute chain result: {}", result);

    println!("All refcount optimization tests passed!");
}

struct Box {
    data: i64,
}

fn use_twice(val: i64, val2: i64) {
    println!("First use: {}", val);
    println!("Second use: {}", val2);
}

fn increment(x: i64) -> i64 {
    x + 1
}

fn compute_chain(x: i64) -> i64 {
    let a = x + 1;
    let b = a + 2;
    let c = b + 3;
    c
}
