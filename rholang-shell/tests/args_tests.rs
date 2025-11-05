use clap::Parser;
use rholang_shell::Args;

#[test]
fn test_args_parse_no_flags() {
    // Ensure Args parses successfully with no flags
    let args = Args::parse_from(["rhosh"]);
    assert!(args.load.is_none());
}

#[test]
fn test_args_parse_with_load_long() {
    let args = Args::parse_from(["rhosh", "--load", "tests/data/sample.rho"]);
    assert!(args.load.is_some());
    assert_eq!(
        args.load.unwrap().to_string_lossy(),
        "tests/data/sample.rho"
    );
}

#[test]
fn test_args_parse_with_load_short() {
    let args = Args::parse_from(["rhosh", "-l", "tests/data/sample.rho"]);
    assert!(args.load.is_some());
    assert_eq!(
        args.load.unwrap().to_string_lossy(),
        "tests/data/sample.rho"
    );
}
