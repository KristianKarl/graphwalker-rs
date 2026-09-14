mod execution;
mod types;

pub use execution::ExecutionRegistry;
pub use types::{
    ConversionResult, ElementKind, ElementStatus, ExecutionId, ExecutionLimits,
    ExecutionStatistics, ExecutionStatus, ModelResult, RestartResult, ServiceError,
    ServiceErrorCode, SetDataResult, StartExecution, StartExecutionResult, StepElement, StepResult,
    ValidationIssue, ValidationResult,
};

use graphwalker_io::graphml::read_graphml_string;
use graphwalker_io::json::{read_json_string, write_json_string};
use serde_json::Value;

/// Validate a GraphWalker JSON model without creating execution state.
pub fn validate_model(model: &Value) -> Result<ValidationResult, ServiceError> {
    let contexts = read_json_string(&model.to_string())
        .map_err(|error| ServiceError::invalid_model(error.to_string()))?;
    let issues = graphwalker_model_checker::check_contexts(&contexts)
        .into_iter()
        .map(|issue| ValidationIssue {
            message: issue.message,
        })
        .collect::<Vec<_>>();

    Ok(ValidationResult {
        valid: issues.is_empty(),
        issues,
    })
}

/// Convert a GraphML document into the canonical GraphWalker JSON object.
pub fn convert_graphml(graphml: &str) -> Result<ConversionResult, ServiceError> {
    let contexts = read_graphml_string(graphml)
        .map_err(|error| ServiceError::invalid_model(error.to_string()))?;
    let json =
        write_json_string(&contexts).map_err(|error| ServiceError::internal(error.to_string()))?;
    let model =
        serde_json::from_str(&json).map_err(|error| ServiceError::internal(error.to_string()))?;
    Ok(ConversionResult { model })
}
