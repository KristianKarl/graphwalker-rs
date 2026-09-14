use std::collections::HashMap;
use std::sync::{Arc, Barrier};
use std::time::Duration;

use graphwalker_io::json::read_json_string;
use graphwalker_service::{
    AddEdge, AddVertex, CreateModel, DraftId, DraftLimits, DraftRegistry, ExecutionRegistry,
    FieldPatch, RemoveElement, ServiceErrorCode, StartExecution, UpdateEdge, UpdateModel,
    UpdateVertex,
};
use serde_json::{json, Value};

fn create(registry: &DraftRegistry, generator: Option<&str>) -> DraftId {
    registry
        .create_model(CreateModel {
            generator: generator.map(str::to_string),
            ..CreateModel::default()
        })
        .unwrap()
        .draft_id
}

fn add_vertex(
    registry: &DraftRegistry,
    draft_id: &DraftId,
    id: Option<&str>,
    expected_revision: Option<u64>,
) -> graphwalker_service::VertexResult {
    registry
        .add_vertex(AddVertex {
            draft_id: draft_id.clone(),
            id: id.map(str::to_string),
            name: id.map(|id| format!("v_{id}")),
            shared_state: None,
            actions: Vec::new(),
            requirements: Vec::new(),
            properties: HashMap::new(),
            expected_revision,
        })
        .unwrap()
}

fn edge_request(
    draft_id: &DraftId,
    id: Option<&str>,
    source: Option<&str>,
    target: Option<&str>,
) -> AddEdge {
    AddEdge {
        draft_id: draft_id.clone(),
        id: id.map(str::to_string),
        name: id.map(|id| format!("e_{id}")),
        source_vertex_id: source.map(str::to_string),
        target_vertex_id: target.map(str::to_string),
        guard: None,
        actions: Vec::new(),
        requirements: Vec::new(),
        properties: HashMap::new(),
        weight: None,
        dependency: None,
        expected_revision: None,
    }
}

fn assert_unchanged(registry: &DraftRegistry, draft_id: &DraftId, before: &Value, revision: u64) {
    let after = registry.export_model(draft_id).unwrap();
    assert_eq!(&after.model, before);
    assert_eq!(after.revision, revision);
}

#[test]
fn create_model_supports_minimal_and_fully_populated_metadata() {
    let registry = DraftRegistry::default();
    let minimal = registry.create_model(CreateModel::default()).unwrap();
    let minimal_export = registry.export_model(&minimal.draft_id).unwrap();
    assert_eq!(minimal.revision, 0);
    assert_eq!(minimal_export.revision, 0);
    assert_eq!(minimal_export.model["models"][0]["id"], minimal.model_id);
    assert_eq!(minimal_export.model["models"][0]["vertices"], json!([]));
    assert_eq!(minimal_export.model["models"][0]["edges"], json!([]));

    let populated = registry
        .create_model(CreateModel {
            model_id: Some("model-custom".to_string()),
            name: Some("Checkout".to_string()),
            generator: Some("random(length(10))".to_string()),
            actions: vec!["x=1".to_string()],
            requirements: vec!["REQ-1".to_string()],
            properties: HashMap::from([("owner".to_string(), json!("qa"))]),
        })
        .unwrap();
    let model = &registry.export_model(&populated.draft_id).unwrap().model["models"][0];
    assert_eq!(populated.model_id, "model-custom");
    assert_eq!(model["name"], "Checkout");
    assert_eq!(model["generator"], "random(length(10))");
    assert_eq!(model["actions"], json!(["x=1"]));
    assert_eq!(model["requirements"], json!(["REQ-1"]));
    assert_eq!(model["properties"]["owner"], "qa");
    assert_ne!(minimal.draft_id, populated.draft_id);
}

#[test]
fn failed_creation_does_not_leak_a_draft_and_limit_is_enforced() {
    let registry = DraftRegistry::new(DraftLimits {
        max_drafts: 1,
        ..DraftLimits::default()
    });
    let error = registry
        .create_model(CreateModel {
            model_id: Some(String::new()),
            ..CreateModel::default()
        })
        .unwrap_err();
    assert_eq!(error.code, ServiceErrorCode::InvalidElement);
    assert!(registry.is_empty());

    create(&registry, None);
    let error = registry.create_model(CreateModel::default()).unwrap_err();
    assert_eq!(error.code, ServiceErrorCode::DraftLimitReached);
    assert_eq!(registry.len(), 1);
}

