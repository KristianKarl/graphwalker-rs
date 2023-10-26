use evalexpr::*;
use generator::Generator;
use graph::Model;
use graph::Models;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::Mutex;
use std::{
    cmp::Ordering,
    collections::{BTreeMap, VecDeque},
};

use crate::generator::RandomGenerator;

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

#[derive(Clone, Debug)]
pub struct Data {
    name: String,
    value: evalexpr::Value,
}

impl Serialize for Data {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("Data", 2)?;
        s.serialize_field("name", &self.name)?;

        if self.value.is_boolean() {
            s.serialize_field("value", &self.value.as_boolean().unwrap())?;
        } else if self.value.is_empty() {
            s.serialize_field("value", &self.value.as_empty().unwrap())?;
        } else if self.value.is_float() {
            s.serialize_field("value", &self.value.as_float().unwrap())?;
        } else if self.value.is_int() {
            s.serialize_field("value", &self.value.as_int().unwrap())?;
        } else if self.value.is_number() {
            s.serialize_field("value", &self.value.as_number().unwrap())?;
        } else if self.value.is_string() {
            s.serialize_field("value", &self.value.as_string().unwrap())?;
        }
        s.end()
    }
}

#[derive(Serialize, Clone, Default, Debug)]
pub struct Step {
    pub model_name: String,
    pub element_name: String,
    pub position: Position,
    pub data: Vec<Data>,
}

#[derive(Clone, Default, Debug)]
pub struct Profile {
    pub steps: VecDeque<Step>,
}

impl Profile {
    fn new() -> Self {
        Self {
            steps: VecDeque::new(),
        }
    }

    fn push(&mut self, step: Step) {
        self.steps.push_back(step);
    }
}

#[derive(Default, Clone, Debug)]
pub struct Context {
    id: String,
    model: Arc<Model>,
    fullfillment: f32,
    visited_elements: BTreeMap<String, u32>,
    eval_context: evalexpr::HashMapContext,
    generators: Vec<Arc<dyn Generator>>,
}

impl Context {
    /*
     * Calculates if the specific model has covered the models given their stop conditions
     */
    fn get_fullfilment(&self) -> f32 {
        let element_count = self.visited_elements.len();

        if element_count == 0 {
            return 1f32;
        }

        let visited_count = self
            .visited_elements
            .iter()
            .filter(|(_k, v)| v > &&0)
            .count();
        log::debug!(
            "Fullfillment for model: {:?} is {:?}",
            self.id,
            visited_count as f32 / element_count as f32,
        );
        visited_count as f32 / element_count as f32
    }

