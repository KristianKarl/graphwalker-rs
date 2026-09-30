use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read};
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use assert_cmd::cargo::cargo_bin;
use futures_util::{SinkExt, StreamExt};
use graphwalker_core::condition::StopCondition;
use graphwalker_core::generator::PathGenerator;
use graphwalker_core::machine::{ExecutionContext, Machine};
use graphwalker_core::model::{
    EdgeBuilder, ElementIndex, ModelBuilder, RuntimeModel, VertexBuilder, VertexIndex,
};
use reqwest::Client;
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

const SEED: u64 = 4242;
const MAX_STEPS: usize = 32;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(60);

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[tokio::test]
async fn studio_self_walk() {
    timeout(TOTAL_TIMEOUT, studio_self_walk_inner())
        .await
        .expect("Studio self-walk exceeded its 60 second limit");
}

async fn studio_self_walk_inner() {
    let mut harness = StudioHarness::new();
    let model = build_meta_model();
    let mut context = ExecutionContext::new_with_seed(model, SEED);
    context.set_next_element(Some(ElementIndex::Vertex(VertexIndex(0))));
    let generator = PathGenerator::random(StopCondition::EdgeCoverage(100));
    let mut machine = Machine::new_with_seed(vec![(context, generator)], SEED).unwrap();

    let mut steps = 0;
    while machine.has_next_step() {
        machine.get_next_step().unwrap();
        steps += 1;
        assert!(
            steps <= MAX_STEPS,
            "Studio self-walk exceeded {MAX_STEPS} steps"
        );

        if let Some(ElementIndex::Edge(edge_index)) = machine.current_context().current_element() {
            let name = machine
                .current_context()
                .model()
                .edge(edge_index)
                .name()
                .unwrap_or_default()
                .to_string();
            harness.dispatch(&name).await;
        }
    }

    assert!(
        machine.get_fulfilment(0) >= 0.999999,
        "Studio self-walk stopped without full edge coverage"
    );
}

fn build_meta_model() -> RuntimeModel {
    let vertex_names = [
        "v_NotStarted",
        "v_Served",
        "v_Connected",
        "v_Validated",
        "v_Running",
        "v_Advanced",
        "v_Observed",
        "v_Controlled",
        "v_Finished",
    ];
    let edge_names = [
        "e_launchAndServe",
        "e_connectWebSocket",
        "e_validateModels",
        "e_startSession",
        "e_advanceAndInspect",
        "e_observeSession",
        "e_controlSession",
        "e_cleanup",
    ];
    let vertices: Vec<VertexBuilder> = vertex_names
        .iter()
        .enumerate()
        .map(|(index, name)| VertexBuilder::new().id(format!("v{index}")).name(*name))
        .collect();

    let mut model = ModelBuilder::new();
    for (index, edge_name) in edge_names.iter().enumerate() {
        model.add_edge(
            EdgeBuilder::new()
                .id(format!("e{index}"))
                .name(*edge_name)
                .source_vertex(vertices[index].clone())
                .target_vertex(vertices[index + 1].clone()),
        );
    }
    model.build()
}

struct StudioHarness {
    process: Option<StudioProcess>,
    browser_port: u16,
    websocket_port: u16,
    http: Client,
    owner: Option<WsClient>,
    observer: Option<WsClient>,
    model: Value,
    session_id: Option<String>,
}

impl StudioHarness {
    fn new() -> Self {
        let browser_port = available_port();
        let websocket_port = available_port();
        assert_ne!(browser_port, websocket_port, "ports must be distinct");
        Self {
            process: None,
            browser_port,
            websocket_port,
            http: Client::new(),
            owner: None,
            observer: None,
            model: studio_model(),
            session_id: None,
        }
    }

    async fn dispatch(&mut self, name: &str) {
        match name {
            "e_launchAndServe" => self.launch_and_serve().await,
            "e_connectWebSocket" => self.connect_websocket().await,
            "e_validateModels" => self.validate_models().await,
            "e_startSession" => self.start_session().await,
            "e_advanceAndInspect" => self.advance_and_inspect().await,
            "e_observeSession" => self.observe_session().await,
            "e_controlSession" => self.control_session().await,
            "e_cleanup" => self.cleanup().await,
            other => panic!("Studio self-walk has no scenario for {other}"),
        }
    }

