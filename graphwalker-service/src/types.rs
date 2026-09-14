use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExecutionId(String);

impl ExecutionId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn new(value: String) -> Self {
        Self(value)
    }
}

impl fmt::Display for ExecutionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<String> for ExecutionId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for ExecutionId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionLimits {
    pub max_executions: usize,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self { max_executions: 64 }
    }
}

#[derive(Clone, Debug)]
pub struct StartExecution {
    pub model: Value,
    pub seed: Option<u64>,
    pub global_data: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StartExecutionResult {
    pub execution_id: ExecutionId,
    pub seed: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RestartResult {
    pub seed: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SetDataResult {
    pub data: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ElementKind {
    Edge,
    Vertex,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StepElement {
    pub id: String,
    pub name: String,
    pub model_id: String,
    pub kind: ElementKind,
    pub data: String,
    pub visited_count: u64,
    pub total_count: u64,
    pub stop_condition_fulfillment: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StepResult {
    pub completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element: Option<StepElement>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExecutionStatus {
    pub has_next: bool,
    pub data: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExecutionStatistics {
    pub total_vertices: usize,
    pub total_edges: usize,
    pub visited_vertices: usize,
    pub visited_edges: usize,
    pub unvisited_vertices: usize,
    pub unvisited_edges: usize,
    pub vertex_coverage: u32,
    pub edge_coverage: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ElementStatus {
    pub model_id: String,
    pub element_id: String,
    pub visited_count: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ModelResult {
    pub model: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ConversionResult {
    pub model: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ValidationIssue {
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceErrorCode {
    InvalidModel,
    InvalidGenerator,
    InvalidData,
    ExecutionLimitReached,
    ExecutionNotFound,
    ExecutionUnavailable,
    Internal,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ServiceError {
    pub code: ServiceErrorCode,
    pub message: String,
}

impl ServiceError {
    pub(crate) fn new(code: ServiceErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub(crate) fn invalid_model(message: impl Into<String>) -> Self {
        Self::new(ServiceErrorCode::InvalidModel, message)
    }

    pub(crate) fn internal(message: impl Into<String>) -> Self {
        Self::new(ServiceErrorCode::Internal, message)
    }
}

impl fmt::Display for ServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ServiceError {}
