use std::{fs, path::PathBuf};

fn main() {
    divan::main();
}

#[divan::bench(args = each_corpus_file())]
fn compilation(bencher: divan::Bencher, arg: &PathBuf) {
    let code = fs::read_to_string(arg).expect("expected a readable file");
    bencher.bench_local(|| {
        let vm = rholang_vm::RholangVM::new().expect("failed to create VM");
        let result = vm.compile(&code);
        divan::black_box_drop(result);
    });
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