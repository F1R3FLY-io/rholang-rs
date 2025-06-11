fn main() {
    let src_dir = std::path::Path::new("src");

    let mut c_config = cc::Build::new();
    c_config.std("c11").include(src_dir);

    // Add the tree-sitter header files
    let tree_sitter_dir = std::path::Path::new("tree-sitter");
    if tree_sitter_dir.exists() {
        c_config.include(tree_sitter_dir);
    } else {
        // Try to find the tree-sitter header files in the cargo registry
        let cargo_manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let cargo_manifest_dir = std::path::Path::new(&cargo_manifest_dir);
        let target_dir = cargo_manifest_dir.ancestors().nth(3).unwrap().join("target");
        let tree_sitter_dir = target_dir.join("debug").join("build").join("tree-sitter-sys");
        if tree_sitter_dir.exists() {
            c_config.include(tree_sitter_dir);
        }
    }

    #[cfg(target_env = "msvc")]
    c_config.flag("-utf-8");

    let parser_path = src_dir.join("parser.c");
    c_config.file(&parser_path);
    println!("cargo:rerun-if-changed={}", parser_path.to_str().unwrap());

    // NOTE: if your language uses an external scanner, uncomment this block:
    /*
    let scanner_path = src_dir.join("scanner.c");
    c_config.file(&scanner_path);
    println!("cargo:rerun-if-changed={}", scanner_path.to_str().unwrap());
    */

    c_config.compile("tree-sitter-rholang");
}