    async fn launch_and_serve(&mut self) {
        self.process = Some(StudioProcess::spawn(self.browser_port, self.websocket_port));
        let base = format!("http://127.0.0.1:{}", self.browser_port);
        let mut response = None;
        for _ in 0..40 {
            if let Some(status) = self.process.as_mut().unwrap().try_wait() {
                panic!(
                    "Studio exited before HTTP became ready ({status}); output: {}",
                    self.process.as_ref().unwrap().logs()
                );
            }
            if let Ok(Ok(candidate)) =
                timeout(Duration::from_millis(250), self.http.get(&base).send()).await
            {
                if candidate.status().is_success() {
                    response = Some(candidate);
                    break;
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
        let response = response.unwrap_or_else(|| {
            panic!(
                "Studio HTTP server did not become ready; output: {}",
                self.process.as_ref().unwrap().logs()
            )
        });
        let html = response.text().await.unwrap();
        assert!(html.contains("<title>GraphWalker Studio</title>"));
        assert!(
            html.contains(&format!("window.GW_WS_PORT = {};", self.websocket_port)),
            "Studio did not inject its configured WebSocket port into the page"
        );

        let script_paths = attribute_values(&html, "src");
        let style_paths = attribute_values(&html, "href");
        assert!(script_paths.iter().any(|path| path.ends_with(".js")));
        assert!(style_paths.iter().any(|path| path.ends_with(".css")));
        for path in script_paths.iter().chain(style_paths.iter()) {
            let asset = self.http.get(format!("{base}{path}")).send().await.unwrap();
            assert!(asset.status().is_success(), "asset {path} was not served");
            assert!(
                !asset.bytes().await.unwrap().is_empty(),
                "asset {path} was empty"
            );
        }
    }

    async fn connect_websocket(&mut self) {
        let url = format!("ws://127.0.0.1:{}/", self.websocket_port);
        self.owner = Some(WsClient::connect(&url).await);
    }

    async fn validate_models(&mut self) {
        let owner = self.owner.as_mut().unwrap();
        let valid = owner
            .request(json!({"command": "check", "gw": self.model}), "check")
            .await;
        assert_eq!(valid["success"], true, "{valid}");
        assert_eq!(valid["issues"], json!([]), "{valid}");

        let mut invalid_model = self.model.clone();
        invalid_model["models"][0]["vertices"][1]["name"] = json!("");
        let invalid = owner
            .request(json!({"command": "check", "gw": invalid_model}), "check")
            .await;
        assert_eq!(invalid["success"], true, "{invalid}");
        assert!(
            invalid["issues"]
                .as_array()
                .is_some_and(|issues| !issues.is_empty()),
            "invalid model was accepted: {invalid}"
        );
    }

    async fn start_session(&mut self) {
        let response = self
            .owner
            .as_mut()
            .unwrap()
            .request(
                json!({
                    "command": "start",
                    "gw": self.model,
                    "seed": SEED,
                    "globalData": "counter = 0;",
                    "name": "Studio self-walk",
                }),
                "start",
            )
            .await;
        assert_eq!(response["success"], true, "{response}");
        assert_eq!(response["seed"], json!(SEED));
        let session_id = response["sessionId"].as_str().unwrap().to_string();
        let sessions = self
            .owner
            .as_mut()
            .unwrap()
            .request(json!({"command": "listSessions"}), "sessions")
            .await;
        assert!(sessions["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|session| {
                session["id"] == session_id && session["name"] == "Studio self-walk"
            }));
        self.session_id = Some(session_id);
    }

    async fn advance_and_inspect(&mut self) {
        let owner = self.owner.as_mut().unwrap();
        let first_step = owner
            .request(json!({"command": "getNext"}), "visitedElement")
            .await;
        assert_visited_element(&first_step, &self.model);

        let model = owner
            .request(json!({"command": "getModel"}), "getModel")
            .await;
        let model_text = model["models"].as_str().unwrap();
        let parsed_model: Value = serde_json::from_str(model_text).unwrap();
        assert_eq!(
            parsed_model["models"][0]["name"],
            "Studio integration model"
        );

        let data = owner
            .request(json!({"command": "getData"}), "getData")
            .await;
        assert!(data["data"].is_string(), "{data}");
        let has_next = owner
            .request(json!({"command": "hasNext"}), "hasNext")
            .await;
        assert!(has_next["hasNext"].is_boolean(), "{has_next}");
        let elements = owner
            .request(json!({"command": "updateAllElements"}), "updateAllElements")
            .await;
        assert!(elements["elements"]
            .as_array()
            .unwrap()
            .iter()
            .any(|element| {
                element["elementId"] == first_step["elementId"]
                    && element["visitedCount"].as_u64().unwrap_or(0) > 0
            }));
    }

    async fn observe_session(&mut self) {
        let url = format!("ws://127.0.0.1:{}/", self.websocket_port);
        self.observer = Some(WsClient::connect(&url).await);
        let session_id = self.session_id.as_ref().unwrap().clone();
        let snapshot = self
            .observer
            .as_mut()
            .unwrap()
            .request(
                json!({"command": "subscribeSession", "sessionId": session_id}),
                "subscribeSession",
            )
            .await;
        assert_eq!(snapshot["success"], true, "{snapshot}");
        assert_eq!(snapshot["sessionId"], session_id);
        assert_eq!(
            snapshot["models"]["models"][0]["name"],
            "Studio integration model"
        );
        assert!(snapshot["elements"].as_array().is_some());

        let step = self
            .owner
            .as_mut()
            .unwrap()
            .request(json!({"command": "getNext"}), "visitedElement")
            .await;
        assert_visited_element(&step, &self.model);
        let event = self
            .observer
            .as_mut()
            .unwrap()
            .wait_for_command("visitedElement")
            .await;
        assert_eq!(event["elementId"], step["elementId"]);
        assert_eq!(event["visitedCount"], step["visitedCount"]);
    }

    async fn control_session(&mut self) {
        let session_id = self.session_id.as_ref().unwrap().clone();
        let paused = self
            .observer
            .as_mut()
            .unwrap()
            .request(
                json!({"command": "pauseSession", "sessionId": session_id}),
                "pauseSession",
            )
            .await;
        assert_eq!(paused["success"], true, "{paused}");

        {
            let owner = self.owner.as_mut().unwrap();
            let pending_step = owner.request(json!({"command": "getNext"}), "visitedElement");
            tokio::pin!(pending_step);
            assert!(
                timeout(Duration::from_millis(100), &mut pending_step)
                    .await
                    .is_err(),
                "paused getNext completed without a step command"
            );

            let stepped = self
                .observer
                .as_mut()
                .unwrap()
                .request(
                    json!({"command": "stepSession", "sessionId": session_id}),
                    "stepSession",
                )
                .await;
            assert_eq!(stepped["success"], true, "{stepped}");
            let step = timeout(REQUEST_TIMEOUT, &mut pending_step)
                .await
                .expect("stepSession did not release one getNext");
            assert_visited_element(&step, &self.model);
            let event = self
                .observer
                .as_mut()
                .unwrap()
                .wait_for_command("visitedElement")
                .await;
            assert_eq!(event["elementId"], step["elementId"]);
            assert_eq!(event["visitedCount"], step["visitedCount"]);
        }

        {
            let owner = self.owner.as_mut().unwrap();
            let resumed_step = owner.request(json!({"command": "getNext"}), "visitedElement");
            tokio::pin!(resumed_step);
            assert!(
                timeout(Duration::from_millis(100), &mut resumed_step)
                    .await
                    .is_err(),
                "session did not remain paused after its single step"
            );
            let resumed = self
                .observer
                .as_mut()
                .unwrap()
                .request(
                    json!({"command": "resumeSession", "sessionId": session_id}),
                    "resumeSession",
                )
                .await;
            assert_eq!(resumed["success"], true, "{resumed}");
            let next = timeout(REQUEST_TIMEOUT, &mut resumed_step)
                .await
                .expect("resumeSession did not release getNext");
            assert_visited_element(&next, &self.model);
        }
    }

    async fn cleanup(&mut self) {
        let session_id = self.session_id.as_ref().unwrap().clone();
        let unsubscribed = self
            .observer
            .as_mut()
            .unwrap()
            .request(
                json!({"command": "unsubscribeSession"}),
                "unsubscribeSession",
            )
            .await;
        assert_eq!(unsubscribed["success"], true, "{unsubscribed}");

        self.owner.as_mut().unwrap().close().await;
        let mut removed = false;
        for _ in 0..20 {
            let sessions = self
                .observer
                .as_mut()
                .unwrap()
                .request(json!({"command": "listSessions"}), "sessions")
                .await;
            if !sessions["sessions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|session| session["id"] == session_id)
            {
                removed = true;
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }
        assert!(
            removed,
            "owner disconnect did not remove session {session_id}"
        );
        self.observer.as_mut().unwrap().close().await;
    }
}

fn studio_model() -> Value {
    json!({
        "models": [{
            "name": "Studio integration model",
            "generator": "random(edge_coverage(100))",
            "startElementId": "e_start",
            "vertices": [
                {"id": "v_a", "name": "v_A"},
                {"id": "v_b", "name": "v_B"},
                {"id": "v_c", "name": "v_C"}
            ],
            "edges": [
                {"id": "e_start", "name": "e_Start", "targetVertexId": "v_a"},
                {"id": "e_ab", "name": "e_AB", "sourceVertexId": "v_a", "targetVertexId": "v_b"},
                {"id": "e_bc", "name": "e_BC", "sourceVertexId": "v_b", "targetVertexId": "v_c"},
                {"id": "e_ca", "name": "e_CA", "sourceVertexId": "v_c", "targetVertexId": "v_a"}
            ]
        }]
    })
}

fn assert_visited_element(response: &Value, model: &Value) {
    assert_eq!(response["success"], true, "{response}");
    assert_eq!(response["command"], "visitedElement", "{response}");
    let element_id = response["elementId"].as_str().unwrap();
    let model = &model["models"][0];
    let known = model["vertices"]
        .as_array()
        .unwrap()
        .iter()
        .chain(model["edges"].as_array().unwrap())
        .any(|element| element["id"] == element_id);
    assert!(
        known,
        "Studio reported unknown element {element_id}: {response}"
    );
    assert!(
        response["visitedCount"].as_u64().unwrap_or(0) > 0,
        "{response}"
    );
}

fn attribute_values(html: &str, attribute: &str) -> Vec<String> {
    let needle = format!("{attribute}=\"");
    html.match_indices(&needle)
        .filter_map(|(index, _)| {
            let value_start = index + needle.len();
            let value_end = html[value_start..].find('"')? + value_start;
            Some(html[value_start..value_end].to_string())
        })
        .filter(|value| value.starts_with('/'))
        .collect()
}

fn available_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

struct StudioProcess {
    child: Child,
    logs: Arc<Mutex<String>>,
    readers: Vec<JoinHandle<()>>,
}

impl StudioProcess {
    fn spawn(browser_port: u16, websocket_port: u16) -> Self {
        let mut child = Command::new(cargo_bin("graphwalker-studio"))
            .args([
                "--browser-port",
                &browser_port.to_string(),
                "--websocket-port",
                &websocket_port.to_string(),
            ])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to start graphwalker-studio");
        let logs = Arc::new(Mutex::new(String::new()));
        let readers = vec![
            capture_output(child.stdout.take().unwrap(), Arc::clone(&logs)),
            capture_output(child.stderr.take().unwrap(), Arc::clone(&logs)),
        ];
        Self {
            child,
            logs,
            readers,
        }
    }

    fn try_wait(&mut self) -> Option<std::process::ExitStatus> {
        self.child
            .try_wait()
            .expect("failed checking Studio process")
    }

    fn logs(&self) -> String {
        self.logs.lock().unwrap().clone()
    }
}

fn capture_output<R: Read + Send + 'static>(stream: R, logs: Arc<Mutex<String>>) -> JoinHandle<()> {
    thread::spawn(move || {
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
            logs.lock().unwrap().push_str(&format!("{line}\n"));
        }
    })
}

impl Drop for StudioProcess {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
        for reader in self.readers.drain(..) {
            let _ = reader.join();
        }
    }
}

struct WsClient {
    socket: Socket,
    queued: VecDeque<Value>,
}

impl WsClient {
    async fn connect(url: &str) -> Self {
        let (socket, _) = timeout(REQUEST_TIMEOUT, connect_async(url))
            .await
            .expect("WebSocket connection timed out")
            .expect("failed to connect to Studio WebSocket");
        Self {
            socket,
            queued: VecDeque::new(),
        }
    }

    async fn request(&mut self, request: Value, expected_command: &str) -> Value {
        self.socket
            .send(Message::Text(request.to_string().into()))
            .await
            .expect("failed sending WebSocket request");
        self.wait_for_command(expected_command).await
    }

    async fn wait_for_command(&mut self, expected_command: &str) -> Value {
        if let Some(index) = self
            .queued
            .iter()
            .position(|message| message["command"].as_str() == Some(expected_command))
        {
            return self.queued.remove(index).unwrap();
        }

        timeout(REQUEST_TIMEOUT, async {
            loop {
                let message = self
                    .socket
                    .next()
                    .await
                    .expect("Studio WebSocket closed unexpectedly")
                    .expect("failed reading Studio WebSocket")
                    .into_text()
                    .expect("Studio sent a non-text WebSocket message");
                let value: Value =
                    serde_json::from_str(&message).expect("invalid JSON from Studio");
                if value["command"].as_str() == Some(expected_command) {
                    return value;
                }
                self.queued.push_back(value);
            }
        })
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for WebSocket command {expected_command}"))
    }

    async fn close(&mut self) {
        let _ = self.socket.send(Message::Close(None)).await;
    }
}
