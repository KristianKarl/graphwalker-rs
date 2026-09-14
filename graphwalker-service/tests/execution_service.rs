use std::sync::{Arc, Barrier};

use graphwalker_service::{
    convert_graphml, validate_model, ExecutionId, ExecutionLimits, ExecutionRegistry,
    ServiceErrorCode, StartExecution,
};
use serde_json::{json, Value};

const SMALL_MODEL_JSON: &str = r#"{
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

fn model() -> Value {
    serde_json::from_str(SMALL_MODEL_JSON).unwrap()
}

fn start(
    registry: &ExecutionRegistry,
    seed: Option<u64>,
    global_data: Option<&str>,
) -> (ExecutionId, u64) {
    let started = registry
        .start(StartExecution {
            model: model(),
            seed,
            global_data: global_data.map(str::to_string),
        })
        .unwrap();
    (started.execution_id, started.seed)
}

fn run_to_completion(registry: &ExecutionRegistry, execution_id: &ExecutionId) -> Vec<String> {
    let mut path = Vec::new();
    loop {
        let step = registry.next_step(execution_id).unwrap();
        if step.completed {
            assert!(step.element.is_none());
            break;
        }
        path.push(step.element.unwrap().id);
    }
    path
}

#[test]
fn deterministic_restart_preserves_seed_and_global_data() {
    let registry = ExecutionRegistry::default();
    let (execution_id, seed) = start(&registry, Some(42), Some("x=10;y=20"));

    let first_path = run_to_completion(&registry, &execution_id);
    assert!(!first_path.is_empty());
    assert!(registry.data(&execution_id).unwrap().contains("x=10"));

    let restarted = registry.restart(&execution_id).unwrap();
    assert_eq!(restarted.seed, seed);
    assert!(registry.data(&execution_id).unwrap().contains("x=10"));
    assert!(registry.data(&execution_id).unwrap().contains("y=20"));

    let replayed_path = run_to_completion(&registry, &execution_id);
    assert_eq!(first_path, replayed_path);
}

#[test]
fn generated_seed_can_be_replayed() {
    let registry = ExecutionRegistry::default();
    let (first_id, generated_seed) = start(&registry, None, None);
    let first_path = run_to_completion(&registry, &first_id);

    let (replay_id, replay_seed) = start(&registry, Some(generated_seed), None);
    assert_eq!(replay_seed, generated_seed);
    assert_eq!(run_to_completion(&registry, &replay_id), first_path);
}

#[test]
fn concurrent_executions_are_isolated() {
    let registry = ExecutionRegistry::default();
    let (first_id, _) = start(&registry, Some(42), Some("owner=1"));
    let (second_id, _) = start(&registry, Some(42), Some("owner=2"));
    assert_ne!(first_id, second_id);
    assert!(first_id.as_str().starts_with("execution_"));

    let barrier = Arc::new(Barrier::new(3));
    let first_worker = {
        let registry = registry.clone();
        let execution_id = first_id.clone();
        let barrier = Arc::clone(&barrier);
        std::thread::spawn(move || {
            barrier.wait();
            run_to_completion(&registry, &execution_id)
        })
    };
    let second_worker = {
        let registry = registry.clone();
        let execution_id = second_id.clone();
        let barrier = Arc::clone(&barrier);
        std::thread::spawn(move || {
            barrier.wait();
            run_to_completion(&registry, &execution_id)
        })
    };
    barrier.wait();

    assert_eq!(first_worker.join().unwrap(), second_worker.join().unwrap());
    assert!(registry.data(&first_id).unwrap().contains("owner=1"));
    assert!(registry.data(&second_id).unwrap().contains("owner=2"));
    assert_eq!(
        registry.statistics(&first_id),
        registry.statistics(&second_id)
    );
}

#[test]
fn calls_to_one_execution_are_serialized() {
    let registry = ExecutionRegistry::default();
    let (execution_id, _) = start(&registry, Some(42), Some("counter=0"));

    let workers = (0..16)
        .map(|_| {
            let registry = registry.clone();
            let execution_id = execution_id.clone();
            std::thread::spawn(move || {
                registry
                    .set_data(&execution_id, "global.counter += 1")
                    .unwrap();
            })
        })
        .collect::<Vec<_>>();
    for worker in workers {
        worker.join().unwrap();
    }

    assert!(registry.data(&execution_id).unwrap().contains("counter=16"));
}

#[test]
fn registry_enforces_limits_and_releases_closed_executions() {
    let registry = ExecutionRegistry::new(ExecutionLimits { max_executions: 1 });
    let (execution_id, _) = start(&registry, Some(42), None);

    let error = registry
        .start(StartExecution {
            model: model(),
            seed: Some(43),
            global_data: None,
        })
        .unwrap_err();
    assert_eq!(error.code, ServiceErrorCode::ExecutionLimitReached);

    registry.close(&execution_id).unwrap();
    assert!(registry.is_empty());
    let missing = registry.status(&execution_id).unwrap_err();
    assert_eq!(missing.code, ServiceErrorCode::ExecutionNotFound);
    start(&registry, Some(43), None);
}

#[test]
fn status_statistics_and_model_results_are_typed() {
    let registry = ExecutionRegistry::default();
    let (execution_id, _) = start(&registry, Some(42), None);

    let initial = registry.status(&execution_id).unwrap();
    assert!(initial.has_next);
    let initial_statistics = registry.statistics(&execution_id).unwrap();
    assert_eq!(initial_statistics.total_vertices, 2);
    assert_eq!(initial_statistics.total_edges, 4);
    assert_eq!(initial_statistics.visited_vertices, 0);
    assert_eq!(initial_statistics.visited_edges, 0);

    let step = registry.next_step(&execution_id).unwrap();
    let element = step.element.unwrap();
    assert_eq!(element.id, "e0");
    assert_eq!(element.visited_count, 1);
    assert_eq!(
        registry.model(&execution_id).unwrap().model["models"][0]["name"],
        "Small"
    );
    assert_eq!(registry.elements(&execution_id).unwrap().len(), 6);
}

#[test]
fn validation_returns_typed_issues() {
    let valid = validate_model(&model()).unwrap();
    assert!(valid.valid);
    assert!(valid.issues.is_empty());

    let invalid = validate_model(&json!({
        "models": [{
            "generator": "random(edge_coverage(100))",
            "vertices": [{"id": "n0", "name": ""}],
            "edges": []
        }]
    }))
    .unwrap();
    assert!(!invalid.valid);
    assert!(invalid
        .issues
        .iter()
        .any(|issue| issue.message.contains("empty string")));
}

#[test]
fn errors_have_stable_typed_codes() {
    let registry = ExecutionRegistry::default();
    let missing_generator = json!({
        "models": [{
            "startElementId": "n0",
            "vertices": [{"id": "n0", "name": "v_Start"}],
            "edges": []
        }]
    });
    let error = registry
        .start(StartExecution {
            model: missing_generator,
            seed: Some(42),
            global_data: None,
        })
        .unwrap_err();
    assert_eq!(error.code, ServiceErrorCode::InvalidGenerator);

    let (execution_id, _) = start(&registry, Some(42), None);
    let error = registry
        .set_data(&execution_id, "this is not valid Rhai (")
        .unwrap_err();
    assert_eq!(error.code, ServiceErrorCode::InvalidData);
}

#[test]
fn graphml_conversion_returns_a_json_model_object() {
    let graphml = include_str!("../../graphwalker-io/tests/fixtures/graphml/Login.graphml");
    let converted = convert_graphml(graphml).unwrap();
    assert!(converted.model.is_object());
    assert!(!converted.model["models"].as_array().unwrap().is_empty());
}
