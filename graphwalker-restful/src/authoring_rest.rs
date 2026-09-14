use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use graphwalker_service as service;
use serde::Serialize;
use serde_json::Value;

use crate::authoring_dto::*;
use crate::RestApplicationState;

pub const MAX_AUTHORING_BODY_BYTES: usize = 1024 * 1024;

type ApiResult = Result<(StatusCode, Json<Value>), ApiError>;

pub(crate) fn routes() -> Router<RestApplicationState> {
    Router::new()
        .route("/graphwalker/drafts", post(create_draft))
        .route(
            "/graphwalker/drafts/{draft_id}",
            get(export_draft).patch(update_model).delete(discard_draft),
        )
        .route("/graphwalker/drafts/{draft_id}/vertices", post(add_vertex))
        .route(
            "/graphwalker/drafts/{draft_id}/vertices/{vertex_id}",
            patch(update_vertex),
        )
        .route("/graphwalker/drafts/{draft_id}/edges", post(add_edge))
        .route(
            "/graphwalker/drafts/{draft_id}/edges/{edge_id}",
            patch(update_edge),
        )
        .route(
            "/graphwalker/drafts/{draft_id}/elements/{element_id}",
            delete(remove_element),
        )
        .route(
            "/graphwalker/drafts/{draft_id}/validation",
            get(validate_draft),
        )
        .layer(DefaultBodyLimit::max(MAX_AUTHORING_BODY_BYTES))
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    code: String,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal", message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                code: self.code,
                message: self.message,
            }),
        )
            .into_response()
    }
}

