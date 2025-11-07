use std::path::Path;
use std::process::Command;

/// Integration test: build the WASM package using the repo script and
/// assert that expected artifacts are created.
///
/// Notes:
/// - This test is ignored by default because it requires external tools
///   (wasm-pack) and can take longer to run.
/// - Run with: `cargo test -p rholang-wasm --test build_script_tests -- --ignored --nocapture`
#[test]
#[ignore]
fn build_wasm_script_produces_artifacts() {
    // Ensure prerequisite: wasm-pack
    let has_wasm_pack = Command::new("wasm-pack")
        .arg("--version")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !has_wasm_pack {
        eprintln!(
            "skipped: wasm-pack not found in PATH. Install with `cargo install wasm-pack`."
        );
        return;
    }

    // Run the build script via bash from the crate root. The tests' CWD is the crate dir by default.
    let status = Command::new("bash")
        .arg("../scripts/build_wasm.sh")
        // Debug build is fine and faster; no flags needed.
        .status()
        .expect("failed to invoke build_wasm.sh");

    assert!(status.success(), "build_wasm.sh exited with non-zero status");

    // Verify artifacts exist in the default out-dir (pkg)
    let js = Path::new("pkg").join("rholang_wasm.js");
    let wasm = Path::new("pkg").join("rholang_wasm_bg.wasm");

    assert!(js.exists(), "expected JS shim not found at {:?}", js);
    assert!(wasm.exists(), "expected WASM binary not found at {:?}", wasm);
}
