use std::sync::mpsc;

use graphwalker_service::{
    convert_graphml, validate_model, ExecutionId, ExecutionLimits, ExecutionRegistry,
    StartExecution,
};
use serde_json::{json, Value};
use tokio::sync::oneshot;
use tracing::debug;

/// Legacy transport commands. Domain behavior lives in `graphwalker-service`;
/// this actor only maps typed service results to the established REST and
/// WebSocket JSON shapes.
pub enum Command {
    Load {
        json_body: String,
        seed: Option<u64>,
        global_data: Option<String>,
        reply: oneshot::Sender<Result<Value, String>>,
    },
    Check {
        json_body: String,
        reply: oneshot::Sender<Result<Value, String>>,
    },
    HasNext {
        reply: oneshot::Sender<Result<Value, String>>,
    },
    GetNext {
        verbose: bool,
        reply: oneshot::Sender<Result<Value, String>>,
    },
    GetData {
        reply: oneshot::Sender<Result<Value, String>>,
    },
    SetData {
        script: String,
        reply: oneshot::Sender<Result<Value, String>>,
    },
    Restart {
        reply: oneshot::Sender<Result<Value, String>>,
    },
    GetStatistics {
        reply: oneshot::Sender<Result<Value, String>>,
    },
    GetModel {
        reply: oneshot::Sender<Result<Value, String>>,
    },
    UpdateAllElements {
        reply: oneshot::Sender<Result<Value, String>>,
    },
    ConvertGraphml {
        graphml: String,
        reply: oneshot::Sender<Result<Value, String>>,
    },
}

struct MachineState {
    registry: ExecutionRegistry,
    execution_id: Option<ExecutionId>,
}

impl MachineState {
    fn new() -> Self {
        // Loading is transactional: allow the replacement worker to start
        // before closing the currently active execution.
        Self {
            registry: ExecutionRegistry::new(ExecutionLimits { max_executions: 2 }),
            execution_id: None,
        }
    }

    fn execution_id(&self) -> Result<&ExecutionId, String> {
        self.execution_id
            .as_ref()
            .ok_or_else(|| "No model(s) are loaded.".to_string())
    }
}

pub fn spawn_machine_thread() -> mpsc::Sender<Command> {
    let (tx, rx) = mpsc::channel::<Command>();
    std::thread::spawn(move || {
        let mut state = MachineState::new();
        while let Ok(command) = rx.recv() {
            match command {
                Command::Load {
                    json_body,
                    seed,
                    global_data,
                    reply,
                } => {
                    let _ = reply.send(handle_load(&mut state, &json_body, seed, global_data));
                }
                Command::Check { json_body, reply } => {
                    let _ = reply.send(handle_check(&json_body));
                }
                Command::HasNext { reply } => {
                    let _ = reply.send(handle_has_next(&state));
                }
                Command::GetNext { verbose, reply } => {
                    let _ = reply.send(handle_get_next(&state, verbose));
                }
                Command::GetData { reply } => {
                    let _ = reply.send(handle_get_data(&state));
                }
                Command::SetData { script, reply } => {
                    let _ = reply.send(handle_set_data(&state, &script));
                }
                Command::Restart { reply } => {
                    let _ = reply.send(handle_restart(&state));
                }
                Command::GetStatistics { reply } => {
                    let _ = reply.send(handle_get_statistics(&state));
                }
                Command::GetModel { reply } => {
                    let _ = reply.send(handle_get_model(&state));
                }
                Command::UpdateAllElements { reply } => {
                    let _ = reply.send(handle_update_all_elements(&state));
                }
                Command::ConvertGraphml { graphml, reply } => {
                    let _ = reply.send(handle_convert_graphml(&graphml));
                }
            }
        }
    });
    tx
}

pub fn handle_check(json_body: &str) -> Result<Value, String> {
    let model = serde_json::from_str(json_body).map_err(|error| error.to_string())?;
    let result = validate_model(&model).map_err(|error| error.to_string())?;
    let messages = result
        .issues
        .into_iter()
        .map(|issue| issue.message)
        .collect::<Vec<_>>();
    Ok(json!({"result": "ok", "issues": messages}))
}

fn handle_load(
    state: &mut MachineState,
    json_body: &str,
    seed: Option<u64>,
    global_data: Option<String>,
) -> Result<Value, String> {
    let model = serde_json::from_str(json_body).map_err(|error| error.to_string())?;
    let started = state
        .registry
        .start(StartExecution {
            model,
            seed,
            global_data,
        })
        .map_err(|error| error.to_string())?;

    if let Some(previous) = state.execution_id.replace(started.execution_id) {
        let _ = state.registry.close(&previous);
    }
    Ok(json!({"result": "ok", "seed": started.seed}))
}

fn handle_has_next(state: &MachineState) -> Result<Value, String> {
    let status = state
        .registry
        .status(state.execution_id()?)
        .map_err(|error| error.to_string())?;
    Ok(json!({"result": "ok", "hasNext": status.has_next.to_string()}))
}

fn handle_get_next(state: &MachineState, verbose: bool) -> Result<Value, String> {
    let step = state
        .registry
        .next_step(state.execution_id()?)
        .map_err(|error| error.to_string())?;
    let element = step
        .element
        .ok_or_else(|| "No next step is available".to_string())?;

    debug!(
        model = element.model_id,
        element = element.name,
        data = element.data,
        "getNext"
    );
    let mut response = json!({
        "result": "ok",
        "currentElementName": element.name,
        "currentElementID": element.id,
        "modelId": element.model_id,
    });
    if verbose {
        response["data"] = json!(element.data);
        response["visitedCount"] = json!(element.visited_count);
        response["totalCount"] = json!(element.total_count);
        response["stopConditionFulfillment"] = json!(element.stop_condition_fulfillment);
    }
    Ok(response)
}

