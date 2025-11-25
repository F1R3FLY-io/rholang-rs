use clap::Parser;
use rholang_shell::Args;

// This test verifies that Args struct can be created and parsed without flags
#[test]
fn test_args_creation_and_parse() {
    let _args = Args { load: None, exec: None, file: None };
    let _parsed = Args::try_parse_from(["program_name"]).expect("Failed to parse args");
}
