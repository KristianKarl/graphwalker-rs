use crate::stop_condition::IsFullfilled;

trait GeneratorType {
    fn generator_type(&self) -> &str;
}

struct RandomGenerator {
    stop_conditions: Vec<Box<dyn IsFullfilled>>,
}

impl GeneratorType for RandomGenerator {
    fn generator_type(&self) -> &str {
        "RandomGenerator"
    }
}

impl IsFullfilled for RandomGenerator {
    fn is_fullfilled(&self) -> bool {
        for fullfilment in self.stop_conditions.iter() {
            if !fullfilment.is_fullfilled() {
                return false;
            }
        }
        true
    }
}

impl RandomGenerator {
    pub fn new() -> Self {
        Self {
            stop_conditions: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    use crate::stop_condition::EdgeCoverage;

    #[test]
    fn random() {
        let mut random = RandomGenerator::new();

        let edge_coverage = EdgeCoverage::new(0f32);
        random.stop_conditions.push(Box::new(edge_coverage));

        assert_eq!(random.is_fullfilled(), true, "Should be false");
        assert_eq!(
            random.generator_type(),
            "RandomGenerator",
            "Incorrect condition type found"
        );
    }
}