fn handle_get_data(state: &MachineState) -> Result<Value, String> {
    let data = state
        .registry
        .data(state.execution_id()?)
        .map_err(|error| error.to_string())?;
    Ok(json!({"result": "ok", "data": data}))
}

fn handle_set_data(state: &MachineState, script: &str) -> Result<Value, String> {
    state
        .registry
        .set_data(state.execution_id()?, script)
        .map_err(|error| error.to_string())?;
    Ok(json!({"result": "ok"}))
}

fn handle_restart(state: &MachineState) -> Result<Value, String> {
    state
        .registry
        .restart(state.execution_id()?)
        .map_err(|error| error.to_string())?;
    Ok(json!({"result": "ok"}))
}

fn handle_get_statistics(state: &MachineState) -> Result<Value, String> {
    let statistics = state
        .registry
        .statistics(state.execution_id()?)
        .map_err(|error| error.to_string())?;
    Ok(json!({
        "result": "ok",
        "totalNumberOfVertices": statistics.total_vertices,
        "totalNumberOfEdges": statistics.total_edges,
        "totalNumberOfVisitedVertices": statistics.visited_vertices,
        "totalNumberOfVisitedEdges": statistics.visited_edges,
        "totalNumberOfUnvisitedVertices": statistics.unvisited_vertices,
        "totalNumberOfUnvisitedEdges": statistics.unvisited_edges,
        "vertexCoverage": statistics.vertex_coverage,
        "edgeCoverage": statistics.edge_coverage,
    }))
}

fn handle_get_model(state: &MachineState) -> Result<Value, String> {
    let model = state
        .registry
        .model(state.execution_id()?)
        .map_err(|error| error.to_string())?;
    Ok(json!({"result": "ok", "models": model.model.to_string()}))
}

fn handle_update_all_elements(state: &MachineState) -> Result<Value, String> {
    let elements = state
        .registry
        .elements(state.execution_id()?)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|element| {
            json!({
                "modelId": element.model_id,
                "elementId": element.element_id,
                "visitedCount": element.visited_count,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({"result": "ok", "elements": elements}))
}

pub fn handle_convert_graphml(graphml: &str) -> Result<Value, String> {
    let result = convert_graphml(graphml).map_err(|error| error.to_string())?;
    Ok(json!({"result": "ok", "models": result.model.to_string()}))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODEL: &str = r#"{
        "models": [{
            "name": "Small",
            "generator": "random(edge_coverage(100))",
            "startElementId": "e0",
            "vertices": [
                {"name": "v_A", "id": "n0"},
                {"name": "v_B", "id": "n1"}
            ],
            "edges": [
                {"name": "e_Start", "id": "e0", "targetVertexId": "n0"},
                {"name": "e_AB", "id": "e1", "sourceVertexId": "n0", "targetVertexId": "n1"},
                {"name": "e_Loop", "id": "e2", "sourceVertexId": "n1", "targetVertexId": "n1"},
                {"name": "e_BA", "id": "e3", "sourceVertexId": "n1", "targetVertexId": "n0"}
            ]
        }]
    }"#;

    #[test]
    fn legacy_execution_response_shapes_are_preserved() {
        let mut state = MachineState::new();
        let loaded = handle_load(&mut state, MODEL, Some(42), None).unwrap();
        assert_eq!(loaded, json!({"result": "ok", "seed": 42}));

        let has_next = handle_has_next(&state).unwrap();
        assert_eq!(has_next, json!({"result": "ok", "hasNext": "true"}));

        let step = handle_get_next(&state, false).unwrap();
        assert_eq!(step["result"], "ok");
        assert_eq!(step["currentElementName"], "e_Start");
        assert_eq!(step["currentElementID"], "e0");
        assert!(step.get("data").is_none());

        let statistics = handle_get_statistics(&state).unwrap();
        assert_eq!(statistics["totalNumberOfVertices"], 2);
        assert_eq!(statistics["totalNumberOfEdges"], 4);
        assert_eq!(statistics["totalNumberOfVisitedEdges"], 1);

        let returned_model = handle_get_model(&state).unwrap();
        assert_eq!(returned_model["result"], "ok");
        assert!(returned_model["models"].as_str().is_some());
    }

    #[test]
    fn verbose_websocket_step_shape_is_preserved() {
        let mut state = MachineState::new();
        handle_load(
            &mut state,
            MODEL,
            Some(42),
            Some("sessionValue=7".to_string()),
        )
        .unwrap();

        let step = handle_get_next(&state, true).unwrap();
        assert_eq!(step["visitedCount"], 1);
        assert_eq!(step["totalCount"], 1);
        assert!(step["stopConditionFulfillment"].is_number());
        assert!(step["data"].as_str().unwrap().contains("sessionValue=7"));
    }

    #[test]
    fn failed_load_does_not_replace_the_active_execution() {
        let mut state = MachineState::new();
        handle_load(&mut state, MODEL, Some(42), None).unwrap();
        let execution_id = state.execution_id.clone();

        assert!(handle_load(&mut state, r#"{"models": []}"#, Some(1), None).is_err());
        assert_eq!(state.execution_id, execution_id);
        assert_eq!(handle_has_next(&state).unwrap()["hasNext"], "true");
    }
}
