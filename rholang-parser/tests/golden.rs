use rholang_parser::RholangParser;
use rstest::rstest;
use std::fs;
use std::path::PathBuf;

#[rstest]
fn golden_test(
    #[base_dir = "tests/corpus/"]
    #[files("*.rho")]
    path: PathBuf,
) {
    let mut settings = insta::Settings::new();
    settings.set_snapshot_path("corpus/golden_snapshots");

    settings.bind(|| {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();

        let parser = RholangParser::new();
        let input = fs::read_to_string(path).expect("Failed to read input file");
        let result = parser.parse(input.as_str());

        // Each snapshot is named after the input file
        insta::assert_debug_snapshot!(name, result);
    })
}
