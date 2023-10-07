use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde_derive::Serialize;
use std::collections::BTreeMap;

use graph::Models;

pub mod context;
pub mod profile;

use crate::context::Context;
use crate::profile::Profile;
use crate::profile::Step;

#[derive(Debug, Clone, PartialEq)]
enum MachineStatus {
    NotStarted,
    Running,
    Ended,
}

/// Some execution point of the machine
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Position {
    context_id: Option<String>,
    element_id: Option<String>,
}

impl Default for Position {
    fn default() -> Self {
        Self::new()
    }
}

impl Position {
    #[must_use]
    pub fn new() -> Self {
        Self {
            context_id: None,
            element_id: None,
        }
    }
}
#[derive(Debug, Clone)]
pub struct Machine {
    contexts: BTreeMap<String, Context>,
    profile: Profile,
    current_pos: Position,
    start_pos: Position,
    status: MachineStatus,
    unvisited_elements: BTreeMap<String, u32>,
    rng: ChaCha8Rng,
}

fn step(
    step: Step,
    unvisited_elements: &mut BTreeMap<String, u32>,
    profile: &mut Profile,
) -> Result<(), String> {
    if let Some(value) = unvisited_elements.get(&(step.context_id.clone() + &step.element_id)) {
        let visited = value + 1;
        unvisited_elements.insert(step.context_id.clone() + &step.element_id, visited);
    } else {
        let msg = format!(
            "Expected the key {}{} to be found in unvisited_elements",
            step.context_id, step.element_id
        );
        log::error!("{}", msg);
        return Err(msg);
    }
    profile.push(step);
    log::trace!("Elements visited{:?}", unvisited_elements);
    Ok(())
}

impl Default for Machine {
    fn default() -> Self {
        Self::new()
    }
}

impl Machine {
    #[must_use]
    pub fn new() -> Self {
        Self {
            contexts: BTreeMap::new(),
            profile: Profile::new(),
            current_pos: Position::new(),
            start_pos: Position::new(),
            status: MachineStatus::NotStarted,
            unvisited_elements: BTreeMap::new(),
            rng: ChaCha8Rng::from_entropy(),
        }
    }

    #[must_use]
    pub fn get_profile(&self) -> Profile {
        self.profile.clone()
    }

    /*
     * Calculates if the machine has covered the models given their stop conditions
     */
    fn get_fullfilment(&self) -> Option<f32> {
        let element_count = self.unvisited_elements.len();

        if element_count == 0 {
            log::error!("No elements in the models. This was unexpected");
            return None;
        }

        let visited_count = self
            .unvisited_elements
            .iter()
            .filter(|(_key, visits)| visits > &&0)
            .count();

        log::trace!("Visited elements: {}/{}", visited_count, element_count);
        Some(visited_count as f32 / element_count as f32)
    }

    /**
     * Returns true if more elements exist to visit. The generator dictates
     * when a model in a context is fullfilled.
     * If the macine is not ready toi run, None is returned.
     */
    pub fn has_next(&mut self) -> Option<bool> {
        match self.get_fullfilment() {
            Some(full_filment) => {
                log::debug!("Fullfilment is: {}", full_filment);
                if full_filment < 1. {
                    Some(true)
                } else {
                    self.status = MachineStatus::Ended;
                    Some(false)
                }
            }
            None => None,
        }
    }

