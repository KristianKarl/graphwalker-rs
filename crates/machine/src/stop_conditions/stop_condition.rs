#[derive(Debug, PartialEq)]
pub enum StopConditionKind {
    EdgeCoverage,
    VertexCoverage,
}

impl core::fmt::Display for StopConditionKind {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match *self {
            StopConditionKind::EdgeCoverage => write!(f, "EdgeCoverage"),
            StopConditionKind::VertexCoverage => write!(f, "VertexCoverage"),
        }
    }
}


pub trait StopCondition {
    fn kind(&self) -> StopConditionKind;
    fn is_fullfilled(&self) -> bool;
}

// https://users.rust-lang.org/t/derive-debug-not-playing-well-with-dyn/52398
impl<'a> core::fmt::Debug for dyn StopCondition + 'a {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.kind()) 
    }
}

#[derive(Default, Clone, Debug)]
pub struct EdgeCoverage {
    coverage: f32,
    fullfilment: f32,
}

impl StopCondition for EdgeCoverage {
    fn kind(&self) -> StopConditionKind {
        StopConditionKind::EdgeCoverage
    }

    fn is_fullfilled(&self) -> bool {
        self.fullfilment >= self.coverage
    }
}

impl EdgeCoverage {
    pub fn new(cov: f32) -> Self {
        Self {
            coverage: cov,
            fullfilment: 0f32,
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct VertexCoverage {
    coverage: f32,
    fullfilment: f32,
}

impl StopCondition for VertexCoverage {
    fn kind(&self) -> StopConditionKind {
        StopConditionKind::VertexCoverage
    }

    fn is_fullfilled(&self) -> bool {
        self.fullfilment >= self.coverage
    }
}

impl VertexCoverage {
    pub fn new(coverage: f32) -> Self {
        Self {
            coverage,
            fullfilment: 0f32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn edge_coverage() {
        let edge_coverage = EdgeCoverage::new(1f32);
        assert_eq!(
            edge_coverage.kind(),
            StopConditionKind::EdgeCoverage,
            "Incorrect condition type found"
        );
        assert_eq!(
            edge_coverage.is_fullfilled(),
            false,
            "Incorrect fullfillment"
        );
    }

    #[test]
    fn vertex_coverage() {
        let vertex_coverage = VertexCoverage::new(1f32);
        assert_eq!(
            vertex_coverage.kind(),
            StopConditionKind::VertexCoverage,
            "Incorrect condition type found"
        );
        assert_eq!(
            vertex_coverage.is_fullfilled(),
            false,
            "Incorrect fullfillment"
        );
    }
}
