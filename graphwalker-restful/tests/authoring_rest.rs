use axum::body::{to_bytes, Body};
use axum::http::header::CONTENT_TYPE;
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use graphwalker_restful::MAX_AUTHORING_BODY_BYTES;
use graphwalker_service::{DraftLimits, DraftRegistry};
use serde_json::{json, Value};
use std::time::Duration;
use tower::ServiceExt;

async fn request(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<String>,
    content_type: Option<&str>,
) -> (StatusCode, String, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(content_type) = content_type {
        builder = builder.header(CONTENT_TYPE, content_type);
    }
    let response = app
        .clone()
        .oneshot(
            builder
                .body(body.map_or_else(Body::empty, Body::from))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let response_content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = to_bytes(response.into_body(), MAX_AUTHORING_BODY_BYTES * 2 + 1024)
        .await
        .unwrap();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|error| panic!("non-JSON response ({error}): {bytes:?}"))
    };
    (status, response_content_type, value)
}

async fn json_request(
    app: &Router,
    method: Method,
    uri: &str,
    value: Value,
) -> (StatusCode, String, Value) {
    request(
        app,
        method,
        uri,
        Some(value.to_string()),
        Some("application/json"),
    )
    .await
}

async fn create_draft(app: &Router, name: &str) -> String {
    let (status, content_type, created) = json_request(
        app,
        Method::POST,
        "/graphwalker/drafts",
        json!({ "name": name, "generator": "random(vertex_coverage(100))" }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(content_type.starts_with("application/json"));
    assert!(created.get("result").is_none());
    created["draft_id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn resource_routes_complete_the_authoring_workflow() {
    let app = graphwalker_restful::rest_router(None);
    let (status, content_type, created) = json_request(
        &app,
        Method::POST,
        "/graphwalker/drafts",
        json!({
            "model_id": "rest-model",
            "name": "REST model",
            "generator": "random(vertex_coverage(100))",
            "actions": ["global.created = true"],
            "requirements": ["REQ-1"],
            "properties": { "owner": "rest" }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(content_type.starts_with("application/json"));
    assert_eq!(created["model_id"], "rest-model");
    assert_eq!(created["revision"], 0);
    assert!(created.get("result").is_none());
    let draft_id = created["draft_id"].as_str().unwrap();

    let (status, _, vertex_a) = json_request(
        &app,
        Method::POST,
        &format!("/graphwalker/drafts/{draft_id}/vertices"),
        json!({
            "id": "v_a",
            "name": "v_A",
            "shared_state": "state-a",
            "actions": ["x = 1"],
            "requirements": ["REQ-A"],
            "properties": { "kind": "start" },
            "expected_revision": 0
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(vertex_a["revision"], 1);
    assert_eq!(vertex_a["vertex"]["shared_state"], "state-a");
    let (status, _, vertex_b) = json_request(
        &app,
        Method::POST,
        &format!("/graphwalker/drafts/{draft_id}/vertices"),
        json!({ "id": "v_b", "name": "v_B", "expected_revision": 1 }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(vertex_b["revision"], 2);

    let (status, _, start_edge) = json_request(
        &app,
        Method::POST,
        &format!("/graphwalker/drafts/{draft_id}/edges"),
        json!({
            "id": "e_start",
            "name": "e_Start",
            "target_vertex_id": "v_a",
            "expected_revision": 2
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(start_edge["edge"]["source_vertex_id"], Value::Null);
    assert_eq!(start_edge["revision"], 3);
    let (status, _, normal_edge) = json_request(
        &app,
        Method::POST,
        &format!("/graphwalker/drafts/{draft_id}/edges"),
        json!({
            "id": "e_ab",
            "name": "e_AB",
            "source_vertex_id": "v_a",
            "target_vertex_id": "v_b",
            "guard": "true",
            "actions": ["y = 1"],
            "requirements": ["REQ-E"],
            "properties": { "kind": "normal" },
            "weight": 1.0,
            "dependency": 10,
            "expected_revision": 3
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(normal_edge["revision"], 4);

    let (status, _, updated_vertex) = json_request(
        &app,
        Method::PATCH,
        &format!("/graphwalker/drafts/{draft_id}/vertices/v_a"),
        json!({
            "name": "v_A_updated",
            "shared_state": null,
            "expected_revision": 4
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated_vertex["vertex"]["shared_state"], Value::Null);
    assert_eq!(updated_vertex["revision"], 5);
    let (status, _, updated_edge) = json_request(
        &app,
        Method::PATCH,
        &format!("/graphwalker/drafts/{draft_id}/edges/e_ab"),
        json!({ "guard": null, "weight": 0.5, "expected_revision": 5 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated_edge["edge"]["guard"], Value::Null);
    assert_eq!(updated_edge["revision"], 6);
    let (status, _, updated_model) = json_request(
        &app,
        Method::PATCH,
        &format!("/graphwalker/drafts/{draft_id}"),
        json!({
            "name": "Executable REST model",
            "start_element_id": "e_start",
            "expected_revision": 6
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated_model["model"]["startElementId"], "e_start");
    assert_eq!(updated_model["revision"], 7);

    let (status, _, validation) = request(
        &app,
        Method::GET,
        &format!("/graphwalker/drafts/{draft_id}/validation"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(validation["valid"], true, "{validation}");
    assert_eq!(validation["issues"], json!([]));
    assert_eq!(validation["revision"], 7);

    let (status, _, exported) = request(
        &app,
        Method::GET,
        &format!("/graphwalker/drafts/{draft_id}"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(exported["model"].is_object());
    assert_eq!(exported["model"]["models"][0]["id"], "rest-model");
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
    assert_eq!(exported["revision"], 7);

    let (status, _, removed) = request(
        &app,
        Method::DELETE,
        &format!(
            "/graphwalker/drafts/{draft_id}/elements/e_ab?expected_revision=7&cleanup_references=true"
        ),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(removed["removed_ids"], json!(["e_ab"]));
    assert_eq!(removed["revision"], 8);
    let (status, _, discarded) = request(
        &app,
        Method::DELETE,
        &format!("/graphwalker/drafts/{draft_id}"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(discarded, json!({ "discarded": true }));
    let (status, _, error) = request(
        &app,
        Method::GET,
        &format!("/graphwalker/drafts/{draft_id}"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["code"], "draft_not_found");
}

#[tokio::test]
async fn authoring_errors_have_json_status_codes_and_atomic_revisions() {
    let app = graphwalker_restful::rest_router(None);
    let draft_id = create_draft(&app, "errors").await;

    let (status, content_type, error) = request(
        &app,
        Method::POST,
        "/graphwalker/drafts",
        Some("{}".to_string()),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert!(content_type.starts_with("application/json"));
    assert_eq!(error["code"], "unsupported_media_type");
    let (status, _, error) = request(
        &app,
        Method::POST,
        "/graphwalker/drafts",
        Some("{".to_string()),
        Some("application/json"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["code"], "invalid_json");
    let (status, _, error) = json_request(
        &app,
        Method::POST,
        "/graphwalker/drafts",
        json!({ "actions": "not-an-array" }),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["code"], "invalid_json");

    let vertex_uri = format!("/graphwalker/drafts/{draft_id}/vertices");
    let first = json_request(
        &app,
        Method::POST,
        &vertex_uri,
        json!({ "id": "first", "expected_revision": 0 }),
    );
    let second = json_request(
        &app,
        Method::POST,
        &vertex_uri,
        json!({ "id": "second", "expected_revision": 0 }),
    );
    let (first, second) = tokio::join!(first, second);
    let statuses = [first.0, second.0];
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::CREATED)
            .count(),
        1
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::CONFLICT)
            .count(),
        1
    );
    let conflict = if first.0 == StatusCode::CONFLICT {
        first.2
    } else {
        second.2
    };
    assert_eq!(conflict["code"], "revision_conflict");

    let (status, _, exported) = request(
        &app,
        Method::GET,
        &format!("/graphwalker/drafts/{draft_id}"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(exported["revision"], 1);
    assert_eq!(
        exported["model"]["models"][0]["vertices"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let (status, _, error) = json_request(
        &app,
        Method::POST,
        &format!("/graphwalker/drafts/{draft_id}/edges"),
        json!({ "id": "bad", "target_vertex_id": "unknown", "expected_revision": 1 }),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["code"], "unknown_vertex");
    let (status, _, after_failure) = request(
        &app,
        Method::GET,
        &format!("/graphwalker/drafts/{draft_id}"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(after_failure, exported);

    let (status, _, error) = request(
        &app,
        Method::DELETE,
        &format!("/graphwalker/drafts/{draft_id}/elements/first?expected_revision=nope"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["code"], "invalid_query");
    let (status, _, _) = request(
        &app,
        Method::GET,
        &format!("/graphwalker/drafts/{draft_id}/vertices"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn authoring_request_body_limit_does_not_change_legacy_load_limit() {
    let app = graphwalker_restful::rest_router(None);
    let oversized = format!("{{\"name\":\"{}\"}}", "x".repeat(MAX_AUTHORING_BODY_BYTES));
    let (status, content_type, error) = request(
        &app,
        Method::POST,
        "/graphwalker/drafts",
        Some(oversized.clone()),
        Some("application/json"),
    )
    .await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert!(content_type.starts_with("application/json"));
    assert_eq!(error["code"], "payload_too_large");

    let (legacy_status, _, legacy_body) = request(
        &app,
        Method::POST,
        "/graphwalker/load",
        Some(oversized),
        Some("application/json"),
    )
    .await;
    assert_eq!(legacy_status, StatusCode::OK);
    assert_eq!(legacy_body["result"], "nok");
}

#[tokio::test]
async fn draft_limit_and_expiry_use_distinct_http_statuses() {
    let limited = DraftRegistry::new(DraftLimits {
        max_drafts: 1,
        ..DraftLimits::default()
    });
    let app = graphwalker_restful::rest_router_with_drafts(None, limited);
    create_draft(&app, "first").await;
    let (status, _, error) = json_request(
        &app,
        Method::POST,
        "/graphwalker/drafts",
        json!({ "name": "over limit" }),
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(error["code"], "draft_limit_reached");

    let expiring = DraftRegistry::new(DraftLimits {
        idle_timeout: Duration::ZERO,
        ..DraftLimits::default()
    });
    let app = graphwalker_restful::rest_router_with_drafts(None, expiring);
    let draft_id = create_draft(&app, "expiring").await;
    let (status, content_type, error) = request(
        &app,
        Method::GET,
        &format!("/graphwalker/drafts/{draft_id}"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::GONE);
    assert!(content_type.starts_with("application/json"));
    assert_eq!(error["code"], "draft_expired");
}