    /**
     * Returns the next id of the element to be executed. If no more elements
     * found to be executed, None is returned.
     */
    pub fn next_step(&mut self) -> Result<Option<Position>, String> {
        /*
         * If machine is not started, pick the `start_id` as the first element
         * to be executed.
         * There can only be one starting point in a machine.
         */
        if self.status == MachineStatus::NotStarted {
            if !self.verify_valid_start_postion() {
                let msg = "No valid Context is not defined here. This was unexpected.".to_string();
                log::error!("{}", msg);
                return Err(msg);
            }

            self.status = MachineStatus::Running;
            let context = self.contexts.get_mut(
                &self
                    .start_pos
                    .context_id
                    .clone()
                    .expect("Expected start context id"),
            );

            if context.is_none() {
                let msg = "Context is not defined here. This was unexpected.".to_string();
                log::error!("{}", msg);
                return Err(msg);
            }

            log::debug!(
                "Models has valid start context and start element: {:?}, {:?}",
                self.start_pos.context_id.as_deref(),
                self.start_pos.element_id.as_deref()
            );
            self.current_pos.context_id = self.start_pos.context_id.clone();
            self.current_pos.element_id = self.start_pos.element_id.clone();

            match step(
                Step {
                    context_id: self
                        .start_pos
                        .context_id
                        .clone()
                        .expect("Expected start context id"),
                    element_id: self
                        .start_pos
                        .element_id
                        .clone()
                        .expect("Expected start element id"),
                },
                &mut self.unvisited_elements,
                &mut self.profile,
            ) {
                Err(why) => return Err(why),
                Ok(()) => return Ok(Some(self.start_pos.clone())),
            }
        } else if self.status == MachineStatus::Running {
            if !self.verify_valid_current_position() {
                let msg = "No valid current position is defined. This was unexpected.".to_string();
                log::error!("{}", msg);
                return Err(msg);
            }

            let spare_list = self.contexts.clone();
            if let Some(context) = self.contexts.get_mut(
                &self
                    .current_pos
                    .context_id
                    .clone()
                    .expect("Expected current context id"),
            ) {
                log::trace!("Current model and element are: {:?}", self.current_pos);

                // Is the current element an edge, and does is exist in the model?
                match context.model.edges.get(
                    &self
                        .current_pos
                        .element_id
                        .clone()
                        .expect("Expected current element id"),
                ) {
                    None => {}
                    Some(e) => {
                        self.current_pos.element_id = Some(
                            e.target_vertex_id
                                .clone()
                                .expect("Expected target vertex id"),
                        );

                        match step(
                            Step {
                                context_id: self
                                    .current_pos
                                    .context_id
                                    .clone()
                                    .expect("Expected current context id"),
                                element_id: self
                                    .current_pos
                                    .element_id
                                    .clone()
                                    .expect("Expected currentelement id"),
                            },
                            &mut self.unvisited_elements,
                            &mut self.profile,
                        ) {
                            Err(why) => return Err(why),
                            Ok(()) => return Ok(Some(self.current_pos.clone())),
                        }
                    }
                }

                /*
                 * Is the current element a vertex, and does is exist in the mdoel?
                 */
                if context.model.vertices.contains_key(
                    &self
                        .current_pos
                        .element_id
                        .clone()
                        .expect("Expected current element id"),
                ) {
                    let mut candidate_elements: Vec<(String, String)> = Vec::new();

                    /*
                     * First check if the current vertex is a shared vertex.
                     */
                    if let Some(vertex) = context.model.vertices.get(
                        &self
                            .current_pos
                            .element_id
                            .clone()
                            .expect("Expected current element id"),
                    ) {
                        if vertex.shared_state.is_some() {
                            for spare_list_context in spare_list.values() {
                                if spare_list_context.id == context.id {
                                    continue;
                                }
                                log::trace!(
                                    "Checking in model: {:?} for {:?}",
                                    spare_list_context.model.name.as_deref(),
                                    vertex.shared_state.as_deref()
                                );
                                for other_vertex in spare_list_context.model.vertices.values() {
                                    if other_vertex.shared_state.as_deref()
                                        == vertex.shared_state.as_deref()
                                    {
                                        candidate_elements.push((
                                            spare_list_context.id.clone(),
                                            other_vertex
                                                .id
                                                .clone()
                                                .expect("Expected the other vertex id"),
                                        ));
                                        log::trace!(
                                            "Adding shared state: {:?}",
                                            candidate_elements.last()
                                        );
                                    }
                                }
                            }
                            log::trace!("Matching shared states: {}", candidate_elements.len());
                        }
                    }

                    match context.model.out_edges(self.current_pos.element_id.clone()) {
                        None => {}
                        Some(list) => {
                            for element_id in list {
                                candidate_elements.push((context.id.clone(), element_id));
                            }
                        }
                    }

                    log::trace!("Candidate list: {:?}", candidate_elements);

                    let index = self.rng.gen_range(0..candidate_elements.len());
                    if let Some((ctx_id, elem_id)) = candidate_elements.get(index) {
                        log::trace!(
                            "Selected candidate: {}{}, using index {}",
                            ctx_id,
                            elem_id,
                            index
                        );

                        self.current_pos.context_id = Some(ctx_id.clone());
                        self.current_pos.element_id = Some(elem_id.clone());

                        match step(
                            Step {
                                context_id: ctx_id.clone(),
                                element_id: elem_id.clone(),
                            },
                            &mut self.unvisited_elements,
                            &mut self.profile,
                        ) {
                            Err(why) => return Err(why),
                            Ok(()) => return Ok(Some(self.current_pos.clone())),
                        }
                    } else {
                        let msg = format!("Random selected index: {index}, resulted in failure");
                        log::error!("{}", msg);
                        return Err(msg);
                    }
                }
                self.status = MachineStatus::Ended;
                let msg = "Machine is exhausted".to_string();
                log::error!("{}", msg);
                return Err(msg);
            }
        } else if self.status == MachineStatus::Ended {
            let msg = "Machine is exhausted".to_string();
            log::error!("{}", msg);
            return Err(msg);
        }
        let msg = "Machine is exhausted".to_string();
        log::error!("{}", msg);
        Err(msg)
    }

