use graph::Model;
use serde_derive::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::BTreeMap};

use graph::Models;

#[path = "stop_conditions/stop_condition.rs"]
pub mod stop_condition;

#[path = "generators/generator.rs"]
pub mod generator;

#[derive(Serialize, Deserialize, Clone, Default, Debug, Ord, Eq, PartialEq, PartialOrd)]
pub struct Position {
    pub model_id: String,
    pub element_id: String,
}

impl Position {
    fn new(ctx_id: String, elem_id: String) -> Self {
        Self {
            model_id: ctx_id,
            element_id: elem_id,
        }
    }

    fn is_valid(&self) -> bool {
        if self.model_id.is_empty() || self.element_id.is_empty() {
            return false;
        }
        true
    }
}

#[derive(Clone, Default, Debug)]
pub struct Profile {
    pub steps: Vec<Position>,
}

impl Profile {
    fn new() -> Self {
        Self { steps: Vec::new() }
    }

    fn push(&mut self, step: Position) {
        self.steps.push(step);
    }
}

#[derive(Clone, Debug)]
pub struct Context {
    id: String,
    model: Model,
    fullfillment: f32,
}

impl Context {
    fn new() -> Self {
        Self {
            id: "".to_string(),
            model: Model::new(),
            fullfillment: 0f32,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub enum MachineStatus {
    #[default]
    NotStarted,
    Running,
    Ended,
    Failed,
}

#[derive(Default, Debug, Clone)]
pub struct Machine {
    pub contexts: BTreeMap<String, Context>,
    pub profile: Profile,
    unvisited_elements: BTreeMap<Position, u32>,
    current_pos: Position,
    pub start_pos: Position,
    pub status: MachineStatus,
}

impl Machine {
    #[must_use]
    pub fn new() -> Self {
        Self {
            contexts: BTreeMap::new(),
            profile: Profile::new(),
            unvisited_elements: BTreeMap::new(),
            current_pos: Position::default(),
            start_pos: Position::default(),
            status: MachineStatus::NotStarted,
        }
    }

    /*
     * Calculates if the specific model has covered the models given their stop conditions
     */
    fn get_fullfilment(&self, ctx_id: String) -> f32 {
        let elements: Vec<_> = self
            .unvisited_elements
            .iter()
            .filter(|(k, _v)| k.model_id == ctx_id)
            .map(|(_k, v)| v)
            .collect();
        let element_count = elements.len();

        if element_count == 0 {
            return 1f32;
        }

        let visited_count = elements.iter().filter(|v| v > &&&0).count();
        log::debug!(
            "Fullmillment for model: {:?} is {:?}",
            ctx_id,
            visited_count as f32 / element_count as f32,
        );
        visited_count as f32 / element_count as f32
    }

    fn is_fullfilled(&self, ctx_id: String) -> bool {
        let fullfillment = self.contexts.get(&ctx_id).unwrap().fullfillment;
        if self.get_fullfilment(ctx_id) < fullfillment {
            return false;
        }
        true
    }

    pub fn is_all_fullfilled(&self) -> bool {
        for key in self.contexts.keys() {
            if !self.is_fullfilled(key.to_string()) {
                log::debug!("The model: {:?} is not fullfilled", key);
                return false;
            }
            log::debug!("The model: {:?} is fullfilled", key);
        }
        log::debug!("All models are fullfilled");
        true
    }

    fn log_step(&mut self, step: Position) -> Result<String, String> {
        log::debug!("Step: {:?}", step);
        if let Some(value) = self.unvisited_elements.get(&step.clone()) {
            let visited = value + 1;
            self.unvisited_elements.insert(step.clone(), visited);
        } else {
            let msg = format!(
                "Expected the key {:?} to be found in unvisited_elements",
                step.clone()
            );
            log::error!("{}", msg);
            return Err(msg);
        }
        self.profile.push(step.clone());
        Ok(serde_json::to_string(&step).unwrap())
    }

    pub fn reset(&mut self) -> Result<(), String> {
        log::debug!("Resetting the machine");

        // First check that all start element ids are the same.
        self.start_pos = Position::default();
        for (key, ctx) in &self.contexts {
            if ctx.clone().model.start_element_id.is_some() {
                if self.start_pos.element_id.is_empty() {
                    self.start_pos.element_id = ctx.clone().model.start_element_id.unwrap();
                } else if self.start_pos.element_id != ctx.clone().model.start_element_id.unwrap() {
                    let msg = format!(
                        "Found different starting element id's: {:?} and {:?}",
                        self.start_pos.model_id,
                        key.to_string()
                    );
                    log::error!("{}", msg);
                    return Err(msg);
                }
            }
        }

        // If no start elemet id is found, bail out
        if self.start_pos.element_id.is_empty() {
            let msg = "Did not find any start element id. Cannot contiune".to_string();
            log::error!("{}", msg);
            return Err(msg);
        }

        // Find the model in which the model id exists
        for (key, ctx) in &self.contexts {
            if ctx.model.has_id(self.start_pos.element_id.clone()) {
                self.start_pos.model_id = key.to_string();
            }
        }

        // If no model id is found for the start element, bail out
        if self.start_pos.model_id.is_empty() {
            let msg = format!(
                "Did not find any model in which the start element id: {:?} exists",
                self.start_pos.element_id
            );
            log::error!("{}", msg);
            return Err(msg);
        }

        // Reset visited elements
        let mut elements = BTreeMap::new();

        for (key, ctx) in &self.contexts {
            for k in ctx.model.edges.keys() {
                let position = Position::new(key.clone(), k.to_string());
                elements.insert(position, 0);
            }
            for k in ctx.model.vertices.keys() {
                let position = Position::new(key.clone(), k.to_string());
                elements.insert(position, 0);
            }
        }
        self.unvisited_elements = elements;

        /*
         * Check that there's a start position
         */
        let start_pos = self.start_pos.clone();
        self.current_pos = start_pos;
        self.status = MachineStatus::Running;

        Ok(())
    }

    /*
     * Return a list of vertices that has matching share state name as: `shared_state_str`
     */
    fn get_other_shared_states(&self, shared_state_str: String) -> Vec<Position> {
        let mut list: Vec<Position> = Vec::new();
        for ctx in self.contexts.values() {
            for (k, v) in &ctx.model.vertices {
                if let Some(name) = &v.shared_state {
                    if name.cmp(&shared_state_str) == Ordering::Equal {
                        list.push(Position {
                            model_id: ctx.id.clone(),
                            element_id: k.clone(),
                        });
                    }
                }
            }
        }
        list
    }

    pub fn load_models(&mut self, models: Models) -> Result<(), String> {
        log::debug!("Loading {} models", models.models.len());
        for (key, model) in models.models {
            if self.contexts.contains_key(&key) {
                let msg = format!("Model id: {} is not uniqe. This was unexpected.", &key);
                log::error!("{}", msg);
                return Err(msg);
            }

            let mut context = Context::new();
            context.id = key.clone();
            context.model = model.clone();
            context.fullfillment = 1f32;

            self.contexts.insert(key.clone(), context);
        }
        Ok(())
    }

    pub fn step(&mut self) -> Result<String, String> {
        let current_pos = self.current_pos.clone();
        let msg = match self.log_step(current_pos.clone()) {
            Ok(step_str) => step_str,
            Err(err) => return Err(err),
        };

        let mut model = self
            .contexts
            .get_mut(&current_pos.clone().model_id)
            .unwrap()
            .model
            .clone();

        // Check that the element does exist in the model
        if !model.has_id(current_pos.clone().element_id) {
            let msg = format!(
                "Element {} was not found in model: {}",
                current_pos.clone().element_id,
                current_pos.clone().model_id,
            );
            log::error!("{}", msg);
            return Err(msg);
        }

        // If the current position represents an edge, return that edge
        // The next element is the destination vertex.
        if let Some(edge) = model.edges.clone().get(&current_pos.clone().element_id) {
            self.current_pos.element_id = edge.target_vertex_id.as_ref().unwrap().to_string();
            return Ok(msg);
        }

        // The current position must represent a vertex
        if let Some(vertex) = model.vertices.get(&current_pos.clone().element_id) {
            // Look for shared_states
            let mut candidates: Vec<Position> = Vec::new();
            if let Some(name) = vertex.clone().shared_state {
                candidates.clone_from(&self.get_other_shared_states(name));
                // Remove the current vertex from the candidate list, since we are already at it.
                let index = candidates.iter().position(|x| *x == current_pos).unwrap();
                candidates.remove(index);
            }

            for e in model.out_edges(current_pos.clone().element_id) {
                let pos = Position {
                    model_id: current_pos.clone().model_id,
                    element_id: e.id.unwrap(),
                };
                candidates.push(pos);
            }

            if candidates.is_empty() {
                // Vertex is a cul-de-sac
                let msg = format!("Vertex {} is a cul-de-sac", current_pos.clone().element_id);
                log::warn!("{}", msg);
                return Err(msg);
            }

            let random_index = fastrand::usize(..candidates.len());
            self.current_pos = candidates[random_index].clone();

            return Ok(msg);
        }

        // If reached this code, there is something fishy going on
        let msg = "Could not find vertex nor edge matching the current position".to_string();
        log::warn!("{}", msg);
        Err(msg)
    }

    pub fn walk(&mut self) -> Result<(), String> {
        match self.reset() {
            Ok(()) => {}
            Err(err) => {
                self.status = MachineStatus::Failed;
                return Err(err);
            }
        }

        loop {
            if self.is_all_fullfilled() {
                log::debug!("All models are fullfilled and the machine is running");
                self.status = MachineStatus::Ended;
                log::debug!("The machine has ended");
                return Ok(());
            }

            match self.step() {
                Ok(step) => {
                    println!("{:?}", step);
                    if self.status == MachineStatus::Ended {
                        log::debug!("The machine has ended");
                        return Ok(());
                    }
                }
                Err(err) => {
                    self.status = MachineStatus::Failed;
                    log::debug!("The machine has failed");
                    return Err(err);
                }
            }
        }
    }

    pub fn seed(&mut self, number: u64) {
        fastrand::seed(number);
    }
}
