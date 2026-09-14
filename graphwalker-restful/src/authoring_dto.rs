use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default)]
pub enum RestPatch<T> {
    #[default]
    Keep,
    Set(T),
    Clear,
}

impl<'de, T> Deserialize<'de> for RestPatch<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match Option::<T>::deserialize(deserializer)? {
            Some(value) => Self::Set(value),
            None => Self::Clear,
        })
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct CreateDraftRequest {
    pub model_id: Option<String>,
    pub name: Option<String>,
    pub generator: Option<String>,
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default)]
    pub requirements: Vec<String>,
    #[serde(default)]
    pub properties: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct AddVertexRequest {
    pub id: Option<String>,
    pub name: Option<String>,
    pub shared_state: Option<String>,
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default)]
    pub requirements: Vec<String>,
    #[serde(default)]
    pub properties: HashMap<String, Value>,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct AddEdgeRequest {
    pub id: Option<String>,
    pub name: Option<String>,
    pub source_vertex_id: Option<String>,
    pub target_vertex_id: String,
    pub guard: Option<String>,
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default)]
    pub requirements: Vec<String>,
    #[serde(default)]
    pub properties: HashMap<String, Value>,
    pub weight: Option<f64>,
    pub dependency: Option<i32>,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateModelRequest {
    #[serde(default)]
    pub name: RestPatch<String>,
    #[serde(default)]
    pub generator: RestPatch<String>,
    #[serde(default)]
    pub start_element_id: RestPatch<String>,
    #[serde(default)]
    pub actions: RestPatch<Vec<String>>,
    #[serde(default)]
    pub requirements: RestPatch<Vec<String>>,
    #[serde(default)]
    pub properties: RestPatch<HashMap<String, Value>>,
    #[serde(default)]
    pub predefined_path_edge_ids: RestPatch<Vec<String>>,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateVertexRequest {
    #[serde(default)]
    pub name: RestPatch<String>,
    #[serde(default)]
    pub shared_state: RestPatch<String>,
    #[serde(default)]
    pub actions: RestPatch<Vec<String>>,
    #[serde(default)]
    pub requirements: RestPatch<Vec<String>>,
    #[serde(default)]
    pub properties: RestPatch<HashMap<String, Value>>,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEdgeRequest {
    #[serde(default)]
    pub name: RestPatch<String>,
    #[serde(default)]
    pub source_vertex_id: RestPatch<String>,
    #[serde(default)]
    pub target_vertex_id: RestPatch<String>,
    #[serde(default)]
    pub guard: RestPatch<String>,
    #[serde(default)]
    pub actions: RestPatch<Vec<String>>,
    #[serde(default)]
    pub requirements: RestPatch<Vec<String>>,
    #[serde(default)]
    pub properties: RestPatch<HashMap<String, Value>>,
    #[serde(default)]
    pub weight: RestPatch<f64>,
    #[serde(default)]
    pub dependency: RestPatch<i32>,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RemoveElementQuery {
    #[serde(default)]
    pub cascade: bool,
    #[serde(default)]
    pub cleanup_references: bool,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct DraftCreatedResponse {
    pub draft_id: String,
    pub model_id: String,
    pub revision: u64,
}

#[derive(Debug, Serialize)]
pub struct VertexResponse {
    pub id: String,
    pub name: Option<String>,
    pub shared_state: Option<String>,
    pub actions: Vec<String>,
    pub requirements: Vec<String>,
    pub properties: HashMap<String, Value>,
}

#[derive(Debug, Serialize)]
pub struct VertexMutationResponse {
    pub vertex: VertexResponse,
    pub revision: u64,
}

#[derive(Debug, Serialize)]
pub struct EdgeResponse {
    pub id: String,
    pub name: Option<String>,
    pub source_vertex_id: Option<String>,
    pub target_vertex_id: String,
    pub guard: Option<String>,
    pub actions: Vec<String>,
    pub requirements: Vec<String>,
    pub properties: HashMap<String, Value>,
    pub weight: Option<f64>,
    pub dependency: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct EdgeMutationResponse {
    pub edge: EdgeResponse,
    pub revision: u64,
}

#[derive(Debug, Serialize)]
pub struct ModelMutationResponse {
    pub model: Value,
    pub revision: u64,
}

#[derive(Debug, Serialize)]
pub struct ExportDraftResponse {
    pub model: Value,
    pub revision: u64,
}

#[derive(Debug, Serialize)]
pub struct ValidationResponse {
    pub valid: bool,
    pub issues: Vec<String>,
    pub revision: u64,
}

#[derive(Debug, Serialize)]
pub struct RemoveElementResponse {
    pub removed_ids: Vec<String>,
    pub revision: u64,
}

#[derive(Debug, Serialize)]
pub struct DiscardDraftResponse {
    pub discarded: bool,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
}
