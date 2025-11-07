use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Pick a free TCP port by binding to port 0 and then releasing it.
fn pick_free_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind to ephemeral port");
    let addr: SocketAddr = listener.local_addr().expect("local_addr");
    let port = addr.port();
    drop(listener);
    port
}

fn has_tool(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Simple blocking HTTP GET using std only. Returns (status_line, body).
fn http_get(host: &str, port: u16, path: &str) -> std::io::Result<(String, String)> {
    let mut stream = TcpStream::connect((host, port))?;
    stream.set_read_timeout(Some(Duration::from_millis(500)))?;
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}:{}\r\nConnection: close\r\n\r\n",
        path, host, port
    );
    stream.write_all(request.as_bytes())?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let text = String::from_utf8_lossy(&buf);
    let mut lines = text.lines();
    let status_line = lines.next().unwrap_or("").to_string();

    // Split headers/body
    let parts: Vec<&str> = text.split("\r\n\r\n").collect();
    let body = parts.get(1).unwrap_or(&"").to_string();
    Ok((status_line, body))
}

/// Integration test: run the serve script, fetch index.html, and assert page content.
///
/// Notes:
/// - Ignored by default because it depends on external tools (wasm-pack, python3) and starts a server.
/// - Run with: `cargo test -p rholang-wasm --test serve_script_tests -- --ignored --nocapture`
#[test]
#[ignore]
fn serve_wasm_script_serves_index() {
    if !has_tool("wasm-pack") {
        eprintln!("skipped: wasm-pack not found. Install with `cargo install wasm-pack`.");
        return;
    }
    if !has_tool("python3") {
        eprintln!("skipped: python3 not found in PATH.");
        return;
    }

    let port = pick_free_port();

    // Start server script
    let mut child: Child = Command::new("bash")
        .arg("../scripts/serve_wasm.sh")
        .arg("--port")
        .arg(port.to_string())
        // Avoid inheriting stdout to keep test output clean; keep stderr for debugging if it fails.
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn serve_wasm.sh");

    // Wait for server to become ready, polling index.html
    let deadline = Instant::now() + Duration::from_secs(40);
    let mut ok = false;
    while Instant::now() < deadline {
        match http_get("127.0.0.1", port, "/index.html") {
            Ok((status, body)) => {
                if status.starts_with("HTTP/1.0 200") || status.starts_with("HTTP/1.1 200") {
                    // Check for a stable marker text from the page
                    if body.contains("Rholang WebAssembly Shell") || body.contains("Rholang WASM Eval") {
                        ok = true;
                        break;
                    }
                }
            }
            Err(_) => {}
        }
        thread::sleep(Duration::from_millis(250));
    }

    // Clean up the server process
    let _ = child.kill();
    let _ = child.wait();

    assert!(ok, "server did not serve expected index.html content on time");
}
