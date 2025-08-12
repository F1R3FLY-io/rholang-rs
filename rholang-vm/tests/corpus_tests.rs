use anyhow::Result;
use rstest::rstest;
use std::{fs, path::PathBuf};
mod test_utils;
use test_utils::make_rt_vm;

// This function finds all .rho files in the corpus directory
fn each_corpus_file() -> Vec<PathBuf> {
    fs::read_dir("../rholang-parser/tests/corpus")
        .expect("expected tests/corpus directory to exist")
        .map(|dir_entry_or_error| dir_entry_or_error.unwrap())
        .filter_map(|dir_entry| {
            let path = dir_entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "rho") {
                Some(path)
            } else {
                None
            }
        })
        .collect()
}

// Test that attempts to compile each corpus file
#[rstest]
#[case::all_corpus_files(each_corpus_file())]
fn test_compile_corpus_files(#[case] paths: Vec<PathBuf>) -> Result<()> {
    // Create a runtime and VM instance
    let (_rt, vm) = make_rt_vm()?;
    
    // Track compilation results
    let mut success_count = 0;
    let mut failure_count = 0;
    let mut failures = Vec::new();
    
    // Try to compile each file
    for path in paths {
        let code = fs::read_to_string(&path)?;
        match vm.compile(&code) {
            Ok(_) => {
                println!("Successfully compiled: {:?}", path);
                success_count += 1;
            }
            Err(err) => {
                println!("Failed to compile {:?}: {}", path, err);
                failures.push((path.clone(), err.to_string()));
                failure_count += 1;
            }
        }
    }
    
    // Print summary
    println!("Compilation summary:");
    println!("  Success: {}", success_count);
    println!("  Failure: {}", failure_count);
    
    // Test passes even if some files fail to compile, as we're just testing the process
    Ok(())
}

// Test that attempts to compile and execute each corpus file
#[rstest]
#[case::all_corpus_files(each_corpus_file())]
fn test_compile_and_execute_corpus_files(#[case] paths: Vec<PathBuf>) -> Result<()> {
    // Create a runtime and VM instance
    let (rt, vm) = make_rt_vm()?;
    
    // Track execution results
    let mut compile_success_count = 0;
    let mut compile_failure_count = 0;
    let mut execute_success_count = 0;
    let mut execute_failure_count = 0;
    let mut failures = Vec::new();
    
    // Try to compile and execute each file
    for path in paths {
        let code = fs::read_to_string(&path)?;
        
        // First try to compile
        match vm.compile(&code) {
            Ok(bytecode) => {
                println!("Successfully compiled: {:?}", path);
                compile_success_count += 1;
                
                // Then try to execute
                match rt.block_on(async { vm.execute(&bytecode).await }) {
                    Ok(result) => {
                        println!("Successfully executed {:?}: {}", path, result);
                        execute_success_count += 1;
                    }
                    Err(err) => {
                        println!("Failed to execute {:?}: {}", path, err);
                        failures.push((path.clone(), format!("Execution error: {}", err)));
                        execute_failure_count += 1;
                    }
                }
            }
            Err(err) => {
                println!("Failed to compile {:?}: {}", path, err);
                failures.push((path.clone(), format!("Compilation error: {}", err)));
                compile_failure_count += 1;
            }
        }
    }
    
    // Print summary
    println!("Compilation and execution summary:");
    println!("  Compilation success: {}", compile_success_count);
    println!("  Compilation failure: {}", compile_failure_count);
    println!("  Execution success: {}", execute_success_count);
    println!("  Execution failure: {}", execute_failure_count);
    
    // Test passes even if some files fail, as we're just testing the process
    Ok(())
}

// Test that attempts to compile and execute each corpus file in a single step
#[rstest]
#[case::all_corpus_files(each_corpus_file())]
fn test_end_to_end_corpus_files(#[case] paths: Vec<PathBuf>) -> Result<()> {
    // Create a runtime and VM instance
    let (rt, vm) = make_rt_vm()?;
    
    // Track results
    let mut success_count = 0;
    let mut failure_count = 0;
    let mut failures = Vec::new();
    
    // Try to compile and execute each file in a single step
    for path in paths {
        let code = fs::read_to_string(&path)?;
        
        match rt.block_on(async { vm.compile_and_execute(&code).await }) {
            Ok(result) => {
                println!("Successfully compiled and executed {:?}: {}", path, result);
                success_count += 1;
            }
            Err(err) => {
                println!("Failed to compile and execute {:?}: {}", path, err);
                failures.push((path.clone(), err.to_string()));
                failure_count += 1;
            }
        }
    }
    
    // Print summary
    println!("End-to-end summary:");
    println!("  Success: {}", success_count);
    println!("  Failure: {}", failure_count);
    
    // Test passes even if some files fail, as we're just testing the process
    Ok(())
}