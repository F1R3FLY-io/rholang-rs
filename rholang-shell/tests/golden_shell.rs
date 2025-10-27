use rholang_shell::providers::{InterpreterProvider, RholangParserInterpreterProvider};
use rstest::rstest;
use std::fs;
use std::path::PathBuf;

fn strip_sourcepos(input: &str) -> String {
    // Remove insta front-matter header if present
    let mut start = 0usize;
    if input.starts_with("---") {
        if let Some(idx) = input.find("\n---\n") {
            start = idx + 5; // skip trailing ---\n
        }
    }
    let content = &input[start..];

    let mut out = String::with_capacity(content.len());
    let mut skip_depth: i32 = 0;
    for line in content.lines() {
        if skip_depth > 0 {
            let opens = line.matches('{').count() as i32;
            let closes = line.matches('}').count() as i32;
            skip_depth += opens - closes;
            if skip_depth <= 0 {
                skip_depth = 0;
            }
            continue;
        }
        let trimmed = line.trim_start();
        if trimmed.contains("SourcePos {") || trimmed.starts_with("span: SourceSpan {") {
            let opens = line.matches('{').count() as i32;
            let closes = line.matches('}').count() as i32;
            skip_depth = 1 + (opens - closes - 1).max(0);
            continue;
        }
        if trimmed.starts_with("pos:")
            || trimmed.starts_with("start: SourcePos")
            || trimmed.starts_with("end: SourcePos")
            || trimmed.starts_with("span:")
        {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    if out.ends_with('\n') {
        out.pop();
    }
    out
}

#[rstest]
fn golden_shell_test(
    #[base_dir = "../rholang-parser/tests/corpus/"]
    #[files("*.rho")]
    path: PathBuf,
) {
    let name = path.file_stem().unwrap().to_string_lossy().to_string();

    let input = fs::read_to_string(&path).expect("Failed to read input file");

    // Run the rholang-shell interpreter provider which should print the same AST snapshot
    // format as the parser golden tests, but without source positions
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

    // Load corresponding parser golden snapshot and compare after stripping source positions
    let golden_path = format!(
        "../rholang-parser/tests/corpus/golden_snapshots/golden__{}.snap",
        name
    );
    let golden = fs::read_to_string(&golden_path)
        .unwrap_or_else(|e| panic!("Failed to read golden snapshot {}: {}", golden_path, e));

    let expected = strip_sourcepos(&golden);
    let actual = strip_sourcepos(&output_or_err);

    assert_eq!(
        actual.trim(),
        expected.trim(),
        "Output mismatch for {}",
        name
    );
}
