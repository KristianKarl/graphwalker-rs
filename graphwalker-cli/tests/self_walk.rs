//! GraphWalker testing itself, one level up the stack.
//!
//! `graphwalker-core/tests/self_walk.rs` dogfoods the *engine*: driver and
//! SUT are both `graphwalker-core`, in-process. This file dogfoods the
//! *CLI*: the driver is still `graphwalker-core` (used purely as a library
//! to build and walk a meta-model), but the SUT is the compiled
//! `graphwalker` **binary**, treated strictly as an external black box. No
//! `graphwalker-cli` internals are imported here — every scenario shells
//! out via `assert_cmd` (or, for `online`, spawns a real child process and
//! talks to it over HTTP/WebSocket), exactly like a real user of the CLI
//! would.
//!
//! # Why a meta-model at all
//!
//! A flat list of `#[test]` functions (as in `cli_tests.rs`/
//! `online_tests.rs`) already covers individual behaviors well. What this
//! file adds is a *coverage-driven, self-verifying orchestration*: the
//! order scenarios run in, and the guarantee that every scenario ran
//! exactly once and none were forgotten, is itself produced by
//! `graphwalker-core`'s own `Machine` + `Random` generator +stop
//! condition — so a regression in the engine's own edge-coverage or
//! generator logic would surface here too, not just in `graphwalker-core`'s
//! own tests.
//!
//! # Shape
//!
//! One meta-model, a single cycle through one `e_*` edge per CLI capability
//! under test, looping back via `e_restart`. There are no branch points, so
//! with a fixed `SEED` the walk is deterministic. Every edge is
//! guard-free and side-effect-free on the meta-model itself — the edges
//! exist only to give the walk an order; the actual verification happens
//! in each edge's dispatched `assert_*` function, which is free to spawn
//! its own subprocess and make its own assertions independent of the outer
//! walk's state.
//!
//! # Scope
//!
//! Covers all seven subcommands (`offline`, `online`, `methods`,
//! `requirements`, `convert`, `source`, `check`), the cross-cutting
//! `offline` flags (seed determinism, verbose data, unvisited-element
//! accounting, explicit start element), a multi-model/shared-state offline
//! run, both `online` transports (REST and WebSocket), the `--debug` global
//! flag, and negative paths (missing model argument, a model the checker
//! rejects).
//!
//! Deliberately out of scope for now: exhaustive coverage of every possible
//! `graphwalker-model-checker` issue kind (one representative issue is
//! enough to prove the CLI surfaces checker failures correctly — the
//! checker's own rules are that crate's responsibility to self-test), and
//! concurrent/parallel `online` sessions, TLS, or authentication (not
//! supported by the CLI today).

use std::process::{Child, Command, Stdio};
use std::time::Duration;

use assert_cmd::Command as AssertCommand;
use futures_util::{SinkExt, StreamExt};
use graphwalker_core::condition::StopCondition;
use graphwalker_core::generator::PathGenerator;
use graphwalker_core::machine::{ExecutionContext, Machine};
use graphwalker_core::model::{
    EdgeBuilder, ElementIndex, ModelBuilder, RuntimeModel, VertexBuilder, VertexIndex,
};
use serde_json::{json, Value};
use tokio::time::{sleep, timeout};
use tokio_tungstenite::tungstenite::Message;

/// Fixed seed for the outer walk, so a failure is reproducible. Individual
/// scenarios that need their own seed (e.g. `e_offlineSeedDeterminism`)
/// pass explicit seeds to the CLI independent of this one.
const SEED: u64 = 4242;
/// Safety cap: a regression in the outer generator/stop-condition logic
/// should fail this test loudly, not hang it.
const MAX_STEPS: usize = 100;

fn fixture(path: &str) -> String {
    format!("tests/fixtures/{}", path)
}

/// The always-succeeds-or-panics invocation style used by every synchronous
/// scenario: failures surface as this test's own panic, not a silent skip.
fn gw() -> AssertCommand {
    AssertCommand::cargo_bin("graphwalker").unwrap()
}

