//! Example demonstrating lifetime elision feature
//!
//! This example shows how the lifetime elision analyzer works
//! to automatically infer lifetimes in function signatures.

use gaiarusted::borrowchecker::lifetime_elision::{LifetimeElisionAnalyzer, LifetimeElisionConfig};
use gaiarusted::borrowchecker::lifetimes::LifetimeContext;
use gaiarusted::lowering::HirType;

fn main() {
    println!("=== Lifetime Elision Example ===\n");

    // Create analyzer and lifetime context
    let mut analyzer = LifetimeElisionAnalyzer::new();
    let mut lifetime_ctx = LifetimeContext::new();

    // Example 1: Rule 1 - Single input reference
    println!("Example 1: fn foo(x: &str) -> &str");
    let params = vec![HirType::Reference(Box::new(HirType::String))];
    let return_type = Some(HirType::Reference(Box::new(HirType::String)));

    let result = analyzer.infer_lifetimes(&params, &return_type, &mut lifetime_ctx);
    println!("  Applied rule: {:?}", result.rule_applied);
    println!("  Input lifetimes: {:?}", result.input_lifetimes);
    println!("  Output lifetime: {:?}", result.output_lifetime);
    println!("  → Inferred: fn foo<'a>(x: &'a str) -> &'a str\n");

    // Example 2: Rule 2 - Multiple input references
    println!("Example 2: fn bar(x: &str, y: &str) -> &str");
    let params = vec![
        HirType::Reference(Box::new(HirType::String)),
        HirType::Reference(Box::new(HirType::String)),
    ];
    let return_type = Some(HirType::Reference(Box::new(HirType::String)));

    let result = analyzer.infer_lifetimes(&params, &return_type, &mut lifetime_ctx);
    println!("  Applied rule: {:?}", result.rule_applied);
    println!("  Input lifetimes: {:?}", result.input_lifetimes);
    println!("  Output lifetime: {:?}", result.output_lifetime);
    println!("  → Inferred: fn bar<'a, 'b>(x: &'a str, y: &'b str) -> &'a str\n");

    // Example 3: Rule 3 - Method with self
    println!("Example 3: fn baz(&self) -> &str");
    let params = vec![HirType::Reference(Box::new(HirType::String))];
    let return_type = Some(HirType::Reference(Box::new(HirType::String)));

    let result = analyzer.infer_lifetimes(&params, &return_type, &mut lifetime_ctx);
    println!("  Applied rule: {:?}", result.rule_applied);
    println!("  Input lifetimes: {:?}", result.input_lifetimes);
    println!("  Output lifetime: {:?}", result.output_lifetime);
    println!("  → Inferred: fn baz<'a>(&'a self) -> &'a str\n");

    // Example 4: No elision - no return reference
    println!("Example 4: fn qux(x: &str) -> i32");
    let params = vec![HirType::Reference(Box::new(HirType::String))];
    let return_type = Some(HirType::Int32);

    let result = analyzer.infer_lifetimes(&params, &return_type, &mut lifetime_ctx);
    println!("  Applied rule: {:?}", result.rule_applied);
    println!("  Input lifetimes: {:?}", result.input_lifetimes);
    println!("  Output lifetime: {:?}", result.output_lifetime);
    println!("  → Inferred: fn qux<'a>(x: &'a str) -> i32\n");

    // Example 5: No elision - no references
    println!("Example 5: fn quux(x: i32) -> i32");
    let params = vec![HirType::Int32];
    let return_type = Some(HirType::Int32);

    let result = analyzer.infer_lifetimes(&params, &return_type, &mut lifetime_ctx);
    println!("  Applied rule: {:?}", result.rule_applied);
    println!("  Input lifetimes: {:?}", result.input_lifetimes);
    println!("  Output lifetime: {:?}", result.output_lifetime);
    println!("  → Inferred: fn quux(x: i32) -> i32\n");

    // Show all analyzed functions
    println!("=== Summary ===");
    println!(
        "Total functions analyzed: {}",
        analyzer.get_all_results().len()
    );

    for (name, result) in analyzer.get_all_results() {
        println!("  {}: {:?}", name, result.rule_applied);
    }

    println!("\n✅ Lifetime elision analysis complete!");
}
