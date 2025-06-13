use anyhow::Result;
use rholang_parser::RholangParser;
use std::fs;
use std::path::Path;

fn main() -> Result<()> {
    // Create a new parser
    let mut parser = RholangParser::new()?;

    // Path to the corpus directory
    // First try to find the corpus directory relative to the current directory
    let mut corpus_dir = Path::new("rholang-parser/corpus").to_path_buf();

    // If that doesn't exist, try relative to the crate root
    if !corpus_dir.exists() {
        corpus_dir = Path::new("corpus").to_path_buf();
    }

    println!("Looking for Rholang files in: {}", corpus_dir.display());

    // Get all .rho files in the corpus directory
    let entries = fs::read_dir(&corpus_dir)?;
    let rho_files: Vec<_> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()? == "rho" {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    println!(
        "Found {} Rholang files in the corpus directory",
        rho_files.len()
    );

    // Parse each file
    let mut success_count = 0;
    let mut error_count = 0;

    for file_path in &rho_files {
        let file_name = file_path.file_name().unwrap().to_string_lossy();
        print!("Parsing {}: ", file_name);

        // Read the file content
        let content = fs::read_to_string(file_path)?;

        // Check if the code is valid
        if parser.is_valid(&content) {
            println!("Valid ✓");
            success_count += 1;
        } else {
            println!("Invalid ✗");
            error_count += 1;

            // Try to get more detailed error information
            if let rholang_parser::errors::ParseResult::Error(err) = parser.parse(&content) {
                println!("  Error: {}", err);
            }
        }
    }

    println!("\nSummary:");
    println!("  Total files: {}", rho_files.len());
    println!("  Valid: {}", success_count);
    println!("  Invalid: {}", error_count);

    Ok(())
}
