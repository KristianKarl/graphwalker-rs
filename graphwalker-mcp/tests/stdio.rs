use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

const TOOL_NAMES: &[&str] = &[
    "add_edge",
    "add_vertex",
    "close_execution",
    "convert_graphml",
    "create_model",
    "discard_model",
    "execution_status",
    "export_model",
    "health",
    "next_step",
    "remove_element",
    "restart_execution",
    "set_execution_data",
    "start_execution",
    "update_edge",
    "update_model",
    "update_vertex",
    "validate_model",
];

struct Client {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
    protocol_output: String,
}

impl Client {
    fn initialize() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_graphwalker-mcp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn graphwalker-mcp");
        let stdin = child.stdin.take().expect("child stdin");
        let stdout = BufReader::new(child.stdout.take().expect("child stdout"));
        let mut client = Self {
            child,
            stdin: Some(stdin),
            stdout,
            next_id: 1,
            protocol_output: String::new(),
        };
        let initialized = client.request(
            "initialize",
            json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "graphwalker-mcp-test", "version": "0.1.0" }
            }),
        );
        assert_eq!(initialized["protocolVersion"], "2025-11-25");
        assert_eq!(initialized["serverInfo"]["name"], "graphwalker-mcp");
        assert!(initialized["capabilities"]["tools"].is_object());
        client.send(json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }));
        client
    }

    fn send(&mut self, message: Value) {
        let stdin = self.stdin.as_mut().expect("open child stdin");
        writeln!(stdin, "{message}").expect("write MCP message");
        stdin.flush().expect("flush MCP message");
    }

    fn receive(&mut self) -> Value {
        let mut line = String::new();
        self.stdout.read_line(&mut line).expect("read MCP response");
        assert!(!line.is_empty(), "MCP server closed before responding");
        self.protocol_output.push_str(&line);
        serde_json::from_str(&line).expect("stdout line must be JSON-RPC")
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }));
        let response = self.receive();
        assert_eq!(response["id"], id, "response ID for {method}");
        assert!(
            response.get("error").is_none(),
            "RPC error for {method}: {response}"
        );
        response["result"].clone()
    }

    fn list_tools(&mut self) -> Vec<Value> {
        self.request("tools/list", json!({}))["tools"]
            .as_array()
            .expect("tools array")
            .clone()
    }

    fn call_result(&mut self, name: &str, arguments: Value) -> Value {
        self.request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        )
    }

    fn call(&mut self, name: &str, arguments: Value) -> Value {
        let result = self.call_result(name, arguments);
        assert_eq!(result["isError"], false, "tool {name} failed: {result}");
        result["structuredContent"].clone()
    }

    fn call_error(&mut self, name: &str, arguments: Value, expected_code: &str) -> Value {
        let result = self.call_result(name, arguments);
        assert_eq!(
            result["isError"], true,
            "tool {name} unexpectedly succeeded"
        );
        assert_eq!(
            result["structuredContent"]["code"], expected_code,
            "wrong error from {name}: {result}"
        );
        assert!(result["structuredContent"]["message"].is_string());
        result["structuredContent"].clone()
    }

    fn finish(mut self) {
        drop(self.stdin.take());
        self.stdout
            .read_to_string(&mut self.protocol_output)
            .expect("drain child stdout");
        let status = self.child.wait().expect("wait for graphwalker-mcp");
        assert!(status.success(), "server should shut down cleanly on EOF");

        for line in self.protocol_output.lines().filter(|line| !line.is_empty()) {
            serde_json::from_str::<Value>(line)
                .unwrap_or_else(|error| panic!("non-protocol stdout ({error}): {line:?}"));
        }
        let mut stderr = String::new();
        self.child
            .stderr
            .take()
            .expect("child stderr")
            .read_to_string(&mut stderr)
            .expect("read child stderr");
        assert!(stderr.is_empty(), "unexpected server diagnostics: {stderr}");
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if self.stdin.take().is_some() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[test]
fn stdio_lists_stable_schema_backed_tools_and_health() {
    let mut client = Client::initialize();
    let tools = client.list_tools();
    let names = tools
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect::<Vec<_>>();
    assert_eq!(names, TOOL_NAMES);

    for tool in &tools {
        assert_eq!(tool["inputSchema"]["type"], "object", "{tool}");
        assert_eq!(tool["outputSchema"]["type"], "object", "{tool}");
        assert!(tool["description"]
            .as_str()
            .is_some_and(|text| !text.is_empty()));
        assert_eq!(tool["annotations"]["openWorldHint"], false);
    }
    let add_edge = tools
        .iter()
        .find(|tool| tool["name"] == "add_edge")
        .unwrap();
    assert!(add_edge["inputSchema"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "target_vertex_id"));
    let remove = tools
        .iter()
        .find(|tool| tool["name"] == "remove_element")
        .unwrap();
    assert_eq!(remove["annotations"]["destructiveHint"], true);
    assert_eq!(remove["annotations"]["readOnlyHint"], false);

    for (tool_name, schema_path) in [
        ("convert_graphml", "/outputSchema/properties/model"),
        ("export_model", "/outputSchema/properties/model"),
        ("start_execution", "/inputSchema/properties/model"),
        ("update_model", "/outputSchema/properties/model"),
        ("validate_model", "/inputSchema/properties/model"),
    ] {
        let tool = tools.iter().find(|tool| tool["name"] == tool_name).unwrap();
        let model_schema = tool.pointer(schema_path).unwrap();
        assert!(
            model_schema.is_object(),
            "{tool_name} must advertise an object schema instead of a bare true schema"
        );
    }

    let health = client.call("health", json!({}));
    assert_eq!(health["status"], "ok");
    assert_eq!(health["server_version"], env!("CARGO_PKG_VERSION"));
    client.finish();
}

#[test]
fn stdio_authoring_execution_and_error_workflow() {
    let mut client = Client::initialize();
    let created = client.call(
        "create_model",
        json!({
            "model_id": "model-mcp",
            "name": "MCP model",
            "generator": "random(vertex_coverage(100))",
            "actions": ["global.created = true"],
            "requirements": ["REQ-1"],
            "properties": { "owner": "mcp" }
        }),
    );
    let draft_id = created["draft_id"].as_str().unwrap().to_string();
    assert!(draft_id.starts_with("draft_"));
    assert_eq!(created["model_id"], "model-mcp");
    assert_eq!(created["revision"], 0);

    let vertex_a = client.call(
        "add_vertex",
        json!({
            "draft_id": draft_id,
            "id": "v_a",
            "name": "v_A",
            "shared_state": "state-a",
            "actions": ["x = 1"],
            "requirements": ["REQ-A"],
            "properties": { "role": "start" },
            "expected_revision": 0
        }),
    );
    assert_eq!(vertex_a["revision"], 1);
    assert_eq!(vertex_a["vertex"]["shared_state"], "state-a");
    assert_eq!(
        client.call(
            "add_vertex",
            json!({
                "draft_id": draft_id,
                "id": "v_b",
                "name": "v_B",
                "expected_revision": 1
            }),
        )["revision"],
        2
    );
    assert_eq!(
        client.call(
            "add_vertex",
            json!({
                "draft_id": draft_id,
                "id": "v_temp",
                "name": "v_Temp",
                "expected_revision": 2
            }),
        )["revision"],
        3
    );

    let start_edge = client.call(
        "add_edge",
        json!({
            "draft_id": draft_id,
            "id": "e_start",
            "name": "e_Start",
            "target_vertex_id": "v_a",
            "expected_revision": 3
        }),
    );
    assert_eq!(start_edge["edge"]["source_vertex_id"], Value::Null);
    assert_eq!(start_edge["revision"], 4);
    assert_eq!(
        client.call(
            "add_edge",
            json!({
                "draft_id": draft_id,
                "id": "e_ab",
                "name": "e_AB",
                "source_vertex_id": "v_a",
                "target_vertex_id": "v_b",
                "guard": "true",
                "weight": 1.0,
                "dependency": 10,
                "expected_revision": 4
            }),
        )["revision"],
        5
    );
    assert_eq!(
        client.call(
            "add_edge",
            json!({
                "draft_id": draft_id,
                "id": "e_temp",
                "source_vertex_id": "v_b",
                "target_vertex_id": "v_temp",
                "expected_revision": 5
            }),
        )["revision"],
        6
    );

    assert_eq!(
        client.call(
            "update_vertex",
            json!({
                "draft_id": draft_id,
                "vertex_id": "v_a",
                "name": "v_A_updated",
                "shared_state": null,
                "expected_revision": 6
            }),
        )["revision"],
        7
    );
    let edge = client.call(
        "update_edge",
        json!({
            "draft_id": draft_id,
            "edge_id": "e_ab",
            "guard": null,
            "actions": ["y = 1"],
            "weight": 0.5,
            "expected_revision": 7
        }),
    );
    assert_eq!(edge["edge"]["guard"], Value::Null);
    assert_eq!(edge["revision"], 8);
    assert_eq!(
        client.call(
            "update_model",
            json!({
                "draft_id": draft_id,
                "name": "Executable MCP model",
                "start_element_id": "e_start",
                "expected_revision": 8
            }),
        )["revision"],
        9
    );

    client.call_error(
        "add_vertex",
        json!({
            "draft_id": draft_id,
            "id": "stale",
            "expected_revision": 0
        }),
        "revision_conflict",
    );
    assert_eq!(
        client.call(
            "remove_element",
            json!({
                "draft_id": draft_id,
                "element_id": "e_temp",
                "expected_revision": 9
            }),
        )["revision"],
        10
    );
    assert_eq!(
        client.call(
            "remove_element",
            json!({
                "draft_id": draft_id,
                "element_id": "v_temp",
                "expected_revision": 10
            }),
        )["revision"],
        11
    );

    let exported = client.call("export_model", json!({ "draft_id": draft_id }));
    assert_eq!(exported["revision"], 11);
    assert_eq!(
        exported["model"]["models"][0]["vertices"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        exported["model"]["models"][0]["edges"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let validation = client.call("validate_model", json!({ "draft_id": draft_id }));
    assert_eq!(validation["valid"], true, "{validation}");
    assert_eq!(validation["revision"], 11);
    let inline_validation = client.call(
        "validate_model",
        json!({ "model": exported["model"].clone() }),
    );
    assert_eq!(inline_validation["valid"], true);
    assert_eq!(inline_validation["revision"], Value::Null);
    client.call_error(
        "validate_model",
        json!({ "draft_id": draft_id, "model": exported["model"].clone() }),
        "invalid_input",
    );

    let started = client.call(
        "start_execution",
        json!({
            "draft_id": draft_id,
            "revision": 11,
            "seed": 42,
            "global_data": "initial = 1"
        }),
    );
    let execution_id = started["execution_id"].as_str().unwrap().to_string();
    assert_eq!(started["seed"], 42);
    assert_eq!(started["source_revision"], 11);
    let data = client.call(
        "set_execution_data",
        json!({ "execution_id": execution_id, "script": "observed = 2" }),
    );
    assert!(data["data"].as_str().unwrap().contains("observed=2"));
    let status = client.call(
        "execution_status",
        json!({ "execution_id": execution_id, "include_elements": true }),
    );
    assert_eq!(status["has_next"], true);
    assert_eq!(status["statistics"]["total_vertices"], 2);
    assert_eq!(status["statistics"]["total_edges"], 2);
    assert_eq!(status["elements"].as_array().unwrap().len(), 4);

    let mut completed = false;
    for _ in 0..20 {
        let step = client.call("next_step", json!({ "execution_id": execution_id }));
        if step["completed"] == true {
            assert_eq!(step["element"], Value::Null);
            completed = true;
            break;
        }
        assert!(matches!(
            step["element"]["kind"].as_str(),
            Some("edge" | "vertex")
        ));
        assert!(step["element"]["id"].is_string());
    }
    assert!(
        completed,
        "execution did not complete within the bounded test path"
    );

    let restarted = client.call("restart_execution", json!({ "execution_id": execution_id }));
    assert_eq!(restarted["restarted"], true);
    assert_eq!(restarted["seed"], 42);
    assert_eq!(
        client.call("close_execution", json!({ "execution_id": execution_id }))["closed"],
        true
    );
    client.call_error(
        "execution_status",
        json!({ "execution_id": execution_id }),
        "execution_not_found",
    );

    let graphml = include_str!("../../graphwalker-io/tests/fixtures/graphml/Login.graphml");
    let converted = client.call("convert_graphml", json!({ "graphml": graphml }));
    assert!(!converted["model"]["models"].as_array().unwrap().is_empty());
    assert_eq!(
        client.call("discard_model", json!({ "draft_id": draft_id }))["discarded"],
        true
    );
    client.call_error(
        "export_model",
        json!({ "draft_id": draft_id }),
        "draft_not_found",
    );
    client.finish();
}

#[test]
fn stdio_keeps_interleaved_drafts_and_executions_isolated() {
    let mut client = Client::initialize();
    let first = client.call(
        "create_model",
        json!({ "name": "first", "generator": "random(length(2))" }),
    );
    let second = client.call(
        "create_model",
        json!({ "name": "second", "generator": "random(length(2))" }),
    );
    let first_draft = first["draft_id"].as_str().unwrap().to_string();
    let second_draft = second["draft_id"].as_str().unwrap().to_string();
    assert_ne!(first_draft, second_draft);

    client.call(
        "add_vertex",
        json!({ "draft_id": first_draft, "id": "shared-id", "name": "first vertex" }),
    );
    client.call(
        "add_vertex",
        json!({ "draft_id": second_draft, "id": "shared-id", "name": "second vertex" }),
    );
    let first_export = client.call("export_model", json!({ "draft_id": first_draft }));
    let second_export = client.call("export_model", json!({ "draft_id": second_draft }));
    assert_eq!(
        first_export["model"]["models"][0]["vertices"][0]["name"],
        "first vertex"
    );
    assert_eq!(
        second_export["model"]["models"][0]["vertices"][0]["name"],
        "second vertex"
    );

    let executable_model = json!({
        "models": [{
            "id": "isolated-model",
            "generator": "random(length(2))",
            "startElementId": "e_start",
            "vertices": [{ "id": "v_a", "name": "v_A" }],
            "edges": [{ "id": "e_start", "name": "e_Start", "targetVertexId": "v_a" }]
        }]
    });
    let execution_a = client.call(
        "start_execution",
        json!({ "model": executable_model, "seed": 7 }),
    )["execution_id"]
        .as_str()
        .unwrap()
        .to_string();
    let execution_b = client.call(
        "start_execution",
        json!({ "model": executable_model, "seed": 7 }),
    )["execution_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(execution_a, execution_b);
    client.call(
        "set_execution_data",
        json!({ "execution_id": execution_a, "script": "only_a = 1" }),
    );
    let status_a = client.call("execution_status", json!({ "execution_id": execution_a }));
    let status_b = client.call("execution_status", json!({ "execution_id": execution_b }));
    assert!(status_a["data"].as_str().unwrap().contains("only_a=1"));
    assert!(!status_b["data"].as_str().unwrap().contains("only_a"));

    client.send(json!({
        "jsonrpc": "2.0",
        "method": "notifications/cancelled",
        "params": { "requestId": 999, "reason": "test cancellation" }
    }));
    assert_eq!(client.call("health", json!({}))["status"], "ok");

    for execution_id in [&execution_a, &execution_b] {
        client.call("close_execution", json!({ "execution_id": execution_id }));
    }
    for draft_id in [&first_draft, &second_draft] {
        client.call("discard_model", json!({ "draft_id": draft_id }));
    }
    client.finish();
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

    writeln!(
        stdin,
        "{}",
        json!({
            "jsonrpc": "2.0",
            "id": "discover",
            "method": "server/discover",
            "params": { "_meta": request_meta.clone() }
        })
    )
    .unwrap();
    stdin.flush().unwrap();
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    let discovered: Value = serde_json::from_str(&line).unwrap();
    assert!(discovered["result"]["supportedVersions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|version| version == "2026-07-28"));

    writeln!(
        stdin,
        "{}",
        json!({
            "jsonrpc": "2.0",
            "id": "list",
            "method": "tools/list",
            "params": { "_meta": request_meta }
        })
    )
    .unwrap();
    stdin.flush().unwrap();
    line.clear();
    stdout.read_line(&mut line).unwrap();
    let tools: Value = serde_json::from_str(&line).unwrap();
    let names = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(names, TOOL_NAMES);

    drop(stdin);
    let mut remaining_stdout = String::new();
    stdout.read_to_string(&mut remaining_stdout).unwrap();
    assert!(remaining_stdout.is_empty());
    assert!(child.wait().unwrap().success());
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(stderr.is_empty(), "unexpected server diagnostics: {stderr}");
}