    fn verify_valid_start_postion(&mut self) -> bool {
        if self.start_pos.context_id.is_none() && self.start_pos.element_id.is_none() {
            return false;
        }
        match self.contexts.get_mut(
            &self
                .start_pos
                .context_id
                .clone()
                .expect("Expected start context id"),
        ) {
            Some(context) => {
                if context.model.has_id(
                    self.start_pos
                        .element_id
                        .clone()
                        .expect("Expected start element id"),
                ) {
                    return true;
                }
                false
            }
            None => false,
        }
    }

    fn verify_valid_current_position(&mut self) -> bool {
        if self.current_pos.context_id.is_none() && self.current_pos.element_id.is_none() {
            return false;
        }
        match self.contexts.get_mut(
            &self
                .current_pos
                .context_id
                .clone()
                .expect("Expected current context id"),
        ) {
            Some(context) => {
                if context.model.has_id(
                    self.current_pos
                        .element_id
                        .clone()
                        .expect("Expected current element id"),
                ) {
                    return true;
                }
                false
            }
            None => false,
        }
    }

    pub fn load_models(&mut self, models: Models) -> Result<(), String> {
        log::debug!("Loading {} models", models.models.len());
        for (key, model) in models.models {
            if self.contexts.contains_key(&key) {
                let msg = format!("Model id: {} is not uniqe. This was unexpected.", &key);
                log::error!("{}", msg);
                return Err(msg);
            }

            self.contexts.insert(
                key.clone(),
                Context {
                    id: key.clone(),
                    model: model.clone(),
                    generators: Vec::new(),
                },
            );

            // The start_element_id can be defined in one or many models, but only one value should be used.
            // Graphwalker studio saves the same value in all models in a json file.
            if model.start_element_id.is_some() {
                log::debug!(
                    "Found start element: {:?}",
                    model.start_element_id.as_deref()
                );
                self.start_pos.element_id =
                    Some(model.start_element_id.expect("Expected start elem,ent id"));
            }

            for (k, _e) in model.edges {
                let uk = key.clone() + &k;
                self.unvisited_elements.insert(uk, 0);
            }
            for (k, _v) in model.vertices {
                let uk = key.clone() + &k;
                self.unvisited_elements.insert(uk, 0);
            }
        }

        for (key, context) in &self.contexts {
            if context.model.has_id(
                self.start_pos
                    .element_id
                    .clone()
                    .expect("Expected start element id"),
            ) {
                self.start_pos.context_id = Some(key.clone());
            }
        }

        /*
         * Verify the corectness of starting element and context.
         */
        match self.start_pos.context_id {
            Some(_) => Ok(()),
            None => {
                if self.start_pos.element_id.is_some() {
                    Ok(())
                } else {
                    let msg = "Could not determine what model to start in. Is the startElementId correct?".to_string();
                    log::error!("{}", msg);
                    Err(msg)
                }
            }
        }
    }

    pub fn walk(&mut self) -> Result<(), &'static str> {
        loop {
            match self.next_step() {
                Ok(_next_id) => match self.has_next() {
                    Some(has_next) => {
                        if !has_next {
                            break;
                        }
                    }
                    None => {
                        break;
                    }
                },
                Err(why) => {
                    log::debug!("{}", why);
                    break;
                }
            }
        }

