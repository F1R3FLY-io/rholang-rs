use anyhow::{anyhow, Result};
use shell::providers::{
    InterpretationResult, InterpreterProvider, RholangParserInterpreterProvider,
};
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
    fs::read_to_string(path).map_err(|e| anyhow!("Failed to read file {}: {}", path.display(), e))
}

/// Process a Rholang file using the RholangParserInterpreterProvider
async fn process_file(interpreter: &RholangParserInterpreterProvider, path: &Path) -> Result<()> {
    println!("Processing file: {}", path.display());

    // Read the file content
    let content = read_file(path)?;

    // Process the content using the interpreter
    match interpreter.interpret(&content).await {
        InterpretationResult::Success(result) => {
            println!("Result: {}", result);
            Ok(())
        }
        InterpretationResult::Error(e) => {
            println!("Error: {}", e);
            Err(anyhow!(
                "Failed to interpret file {}: {}",
                path.display(),
                e
            ))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create the interpreter
    let interpreter = RholangParserInterpreterProvider::new()?;

    // Find all Rholang files in the corpus directory
    let examples_dir = Path::new("rholang-parser/corpus");
    let rholang_files = find_rholang_files(examples_dir)?;

    println!("Found {} Rholang files", rholang_files.len());

    // Process each file
    let mut success_count = 0;
    let mut error_count = 0;

    for file in rholang_files {
        match process_file(&interpreter, &file).await {
            Ok(_) => {
                success_count += 1;
                println!("Successfully processed file: {}", file.display());
            }
            Err(e) => {
                error_count += 1;
                println!("Failed to process file {}: {}", file.display(), e);
            }
        }
        println!("-----------------------------------");
    }

    println!(
        "Processing complete. Successfully processed {} files, failed to process {} files",
        success_count, error_count
    );

    Ok(())
}
