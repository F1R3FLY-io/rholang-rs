use rholang_shell::providers::{InterpreterProvider, RholangParserInterpreterProvider};
use rstest::rstest;
use std::fs;
use std::path::PathBuf;

#[rstest]
fn golden_shell_test(
    #[base_dir = "../rholang-parser/tests/corpus/"]
    #[files("*.rho")]
    path: PathBuf,
) {
    let mut settings = insta::Settings::new();
    // Store snapshots within the shell crate under tests/corpus/golden_snapshots
    settings.set_snapshot_path("corpus/golden_snapshots");

    settings.bind(|| {
        let name = path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();

        let input = fs::read_to_string(&path).expect("Failed to read input file");

        // Run the rholang-shell interpreter provider which should print the same AST snapshot
        // format as the parser golden tests
        let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        let output_or_err = rt.block_on(async {
            let interpreter =
                RholangParserInterpreterProvider::new().expect("Failed to create interpreter");
            match interpreter.interpret(input.as_str()).await {
                rholang_shell::providers::InterpretationResult::Success(out) => out,
                rholang_shell::providers::InterpretationResult::Error(err) => {
                    format!("Error: {}", err)
                }
            }
        });

        // Each snapshot is named after the input file
        insta::assert_snapshot!(name, output_or_err);
    })
}