#[test]
fn add_vertex_preserves_all_fields_and_rejects_cross_kind_duplicates_atomically() {
    let registry = DraftRegistry::default();
    let draft_id = create(&registry, None);
    let generated = add_vertex(&registry, &draft_id, None, Some(0));
    assert!(!generated.vertex.id.is_empty());
    assert_eq!(generated.revision, 1);

    let populated = registry
        .add_vertex(AddVertex {
            draft_id: draft_id.clone(),
            id: Some("v_full".to_string()),
            name: Some("v_Full".to_string()),
            shared_state: Some("signed_in".to_string()),
            actions: vec!["x=1".to_string()],
            requirements: vec!["REQ-2".to_string()],
            properties: HashMap::from([("x".to_string(), json!(12))]),
            expected_revision: Some(1),
        })
        .unwrap();
    assert_eq!(populated.revision, 2);
    assert_eq!(populated.vertex.shared_state.as_deref(), Some("signed_in"));
    assert_eq!(populated.vertex.actions, vec!["x=1"]);

    let before = registry.export_model(&draft_id).unwrap();
    let mut edge = edge_request(&draft_id, Some(&generated.vertex.id), None, Some("v_full"));
    edge.expected_revision = Some(before.revision);
    let error = registry.add_edge(edge).unwrap_err();
    assert_eq!(error.code, ServiceErrorCode::DuplicateElementId);
    assert_unchanged(&registry, &draft_id, &before.model, before.revision);

    let error = registry
        .add_vertex(AddVertex {
            draft_id: draft_id.clone(),
            id: Some("stale".to_string()),
            name: None,
            shared_state: None,
            actions: Vec::new(),
            requirements: Vec::new(),
            properties: HashMap::new(),
            expected_revision: Some(0),
        })
        .unwrap_err();
    assert_eq!(error.code, ServiceErrorCode::RevisionConflict);
    assert_unchanged(&registry, &draft_id, &before.model, before.revision);

    registry
        .add_edge(edge_request(
            &draft_id,
            Some("existing_edge"),
            None,
            Some("v_full"),
        ))
        .unwrap();
    let before = registry.export_model(&draft_id).unwrap();
    let error = registry
        .add_vertex(AddVertex {
            draft_id: draft_id.clone(),
            id: Some("existing_edge".to_string()),
            name: None,
            shared_state: None,
            actions: Vec::new(),
            requirements: Vec::new(),
            properties: HashMap::new(),
            expected_revision: Some(before.revision),
        })
        .unwrap_err();
    assert_eq!(error.code, ServiceErrorCode::DuplicateElementId);
    assert_unchanged(&registry, &draft_id, &before.model, before.revision);
}

#[test]
fn add_edge_supports_variants_boundaries_and_all_optional_fields() {
    let registry = DraftRegistry::default();
    let draft_id = create(&registry, None);
    add_vertex(&registry, &draft_id, Some("a"), None);
    add_vertex(&registry, &draft_id, Some("b"), None);

    let start = registry
        .add_edge(edge_request(&draft_id, Some("start"), None, Some("a")))
        .unwrap();
    assert!(start.edge.source_vertex_id.is_none());
    let self_loop = registry
        .add_edge(edge_request(&draft_id, Some("loop"), Some("a"), Some("a")))
        .unwrap();
    assert_eq!(self_loop.edge.source_vertex_id.as_deref(), Some("a"));

    let mut full = edge_request(&draft_id, None, Some("a"), Some("b"));
    full.name = Some("e_Full".to_string());
    full.guard = Some("allowed".to_string());
    full.actions = vec!["count += 1".to_string()];
    full.requirements = vec!["REQ-3".to_string()];
    full.properties = HashMap::from([("priority".to_string(), json!("high"))]);
    full.weight = Some(1.0);
    full.dependency = Some(100);
    let full = registry.add_edge(full).unwrap();
    assert!(!full.edge.id.is_empty());
    assert_eq!(full.edge.guard.as_deref(), Some("allowed"));
    assert_eq!(full.edge.weight, Some(1.0));
    assert_eq!(full.edge.dependency, Some(100));

    let mut lower = edge_request(&draft_id, Some("lower"), Some("b"), Some("a"));
    lower.weight = Some(0.0);
    lower.dependency = Some(0);
    registry.add_edge(lower).unwrap();
}

