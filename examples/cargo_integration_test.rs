use gaiarusted::{CargoProject, CargoAPI, CargoManifest};

fn main() {
    println!("=== GaiaRusted Cargo Integration Test ===\n");

    // Test 1: Parse Cargo.toml
    println!("Test 1: Parse Cargo manifest");
    let toml_content = r#"
[package]
name = "test_project"
version = "0.2.0"
edition = "2021"
authors = ["Test Author"]
description = "A test project"

[dependencies]
serde = "1.0"
tokio = "1.0"
"#;

    match CargoManifest::from_str(toml_content) {
        Ok(manifest) => {
            println!("✓ Manifest parsed successfully");
            println!("  Name: {}", manifest.name);
            println!("  Version: {}", manifest.version);
            println!("  Edition: {}", manifest.edition);
            println!("  Dependencies: {}", manifest.dependencies.len());
            for (name, _dep) in &manifest.dependencies {
                println!("    - {}", name);
            }
        }
        Err(e) => println!("✗ Failed to parse manifest: {}", e),
    }
    println!();

    // Test 2: Load project from directory
    println!("Test 2: Load project from directory");
    match CargoProject::open("../test_projects/cargo_integration_demo") {
        Ok(project) => {
            println!("✓ Project loaded successfully");
            println!("  Name: {}", project.manifest.name);
            println!("  Version: {}", project.manifest.version);
            println!("  Manifest dir: {}", project.manifest_dir.display());

            // Test 3: Get source files
            println!("\nTest 3: Discover source files");
            match project.source_files() {
                Ok(files) => {
                    println!("✓ Found {} source files:", files.len());
                    for file in &files {
                        let file_name = file.file_name().unwrap_or_default().to_string_lossy();
                        println!("  - {}", file_name);
                    }
                }
                Err(e) => println!("✗ Error: {}", e),
            }
        }
        Err(e) => println!("✗ Failed to load project: {}", e),
    }
    println!();

    // Test 4: Dependency resolution
    println!("Test 4: Dependency graph resolution");
    match CargoProject::open("../test_projects/cargo_integration_demo") {
        Ok(mut project) => {
            match project.resolve_dependencies() {
                Ok(_) => {
                    println!("✓ Dependencies resolved");
                    if let Some(graph) = &project.dependency_graph {
                        println!("  Root: {}", graph.root);
                        println!("  Total nodes: {}", graph.nodes.len());
                        for (name, node) in &graph.nodes {
                            println!("    - {} v{}", name, node.version);
                        }
                    }
                }
                Err(e) => println!("✗ Error: {}", e),
            }
        }
        Err(e) => println!("✗ Failed to load project: {}", e),
    }
    println!();

    // Test 5: List packages
    println!("Test 5: List packages in project");
    match CargoAPI::list_packages("../test_projects/cargo_integration_demo") {
        Ok(packages) => {
            println!("✓ Found {} packages:", packages.len());
            for pkg in packages {
                println!("  - {} v{}", pkg.name, pkg.version);
            }
        }
        Err(e) => println!("✗ Error: {}", e),
    }
    println!();

    println!("=== All Tests Complete ===");
}
