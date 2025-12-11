//! Integration tests for the scriptable shell CLI options
//!
//! Tests the --exec/-e, --file/-f, --disassemble/-d, and --both/-b options

use std::process::Command;

/// Get project root directory (where workspace Cargo.toml is)
fn get_project_root() -> std::path::PathBuf {
    let mut path = std::env::current_dir().expect("Failed to get current dir");
    // Walk up to find the workspace root with rholang-shell directory
    while !path.join("rholang-shell").exists() {
        if !path.pop() {
            panic!("Could not find project root");
        }
    }
    path
}

/// Helper to run rhosh with arguments and capture output
fn run_rhosh(args: &[&str]) -> (String, String, bool) {
    let output = Command::new("cargo")
        .args(["run", "--bin", "rhosh", "--"])
        .args(args)
        .current_dir(get_project_root())
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    (stdout, stderr, success)
}

/// Helper to run rhosh with stdin input
fn run_rhosh_with_stdin(args: &[&str], input: &str) -> (String, String, bool) {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new("cargo")
        .args(["run", "--bin", "rhosh", "--"])
        .args(args)
        .current_dir(get_project_root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn command");

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes()).expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    (stdout, stderr, success)
}

// ============================================================================
// --exec/-e tests
// ============================================================================

#[test]
fn test_exec_simple_arithmetic() {
    let (stdout, _stderr, success) = run_rhosh(&["-e", "1 + 2"]);
    assert!(success);
    assert!(stdout.contains("3"), "Expected 3, got: {}", stdout);
}