#[test]
fn invalid_edges_and_revision_conflicts_are_atomic() {
    let registry = DraftRegistry::default();
    let draft_id = create(&registry, None);
    add_vertex(&registry, &draft_id, Some("a"), None);

    let cases = [
        (
            edge_request(&draft_id, Some("missing"), None, None),
            ServiceErrorCode::MissingTargetVertex,
        ),
        (
            edge_request(&draft_id, Some("source"), Some("unknown"), Some("a")),
            ServiceErrorCode::UnknownVertex,
        ),
        (
            edge_request(&draft_id, Some("target"), None, Some("unknown")),
            ServiceErrorCode::UnknownVertex,
        ),
    ];
    for (request, code) in cases {
        let before = registry.export_model(&draft_id).unwrap();
        assert_eq!(registry.add_edge(request).unwrap_err().code, code);
        assert_unchanged(&registry, &draft_id, &before.model, before.revision);
    }

    for weight in [-0.01, 1.01, f64::NAN] {
        let before = registry.export_model(&draft_id).unwrap();
        let mut request = edge_request(&draft_id, Some("weight"), None, Some("a"));
        request.weight = Some(weight);
        assert_eq!(
            registry.add_edge(request).unwrap_err().code,
            ServiceErrorCode::InvalidWeight
        );
        assert_unchanged(&registry, &draft_id, &before.model, before.revision);
    }
    for dependency in [-1, 101] {
        let before = registry.export_model(&draft_id).unwrap();
        let mut request = edge_request(&draft_id, Some("dependency"), None, Some("a"));
        request.dependency = Some(dependency);
        assert_eq!(
            registry.add_edge(request).unwrap_err().code,
            ServiceErrorCode::InvalidDependency
        );
        assert_unchanged(&registry, &draft_id, &before.model, before.revision);
    }

    let before = registry.export_model(&draft_id).unwrap();
    let mut stale = edge_request(&draft_id, Some("stale"), None, Some("a"));
    stale.expected_revision = Some(0);
    assert_eq!(
        registry.add_edge(stale).unwrap_err().code,
        ServiceErrorCode::RevisionConflict
    );
    assert_unchanged(&registry, &draft_id, &before.model, before.revision);
}

#[test]
fn updates_set_and_clear_optional_fields() {
    let registry = DraftRegistry::default();
    let draft_id = create(&registry, None);
    add_vertex(&registry, &draft_id, Some("a"), None);
    add_vertex(&registry, &draft_id, Some("b"), None);
    registry
        .add_edge(edge_request(&draft_id, Some("edge"), Some("a"), Some("b")))
        .unwrap();

    let model = registry
        .update_model(UpdateModel {
            draft_id: draft_id.clone(),
            name: FieldPatch::Set("Updated".to_string()),
            generator: FieldPatch::Set("random(length(2))".to_string()),
            start_element_id: FieldPatch::Set("edge".to_string()),
            actions: FieldPatch::Keep,
            requirements: FieldPatch::Keep,
            properties: FieldPatch::Keep,
            predefined_path_edge_ids: FieldPatch::Set(vec!["edge".to_string()]),
            expected_revision: Some(3),
        })
        .unwrap();
    assert_eq!(model.model.start_element_id.as_deref(), Some("edge"));

    let vertex = registry
        .update_vertex(UpdateVertex {
            draft_id: draft_id.clone(),
            vertex_id: "a".to_string(),
            name: FieldPatch::Clear,
            shared_state: FieldPatch::Set("shared".to_string()),
            actions: FieldPatch::Keep,
            requirements: FieldPatch::Keep,
            properties: FieldPatch::Keep,
            expected_revision: Some(4),
        })
        .unwrap();
    assert!(vertex.vertex.name.is_none());
    assert_eq!(vertex.vertex.shared_state.as_deref(), Some("shared"));

    let edge = registry
        .update_edge(UpdateEdge {
            draft_id: draft_id.clone(),
            edge_id: "edge".to_string(),
            name: FieldPatch::Keep,
            source_vertex_id: FieldPatch::Clear,
            target_vertex_id: FieldPatch::Set("a".to_string()),
            guard: FieldPatch::Set("ready".to_string()),
            actions: FieldPatch::Keep,
            requirements: FieldPatch::Keep,
            properties: FieldPatch::Keep,
            weight: FieldPatch::Set(0.5),
            dependency: FieldPatch::Set(50),
            expected_revision: Some(5),
        })
        .unwrap();
    assert!(edge.edge.source_vertex_id.is_none());
    assert_eq!(edge.edge.target_vertex_id.as_deref(), Some("a"));
    assert_eq!(edge.edge.guard.as_deref(), Some("ready"));

    let model = registry
        .update_model(UpdateModel {
            draft_id: draft_id.clone(),
            name: FieldPatch::Clear,
            generator: FieldPatch::Clear,
            start_element_id: FieldPatch::Clear,
            actions: FieldPatch::Clear,
            requirements: FieldPatch::Clear,
            properties: FieldPatch::Clear,
            predefined_path_edge_ids: FieldPatch::Clear,
            expected_revision: Some(6),
        })
        .unwrap();
    assert!(model.model.name.is_none());
    assert!(model.model.generator.is_none());
    assert!(model.model.start_element_id.is_none());
}

