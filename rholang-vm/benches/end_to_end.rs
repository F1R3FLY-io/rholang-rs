use std::{fs, path::PathBuf};
use tokio::runtime::Runtime;
use once_cell::sync::Lazy;

fn main() {
    divan::main();
}

#[divan::bench(args = each_valid_file())]
fn end_to_end(bencher: divan::Bencher, arg: &PathBuf) {
    let code = fs::read_to_string(arg).expect("expected a readable file");
    
    // Create VM outside the benchmark
    let vm = rholang_vm::RholangVM::new().expect("failed to create VM");
    
    // Create a runtime for executing async code
    let rt = Runtime::new().expect("failed to create runtime");
    
    bencher.bench_local(|| {
        // Compile and execute the code in the runtime
        let result = rt.block_on(async {
            vm.compile_and_execute(&code).await
        });
        divan::black_box_drop(result);
    });
}

// This function filters the corpus files to only include those that can be compiled
fn each_valid_file() -> impl Iterator<Item = &'static PathBuf> {
    // Use a thread-safe lazy initialized static variable to store the valid paths
    static VALID_FILES: Lazy<Vec<PathBuf>> = Lazy::new(|| {
        let vm = rholang_vm::RholangVM::new().expect("failed to create VM");
        
        let mut valid = Vec::new();
        for path in each_corpus_file() {
            let code = fs::read_to_string(&path).expect("expected a readable file");
            match vm.compile(&code) {
                Ok(_) => {
                    valid.push(path);
                }
                Err(err) => {
                    println!("Skipping {:?}: compilation error: {}", path, err);
                }
            }
        }
        
        valid
    });
    
    VALID_FILES.iter()
}

fn each_corpus_file() -> impl Iterator<Item = PathBuf> {
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
}