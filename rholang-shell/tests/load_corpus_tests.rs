use anyhow::Result;
use std::io::Cursor;
use std::path::Path;

use rholang_shell::{
    process_special_command,
    providers::FakeInterpreterProvider,
};

// Helper to collect top-level corpus .rho files from rholang-parser
fn corpus_files() -> Vec<&'static str> {
    // Keep the list minimal and stable: only include files that live at the top-level of tests/corpus
    // (no subdirectories), to avoid platform-specific path handling in CI.
    vec![
        "../rholang-parser/tests/corpus/bank_contract.rho",
        "../rholang-parser/tests/corpus/kv_store.rho",
        "../rholang-parser/tests/corpus/sending_receiving_multiple.rho",
        "../rholang-parser/tests/corpus/shortfast.rho",
        "../rholang-parser/tests/corpus/shortslow.rho",
        "../rholang-parser/tests/corpus/simpleInsertCall.rho",
        "../rholang-parser/tests/corpus/simpleInsertTest.rho",
        "../rholang-parser/tests/corpus/simpleLookupTest.rho",
        "../rholang-parser/tests/corpus/stderr.rho",
        "../rholang-parser/tests/corpus/stdout.rho",
    ]
}

#[tokio::test]
async fn test_load_corpus_files() -> Result<()> {
    let interpreter = FakeInterpreterProvider;

    for file in corpus_files() {
        // Ensure the file actually exists relative to this crate (test will fail early otherwise)
        assert!(Path::new(file).exists(), "Corpus file missing: {}", file);

        let mut buffer: Vec<String> = Vec::new();
        let mut stdout = Cursor::new(Vec::new());

        let cmd = format!(".load {}", file);
        let should_exit = process_special_command(
            &cmd,
            &mut buffer,
            
            &mut stdout,
            |_| Ok(()),
            &interpreter,
        )?;

        assert!(!should_exit, ".load should not cause exit: {}", file);
        assert!(
            !buffer.is_empty(),
            "Buffer should be populated after loading file: {}",
            file
        );

        // Verify output mentions loading
        stdout.set_position(0);
        let output = String::from_utf8(stdout.into_inner())?;
        assert!(
            output.contains("Loaded"),
            "Output should confirm loading for file: {} (was: {:?})",
            file,
            output
        );
    }

    Ok(())
}