#[test]
fn failed_updates_and_removals_leave_the_complete_draft_unchanged() {
    let registry = DraftRegistry::default();
    let draft_id = create(&registry, Some("random(length(2))"));
    add_vertex(&registry, &draft_id, Some("a"), None);
    add_vertex(&registry, &draft_id, Some("b"), None);
    registry
        .add_edge(edge_request(&draft_id, Some("edge"), Some("a"), Some("b")))
        .unwrap();

    let before = registry.export_model(&draft_id).unwrap();
    let invalid_edge = registry
        .update_edge(UpdateEdge {
            draft_id: draft_id.clone(),
            edge_id: "edge".to_string(),
            name: FieldPatch::Set("changed".to_string()),
            source_vertex_id: FieldPatch::Keep,
            target_vertex_id: FieldPatch::Set("missing".to_string()),
            guard: FieldPatch::Keep,
            actions: FieldPatch::Keep,
            requirements: FieldPatch::Keep,
            properties: FieldPatch::Keep,
            weight: FieldPatch::Set(2.0),
            dependency: FieldPatch::Keep,
            expected_revision: Some(before.revision),
        })
        .unwrap_err();
    assert_eq!(invalid_edge.code, ServiceErrorCode::InvalidWeight);
    assert_unchanged(&registry, &draft_id, &before.model, before.revision);

    let invalid_model = registry
        .update_model(UpdateModel {
            draft_id: draft_id.clone(),
            name: FieldPatch::Set("changed".to_string()),
            generator: FieldPatch::Keep,
            start_element_id: FieldPatch::Set("missing".to_string()),
            actions: FieldPatch::Keep,
            requirements: FieldPatch::Keep,
            properties: FieldPatch::Keep,
            predefined_path_edge_ids: FieldPatch::Keep,
            expected_revision: Some(before.revision),
        })
        .unwrap_err();
    assert_eq!(invalid_model.code, ServiceErrorCode::InvalidElement);
    assert_unchanged(&registry, &draft_id, &before.model, before.revision);

    let invalid_vertex = registry
        .update_vertex(UpdateVertex {
            draft_id: draft_id.clone(),
            vertex_id: "missing".to_string(),
            name: FieldPatch::Set("changed".to_string()),
            shared_state: FieldPatch::Keep,
            actions: FieldPatch::Keep,
            requirements: FieldPatch::Keep,
            properties: FieldPatch::Keep,
            expected_revision: Some(before.revision),
        })
        .unwrap_err();
    assert_eq!(invalid_vertex.code, ServiceErrorCode::InvalidElement);
    assert_unchanged(&registry, &draft_id, &before.model, before.revision);

    let invalid_remove = registry
        .remove_element(RemoveElement {
            draft_id: draft_id.clone(),
            element_id: "missing".to_string(),
            cascade: true,
            cleanup_references: true,
            expected_revision: Some(before.revision),
        })
        .unwrap_err();
    assert_eq!(invalid_remove.code, ServiceErrorCode::InvalidElement);
    assert_unchanged(&registry, &draft_id, &before.model, before.revision);
}

