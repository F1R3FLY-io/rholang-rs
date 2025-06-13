use anyhow::Result;
use rholang_parser::RholangParser;
use rstest::rstest;
use std::fs;
use std::path::{Path, PathBuf};

// Helper function to find the corpus directory
fn find_corpus_dir() -> PathBuf {
    // First try to find the corpus directory relative to the project root
    let mut corpus_dir = Path::new("rholang-parser/corpus").to_path_buf();

    // If that doesn't exist, try relative to the crate root
    if !corpus_dir.exists() {
        corpus_dir = Path::new("corpus").to_path_buf();
    }

    // If that still doesn't exist, try one more path
    if !corpus_dir.exists() {
        corpus_dir = Path::new("../corpus").to_path_buf();
    }

    assert!(corpus_dir.exists(), "Corpus directory not found");
    corpus_dir
}

// Helper function to get all .rho files in the corpus directory
fn get_rho_files() -> Vec<PathBuf> {
    let corpus_dir = find_corpus_dir();

    // Get all .rho files in the corpus directory
    let entries = fs::read_dir(&corpus_dir).expect("Failed to read corpus directory");
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

    assert!(
        !rho_files.is_empty(),
        "No .rho files found in corpus directory"
    );

    rho_files
}

// Generate test cases for each .rho file in the corpus directory
#[rstest]
#[case::filestore("filestore.rho", false)] // This file has parsing errors
#[case::fusemap("fuseMap.rho", false)] // This file has parsing errors
#[case::loopback("loopback.rho", false)] // This file has parsing errors
#[case::tut_hello("tut-hello.rho", true)]
// Add more test cases for specific files if needed
fn test_specific_corpus_files(#[case] filename: &str, #[case] should_parse: bool) -> Result<()> {
    let mut parser = RholangParser::new()?;
    let corpus_dir = find_corpus_dir();
    let file_path = corpus_dir.join(filename);

    assert!(file_path.exists(), "File {} not found", filename);

    // Read the file content
    let content = fs::read_to_string(&file_path)?;

    // Parse the file and check the result
    let result = parser.parse(&content);

    if should_parse {
        assert!(
            result.is_success(),
            "Failed to parse {}: {:?}",
            filename,
            result
        );

        // Also check with is_valid
        assert!(
            parser.is_valid(&content),
            "is_valid returned false for {}",
            filename
        );

        // Get the tree string and check that it's not empty
        let tree_result = parser.get_tree_string(&content);
        assert!(
            tree_result.is_success(),
            "Failed to get tree string for {}: {:?}",
            filename,
            tree_result
        );

        let tree_string = tree_result.unwrap();
        assert!(
            !tree_string.is_empty(),
            "Empty tree string for {}",
            filename
        );
    } else {
        // For files that we expect to fail, we just check that they fail
        assert!(
            result.is_error(),
            "Expected {} to fail parsing, but it succeeded",
            filename
        );
    }

    Ok(())
}

// Generate test cases for all corpus files except those with known issues
fn generate_test_cases() -> Vec<PathBuf> {
    get_rho_files()
        .into_iter()
        .filter(|p| {
            let filename = p.file_name().unwrap().to_string_lossy();
            // Skip files with known issues
            filename != "filestore.rho" && filename != "fuseMap.rho" && filename != "loopback.rho"
        })
        .collect()
}

// Test for all corpus files, dynamically generated
#[rstest]
#[case::all_valid_files(generate_test_cases())]
fn test_all_corpus_files(#[case] file_paths: Vec<PathBuf>) -> Result<()> {
    let mut parser = RholangParser::new()?;

    for file_path in file_paths {
        let file_name = file_path.file_name().unwrap().to_string_lossy();

        // Read the file content
        let content = fs::read_to_string(&file_path)?;

        // Parse the file and check the result
        let result = parser.parse(&content);

        // We expect all files in the corpus to be valid Rholang
        assert!(
            result.is_success(),
            "Failed to parse {}: {:?}",
            file_name,
            result
        );

        // Also check with is_valid
        assert!(
            parser.is_valid(&content),
            "is_valid returned false for {}",
            file_name
        );

        // Get the tree string and check that it's not empty
        let tree_result = parser.get_tree_string(&content);
        assert!(
            tree_result.is_success(),
            "Failed to get tree string for {}: {:?}",
            file_name,
            tree_result
        );

        let tree_string = tree_result.unwrap();
        assert!(
            !tree_string.is_empty(),
            "Empty tree string for {}",
            file_name
        );
    }

    Ok(())
}

// Test all .rho files and report which ones parse successfully and which ones fail
#[test]
fn test_all_rho_files_with_parse_status() -> Result<()> {
    let mut parser = RholangParser::new()?;
    let rho_files = get_rho_files();

    println!("Testing {} .rho files for parsing:", rho_files.len());
    println!("----------------------------------------");

    let mut success_count = 0;
    let mut failure_count = 0;

    for file_path in rho_files {
        let file_name = file_path.file_name().unwrap().to_string_lossy();

        // Read the file content
        let content = fs::read_to_string(&file_path)?;

        // Parse the file and check the result
        let result = parser.parse(&content);

        if result.is_success() {
            println!("✅ {} - Successfully parsed", file_name);
            success_count += 1;
        } else {
            println!("❌ {} - Failed to parse", file_name);
            if let rholang_parser::errors::ParseResult::Error(err) = result {
                println!("   Error: {}", err);
            }
            failure_count += 1;
        }
    }

    println!("----------------------------------------");
    println!("Summary:");
    println!("  Total files: {}", success_count + failure_count);
    println!("  Successfully parsed: {}", success_count);
    println!("  Failed to parse: {}", failure_count);

    Ok(())
}

// Test individual files from the corpus
// This allows us to run specific tests for files that might be problematic
#[test]
fn test_parse_hello_world() -> Result<()> {
    let mut parser = RholangParser::new()?;

    // Path to the hello world example
    let mut file_path = Path::new("rholang-parser/corpus/tut-hello.rho").to_path_buf();

    // If that doesn't exist, try relative to the crate root
    if !file_path.exists() {
        file_path = Path::new("corpus/tut-hello.rho").to_path_buf();
    }

    // If that still doesn't exist, try one more path
    if !file_path.exists() {
        file_path = Path::new("../corpus/tut-hello.rho").to_path_buf();
    }

    assert!(file_path.exists(), "Hello world example not found");

    // Read the file content
    let content = fs::read_to_string(&file_path)?;

    // Parse the file and check the result
    let result = parser.parse(&content);
    assert!(result.is_success(), "Failed to parse hello world example");

    // Also check with is_valid
    assert!(
        parser.is_valid(&content),
        "is_valid returned false for hello world example"
    );

    // Get the tree string and check that it's not empty
    let tree_result = parser.get_tree_string(&content);
    assert!(
        tree_result.is_success(),
        "Failed to get tree string for hello world example"
    );

    let tree_string = tree_result.unwrap();
    assert!(
        !tree_string.is_empty(),
        "Empty tree string for hello world example"
    );

    Ok(())
}
