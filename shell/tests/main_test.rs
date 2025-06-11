use clap::Parser;
use shell::Args;

// This test just verifies that the Args struct can be created and accessed
#[test]
fn test_args_creation() {
    let args = Args { multiline: true };
    assert!(args.multiline);
}

// Since main() is difficult to test directly, we'll test the components it uses
// The run_shell function is already tested in lib_test.rs
// The RholangFakeInterpreterProvider is already tested in providers_test.rs
// So we just need to test Args::parse, which we can do with a simple test

#[test]
fn test_args_parse() {
    // Test with no arguments (default values)
    let args = Args::try_parse_from(["program_name"]).expect("Failed to parse args");
    assert!(!args.multiline);

    // Test with multiline flag
    let args = Args::try_parse_from(["program_name", "--multiline"]).expect("Failed to parse args");
    assert!(args.multiline);

    // Test with short flag
    let args = Args::try_parse_from(["program_name", "-m"]).expect("Failed to parse args");
    assert!(args.multiline);
}
