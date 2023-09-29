use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Step {
    pub context_id: String,
    pub element_id: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Profile {
    pub steps: Vec<Step>,
}

impl Profile {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn push(&mut self, step: Step) {
        log::trace!("{:?}", step);
        self.steps.push(step);
    }
}
