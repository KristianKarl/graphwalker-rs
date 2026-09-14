use std::collections::HashMap;

use rmcp::schemars;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default)]
pub enum McpPatch<T> {
    #[default]
    Keep,
    Set(T),
    Clear,
}

impl<'de, T> Deserialize<'de> for McpPatch<T>
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

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct CreateModelInput {
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

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddVertexInput {
    pub draft_id: String,
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

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddEdgeInput {
    pub draft_id: String,
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

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateModelInput {
    pub draft_id: String,
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    pub name: McpPatch<String>,
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    pub generator: McpPatch<String>,
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    pub start_element_id: McpPatch<String>,
    #[serde(default)]
    #[schemars(with = "Option<Vec<String>>")]
    pub actions: McpPatch<Vec<String>>,
    #[serde(default)]
    #[schemars(with = "Option<Vec<String>>")]
    pub requirements: McpPatch<Vec<String>>,
    #[serde(default)]
    #[schemars(with = "Option<HashMap<String, Value>>")]
    pub properties: McpPatch<HashMap<String, Value>>,
    #[serde(default)]
    #[schemars(with = "Option<Vec<String>>")]
    pub predefined_path_edge_ids: McpPatch<Vec<String>>,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateVertexInput {
    pub draft_id: String,
    pub vertex_id: String,
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    pub name: McpPatch<String>,
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    pub shared_state: McpPatch<String>,
    #[serde(default)]
    #[schemars(with = "Option<Vec<String>>")]
    pub actions: McpPatch<Vec<String>>,
    #[serde(default)]
    #[schemars(with = "Option<Vec<String>>")]
    pub requirements: McpPatch<Vec<String>>,
    #[serde(default)]
    #[schemars(with = "Option<HashMap<String, Value>>")]
    pub properties: McpPatch<HashMap<String, Value>>,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateEdgeInput {
    pub draft_id: String,
    pub edge_id: String,
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    pub name: McpPatch<String>,
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    pub source_vertex_id: McpPatch<String>,
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    pub target_vertex_id: McpPatch<String>,
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    pub guard: McpPatch<String>,
    #[serde(default)]
    #[schemars(with = "Option<Vec<String>>")]
    pub actions: McpPatch<Vec<String>>,
    #[serde(default)]
    #[schemars(with = "Option<Vec<String>>")]
    pub requirements: McpPatch<Vec<String>>,
    #[serde(default)]
    #[schemars(with = "Option<HashMap<String, Value>>")]
    pub properties: McpPatch<HashMap<String, Value>>,
    #[serde(default)]
    #[schemars(with = "Option<f64>")]
    pub weight: McpPatch<f64>,
    #[serde(default)]
    #[schemars(with = "Option<i32>")]
    pub dependency: McpPatch<i32>,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RemoveElementInput {
    pub draft_id: String,
    pub element_id: String,
    #[serde(default)]
    pub cascade: bool,
    #[serde(default)]
    pub cleanup_references: bool,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DraftInput {
    pub draft_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ValidateModelInput {
    #[schemars(with = "Option<HashMap<String, Value>>")]
    pub model: Option<Value>,
    pub draft_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ConvertGraphmlInput {
    pub graphml: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StartExecutionInput {
    #[schemars(with = "Option<HashMap<String, Value>>")]
    pub model: Option<Value>,
    pub draft_id: Option<String>,
    pub revision: Option<u64>,
    pub seed: Option<u64>,
    pub global_data: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExecutionInput {
    pub execution_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExecutionStatusInput {
    pub execution_id: String,
    #[serde(default)]
    pub include_elements: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetExecutionDataInput {
    pub execution_id: String,
    pub script: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct HealthOutput {
    pub status: &'static str,
    pub server_version: &'static str,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CreateModelOutput {
    pub draft_id: String,
    pub model_id: String,
    pub revision: u64,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct VertexOutput {
    pub id: String,
    pub name: Option<String>,
    pub shared_state: Option<String>,
    pub actions: Vec<String>,
    pub requirements: Vec<String>,
    pub properties: HashMap<String, Value>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct VertexMutationOutput {
    pub vertex: VertexOutput,
    pub revision: u64,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct EdgeOutput {
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

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct EdgeMutationOutput {
    pub edge: EdgeOutput,
    pub revision: u64,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ModelMutationOutput {
    #[schemars(with = "HashMap<String, Value>")]
    pub model: Value,
    pub revision: u64,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct RemoveElementOutput {
    pub removed_ids: Vec<String>,
    pub revision: u64,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ExportModelOutput {
    #[schemars(with = "HashMap<String, Value>")]
    pub model: Value,
    pub revision: u64,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct DiscardModelOutput {
    pub discarded: bool,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ValidateModelOutput {
    pub valid: bool,
    pub issues: Vec<String>,
    pub revision: Option<u64>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ConvertGraphmlOutput {
    #[schemars(with = "HashMap<String, Value>")]
    pub model: Value,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct StartExecutionOutput {
    pub execution_id: String,
    pub seed: u64,
    pub source_revision: Option<u64>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct StepElementOutput {
    pub id: String,
    pub name: String,
    pub model_id: String,
    pub kind: String,
    pub data: String,
    pub visited_count: u64,
    pub total_count: u64,
    pub stop_condition_fulfillment: f64,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct NextStepOutput {
    pub completed: bool,
    pub element: Option<StepElementOutput>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct StatisticsOutput {
    pub total_vertices: usize,
    pub total_edges: usize,
    pub visited_vertices: usize,
    pub visited_edges: usize,
    pub unvisited_vertices: usize,
    pub unvisited_edges: usize,
    pub vertex_coverage: u32,
    pub edge_coverage: u32,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ElementStatusOutput {
    pub model_id: String,
    pub element_id: String,
    pub visited_count: u64,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ExecutionStatusOutput {
    pub has_next: bool,
    pub data: String,
    pub statistics: StatisticsOutput,
    pub elements: Option<Vec<ElementStatusOutput>>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct SetExecutionDataOutput {
    pub data: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct RestartExecutionOutput {
    pub restarted: bool,
    pub seed: u64,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CloseExecutionOutput {
    pub closed: bool,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ToolErrorOutput {
    pub code: String,
    pub message: String,
}