impl From<service::ServiceError> for ApiError {
    fn from(error: service::ServiceError) -> Self {
        use service::ServiceErrorCode as Code;

        let status = match error.code {
            Code::DraftNotFound => StatusCode::NOT_FOUND,
            Code::DraftExpired => StatusCode::GONE,
            Code::RevisionConflict | Code::DuplicateElementId | Code::ReferencedElement => {
                StatusCode::CONFLICT
            }
            Code::DraftLimitReached | Code::ModelLimitReached => StatusCode::TOO_MANY_REQUESTS,
            Code::InvalidModel
            | Code::InvalidGenerator
            | Code::InvalidData
            | Code::UnknownVertex
            | Code::MissingTargetVertex
            | Code::InvalidWeight
            | Code::InvalidDependency
            | Code::InvalidElement => StatusCode::UNPROCESSABLE_ENTITY,
            Code::ExecutionLimitReached
            | Code::ExecutionNotFound
            | Code::ExecutionUnavailable
            | Code::DraftUnavailable
            | Code::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let code = serde_json::to_value(error.code)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_else(|| "internal".to_string());
        Self::new(status, code, error.message)
    }
}

fn body<T>(body: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    body.map(|Json(value)| value).map_err(|rejection| {
        let status = rejection.status();
        let code = match status {
            StatusCode::UNSUPPORTED_MEDIA_TYPE => "unsupported_media_type",
            StatusCode::PAYLOAD_TOO_LARGE => "payload_too_large",
            _ => "invalid_json",
        };
        ApiError::new(status, code, rejection.body_text())
    })
}

fn query<T>(query: Result<Query<T>, QueryRejection>) -> Result<T, ApiError> {
    query.map(|Query(value)| value).map_err(|rejection| {
        ApiError::new(rejection.status(), "invalid_query", rejection.body_text())
    })
}

fn response<T: Serialize>(status: StatusCode, value: T) -> ApiResult {
    serde_json::to_value(value)
        .map(|value| (status, Json(value)))
        .map_err(|error| ApiError::internal(format!("Could not serialize REST response: {error}")))
}

fn patch_value<T>(value: RestPatch<T>) -> service::FieldPatch<T> {
    match value {
        RestPatch::Keep => service::FieldPatch::Keep,
        RestPatch::Set(value) => service::FieldPatch::Set(value),
        RestPatch::Clear => service::FieldPatch::Clear,
    }
}

fn vertex_response(vertex: graphwalker_io::json::JsonVertex) -> VertexResponse {
    VertexResponse {
        id: vertex.id,
        name: vertex.name,
        shared_state: vertex.shared_state,
        actions: vertex.actions,
        requirements: vertex.requirements,
        properties: vertex.properties,
    }
}

fn edge_response(
    edge: graphwalker_io::json::JsonEdge,
) -> Result<EdgeResponse, service::ServiceError> {
    let target_vertex_id = edge.target_vertex_id.ok_or_else(|| {
        service::ServiceError::new(
            service::ServiceErrorCode::Internal,
            "The service returned an edge without a target vertex",
        )
    })?;
    Ok(EdgeResponse {
        id: edge.id,
        name: edge.name,
        source_vertex_id: edge.source_vertex_id,
        target_vertex_id,
        guard: edge.guard,
        actions: edge.actions,
        requirements: edge.requirements,
        properties: edge.properties,
        weight: edge.weight,
        dependency: edge.dependency,
    })
}

async fn create_draft(
    State(drafts): State<service::DraftRegistry>,
    payload: Result<Json<CreateDraftRequest>, JsonRejection>,
) -> ApiResult {
    let request = body(payload)?;
    let created = drafts.create_model(service::CreateModel {
        model_id: request.model_id,
        name: request.name,
        generator: request.generator,
        actions: request.actions,
        requirements: request.requirements,
        properties: request.properties,
    })?;
    response(
        StatusCode::CREATED,
        DraftCreatedResponse {
            draft_id: created.draft_id.to_string(),
            model_id: created.model_id,
            revision: created.revision,
        },
    )
}

async fn export_draft(
    State(drafts): State<service::DraftRegistry>,
    Path(draft_id): Path<String>,
) -> ApiResult {
    let exported = drafts.export_model(&draft_id.as_str().into())?;
    response(
        StatusCode::OK,
        ExportDraftResponse {
            model: exported.model,
            revision: exported.revision,
        },
    )
}

async fn update_model(
    State(drafts): State<service::DraftRegistry>,
    Path(draft_id): Path<String>,
    payload: Result<Json<UpdateModelRequest>, JsonRejection>,
) -> ApiResult {
    let request = body(payload)?;
    let updated = drafts.update_model(service::UpdateModel {
        draft_id: draft_id.as_str().into(),
        name: patch_value(request.name),
        generator: patch_value(request.generator),
        start_element_id: patch_value(request.start_element_id),
        actions: patch_value(request.actions),
        requirements: patch_value(request.requirements),
        properties: patch_value(request.properties),
        predefined_path_edge_ids: patch_value(request.predefined_path_edge_ids),
        expected_revision: request.expected_revision,
    })?;
    let model = serde_json::to_value(updated.model)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    response(
        StatusCode::OK,
        ModelMutationResponse {
            model,
            revision: updated.revision,
        },
    )
}

async fn add_vertex(
    State(drafts): State<service::DraftRegistry>,
    Path(draft_id): Path<String>,
    payload: Result<Json<AddVertexRequest>, JsonRejection>,
) -> ApiResult {
    let request = body(payload)?;
    let added = drafts.add_vertex(service::AddVertex {
        draft_id: draft_id.as_str().into(),
        id: request.id,
        name: request.name,
        shared_state: request.shared_state,
        actions: request.actions,
        requirements: request.requirements,
        properties: request.properties,
        expected_revision: request.expected_revision,
    })?;
    response(
        StatusCode::CREATED,
        VertexMutationResponse {
            vertex: vertex_response(added.vertex),
            revision: added.revision,
        },
    )
}

async fn update_vertex(
    State(drafts): State<service::DraftRegistry>,
    Path((draft_id, vertex_id)): Path<(String, String)>,
    payload: Result<Json<UpdateVertexRequest>, JsonRejection>,
) -> ApiResult {
    let request = body(payload)?;
    let updated = drafts.update_vertex(service::UpdateVertex {
        draft_id: draft_id.as_str().into(),
        vertex_id,
        name: patch_value(request.name),
        shared_state: patch_value(request.shared_state),
        actions: patch_value(request.actions),
        requirements: patch_value(request.requirements),
        properties: patch_value(request.properties),
        expected_revision: request.expected_revision,
    })?;
    response(
        StatusCode::OK,
        VertexMutationResponse {
            vertex: vertex_response(updated.vertex),
            revision: updated.revision,
        },
    )
}

async fn add_edge(
    State(drafts): State<service::DraftRegistry>,
    Path(draft_id): Path<String>,
    payload: Result<Json<AddEdgeRequest>, JsonRejection>,
) -> ApiResult {
    let request = body(payload)?;
    let added = drafts.add_edge(service::AddEdge {
        draft_id: draft_id.as_str().into(),
        id: request.id,
        name: request.name,
        source_vertex_id: request.source_vertex_id,
        target_vertex_id: Some(request.target_vertex_id),
        guard: request.guard,
        actions: request.actions,
        requirements: request.requirements,
        properties: request.properties,
        weight: request.weight,
        dependency: request.dependency,
        expected_revision: request.expected_revision,
    })?;
    let edge = edge_response(added.edge)?;
    response(
        StatusCode::CREATED,
        EdgeMutationResponse {
            edge,
            revision: added.revision,
        },
    )
}

async fn update_edge(
    State(drafts): State<service::DraftRegistry>,
    Path((draft_id, edge_id)): Path<(String, String)>,
    payload: Result<Json<UpdateEdgeRequest>, JsonRejection>,
) -> ApiResult {
    let request = body(payload)?;
    let updated = drafts.update_edge(service::UpdateEdge {
        draft_id: draft_id.as_str().into(),
        edge_id,
        name: patch_value(request.name),
        source_vertex_id: patch_value(request.source_vertex_id),
        target_vertex_id: patch_value(request.target_vertex_id),
        guard: patch_value(request.guard),
        actions: patch_value(request.actions),
        requirements: patch_value(request.requirements),
        properties: patch_value(request.properties),
        weight: patch_value(request.weight),
        dependency: patch_value(request.dependency),
        expected_revision: request.expected_revision,
    })?;
    let edge = edge_response(updated.edge)?;
    response(
        StatusCode::OK,
        EdgeMutationResponse {
            edge,
            revision: updated.revision,
        },
    )
}

async fn remove_element(
    State(drafts): State<service::DraftRegistry>,
    Path((draft_id, element_id)): Path<(String, String)>,
    query_params: Result<Query<RemoveElementQuery>, QueryRejection>,
) -> ApiResult {
    let request = query(query_params)?;
    let removed = drafts.remove_element(service::RemoveElement {
        draft_id: draft_id.as_str().into(),
        element_id,
        cascade: request.cascade,
        cleanup_references: request.cleanup_references,
        expected_revision: request.expected_revision,
    })?;
    response(
        StatusCode::OK,
        RemoveElementResponse {
            removed_ids: removed.removed_ids,
            revision: removed.revision,
        },
    )
}

async fn validate_draft(
    State(drafts): State<service::DraftRegistry>,
    Path(draft_id): Path<String>,
) -> ApiResult {
    let validation = drafts.validate(&draft_id.as_str().into())?;
    response(
        StatusCode::OK,
        ValidationResponse {
            valid: validation.valid,
            issues: validation
                .issues
                .into_iter()
                .map(|issue| issue.message)
                .collect(),
            revision: validation.revision,
        },
    )
}

async fn discard_draft(
    State(drafts): State<service::DraftRegistry>,
    Path(draft_id): Path<String>,
) -> ApiResult {
    let discarded = drafts.discard(&draft_id.as_str().into())?;
    response(
        StatusCode::OK,
        DiscardDraftResponse {
            discarded: discarded.discarded,
        },
    )
}
