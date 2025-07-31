use std::{fs, path::PathBuf};

fn main() {
    divan::main();
}

#[divan::bench(args = each_corpus_file())]
fn parsing(arg: &PathBuf) {
    let code = fs::read_to_string(arg).expect("expected a readable file");
    let parser = rholang_parser::RholangParser::new();
    let result = parser.parse(&code);
    divan::black_box_drop(result);
}

fn each_corpus_file() -> impl Iterator<Item = PathBuf> {
    fs::read_dir("tests/corpus")
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
