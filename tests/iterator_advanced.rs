// Advanced iterator tests

fn main() {
    test_nested_iteration();
    test_break_in_loop();
    test_continue_in_loop();
    test_multiple_collections();
}

fn test_nested_iteration() {
    println!("=== Test 1: Nested Iteration ===");
    
    let mut outer_sum = 0;
    for i in 0..3 {
        let mut inner_sum = 0;
        for j in 0..2 {
            inner_sum = inner_sum + j;
        }
        outer_sum = outer_sum + inner_sum;
        println!("i={}, inner_sum={}", i, inner_sum);
    }
    println!("outer_sum: {}", outer_sum);
}

fn test_break_in_loop() {
    println!("=== Test 2: Break in Loop ===");
    
    let mut count = 0;
    for i in 0..10 {
        if i == 5 {
            break;
        }
        count = count + 1;
    }
    println!("Count before break: {}", count);
}

fn test_continue_in_loop() {
    println!("=== Test 3: Continue in Loop ===");
    
    let mut sum = 0;
    for i in 0..5 {
        if i == 2 {
            continue;
        }
        sum = sum + i;
    }
    println!("Sum (skipping i=2): {}", sum);
}

fn test_multiple_collections() {
    println!("=== Test 4: Multiple Collections ===");
    
    let mut vec1 = Vec::new();
    vec1.push(1);
    vec1.push(2);
    
    let mut vec2 = Vec::new();
    vec2.push(10);
    vec2.push(20);
    vec2.push(30);
    
    let mut total = 0;
    for v1_item in &vec1 {
        for v2_item in &vec2 {
            total = total + v1_item + v2_item;
        }
    }
    println!("Total from nested vecs: {}", total);
}
