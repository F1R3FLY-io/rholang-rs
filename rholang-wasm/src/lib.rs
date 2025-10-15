#![cfg(target_arch = "wasm32")]

use wasm_bindgen::prelude::*;

/// Minimal WASM facade crate for the Rholang web demo.
/// Currently the browser UI talks to the server via /api/run. We still
/// expose a tiny API so wasm-pack produces a JS glue package.
#[wasm_bindgen]
pub fn help_message() -> String {
    "Available commands:\n  .help\n  .mode\n  .list\n  .delete or .del\n  .reset or Ctrl+C\n  .ps\n  .kill <index>\n  .quit".to_string()
}
