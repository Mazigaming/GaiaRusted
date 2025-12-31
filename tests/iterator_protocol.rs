// Test iterator protocol implementation

fn main() {
    test_range_iteration();
    test_vector_iteration();
    test_array_iteration();
}

fn test_range_iteration() {
    println!("=== Test 1: Range Iteration ===");
    let mut sum = 0;
    for i in 0..5 {
        sum = sum + i;
    }
    println!("Sum of 0..5: {}", sum);
    
    let mut product = 1;
    for i in 1..4 {
        product = product * i;
    }
    println!("Product of 1..4: {}", product);
}

fn test_vector_iteration() {
    println!("=== Test 2: Vector Iteration ===");
    
    let mut v = Vec::new();
    v.push(1);
    v.push(2);
    v.push(3);
    
    let mut sum = 0;
    for item in v {
        sum = sum + item;
    }
    println!("Sum of vector [1, 2, 3]: {}", sum);
}

fn test_array_iteration() {
    println!("=== Test 3: Array Iteration ===");
    
    let arr = [10, 20, 30];
    
    let mut count = 0;
    for _ in arr {
        count = count + 1;
    }
    println!("Array element count: {}", count);
}
