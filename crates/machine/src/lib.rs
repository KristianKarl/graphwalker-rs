use evalexpr::*;
use graph::Edge;
use graph::Model;
use graph::Models;
use serde_derive::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, VecDeque},
};

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
    pub steps: VecDeque<Position>,
}

impl Profile {
    fn new() -> Self {
        Self {
            steps: VecDeque::new(),
        }
    }

    fn push(&mut self, step: Position) {
        self.steps.push_back(step);
    }
}

#[derive(Clone, Debug)]
pub struct Context {
    id: String,
    model: Model,
    fullfillment: f32,
    visited_elements: BTreeMap<String, u32>,
    eval_context: evalexpr::HashMapContext,
}

impl Context {
    fn new() -> Self {
        Self {
            id: "".to_string(),
            model: Model::new(),
            fullfillment: 0f32,
            visited_elements: BTreeMap::new(),
            eval_context: evalexpr::HashMapContext::default(),
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
    current_pos: Position,
    pub start_pos: Position,
    pub status: MachineStatus,
    walk_this_way: VecDeque<Position>,
    unvisited_edges: Vec<Position>,
}

impl Machine {
    #[must_use]
    pub fn new() -> Self {
        Self {
            contexts: BTreeMap::new(),
            profile: Profile::new(),
            current_pos: Position::default(),
            start_pos: Position::default(),
            status: MachineStatus::NotStarted,
            walk_this_way: VecDeque::default(),
            unvisited_edges: Vec::default(),
        }
    }

    /*
     * Calculates if the specific model has covered the models given their stop conditions
     */
    fn get_fullfilment(&self, ctx: &Context) -> f32 {
        let element_count = ctx.visited_elements.len();

        if element_count == 0 {
            return 1f32;
        }

        let visited_count = ctx
            .visited_elements
            .iter()
            .filter(|(_k, v)| v > &&0)
            .count();
        log::debug!(
            "Fullfillment for model: {:?} is {:?}",
            ctx.id,
            visited_count as f32 / element_count as f32,
        );
        visited_count as f32 / element_count as f32
    }

    fn is_fullfilled(&self, ctx: &Context) -> bool {
        if self.get_fullfilment(ctx) < ctx.fullfillment {
            return false;
        }
        true
    }

    pub fn is_all_fullfilled(&self) -> bool {
        for ctx in self.contexts.values() {
            if !self.is_fullfilled(ctx) {
                log::debug!("The model: {:?} is not fullfilled", ctx.id);
                return false;
            }
            log::debug!("The model: {:?} is fullfilled", ctx.id);
        }
        log::debug!("All models are fullfilled");
        true
    }

    fn log_step(&mut self, step: &Position) -> Result<(), String> {
        log::debug!("Step: {:?}", step);

        if let Some(ctx) = self.contexts.get_mut(&step.model_id) {
            log::debug!("Data: {:?}", ctx.eval_context);

            if let Some(value) = ctx.visited_elements.get(&step.element_id) {
                let visited = value + 1;
                ctx.visited_elements
                    .insert(step.clone().element_id, visited);
            } else {
                let msg = format!(
                    "Expected the key {:?} to be found in unvisited_elements",
                    step.clone()
                );
                log::error!("{}", msg);
                return Err(msg);
            }
            self.profile.push(step.clone());
            Ok(())
        } else {
            let msg = format!(
                "The model id {:?} was not found in the machine",
                step.model_id
            );
            log::error!("{}", msg);
            Err(msg)
        }
    }

    pub fn reset(&mut self) -> Result<(), String> {
        log::debug!("Resetting the machine");
        log::info!("The seed is: {:?}", fastrand::get_seed());

        for ctx in self.contexts.values_mut() {
            ctx.eval_context = HashMapContext::default();

            for action in &ctx.model.actions {
                log::debug!("Will run model sction: {:?}", action);

                match eval_with_context_mut(action, &mut ctx.eval_context) {
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

        // Find the model in which the start element id exists
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

        // Reset visited elements and unvisited edges
        self.unvisited_edges = Vec::default();
        for (key, ctx) in &mut self.contexts {
            let mut visited_elements = BTreeMap::new();
            let mut unvisited_edges = Vec::new();
            for k in ctx.model.edges.keys() {
                visited_elements.insert(k.to_string(), 0);
                unvisited_edges.push(Position::new(key.to_string(), k.to_string()));
            }
            for k in ctx.model.vertices.keys() {
                visited_elements.insert(k.to_string(), 0);
            }
            ctx.visited_elements = visited_elements;
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

    /*/
     * From current position, which mush represent a vertex, select the next step (edge)
     */
    fn select_next_edge(
        &mut self,
        current_pos: &Position,
        model: &mut Model,
    ) -> Result<Position, String> {
        if let Some(vertex) = model.vertices.get(&current_pos.element_id) {
            // Build a list of candidates of edges to select
            // Look for shared_states
            let mut candidates: Vec<Position> = Vec::new();
            if let Some(name) = vertex.clone().shared_state {
                candidates.clone_from(&self.get_other_shared_states(name));
                // Remove the current vertex from the candidate list, since we are already at it.
                let index = candidates
                    .iter()
                    .position(|x| *x == current_pos.clone())
                    .unwrap();
                candidates.remove(index);
            }

            for e in model.out_edges(current_pos.element_id.clone()) {
                let pos = Position {
                    model_id: current_pos.model_id.clone(),
                    element_id: e.id.clone().unwrap(),
                };
                if self.is_selectable(model.id.clone().unwrap(), &e) {
                    log::trace!("Adding {:?} to the candidates list", pos);
                    candidates.push(pos);
                }
            }

            if candidates.is_empty() {
                // Vertex is a cul-de-sac
                let msg = format!("Vertex {} is a cul-de-sac", current_pos.element_id);
                log::warn!("{}", msg);
                return Err(msg);
            }

            let random_index = fastrand::usize(..candidates.len());
            self.current_pos = candidates[random_index].clone();

            return Ok(current_pos.clone());
        }

        // If reached this code, there is something fishy going on
        let msg = format!(
            "Could not find vertex nor edge matching the current position: {:?}",
            current_pos
        );
        log::warn!("{}", msg);
        Err(msg)
    }

    // fn popuplate_walk_this_way(&mut self) {
    //     if !self.walk_this_way.is_empty() {
    //         return;
    //     }
    //     let random_index = fastrand::usize(..self.unvisited_edges.len());
    //     let pos = self.unvisited_edges[random_index].clone();
    // }

    pub fn step(&mut self) -> Result<Position, String> {
        let current_pos = self.current_pos.clone();

        match self.log_step(&current_pos) {
            Ok(()) => {}
            Err(err) => return Err(err),
        };

        match self.run_action(&current_pos) {
            Ok(_) => {}
            Err(err) => return Err(err),
        };

        // self.popuplate_walk_this_way();
        // if !self.walk_this_way.is_empty() {
        //     if let Some(pos) = self.walk_this_way.pop_front() {
        //         self.current_pos = pos;
        //         return Ok(current_pos);
        //     }
        //     let msg = "Unexpected problem. walk_this_way was not empty, but was not able to get a Position".to_string();
        //     log::warn!("{}", msg);
        //     return Err(msg);
        // } else {
        //     let msg = "Machine is exhausted".to_string();
        //     log::warn!("{}", msg);
        //     return Err(msg);
        // }

        let mut model;
        if let Some(ctx) = self.contexts.get(&current_pos.model_id) {
            model = ctx.model.clone();
        } else {
            let msg = format!("Could not find model id: {}", &current_pos.model_id);
            log::warn!("{}", msg);
            return Err(msg);
        }

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
            return Ok(current_pos);
        }

        // If we have not found a step yet, the next step must be a an edge.
        self.select_next_edge(&current_pos, &mut model)
    }

    /*
     * Return the id of the context and a list of actions matching the position
     * of the element.
     */
    fn get_actions(&self, pos: &Position) -> (String, Vec<String>) {
        for (k, v) in &self.contexts {
            if let Some(edge) = v.model.edges.get(&pos.element_id) {
                return (k.clone(), edge.actions.clone());
            }
            if let Some(vertex) = v.model.vertices.get(&pos.element_id) {
                return (k.clone(), vertex.actions.clone());
            }
        }
        ("".to_string(), Vec::default())
    }

    fn run_action(&mut self, pos: &Position) -> Result<Value, String> {
        let (ctx_id, actions) = self.get_actions(pos);

        if ctx_id.is_empty() || actions.is_empty() {
            return Ok(Value::Empty);
        }

        if let Some(ctx) = self.contexts.get_mut(&ctx_id) {
            if let Some(action) = actions.into_iter().next() {
                log::debug!("Will run: {:?}", action);

                match eval_with_context_mut(&action, &mut ctx.eval_context) {
                    Ok(value) => {
                        log::debug!("Action evaluated to: {:?}", value);
                        return Ok(value);
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
        Ok(Value::Empty)
    }

    /*
     * Returns true if no guard exists for an edge, or if the guard evaluates to true.
     * Else returns false
     */
    fn is_selectable(&mut self, ctx_id: String, e: &Edge) -> bool {
        if let Some(guard) = e.guard.clone() {
            log::debug!("Edge has guard: {:?}", guard);

            if let Some(ctx) = self.contexts.get_mut(&ctx_id) {
                match eval_with_context_mut(&guard, &mut ctx.eval_context) {
                    Ok(value) => match value.as_boolean() {
                        Ok(res) => {
                            log::debug!("The guard evaluated to: {:?}", res);
                            return res;
                        }
                        Err(err) => {
                            let msg = format!(
                                "Evaluating guard {:?}, failed with error: {:?}",
                                guard, err
                            );
                            log::error!("{}", msg);
                            return true;
                        }
                    },
                    Err(err) => {
                        let msg =
                            format!("Evaluating guard {:?}, failed with error: {:?}", guard, err);
                        log::error!("{}", msg);
                        return true;
                    }
                }
            }
        }
        log::trace!("No guard");
        true
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