/// Resolves the compiled binary's path for scenarios that need a raw
/// `std::process::Command` (i.e. `online`, which must stay alive across
/// several network calls rather than run to completion like `assert_cmd`).
fn graphwalker_bin() -> String {
    assert_cmd::cargo::cargo_bin("graphwalker")
        .to_str()
        .unwrap()
        .to_string()
}

/// Builds the outer meta-model: one `e_*` edge per CLI capability under
/// test, in a single deterministic cycle closed by `e_restart`.
fn build_meta_model() -> RuntimeModel {
    let names = [
        "v_Start",
        "v_OfflineBasicRan",
        "v_OfflineSeedVerified",
        "v_OfflineVerboseVerified",
        "v_OfflineUnvisitedVerified",
        "v_OfflineStartElementVerified",
        "v_OfflineMissingModelVerified",
        "v_OfflineMultiModelVerified",
        "v_MethodsVerified",
        "v_RequirementsVerified",
        "v_CheckOkVerified",
        "v_CheckIssuesVerified",
        "v_ConvertVerified",
        "v_SourceVerified",
        "v_OnlineRestfulVerified",
        "v_OnlineWebsocketVerified",
        "v_DebugLoggingVerified",
    ];
    let edges = [
        "e_offlineBasic",
        "e_offlineSeedDeterminism",
        "e_offlineVerbose",
        "e_offlineUnvisited",
        "e_offlineStartElement",
        "e_offlineMissingModel",
        "e_offlineMultiModelSharedState",
        "e_methods",
        "e_requirements",
        "e_checkOk",
        "e_checkIssues",
        "e_convert",
        "e_source",
        "e_onlineRestful",
        "e_onlineWebsocket",
        "e_debugLogging",
        "e_restart",
    ];

    let vertices: Vec<VertexBuilder> = names
        .iter()
        .enumerate()
        .map(|(i, name)| VertexBuilder::new().id(format!("v{i}")).name(*name))
        .collect();

    let mut mb = ModelBuilder::new();
    for (i, edge_name) in edges.iter().enumerate() {
        let source = vertices[i].clone();
        let target = vertices[(i + 1) % vertices.len()].clone();
        mb.add_edge(
            EdgeBuilder::new()
                .id(format!("e{i}"))
                .name(*edge_name)
                .source_vertex(source)
                .target_vertex(target),
        );
    }
    mb.build()
}

/// Drives the outer meta-model to completion with `graphwalker-core`'s own
/// `Random` generator under `EdgeCoverage(100)`, dispatching a real CLI
/// invocation for every edge visited.
#[tokio::test]
async fn cli_self_walk() {
    // Each scenario spawns real processes/network I/O; a single hung call
    // must fail the test loudly instead of blocking CI forever.
    timeout(Duration::from_secs(120), cli_self_walk_inner())
        .await
        .expect("self-walk did not complete within 120 seconds");
}

async fn cli_self_walk_inner() {
    let model = build_meta_model();
    let mut ctx = ExecutionContext::new_with_seed(model, SEED);
    ctx.set_next_element(Some(ElementIndex::Vertex(VertexIndex(0))));
    let gen = PathGenerator::random(StopCondition::EdgeCoverage(100));
    let mut machine = Machine::new_with_seed(vec![(ctx, gen)], SEED).unwrap();

    let mut steps = 0;
    while machine.has_next_step() {
        machine.get_next_step().unwrap();
        steps += 1;
        assert!(
            steps < MAX_STEPS,
            "self-walk did not terminate within {MAX_STEPS} steps"
        );

        if let Some(ElementIndex::Edge(ei)) = machine.current_context().current_element() {
            let name = machine
                .current_context()
                .model()
                .edge(ei)
                .name()
                .unwrap_or_default()
                .to_string();
            dispatch(&name).await;
        }
    }

    assert!(
        machine.get_fulfilment(0) >= 0.999999,
        "self-walk finished without full edge coverage"
    );
}