#[test]
fn connected_vertex_removal_requires_cascade_and_reference_cleanup() {
    let registry = DraftRegistry::default();
    let draft_id = create(&registry, None);
    add_vertex(&registry, &draft_id, Some("a"), None);
    add_vertex(&registry, &draft_id, Some("b"), None);
    registry
        .add_edge(edge_request(&draft_id, Some("edge"), Some("a"), Some("b")))
        .unwrap();
    registry
        .update_model(UpdateModel {
            draft_id: draft_id.clone(),
            name: FieldPatch::Keep,
            generator: FieldPatch::Keep,
            start_element_id: FieldPatch::Set("edge".to_string()),
            actions: FieldPatch::Keep,
            requirements: FieldPatch::Keep,
            properties: FieldPatch::Keep,
            predefined_path_edge_ids: FieldPatch::Set(vec!["edge".to_string()]),
            expected_revision: None,
        })
        .unwrap();
    let before = registry.export_model(&draft_id).unwrap();

    let request = |cascade, cleanup_references| RemoveElement {
        draft_id: draft_id.clone(),
        element_id: "a".to_string(),
        cascade,
        cleanup_references,
        expected_revision: Some(before.revision),
    };
    assert_eq!(
        registry
            .remove_element(request(false, false))
            .unwrap_err()
            .code,
        ServiceErrorCode::ReferencedElement
    );
    assert_unchanged(&registry, &draft_id, &before.model, before.revision);
    assert_eq!(
        registry
            .remove_element(request(true, false))
            .unwrap_err()
            .code,
        ServiceErrorCode::ReferencedElement
    );
    assert_unchanged(&registry, &draft_id, &before.model, before.revision);

    let removed = registry.remove_element(request(true, true)).unwrap();
    assert_eq!(removed.removed_ids, vec!["a", "edge"]);
    let model = registry.export_model(&draft_id).unwrap().model;
    assert!(model["models"][0].get("startElementId").is_none());
    assert!(model["models"][0].get("predefinedPathEdgeIds").is_none());
}

#[test]
fn draft_limits_expiry_discard_and_isolation_are_enforced() {
    let limited = DraftRegistry::new(DraftLimits {
        max_drafts: 4,
        max_vertices_per_draft: 1,
        max_edges_per_draft: 1,
        idle_timeout: Duration::from_secs(60),
    });
    let first = create(&limited, None);
    let second = create(&limited, None);
    add_vertex(&limited, &first, Some("a"), None);
    add_vertex(&limited, &second, Some("b"), None);
    let before_first = limited.export_model(&first).unwrap();
    let error = limited
        .add_vertex(AddVertex {
            draft_id: first.clone(),
            id: Some("too-many".to_string()),
            name: None,
            shared_state: None,
            actions: Vec::new(),
            requirements: Vec::new(),
            properties: HashMap::new(),
            expected_revision: Some(before_first.revision),
        })
        .unwrap_err();
    assert_eq!(error.code, ServiceErrorCode::ModelLimitReached);
    assert_unchanged(&limited, &first, &before_first.model, before_first.revision);
    assert_eq!(
        limited.export_model(&second).unwrap().model["models"][0]["vertices"][0]["id"],
        "b"
    );

    limited
        .add_edge(edge_request(&second, Some("start"), None, Some("b")))
        .unwrap();
    let before_second = limited.export_model(&second).unwrap();
    let error = limited
        .add_edge(edge_request(&second, Some("too-many"), None, Some("b")))
        .unwrap_err();
    assert_eq!(error.code, ServiceErrorCode::ModelLimitReached);
    assert_unchanged(
        &limited,
        &second,
        &before_second.model,
        before_second.revision,
    );

    limited.discard(&first).unwrap();
    assert_eq!(
        limited.export_model(&first).unwrap_err().code,
        ServiceErrorCode::DraftNotFound
    );

    let expiring = DraftRegistry::new(DraftLimits {
        idle_timeout: Duration::ZERO,
        ..DraftLimits::default()
    });
    let expired = create(&expiring, None);
    assert_eq!(
        expiring.export_model(&expired).unwrap_err().code,
        ServiceErrorCode::DraftExpired
    );
}

#[test]
fn generated_ids_are_unique_under_concurrent_mutation() {
    let registry = DraftRegistry::default();
    let draft_id = create(&registry, None);
    let barrier = Arc::new(Barrier::new(17));
    let workers = (0..16)
        .map(|_| {
            let registry = registry.clone();
            let draft_id = draft_id.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                add_vertex(&registry, &draft_id, None, None).vertex.id
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let ids = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), 16);
    assert_eq!(registry.export_model(&draft_id).unwrap().revision, 16);
}

