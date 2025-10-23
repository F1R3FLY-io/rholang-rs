fn main() {
    let src_dir = std::path::Path::new("src");

    let mut c_config = cc::Build::new();
    c_config.std("c11").include(src_dir);

    #[cfg(target_env = "msvc")]
    c_config.flag("-utf-8");

    // Select parser variant based on named-comments feature
    #[cfg(feature = "named-comments")]
    let parser_file = "parser.c";
    #[cfg(not(feature = "named-comments"))]
    let parser_file = "parser_without_comments.c";

    let parser_path = src_dir.join(parser_file);
    c_config.file(&parser_path);
    println!("cargo:rerun-if-changed={}", parser_path.to_str().unwrap());

    // Also watch both parser variants and grammar.js for changes
    println!("cargo:rerun-if-changed=src/parser.c");
    println!("cargo:rerun-if-changed=src/parser_without_comments.c");
    println!("cargo:rerun-if-changed=grammar.js");

    let scanner_path = src_dir.join("scanner.c");
    if scanner_path.exists() {
        c_config.file(&scanner_path);
        println!("cargo:rerun-if-changed={}", scanner_path.to_str().unwrap());
    }

    c_config.compile("tree-sitter-rholang");
}
