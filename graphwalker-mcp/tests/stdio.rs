use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

fn send(stdin: &mut impl Write, message: Value) {
    writeln!(stdin, "{message}").expect("write MCP message");
    stdin.flush().expect("flush MCP message");
}

fn receive(stdout: &mut impl BufRead) -> (String, Value) {
    let mut line = String::new();
    stdout.read_line(&mut line).expect("read MCP response");
    assert!(!line.is_empty(), "MCP server closed before responding");
    let value = serde_json::from_str(&line).expect("stdout line must be JSON-RPC");
    (line, value)
}

#[test]
fn stdio_lifecycle_lists_and_calls_health_tool() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_graphwalker-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn graphwalker-mcp");

    let mut stdin = child.stdin.take().expect("child stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("child stdout"));
    let mut protocol_output = String::new();

    send(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "graphwalker-mcp-test", "version": "0.1.0" }
            }
        }),
    );
    let (line, initialized) = receive(&mut stdout);
    protocol_output.push_str(&line);
    assert_eq!(initialized["id"], 1);
    assert_eq!(initialized["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(
        initialized["result"]["serverInfo"]["name"],
        "graphwalker-mcp"
    );
    assert!(initialized["result"]["capabilities"]["tools"].is_object());

    send(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
    );
    send(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
    );
    let (line, tools) = receive(&mut stdout);
    protocol_output.push_str(&line);
    assert_eq!(tools["id"], 2);
    let listed = tools["result"]["tools"].as_array().expect("tools array");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["name"], "health");
    assert_eq!(listed[0]["inputSchema"]["type"], "object");

    send(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": { "name": "health", "arguments": {} }
        }),
    );
    let (line, health) = receive(&mut stdout);
    protocol_output.push_str(&line);
    assert_eq!(health["id"], 3);
    assert_eq!(health["result"]["structuredContent"]["status"], "ok");
    assert_eq!(
        health["result"]["structuredContent"]["server_version"],
        env!("CARGO_PKG_VERSION")
    );

    drop(stdin);
    stdout
        .read_to_string(&mut protocol_output)
        .expect("drain child stdout");
    let status = child.wait().expect("wait for graphwalker-mcp");
    assert!(status.success(), "server should shut down cleanly on EOF");

    for line in protocol_output.lines().filter(|line| !line.is_empty()) {
        serde_json::from_str::<Value>(line)
            .unwrap_or_else(|error| panic!("non-protocol stdout ({error}): {line:?}"));
    }

    let mut stderr = String::new();
    child
        .stderr
        .take()
        .expect("child stderr")
        .read_to_string(&mut stderr)
        .expect("read child stderr");
    assert!(stderr.is_empty(), "unexpected server diagnostics: {stderr}");
}

#[test]
fn stdio_discovery_lifecycle_supports_current_protocol() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_graphwalker-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn graphwalker-mcp");

    let mut stdin = child.stdin.take().expect("child stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("child stdout"));
    let request_meta = json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": {
            "name": "graphwalker-mcp-test",
            "version": "0.1.0"
        },
        "io.modelcontextprotocol/clientCapabilities": {}
    });

    send(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": "discover",
            "method": "server/discover",
            "params": { "_meta": request_meta.clone() }
        }),
    );
    let (_, discovered) = receive(&mut stdout);
    assert_eq!(discovered["id"], "discover");
    assert!(discovered["result"]["supportedVersions"]
        .as_array()
        .expect("supported protocol versions")
        .iter()
        .any(|version| version == "2026-07-28"));
    assert_eq!(
        discovered["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "graphwalker-mcp"
    );

    send(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": "list",
            "method": "tools/list",
            "params": { "_meta": request_meta.clone() }
        }),
    );
    let (_, tools) = receive(&mut stdout);
    assert_eq!(tools["id"], "list");
    assert_eq!(tools["result"]["tools"][0]["name"], "health");

    send(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": "health",
            "method": "tools/call",
            "params": {
                "name": "health",
                "arguments": {},
                "_meta": request_meta
            }
        }),
    );
    let (_, health) = receive(&mut stdout);
    assert_eq!(health["id"], "health");
    assert_eq!(health["result"]["structuredContent"]["status"], "ok");

    drop(stdin);
    let mut remaining_stdout = String::new();
    stdout
        .read_to_string(&mut remaining_stdout)
        .expect("drain child stdout");
    assert!(
        remaining_stdout.is_empty(),
        "unexpected additional stdout: {remaining_stdout:?}"
    );

    let status = child.wait().expect("wait for graphwalker-mcp");
    assert!(status.success(), "server should shut down cleanly on EOF");

    let mut stderr = String::new();
    child
        .stderr
        .take()
        .expect("child stderr")
        .read_to_string(&mut stderr)
        .expect("read child stderr");
    assert!(stderr.is_empty(), "unexpected server diagnostics: {stderr}");
}
