use gaiarusted::error_codes;

fn main() {
    println!("=== Error Code Mapping Tests ===\n");
    
    let test_cases = vec![
        ("undefined variable: x", "E022"),
        ("type mismatch", "E021"),
        ("value moved", "E041"),
        ("already borrowed", "E042"),
        ("unsafe", "E082"),
    ];
    
    let mut passed = 0;
    let mut failed = 0;
    
    for (msg, expected) in test_cases {
        match error_codes::get_error_code_for_message(msg) {
            Some(code) if code == expected => {
                println!("✓ '{}' → {} (CORRECT)", msg, code);
                passed += 1;
            }
            Some(code) => {
                println!("✗ '{}' → {} (EXPECTED {})", msg, code, expected);
                failed += 1;
            }
            None => {
                println!("✗ '{}' → None (EXPECTED {})", msg, expected);
                failed += 1;
            }
        }
    }
    
    println!("\n=== Error Code Lookup Tests ===\n");
    
    for code_str in &["E001", "E021", "E041", "E082", "E095", "E999"] {
        match error_codes::get_error_code(code_str) {
            Some(ec) => {
                println!("✓ {} exists: {}", code_str, ec.title);
                passed += 1;
            }
            None => {
                if *code_str == "E999" {
                    println!("✓ {} correctly returns None", code_str);
                    passed += 1;
                } else {
                    println!("✗ {} should exist!", code_str);
                    failed += 1;
                }
            }
        }
    }
    
    println!("\n=== SUMMARY ===");
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);
    println!("Total:  {}", passed + failed);
    
    if failed == 0 {
        println!("\n✓✓✓ ALL TESTS PASSED ✓✓✓");
        std::process::exit(0);
    } else {
        println!("\n✗✗✗ SOME TESTS FAILED ✗✗✗");
        std::process::exit(1);
    }
}