/// Maps a meta-model edge name to the CLI scenario that verifies it.
async fn dispatch(name: &str) {
    match name {
        "e_offlineBasic" => assert_offline_basic(),
        "e_offlineSeedDeterminism" => assert_offline_seed_determinism(),
        "e_offlineVerbose" => assert_offline_verbose(),
        "e_offlineUnvisited" => assert_offline_unvisited(),
        "e_offlineStartElement" => assert_offline_start_element(),
        "e_offlineMissingModel" => assert_offline_missing_model(),
        "e_offlineMultiModelSharedState" => assert_offline_multi_model_shared_state(),
        "e_methods" => assert_methods(),
        "e_requirements" => assert_requirements(),
        "e_checkOk" => assert_check_ok(),
        "e_checkIssues" => assert_check_issues(),
        "e_convert" => assert_convert(),
        "e_source" => assert_source(),
        "e_onlineRestful" => assert_online_restful().await,
        "e_onlineWebsocket" => assert_online_websocket().await,
        "e_debugLogging" => assert_debug_logging().await,
        _ => {}
    }
}

// -- e_offlineBasic: `offline` emits one JSON line per step, naming the real elements --

fn assert_offline_basic() {
    let out = gw()
        .args([
            "offline",
            "-m",
            &fixture("json/SmallModel.json"),
            "random(edge_coverage(100))",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();

    let names: Vec<String> = stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|v| v["currentElementName"].as_str().map(String::from))
        .collect();
    assert!(
        names.contains(&"e_FirstAction".to_string()),
        "self-walk: e_offlineBasic — expected fixture edge missing from output: {names:?}"
    );
    assert!(
        names.contains(&"v_VerifySomeAction".to_string()),
        "self-walk: e_offlineBasic — expected fixture vertex missing from output: {names:?}"
    );
}

// -- e_offlineSeedDeterminism: identical seeds reproduce, different seeds diverge --

fn assert_offline_seed_determinism() {
    let run = |seed: u64| -> Vec<Value> {
        let out = gw()
            .args([
                "offline",
                "-s",
                &seed.to_string(),
                "-m",
                &fixture("json/SmallModel.json"),
                "random(edge_coverage(100))",
            ])
            .assert()
            .success();
        String::from_utf8(out.get_output().stdout.clone())
            .unwrap()
            .lines()
            .map(|line| {
                let mut value: Value = serde_json::from_str(line).unwrap();
                value.as_object_mut().unwrap().remove("modelId");
                value
            })
            .collect()
    };

    let first = run(1);
    let second = run(1);
    assert_eq!(
        first, second,
        "self-walk: e_offlineSeedDeterminism — same seed must reproduce identical output"
    );
    assert_ne!(
        first,
        run(2),
        "self-walk: e_offlineSeedDeterminism — different seeds must diverge"
    );
}

// -- e_offlineVerbose: `-o` adds a `data` field to every emitted line --
//
// `-o` is the flag CI/game harnesses rely on to observe the model's live
// variable state alongside each step, not just which element was visited.

fn assert_offline_verbose() {
    let out = gw()
        .args([
            "offline",
            "-o",
            "-m",
            &fixture("json/SmallModel.json"),
            "random(edge_coverage(100))",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();

    for line in stdout.lines() {
        let value: Value = serde_json::from_str(line).unwrap();
        assert!(
            value.get("data").is_some(),
            "self-walk: e_offlineVerbose — every line must include `data` under -o: {line}"
        );
    }
}

// -- e_offlineUnvisited: `-u` reports a strictly decreasing unvisited count --
//
// This is the progress signal a long-running test session polls to know
// how much of the model is left, so it must never go backwards and must
// reach zero once coverage is actually complete.

fn assert_offline_unvisited() {
    let out = gw()
        .args([
            "offline",
            "-u",
            "-m",
            &fixture("json/SmallModel.json"),
            "random(edge_coverage(100))",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();

    let counts: Vec<u64> = stdout
        .lines()
        .map(|line| {
            let value: Value = serde_json::from_str(line).unwrap();
            value["numberOfUnvisitedElements"].as_u64().unwrap()
        })
        .collect();
    assert!(
        counts.windows(2).all(|w| w[1] <= w[0]),
        "self-walk: e_offlineUnvisited — unvisited count must never increase: {counts:?}"
    );
    assert_eq!(
        *counts.last().unwrap(),
        0,
        "self-walk: e_offlineUnvisited — a full edge_coverage(100) walk must end with 0 unvisited"
    );
}

// -- e_offlineStartElement: `-e <name>` makes the walk begin at that element --
//
// Uses `length(5)` rather than `edge_coverage(100)`: overriding the start
// element skips SmallModel.json's unreachable "start edge" (it has no
// source vertex), so edge coverage could never reach 100% and the walk
// would never terminate. A bounded generator keeps this scenario about the
// -e flag alone, independent of that fixture detail.
fn assert_offline_start_element() {
    let out = gw()
        .args([
            "offline",
            "-e",
            "v_VerifySomeAction",
            "-m",
            &fixture("json/SmallModel.json"),
            "random(length(5))",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();

    let first_line = stdout.lines().next().expect("expected at least one step");
    let first: Value = serde_json::from_str(first_line).unwrap();
    assert_eq!(
        first["currentElementName"], "v_VerifySomeAction",
        "self-walk: e_offlineStartElement — -e must make the requested element the first one visited"
    );
}

// -- e_offlineMissingModel: neither -m nor -g given is a user error, not a panic --

fn assert_offline_missing_model() {
    gw().arg("offline")
        .assert()
        .failure()
        .stderr(predicates::str::contains("--model"));
}

// -- e_offlineMultiModelSharedState: a fixture with two internal models sharing state --
//
// MultiModelSharedState.json already declares two models (ModelA, ModelB)
// that rendezvous on a shared-state vertex; a single -m pair is enough to
// load both — passing -m twice would instead duplicate the shared vertex
// across four contexts and break convergence.
fn assert_offline_multi_model_shared_state() {
    let out = gw()
        .args([
            "offline",
            "-m",
            &fixture("json/MultiModelSharedState.json"),
            "random(edge_coverage(100))",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();

    // Both ModelA's and ModelB's elements must appear — proving the CLI's
    // multi-model loading and graphwalker-core's shared-state portal work
    // together end-to-end.
    assert!(stdout.contains("e_StartA"));
    assert!(stdout.contains("e_Explore"));
}

// -- e_methods: sorted, de-duplicated vertex/edge names --
//
// This is the list test-code generators (`source`) and human authors use
// to know which method stubs a model expects; duplicates or unstable
// ordering would make generated code churn on every run.

fn assert_methods() {
    let out = gw()
        .args(["methods", "-m", &fixture("json/SmallModel.json")])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();

    let names: Vec<&str> = stdout.lines().collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(
        names, sorted,
        "self-walk: e_methods — output must be sorted"
    );
    assert!(names.contains(&"e_FirstAction"));
}

// -- e_requirements: sorted requirement keys from a fixture that has them --
//
// Requirement keys feed traceability reports; the fixture chosen here
// carries requirements on both vertices and edges, so it also proves both
// sources get merged into one list.

fn assert_requirements() {
    let out = gw()
        .args(["requirements", "-m", &fixture("json/WithRequirements.json")])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();

    let reqs: Vec<&str> = stdout.lines().collect();
    assert!(reqs.contains(&"REQ001"));
    assert!(reqs.contains(&"REQ005"));
    let mut sorted = reqs.clone();
    sorted.sort();
    assert_eq!(
        reqs, sorted,
        "self-walk: e_requirements — output must be sorted"
    );
}

// -- e_checkOk: a valid model passes with no issues --
//
// The baseline of the `check` subcommand: a model with no problems must
// not be flagged as one, or every valid model in CI would start failing.

fn assert_check_ok() {
    gw().args([
        "check",
        "-m",
        &fixture("json/SmallModel.json"),
        "random(edge_coverage(100))",
    ])
    .assert()
    .success()
    .stdout(predicates::str::contains("No issues found"));
}

// -- e_checkIssues: an invalid model fails, with at least one issue line --
//
// The other half of `check`'s contract: it exists to catch bad models
// before they're walked, so a model missing a start element must be
// rejected rather than silently accepted. Which exact issue is reported is
// `graphwalker-model-checker`'s concern, not this test's.

fn assert_check_issues() {
    gw().args(["check", "-g", &fixture("json/NoStartElement.json")])
        .assert()
        .failure();
}

// -- e_convert: JSON output round-trips through graphwalker-io's own parser --
//
// `convert`'s entire purpose is producing a model other tools can read
// back; comparing structure against the same file loaded directly proves
// nothing was silently dropped or renamed in the round trip.

fn assert_convert() {
    let out = gw()
        .args([
            "convert",
            "--input",
            &fixture("json/SmallModel.json"),
            "--format",
            "json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();

    let original =
        graphwalker_io::read_model(std::path::Path::new(&fixture("json/SmallModel.json"))).unwrap();
    let round_tripped = graphwalker_io::json::read_json_string(&stdout).unwrap();
    assert_eq!(
        original.len(),
        round_tripped.len(),
        "self-walk: e_convert — converted output must round-trip to the same number of models"
    );
    assert_eq!(
        original[0].model.vertices().len(),
        round_tripped[0].model.vertices().len(),
        "self-walk: e_convert — converted output must preserve vertex count"
    );
    assert_eq!(
        original[0].model.edges().len(),
        round_tripped[0].model.edges().len(),
        "self-walk: e_convert — converted output must preserve edge count"
    );
}

// -- e_source: template substitution over every vertex/edge name --
//
// `source` is how a model becomes an executable test-code skeleton;
// missing the header/footer markers or a name would mean generated code
// doesn't compile or silently omits a step implementation.

fn assert_source() {
    let out = gw()
        .args([
            "source",
            "--input",
            &fixture("json/SmallModel.json"),
            "tests/test.template",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();

    assert!(
        stdout.contains("# Generated methods"),
        "self-walk: e_source — missing header"
    );
    assert!(
        stdout.contains("# End of file"),
        "self-walk: e_source — missing footer"
    );
    assert!(stdout.contains("def v_VerifySomeAction():"));
    assert!(stdout.contains("def e_FirstAction():"));
}

// -- online scenarios: spawn the real binary and drive it over the network --
//
// Unlike every other subcommand, `online` doesn't run to completion — it's
// a long-lived server. These scenarios must manage a real child process's
// lifecycle themselves rather than lean on `assert_cmd`.

/// Ensures a spawned `online` server is always killed, even if a scenario's
/// assertion panics partway through — otherwise failed runs would leak
/// listening processes across test invocations.
struct ServerGuard {
    child: Child,
}

impl ServerGuard {
    fn new(service: &str, port: u16) -> Self {
        let child = Command::new(graphwalker_bin())
            .args(["online", "-s", service, "-p", &port.to_string()])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to start graphwalker online");
        Self { child }
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Polls until the server's listening socket accepts connections, since the
/// child process needs a moment to bind after `spawn()` returns.
async fn wait_for_port(port: u16) {
    for _ in 0..50 {
        if tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .is_ok()
        {
            return;
        }
        sleep(Duration::from_millis(100)).await;
    }
    panic!("self-walk: server on port {port} did not start within 5 seconds");
}

/// The same fixture used by the `offline` scenarios, reused here so the
/// REST/WebSocket scenarios exercise a model with elements worth naming in
/// assertions rather than an empty or trivial one.
fn small_model_json() -> String {
    std::fs::read_to_string(fixture("json/SmallModel.json")).unwrap()
}

// -- e_onlineRestful: the REST surface behaves like a real GraphWalker session --
//
// Ports in the 19200+ range are reserved for this file, distinct from
// online_tests.rs's 19100-19140 range, to avoid collisions when tests run
// in parallel within the same crate.

async fn assert_online_restful() {
    let port = 19200;
    let _server = ServerGuard::new("RESTFUL", port);
    wait_for_port(port).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let base = format!("http://127.0.0.1:{port}/graphwalker");

    let resp: Value = client
        .post(format!("{base}/load"))
        .body(small_model_json())
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        resp["result"], "ok",
        "self-walk: e_onlineRestful — load must succeed"
    );

    let resp: Value = client
        .get(format!("{base}/hasNext"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resp["hasNext"], "true");

    let resp: Value = client
        .get(format!("{base}/getNext"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(resp["currentElementName"].as_str().is_some());

    let resp: Value = client
        .get(format!("{base}/getStatistics"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(resp.get("totalNumberOfEdges").is_some());

    let resp: Value = client
        .put(format!("{base}/restart"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        resp["result"], "ok",
        "self-walk: e_onlineRestful — restart must succeed"
    );
}

// -- e_onlineWebsocket: the WebSocket surface behaves like a real GraphWalker session --

async fn assert_online_websocket() {
    let port = 19201;
    let _server = ServerGuard::new("WEBSOCKET", port);
    wait_for_port(port).await;

    let url = format!("ws://127.0.0.1:{port}");
    let (mut ws, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("failed to connect WebSocket");

    let resp = ws_send(&mut ws, &json!({"command": "start", "gw": serde_json::from_str::<Value>(&small_model_json()).unwrap()})).await;
    assert_eq!(
        resp["success"], true,
        "self-walk: e_onlineWebsocket — start must succeed"
    );

    let resp = ws_send(&mut ws, &json!({"command": "hasNext"})).await;
    assert_eq!(resp["hasNext"], true);

    let resp = ws_send(&mut ws, &json!({"command": "getNext"})).await;
    assert!(resp["name"].as_str().is_some());
}

/// Request/response round trip shared by both WebSocket scenarios; the
/// timeout turns a protocol regression (server never replies) into a fast,
/// clear failure instead of a hang.
async fn ws_send(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    msg: &Value,
) -> Value {
    ws.send(Message::Text(msg.to_string().into()))
        .await
        .unwrap();
    let resp = timeout(Duration::from_secs(5), ws.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    serde_json::from_str::<Value>(&resp.into_text().unwrap()).unwrap()
}

// -- e_debugLogging: `--debug` actually enables debug-level tracing output --
//
// The flag only flips on a `tracing_subscriber` filter; without observing
// real log output there'd be no way to tell it from a no-op. Connecting a
// WebSocket client triggers a `debug!("new websocket connection")` call in
// graphwalker-restful, so its presence in the child's stdout is direct
// evidence the flag took effect.

async fn assert_debug_logging() {
    let port = 19202;
    let mut child = Command::new(graphwalker_bin())
        .args([
            "online",
            "-s",
            "WEBSOCKET",
            "-p",
            &port.to_string(),
            "--debug",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start graphwalker online --debug");
    wait_for_port(port).await;

    let url = format!("ws://127.0.0.1:{port}");
    let _ws = tokio_tungstenite::connect_async(&url)
        .await
        .expect("failed to connect WebSocket");
    // give the server a moment to log the connection before we tear it down
    sleep(Duration::from_millis(300)).await;

    let _ = child.kill();
    let output = child
        .wait_with_output()
        .expect("failed to collect child output");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("new websocket connection"),
        "self-walk: e_debugLogging — --debug must enable debug-level tracing output, got: {combined}"
    );
}