#[test]
fn test_exec_string_literal() {
    let (stdout, _stderr, success) = run_rhosh(&["-e", r#""hello""#]);
    assert!(success);
    assert!(stdout.contains("hello"), "Expected hello, got: {}", stdout);
}

#[test]
fn test_exec_boolean() {
    let (stdout, _stderr, success) = run_rhosh(&["-e", "true"]);
    assert!(success);
    assert!(stdout.contains("true"), "Expected true, got: {}", stdout);
}

#[test]
fn test_exec_nil() {
    let (stdout, _stderr, success) = run_rhosh(&["-e", "Nil"]);
    assert!(success);
    assert!(stdout.contains("Nil"), "Expected Nil, got: {}", stdout);
}

#[test]
fn test_exec_list() {
    let (stdout, _stderr, success) = run_rhosh(&["-e", "[1, 2, 3]"]);
    assert!(success);
    assert!(stdout.contains("[1, 2, 3]"), "Expected [1, 2, 3], got: {}", stdout);
}

// ============================================================================
// --disassemble/-d tests
// ============================================================================

#[test]
fn test_disassemble_simple_arithmetic() {
    let (stdout, _stderr, success) = run_rhosh(&["-e", "1 + 2", "-d"]);
    assert!(success);
    assert!(stdout.contains("PUSH_INT"), "Expected PUSH_INT instruction, got: {}", stdout);
    assert!(stdout.contains("ADD"), "Expected ADD instruction, got: {}", stdout);
    assert!(stdout.contains("HALT"), "Expected HALT instruction, got: {}", stdout);
    // Should NOT contain the result "3" since we only disassemble
    assert!(!stdout.lines().any(|l| l.trim() == "3"), "Should not execute when -d is used");
}

#[test]
fn test_disassemble_string() {
    let (stdout, _stderr, success) = run_rhosh(&["-e", r#""hello""#, "-d"]);
    assert!(success);
    assert!(stdout.contains("PUSH_STR"), "Expected PUSH_STR instruction, got: {}", stdout);
}

#[test]
fn test_disassemble_if_else() {
    let (stdout, _stderr, success) = run_rhosh(&["-e", "if (true) { 1 } else { 2 }", "-d"]);
    assert!(success);
    assert!(stdout.contains("PUSH_BOOL"), "Expected PUSH_BOOL instruction, got: {}", stdout);
    assert!(stdout.contains("BRANCH_FALSE"), "Expected BRANCH_FALSE instruction, got: {}", stdout);
}

#[test]
fn test_disassemble_channel_operations() {
    let (stdout, _stderr, success) = run_rhosh(&["-e", "new x in { x!(42) }", "-d"]);
    assert!(success);
    assert!(stdout.contains("NAME_CREATE"), "Expected NAME_CREATE instruction, got: {}", stdout);
    assert!(stdout.contains("TELL"), "Expected TELL instruction, got: {}", stdout);
}

// ============================================================================
// --both/-b tests
// ============================================================================

#[test]
fn test_both_simple_arithmetic() {
    let (stdout, _stderr, success) = run_rhosh(&["-e", "1 + 2", "-b"]);
    assert!(success);
    // Should have both sections
    assert!(stdout.contains("=== Disassembly ==="), "Expected disassembly header, got: {}", stdout);
    assert!(stdout.contains("=== Execution ==="), "Expected execution header, got: {}", stdout);
    // Should have disassembly content
    assert!(stdout.contains("PUSH_INT"), "Expected PUSH_INT instruction, got: {}", stdout);
    assert!(stdout.contains("ADD"), "Expected ADD instruction, got: {}", stdout);
    // Should have execution result
    assert!(stdout.contains("3"), "Expected result 3, got: {}", stdout);
}

#[test]
fn test_both_complex_expression() {
    let (stdout, _stderr, success) = run_rhosh(&["-e", "(1 + 2) * (3 + 4)", "-b"]);
    assert!(success);
    assert!(stdout.contains("=== Disassembly ==="));
    assert!(stdout.contains("=== Execution ==="));
    assert!(stdout.contains("MUL"), "Expected MUL instruction, got: {}", stdout);
    assert!(stdout.contains("21"), "Expected result 21, got: {}", stdout);
}

// ============================================================================
// --file/-f tests
// ============================================================================

#[test]
fn test_file_execute() {
    let (stdout, stderr, success) = run_rhosh(&["-f", "rholang-shell/tests/examples/complex_example.rho"]);
    assert!(success, "Failed with stderr: {}", stderr);
    // Should execute and return some result
    assert!(!stdout.trim().is_empty(), "Expected some output");
}

#[test]
fn test_file_disassemble() {
    let (stdout, _stderr, success) = run_rhosh(&["-f", "rholang-shell/tests/examples/complex_example.rho", "-d"]);
    assert!(success);
    assert!(stdout.contains("NAME_CREATE"), "Expected NAME_CREATE instruction");
    assert!(stdout.contains("TELL"), "Expected TELL instruction");
    assert!(stdout.contains("ASK"), "Expected ASK instruction");
}

#[test]
fn test_file_both() {
    let (stdout, _stderr, success) = run_rhosh(&["-f", "rholang-shell/tests/examples/complex_example.rho", "-b"]);
    assert!(success);
    assert!(stdout.contains("=== Disassembly ==="));
    assert!(stdout.contains("=== Execution ==="));
}

// ============================================================================
// stdin tests
// ============================================================================

#[test]
fn test_stdin_execute() {
    let (stdout, _stderr, success) = run_rhosh_with_stdin(&[], "1 + 2");
    assert!(success);
    assert!(stdout.contains("3"), "Expected 3, got: {}", stdout);
}

#[test]
fn test_stdin_disassemble() {
    let (stdout, _stderr, success) = run_rhosh_with_stdin(&["-d"], "1 + 2");
    assert!(success);
    assert!(stdout.contains("PUSH_INT"), "Expected PUSH_INT instruction");
    assert!(stdout.contains("ADD"), "Expected ADD instruction");
}

#[test]
fn test_stdin_both() {
    let (stdout, _stderr, success) = run_rhosh_with_stdin(&["-b"], "1 + 2 * 3");
    assert!(success);
    assert!(stdout.contains("=== Disassembly ==="));
    assert!(stdout.contains("=== Execution ==="));
    assert!(stdout.contains("MUL"), "Expected MUL instruction");
    assert!(stdout.contains("7"), "Expected result 7");
}

#[test]
fn test_stdin_multiline_code() {
    let code = r#"new x in {
    x!(42) |
    for (y <- x) { y }
}"#;
    let (stdout, _stderr, success) = run_rhosh_with_stdin(&["-b"], code);
    assert!(success);
    assert!(stdout.contains("=== Disassembly ==="));
    assert!(stdout.contains("NAME_CREATE"), "Expected NAME_CREATE instruction");
    assert!(stdout.contains("TELL"), "Expected TELL instruction");
    assert!(stdout.contains("ASK"), "Expected ASK instruction");
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn test_empty_stdin() {
    let (stdout, _stderr, success) = run_rhosh_with_stdin(&[], "");
    assert!(success);
    assert!(stdout.trim().is_empty(), "Empty input should produce no output");
}

#[test]
fn test_whitespace_only_stdin() {
    let (stdout, _stderr, success) = run_rhosh_with_stdin(&[], "   \n\n   ");
    assert!(success);
    assert!(stdout.trim().is_empty(), "Whitespace-only input should produce no output");
}

#[test]
fn test_syntax_error_exec() {
    let (_stdout, stderr, _success) = run_rhosh(&["-e", "("]);
    // Should report an error
    assert!(stderr.contains("error") || stderr.contains("Error"), "Expected error message, got stderr: {}", stderr);
}
