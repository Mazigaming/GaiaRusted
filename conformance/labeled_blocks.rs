// Labeled blocks, which `break 'label value` leaves; unlabeled `break` skips them.
fn classify(values: &[i32]) -> &'static str {
    'check: {
        if values.is_empty() {
            break 'check "empty";
        }
        for &value in values {
            if value < 0 {
                break 'check "has negatives";
            }
            if value == 0 {
                break;
            }
        }
        "fine"
    }
}

fn main() {
    println!("{} {} {} {}", classify(&[]), classify(&[1, -2]), classify(&[0, -1]), classify(&[3, 4]));
    let total = 15;
    let label = 'block: { if total > 10 { break 'block "big"; } "small" };
    println!("{}", label);
    let mut count = 0;
    'outer: for i in 0..4 {
        'inner: {
            if i % 2 == 0 {
                break 'inner;
            }
            count += 10;
            if i == 3 {
                break 'outer;
            }
        }
        count += 1;
    }
    println!("{}", count);
}