#[test]
fn golden_authoring_workflow_exports_valid_executable_json() {
    let drafts = DraftRegistry::default();
    let created = drafts
        .create_model(CreateModel {
            model_id: Some("model-golden".to_string()),
            name: Some("Golden".to_string()),
            generator: Some("random(vertex_coverage(100))".to_string()),
            ..CreateModel::default()
        })
        .unwrap();
    for (id, name, revision) in [("v_a", "v_A", 0), ("v_b", "v_B", 1)] {
        drafts
            .add_vertex(AddVertex {
                draft_id: created.draft_id.clone(),
                id: Some(id.to_string()),
                name: Some(name.to_string()),
                shared_state: None,
                actions: Vec::new(),
                requirements: Vec::new(),
                properties: HashMap::new(),
                expected_revision: Some(revision),
            })
            .unwrap();
    }
    let mut start = edge_request(&created.draft_id, Some("e_start"), None, Some("v_a"));
    start.name = Some("e_Start".to_string());
    drafts.add_edge(start).unwrap();
    let mut normal = edge_request(&created.draft_id, Some("e_ab"), Some("v_a"), Some("v_b"));
    normal.name = Some("e_AB".to_string());
    drafts.add_edge(normal).unwrap();
    drafts
        .update_model(UpdateModel {
            draft_id: created.draft_id.clone(),
            name: FieldPatch::Keep,
            generator: FieldPatch::Keep,
            start_element_id: FieldPatch::Set("e_start".to_string()),
            actions: FieldPatch::Keep,
            requirements: FieldPatch::Keep,
            properties: FieldPatch::Keep,
            predefined_path_edge_ids: FieldPatch::Keep,
            expected_revision: Some(4),
        })
        .unwrap();

    let exported = drafts.export_model(&created.draft_id).unwrap();
    assert_eq!(drafts.export_model(&created.draft_id).unwrap(), exported);
    let golden: Value =
        serde_json::from_str(include_str!("fixtures/authoring_golden.json")).unwrap();
    assert_eq!(exported.model, golden);
    assert_eq!(exported.revision, 5);
    assert_eq!(
        read_json_string(&exported.model.to_string()).unwrap().len(),
        1
    );
    let validation = drafts.validate(&created.draft_id).unwrap();
    assert!(
        validation.valid,
        "unexpected issues: {:?}",
        validation.issues
    );

    let snapshot = drafts.snapshot(&created.draft_id, Some(5)).unwrap();
    let executions = ExecutionRegistry::default();
    let started = executions
        .start(StartExecution {
            model: snapshot.model.clone(),
            seed: Some(42),
            global_data: None,
        })
        .unwrap();

    drafts
        .add_vertex(AddVertex {
            draft_id: created.draft_id.clone(),
            id: Some("later_vertex".to_string()),
            name: Some("v_Later".to_string()),
            shared_state: None,
            actions: Vec::new(),
            requirements: Vec::new(),
            properties: HashMap::new(),
            expected_revision: Some(5),
        })
        .unwrap();
    assert_eq!(
        executions.model(&started.execution_id).unwrap().model["models"][0]["vertices"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        drafts.export_model(&created.draft_id).unwrap().model["models"][0]["vertices"]
            .as_array()
            .unwrap()
            .len(),
        3
    );

    let first = executions.next_step(&started.execution_id).unwrap();
    assert_eq!(first.element.unwrap().id, "e_start");
    let mut path = vec!["e_start".to_string()];
    loop {
        let step = executions.next_step(&started.execution_id).unwrap();
        if step.completed {
            break;
        }
        path.push(step.element.unwrap().id);
    }
    assert_eq!(path, vec!["e_start", "v_a", "e_ab", "v_b"]);
}

#[test]
fn execution_snapshot_is_immutable_after_later_draft_edits() {
    let drafts = DraftRegistry::default();
    let draft_id = create(&drafts, Some("random(vertex_coverage(100))"));
    add_vertex(&drafts, &draft_id, Some("a"), None);
    let snapshot = drafts.snapshot(&draft_id, Some(1)).unwrap();

    add_vertex(&drafts, &draft_id, Some("b"), Some(1));
    assert_eq!(
        drafts.export_model(&draft_id).unwrap().model["models"][0]["vertices"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        snapshot.model["models"][0]["vertices"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