        if self.status != MachineStatus::Ended {
            return Err("Walking the models did not end as expected.");
        }
        Ok(())
    }

    pub fn seed(&mut self, number: u64) {
        self.rng = ChaCha8Rng::seed_from_u64(number);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn resource_path(resource: &str) -> std::path::PathBuf {
        let mut path = std::path::PathBuf::new();
        path.push(env!("CARGO_MANIFEST_DIR"));
        path.push("..");
        path.push("..");
        path.push("resources");
        path.push("models");
        path.push(resource);
        path
    }

    #[test]
    fn walk_multiple_model() {
        let mut machine = Machine::new();
        let res = machine.load_models(
            io::json_read::read(resource_path("simpleMultiModel.json").to_str().unwrap())
                .expect("Expexted the test file to be loaded"),
        );
        assert_eq!(res.is_ok(), true);

        let res = machine.walk();
        assert_eq!(res.is_ok(), true, "{:?}", Err::<(), Result<(), &str>>(res));
        assert!(machine.get_profile().steps.len() > 10);
    }

    #[test]
    fn walk_single_model() {
        let mut machine = Machine::new();
        let res = machine.load_models(
            io::json_read::read(resource_path("simpleSingleModel.json").to_str().unwrap())
                .expect("Expexted the test file to be loaded"),
        );
        assert_eq!(res.is_ok(), true);

        let res = machine.walk();
        assert_eq!(res.is_ok(), true, "{:?}", Err::<(), Result<(), &str>>(res));
    }
    #[test]
    fn test_seed() {
        let mut machine = Machine::new();
        machine.seed(8739438725484);
        let index = machine.rng.gen_range(0..1000);
        assert_eq!(index, 144);
        let index = machine.rng.gen_range(0..1000);
        assert_eq!(index, 124);
        let index = machine.rng.gen_range(0..1000);
        assert_eq!(index, 629);
        let index = machine.rng.gen_range(0..1000);
        assert_eq!(index, 577);
    }

    #[test]
    fn machine() {
        let mut machine = Machine::new();
        machine.seed(8739438725484);
        assert!(machine
            .load_models(
                io::json_read::read(resource_path("login.json").to_str().unwrap())
                    .expect("Expexted the test file to be loaded")
            )
            .is_ok());

        assert_eq!(machine.contexts.len(), 1);
        assert_eq!(
            machine
                .start_pos
                .context_id
                .clone()
                .expect("Expected start context id"),
            "853429e2-0528-48b9-97b3-7725eafbb8b5".to_string()
        );
        assert_eq!(
            machine
                .start_pos
                .element_id
                .clone()
                .expect("Expected start element id"),
            "e0".to_string()
        );
        assert_eq!(machine.status, MachineStatus::NotStarted);

        let mut path = Vec::new();
        loop {
            if let Ok(next_pos) = machine.next_step() {
                match next_pos {
                    Some(position) => {
                        path.push(position.element_id.unwrap());
                    }
                    None => assert!(false),
                }

                assert_eq!(machine.status, MachineStatus::Running);

                match machine.has_next() {
                    Some(has_next) => {
                        if !has_next {
                            assert_eq!(machine.status, MachineStatus::Ended);
                            break;
                        }
                    }
                    None => {
                        assert_eq!(machine.status, MachineStatus::Ended);
                        break;
                    }
                }
            } else {
                assert_eq!(machine.status, MachineStatus::Ended);
                break;
            }
        }

        let expected_path = vec![
            "e0", "n1", "e0", "n1", "e0", "n1", "e1", "n2", "e6", "n1", "e7", "n3", "e4",
            "n1", "e1", "n2", "e5", "n2", "e6", "n1", "e0", "n1", "e0", "n1", "e7", "n3",
            "e3", "n2", "e8", "n2", "e6", "n1", "e1", "n2", "e5", "n2", "e2",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        assert!(do_vecs_match(&expected_path, &path));
    }

    fn do_vecs_match<T: PartialEq>(a: &Vec<T>, b: &Vec<T>) -> bool {
        let matching = a.iter().zip(b.iter()).filter(|&(a, b)| a == b).count();
        matching == a.len() && matching == b.len()
    }
}