    fn is_fullfilled(&self) -> bool {
        if self.get_fullfilment() < self.fullfillment {
            return false;
        }
        true
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

#[derive(Default, Debug, Clone, PartialEq)]
pub struct SharedState {
    name: String,
    positions: Vec<Position>,
}

#[derive(Default, Debug, Clone)]
pub struct Machine {
    pub contexts: BTreeMap<String, Arc<Mutex<Context>>>,
    pub profile: Profile,
    current_pos: Position,
    pub start_pos: Position,
    pub status: MachineStatus,
    unvisited_edges: Vec<Position>,
    shared_states: Vec<SharedState>,
}

impl Machine {
    pub fn is_all_fullfilled(&self) -> bool {
        for ctx in self.contexts.values() {
            let c = ctx.lock().unwrap();
            if !c.is_fullfilled() {
                log::debug!("The model: {:?} is not fullfilled", c.id);
                return false;
            }
            log::debug!("The model: {:?} is fullfilled", c.id);
        }
        log::debug!("All models are fullfilled");
        true
    }

    fn log_step(&mut self, position: &Position) -> Result<Step, String> {
        log::debug!("Position: {:?}", position);
        let mut step = Step {
            position: position.clone(),
            ..Default::default()
        };

        if let Some(ctx) = self.contexts.get_mut(&step.position.model_id) {
            let mut c = ctx.lock().unwrap();
            if let Some(name) = c.model.name.clone() {
                step.model_name = name;
            }
            if let Some(name) = c.model.get_name_for_id(&position.element_id) {
                step.element_name = name;
            }

            if c.eval_context.iter_variables().len() > 0 {
                for (n, v) in c.eval_context.iter_variables() {
                    let data = Data { name: n, value: v };
                    step.data.push(data);
                }
                log::debug!("Data: {:?}", step);
            }

            if let Some(value) = c.visited_elements.get(&step.position.element_id) {
                let visited = value + 1;
                c.visited_elements
                    .insert(step.position.clone().element_id, visited);
            } else {
                let msg = format!(
                    "Expected the key {:?} to be found in unvisited_elements",
                    step.clone()
                );
                log::error!("{}", msg);
                return Err(msg);
            }
            self.profile.push(step.clone());
            Ok(step)
        } else {
            let msg = format!(
                "The model id {:?} was not found in the machine",
                step.position.model_id
            );
            log::error!("{}", msg);
            Err(msg)
        }
    }

    pub fn reset(&mut self) -> Result<(), String> {
        log::debug!("Resetting the machine");
        log::info!("The seed is: {:?}", fastrand::get_seed());

        for ctx in self.contexts.values_mut() {
            let mut c = ctx.lock().unwrap();
            c.eval_context = HashMapContext::default();

            for action in c.model.actions.clone() {
                log::debug!("Will run model action: {:?}", action);

                match eval_with_context_mut(action.as_str(), &mut c.eval_context) {
                    Ok(value) => {
                        log::debug!("Action evaluated to: {:?}", value);
                    }
                    Err(err) => {
                        let msg = format!(
                            "Evaluating action {:?}, failed with error: {:?}",
                            action, err
                        );
                        log::error!("{}", msg);
                        return Err(msg);
                    }
                }
            }
        }

        // If no start elemet id is found, bail out
        if self.start_pos.element_id.is_empty() {
            let msg = "Did not find any start element id. Cannot contiune".to_string();
            log::error!("{}", msg);
            return Err(msg);
        }

        // Find the model in which the start element id exists
        for (key, ctx) in &self.contexts {
            let c = ctx.lock().unwrap();

            if c.model.has_id(self.start_pos.element_id.clone()) {
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

        // Reset visited elements and unvisited edges
        self.unvisited_edges = Vec::default();
        for (key, ctx) in &mut self.contexts {
            let mut c = ctx.lock().unwrap();
            let mut visited_elements = BTreeMap::new();
            let mut unvisited_edges = Vec::new();

            for k in c.model.edges.keys() {
                visited_elements.insert(k.to_string(), 0);
                unvisited_edges.push(Position::new(key.to_string(), k.to_string()));
            }
            for k in c.model.vertices.keys() {
                visited_elements.insert(k.to_string(), 0);
            }
            c.visited_elements = visited_elements;
            self.unvisited_edges.extend(unvisited_edges);
        }

        /*
         * Check that there's a start position
         */
        let start_pos = self.start_pos.clone();
        self.current_pos = start_pos;
        self.status = MachineStatus::Running;

        Ok(())
    }

    pub fn load_models(&mut self, models: Models) -> Result<(), String> {
        log::debug!("Loading {} models", models.models.len());
        if let Some(pos) = models.start_element_id {
            self.start_pos.element_id = pos;
        } else {
            let msg = "The Models data did not have a start_element_id. Start id is mandatory.";
            log::error!("{}", msg);
            return Err(msg.to_string());
        }

        for (key, model) in models.models {
            if self.contexts.contains_key(&key) {
                let msg = format!("Model id: {} is not uniqe. This was unexpected.", &key);
                log::error!("{}", msg);
                return Err(msg);
            }

            let mut context = Context::default();
            context.id = key.clone();
            context.model = model.clone();
            context.fullfillment = 1f32;
            context
                .generators
                .push(Arc::new(RandomGenerator::default()));

            self.contexts
                .insert(key.clone(), Arc::new(Mutex::new(context)));

            // Populate list of shared states.
            for (k, v) in &model.vertices {
                if let Some(name) = &v.shared_state {
                    let c = self
                        .shared_states
                        .iter()
                        .filter(|x| &x.name == name)
                        .count();

                    if c > 0 {
                        let mut s = SharedState::default();
                        s.name = name.to_string();
                        s.positions.push(Position {
                            model_id: key.clone(),
                            element_id: k.to_string(),
                        });
                        self.shared_states.push(s);
                    } else {
                        let index = self
                            .shared_states
                            .iter()
                            .position(|x| &x.name == name)
                            .unwrap();
                        if let Some(s) = self.shared_states.get_mut(index) {
                            s.positions.push(Position {
                                model_id: key.clone(),
                                element_id: k.to_string(),
                            })
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn step(&mut self) -> Result<Step, String> {
        let current_pos = self.current_pos.clone();

        let step = match self.log_step(&current_pos) {
            Ok(s) => s,
            Err(err) => return Err(err),
        };

        match self.run_action(&current_pos) {
            Ok(_) => {}
            Err(err) => return Err(err),
        };

        if let Some(ctx) = self.contexts.get_mut(&current_pos.model_id) {
            let model = &ctx.lock().unwrap().model;
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

            // If the current position represents an edge the next element should be a Vertex.
            // That vertex is extracted from the edge target vertex.
            if let Some(edge) = model.edges.clone().get(&current_pos.clone().element_id) {
                self.current_pos.element_id = edge.target_vertex_id.clone();
                return Ok(step);
            }
        }

        // If we have not found a step yet, the next step must be a an edge.
        // First use the current generator strategy.
        if let Some(ctx) = self.contexts.get_mut(&current_pos.model_id) {
            if let Some(generator) = ctx.lock().unwrap().generators.get_mut(0) {
                match generator.get_next_edge(Arc::clone(&ctx), &self.shared_states, current_pos) {
                    Ok(pos) => self.current_pos = pos,
                    Err(err) => {
                        let msg = format!("get next step failed: {}", err);
                        log::error!("{}", msg);
                        return Err(msg);
                    }
                }
            }
        }
        Ok(step)
    }

    /*
     * Return the id of the context and a list of actions matching the position
     * of the element.
     */
    fn get_actions(&self, pos: &Position) -> (String, Vec<String>) {
        for (k, ctx) in &self.contexts {
            let c = ctx.lock().unwrap();

            if let Some(edge) = c.model.edges.get(&pos.element_id) {
                return (k.clone(), edge.actions.clone());
            }
            if let Some(vertex) = c.model.vertices.get(&pos.element_id) {
                return (k.clone(), vertex.actions.clone());
            }
        }
        ("".to_string(), Vec::default())
    }

    fn run_action(&mut self, pos: &Position) -> Result<(), String> {
        let (ctx_id, actions) = self.get_actions(pos);

        if ctx_id.is_empty() || actions.is_empty() {
            return Ok(());
        }

        if let Some(ctx) = self.contexts.get_mut(&ctx_id) {
            for action in actions {
                let mut c = ctx.lock().unwrap();
                log::debug!("Will run: {:?}", action);

                match eval_with_context_mut(&action, &mut c.eval_context) {
                    Ok(value) => {
                        log::debug!("Action evaluated to: {:?}", value);
                    }
                    Err(err) => {
                        let msg = format!(
                            "Evaluating action {:?}, failed with error: {:?}",
                            action, err
                        );
                        log::error!("{}", msg);
                        return Err(msg);
                    }
                }
            }
        }
        Ok(())
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
                Ok(step) => match serde_json::to_string(&step) {
                    Ok(step_json_str) => {
                        println!("{}", step_json_str);
                        if self.status == MachineStatus::Ended {
                            log::debug!("The machine has ended");
                            return Ok(());
                        }
                    }
                    Err(err) => {
                        let msg = format!("Could extract the json str from step: {:?}", err);
                        log::warn!("{}", msg);
                        return Err(msg);
                    }
                },
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
