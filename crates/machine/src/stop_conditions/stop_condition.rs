pub trait IsFullfilled {
    fn is_fullfilled(&self) -> bool;
}

pub trait StopCondition {
    fn condition_type(&self) -> &str;
}

pub struct EdgeCoverage {
    coverage: f32,
    fullfilment: f32,
}

impl StopCondition for EdgeCoverage {
    fn condition_type(&self) -> &str {
        "EdgeCoverage"
    }
}

impl IsFullfilled for EdgeCoverage {
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

pub struct VertexCoverage {
    coverage: f32,
    fullfilment: f32,
}

impl StopCondition for VertexCoverage {
    fn condition_type(&self) -> &str {
        "VertexCoverage"
    }
}

impl IsFullfilled for VertexCoverage {
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
            edge_coverage.condition_type(),
            "EdgeCoverage",
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
            vertex_coverage.condition_type(),
            "VertexCoverage",
            "Incorrect condition type found"
        );
        assert_eq!(
            vertex_coverage.is_fullfilled(),
            false,
            "Incorrect fullfillment"
        );
    }
}
