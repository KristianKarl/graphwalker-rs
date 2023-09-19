#[path = "../stop_condition/edge_coverage.rs"]
pub mod stop_condition;

use stop_condition::EdgeCoverage;

#[derive(Debug, Clone)]
pub struct Random {
    _stop_conditions: Vec<EdgeCoverage>,
}
