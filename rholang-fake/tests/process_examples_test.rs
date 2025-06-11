use anyhow::Result;
use rholang_fake::{FakeRholangInterpreter, InterpretationResult};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Find all Rholang files (*.rho) in a directory and its subdirectories
fn find_rholang_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // Recursively search subdirectories
                let mut subdirectory_files = find_rholang_files(&path)?;
                result.append(&mut subdirectory_files);
            } else if let Some(extension) = path.extension() {
                // Check if the file has a .rho extension
                if extension == "rho" {
                    result.push(path);
                }
            }
        }
    }
    Ok(result)
}

/// Read the content of a file
fn read_file(path: &Path) -> Result<String> {
    Ok(fs::read_to_string(path)?)
}

#[tokio::test]
async fn test_process_examples() -> Result<()> {
    // Create the interpreter
    let mut interpreter = FakeRholangInterpreter::new()?;

    // Find all Rholang files in the examples directory
    let current_dir = env::current_dir()?;
    // Look for the workspace Cargo.toml file, which should be in the project root
    let project_root = current_dir
        .ancestors()
        .find(|p| {
            let cargo_toml = p.join("Cargo.toml");
            if cargo_toml.exists() {
                if let Ok(content) = fs::read_to_string(&cargo_toml) {
                    content.contains("[workspace]")
                } else {
                    false
                }
            } else {
                false
            }
        })
        .unwrap_or(&current_dir);
    let examples_dir = project_root.join("rholang-fake").join("examples");
    println!("Looking for Rholang files in: {}", examples_dir.display());
    let rholang_files = find_rholang_files(&examples_dir)?;

    // Make sure we found some files
    assert!(
        !rholang_files.is_empty(),
        "No Rholang files found in examples directory"
    );

    // Process each file
    let mut success_count = 0;
    let mut error_count = 0;

    for file in rholang_files {
        // Read the file content
        let content = match read_file(&file) {
            Ok(content) => content,
            Err(e) => {
                println!("Failed to read file {}: {}", file.display(), e);
                error_count += 1;
                continue;
            }
        };

        // Process the content using the interpreter
        match interpreter.interpret_async(&content).await {
            InterpretationResult::Success(_result) => {
                println!("Successfully interpreted file: {}", file.display());
                success_count += 1;
            }
            InterpretationResult::Error(e) => {
                println!("Failed to interpret file {}: {}", file.display(), e);
                error_count += 1;
            }
        }
    }

    println!(
        "Processing complete. Successfully processed {} files, failed to process {} files",
        success_count, error_count
    );

    // Make sure we processed at least some files successfully
    assert!(success_count > 0, "No files were processed successfully");

    Ok(())
}

// Test with a specific example file that we know should work
#[tokio::test]
async fn test_process_hello_world() -> Result<()> {
    // Create the interpreter
    let mut interpreter = FakeRholangInterpreter::new()?;

    // Path to the hello world example
    let current_dir = env::current_dir()?;
    // Look for the workspace Cargo.toml file, which should be in the project root
    let project_root = current_dir
        .ancestors()
        .find(|p| {
            let cargo_toml = p.join("Cargo.toml");
            if cargo_toml.exists() {
                if let Ok(content) = fs::read_to_string(&cargo_toml) {
                    content.contains("[workspace]")
                } else {
                    false
                }
            } else {
                false
            }
        })
        .unwrap_or(&current_dir);
    let file_path = project_root
        .join("rholang-fake")
        .join("examples")
        .join("tut-hello.rho");

    // Make sure the file exists
    println!(
        "Looking for hello world example at: {}",
        file_path.display()
    );
    assert!(file_path.exists(), "Hello world example file not found");

    // Read the file content
    let content = read_file(&file_path)?;

    // Process the content using the interpreter
    let result = interpreter.interpret_async(&content).await;

    match result {
        InterpretationResult::Success(output) => {
            // Check that the result is not empty
            assert!(!output.is_empty(), "Result is empty");

            // Print the result for debugging
            println!("Result: {}", output);
        }
        InterpretationResult::Error(err) => {
            panic!("Failed to interpret hello world example: {}", err);
        }
    }

    Ok(())
}
