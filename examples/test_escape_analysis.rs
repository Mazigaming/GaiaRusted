// Test program specifically for Escape Analysis functionality (v0.12.0)
// Demonstrates stack vs heap allocation scenarios

fn main() {
    // Scenario 1: Simple stack variable (does not escape)
    let stack_only = 42;
    use_value(stack_only);

    // Scenario 2: Variable used in struct (may escape)
    struct Container {
        data: i64,
    }

    let container = Container { data: 100 };
    println!("Container data: {}", container.data);

    // Scenario 3: Multiple uses (tracks usage count)
    let multi_use = 10;
    println!("Use 1: {}", multi_use);
    println!("Use 2: {}", multi_use);
    println!("Use 3: {}", multi_use);

    // Scenario 4: Return value (escapes function)
    let returned = return_value(50);
    println!("Returned value: {}", returned);

    // Scenario 5: Nested scope tracking
    {
        let inner = 25;
        println!("Inner: {}", inner);
    }

    // Scenario 6: Field access patterns
    struct Point {
        x: i64,
        y: i64,
        z: i64,
    }

    let point = Point { x: 1, y: 2, z: 3 };
    println!("Point x: {}", point.x);
    println!("Point y: {}", point.y);

    // Scenario 7: Array iteration
    let arr = [1, 2, 3, 4, 5];
    let mut idx = 0;
    while idx < 5 {
        println!("Array[{}]: {}", idx, arr[idx]);
        idx = idx + 1;
    }

    // Scenario 8: Method calls with self
    let obj = MyObject { value: 77 };
    let extracted = obj.get_value();
    println!("Extracted: {}", extracted);

    // Scenario 9: Loop-scoped allocation
    let mut loop_sum = 0;
    let mut counter = 0;
    while counter < 10 {
        let temp = counter * 2;
        loop_sum = loop_sum + temp;
        counter = counter + 1;
    }
    println!("Loop sum: {}", loop_sum);

    // Scenario 10: Complex calculation chain
    let v1 = 5;
    let v2 = calculate_step1(v1);
    let v3 = calculate_step2(v2);
    let v4 = calculate_step3(v3);
    println!("Final result: {}", v4);

    println!("All escape analysis test scenarios completed!");
}

fn use_value(val: i64) {
    println!("Using value: {}", val);
}

fn return_value(val: i64) -> i64 {
    val + 10
}

struct MyObject {
    value: i64,
}

impl MyObject {
    fn get_value(self) -> i64 {
        self.value
    }
}

fn calculate_step1(x: i64) -> i64 {
    x * 2
}

fn calculate_step2(x: i64) -> i64 {
    x + 10
}

fn calculate_step3(x: i64) -> i64 {
    x * 3
}
