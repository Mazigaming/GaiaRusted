// Test program for Memory Pool Allocation (v0.12.0)
// Tests lifetime-based memory allocation scenarios

fn main() {
    println!("=== Memory Pool Tests ===");

    // Function-scoped allocation
    function_scope_test();

    // Loop-scoped allocation
    loop_scope_test();

    // Block-scoped allocation
    block_scope_test();

    // Complex allocation patterns
    complex_allocation_test();

    println!("All memory pool tests completed!");
}

fn function_scope_test() {
    println!("\n--- Function Scope Test ---");
    
    // All these allocations should come from function pool
    let var1 = 10;
    let var2 = 20;
    let var3 = var1 + var2;
    
    println!("var1: {}", var1);
    println!("var2: {}", var2);
    println!("var3: {}", var3);

    struct FuncData {
        x: i64,
        y: i64,
    }

    let data = FuncData { x: 100, y: 200 };
    println!("FuncData x: {}", data.x);
    println!("FuncData y: {}", data.y);
}

fn loop_scope_test() {
    println!("\n--- Loop Scope Test ---");
    
    let mut sum = 0;
    let mut i = 0;
    
    while i < 5 {
        // Each iteration could use loop-scoped pool
        let temp = i * 2;
        sum = sum + temp;
        
        if i == 2 {
            println!("At i=2, temp={}, sum={}", temp, sum);
        }
        
        i = i + 1;
    }
    
    println!("Final sum: {}", sum);
}

fn block_scope_test() {
    println!("\n--- Block Scope Test ---");
    
    let outer = 50;
    println!("Outer: {}", outer);
    
    {
        // Inner block gets separate pool
        let inner = 25;
        println!("Inner block var: {}", inner);
    }
    
    {
        // Another block
        let another = 75;
        println!("Another block var: {}", another);
    }
    
    println!("After blocks: {}", outer);
}

fn complex_allocation_test() {
    println!("\n--- Complex Allocation Test ---");
    
    // Multi-level nesting
    let level1 = 10;
    
    {
        let level2 = level1 + 5;
        
        {
            let level3 = level2 + 5;
            
            {
                let level4 = level3 + 5;
                println!("Level 4 value: {}", level4);
            }
            
            println!("Level 3 value: {}", level3);
        }
        
        println!("Level 2 value: {}", level2);
    }
    
    println!("Level 1 value: {}", level1);
    
    // Loop with nested blocks
    let mut outer_sum = 0;
    let mut loop_idx = 0;
    
    while loop_idx < 3 {
        {
            let block_val = loop_idx * 10;
            outer_sum = outer_sum + block_val;
        }
        loop_idx = loop_idx + 1;
    }
    
    println!("Outer sum: {}", outer_sum);
    
    // Array with multiple scopes
    let arr = [1, 2, 3, 4, 5];
    let mut idx = 0;
    let mut arr_sum = 0;
    
    while idx < 5 {
        {
            let elem = arr[idx];
            arr_sum = arr_sum + elem;
        }
        idx = idx + 1;
    }
    
    println!("Array sum: {}", arr_sum);
}
