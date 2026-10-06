// Test program for Data Structure Layout Optimization (v0.12.0)
// Tests struct field layouts and padding optimization

fn main() {
    println!("=== Layout Optimization Tests ===");

    // Test 1: Simple struct with different field sizes
    struct Simple {
        a: i64,
        b: i32,
    }

    let s = Simple { a: 100, b: 50 };
    println!("Simple struct - a: {}, b: {}", s.a, s.b);

    // Test 2: Struct with mixed types
    struct Mixed {
        x: i64,
        y: i32,
        z: i64,
    }

    let m = Mixed { x: 1, y: 2, z: 3 };
    println!("Mixed struct - x: {}, y: {}, z: {}", m.x, m.y, m.z);

    // Test 3: Point struct (common case)
    struct Point {
        x: i64,
        y: i64,
    }

    let p = Point { x: 10, y: 20 };
    println!("Point - x: {}, y: {}", p.x, p.y);

    // Test 4: Rectangle struct
    struct Rectangle {
        width: i64,
        height: i64,
    }

    let r = Rectangle { width: 100, height: 50 };
    println!("Rectangle - w: {}, h: {}", r.width, r.height);

    // Test 5: Complex struct with multiple field access
    struct Complex {
        id: i64,
        data: i64,
        count: i32,
    }

    let c = Complex { id: 1, data: 999, count: 42 };
    println!("Complex - id: {}, data: {}, count: {}", c.id, c.data, c.count);

    // Test 6: Array of structs
    let points = [
        Point { x: 0, y: 0 },
        Point { x: 10, y: 10 },
        Point { x: 20, y: 20 },
    ];

    let mut i = 0;
    while i < 3 {
        println!("Point[{}] - x: {}, y: {}", i, points[i].x, points[i].y);
        i = i + 1;
    }

    // Test 7: Nested struct access
    struct Container {
        name_len: i64,
        size: i32,
    }

    let container = Container { name_len: 128, size: 256 };
    println!("Container - name_len: {}, size: {}", container.name_len, container.size);

    // Test 8: Multiple structs with field updates
    struct Counter {
        value: i64,
        max: i64,
    }

    let mut counter = Counter { value: 0, max: 10 };
    let mut j = 0;
    while j < 5 {
        counter.value = counter.value + 1;
        j = j + 1;
    }
    println!("Counter - value: {}, max: {}", counter.value, counter.max);

    // Test 9: Large struct simulation
    struct Large {
        field1: i64,
        field2: i64,
        field3: i64,
        field4: i32,
        field5: i64,
    }

    let large = Large {
        field1: 1,
        field2: 2,
        field3: 3,
        field4: 4,
        field5: 5,
    };
    println!("Large - f1: {}, f2: {}, f3: {}, f4: {}, f5: {}",
        large.field1, large.field2, large.field3, large.field4, large.field5);

    // Test 10: Field access patterns (tests cache locality)
    struct Data {
        x: i64,
        y: i64,
        z: i64,
    }

    let data = Data { x: 100, y: 200, z: 300 };
    
    // Sequential access
    let sum1 = data.x + data.y;
    let sum2 = sum1 + data.z;
    println!("Sequential access sum: {}", sum2);

    // Random access pattern
    println!("Random access - z: {}, x: {}, y: {}", data.z, data.x, data.y);

    println!("\nAll layout optimization tests completed!");
}
