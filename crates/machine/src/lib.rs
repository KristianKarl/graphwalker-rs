use rand::Rng;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

use graph::{Edge, Model, Models};

#[derive(Debug, Clone)]
struct StopCondition {}

#[derive(Debug, Clone)]
struct Generator {
    stop_conditions: Vec<StopCondition>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Step {
    context_id: String,
    element_id: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Profile {
    steps: Vec<Step>,
}

impl Profile {
    fn new() -> Profile {
        Profile { steps: Vec::new() }
    }

    fn push(&mut self, step: Step) {
        log::trace!("{:?}", step.clone());
        self.steps.push(step);
    }
}
#[derive(Debug, Clone)]
struct Context {
    pub id: String,
    model: Model,
    generators: Vec<Generator>,
    current_element_id: Option<String>,
}

#[derive(Debug, PartialEq)]
enum MachineStatus {
    NotStarted,
    Running,
    Ended,
}

pub struct Machine {
    contexts: HashMap<String, Context>,
    profile: Profile,
    current_context_id: Option<String>,
    start_context_id: Option<String>,
    start_element_id: Option<String>,
    status: MachineStatus,
    unvisited_elements: HashMap<String, u32>,
}

fn step(step: Step, unvisited_elements: &mut HashMap<String, u32>, profile: &mut Profile) {
    match unvisited_elements.get(&(step.context_id.clone() + &step.element_id.clone())) {
        Some(value) => {
            let visited = value + 1;
            unvisited_elements.insert(step.context_id.clone() + &step.element_id.clone(), visited);
        }
        None => {
            log::error!(
                "Expected the key {}{} to be found in unvisited_elements",
                step.context_id,
                step.element_id
            );
        }
    }
    profile.push(step);
}

impl Machine {
    pub fn new() -> Machine {
        Machine {
            contexts: HashMap::new(),
            profile: Profile::new(),
            current_context_id: None,
            start_context_id: None,
            start_element_id: None,
            status: MachineStatus::NotStarted,
            unvisited_elements: HashMap::new(),
        }
    }

    pub fn get_profile(&self) -> Profile {
        self.profile.clone()
    }

    /*
     * Calculates if the machine has covered the models given their stop conditions
     */
    fn get_fullfilment(&self) -> Option<f32> {
        let mut element_count = self.unvisited_elements.len();

        if element_count == 0 {
            log::error!("No elements in the models. This was unexpected");
            return None;
        }

        let visited_count = self
            .unvisited_elements
            .iter()
            .filter(|(_key, visits)| visits > &&0)
            .collect::<Vec<_>>()
            .len();

        log::trace!("Visited elements: {}/{}", visited_count, element_count);
        return Some(visited_count as f32 / element_count as f32);
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
                    return Some(true);
                } else {
                    self.status = MachineStatus::Ended;
                    return Some(false);
                }
            }
            None => return None,
        }
    }

    /**
     * Returns the next id of the element to be executed. If no more elements
     * found to be executed, None is returned.
     */
    pub fn next(&mut self) -> Option<String> {
        /*
         * If machine is not started, we pick the `start_id` as the first element
         * to be executed.
         * There can only be one starting point in a machine.
         */
        if self.status == MachineStatus::NotStarted {
            if !self.verify_valid_start_context() {
                log::error!("No valid Context is not defined here. This was unexpected.");
                return None;
            }

            self.status = MachineStatus::Running;
            self.current_context_id = self.start_context_id.clone();
            let context = self
                .contexts
                .get_mut(&self.start_context_id.clone().unwrap());

            if context.is_none() {
                log::error!("Context is not defined here. This was unexpected.");
                return None;
            }

            log::debug!(
                "Models has valid start context and start elements: {:?}, {:?}",
                self.start_context_id.as_deref(),
                self.start_element_id.as_deref()
            );

            let mut ctx = context.unwrap();
            ctx.current_element_id = self.start_element_id.clone();
            step(
                Step {
                    context_id: ctx.id.clone(),
                    element_id: ctx.current_element_id.clone().unwrap(),
                },
                &mut self.unvisited_elements,
                &mut self.profile,
            );
            return Some(ctx.current_element_id.clone().unwrap());
        } else if self.status == MachineStatus::Running {
            if self.current_context_id.is_none() {
                self.status = MachineStatus::Ended;
                return None;
            }

            let spare_list = self.contexts.clone();
            match self
                .contexts
                .get_mut(&self.current_context_id.clone().unwrap())
            {
                Some(context) => {
                    log::trace!(
                        "Current model and element are: {:?}, {:?}",
                        self.current_context_id.as_deref(),
                        context.current_element_id.as_deref()
                    );
                    /*
                     * Is the current element an edge, and does is exist in the mdoel?
                     */
                    if context
                        .model
                        .edges
                        .iter()
                        .any(|t| t.1.id == context.current_element_id)
                    {
                        let e: &Edge = context
                            .model
                            .edges
                            .get(&context.current_element_id.clone().unwrap())
                            .unwrap();
                        context.current_element_id = e.target_vertex_id.clone();
                        step(
                            Step {
                                context_id: context.id.clone(),
                                element_id: context.current_element_id.clone().unwrap(),
                            },
                            &mut self.unvisited_elements,
                            &mut self.profile,
                        );
                        return e.target_vertex_id.clone();
                    }
                    /*
                     * Is the current element a vertex, and does is exist in the mdoel?
                     */
                    else if context
                        .model
                        .vertices
                        .contains_key(&context.current_element_id.clone().unwrap())
                    {
                        /*
                         * First check if the current vertex is a shared vertex.
                         */
                        match context
                            .model
                            .vertices
                            .get(&context.current_element_id.clone().unwrap())
                        {
                            Some(vertex) => {
                                if vertex.shared_state.is_some() {
                                    let mut list_of_same_shared_states =
                                        Vec::<(String, String)>::new();

                                    for (_key, spare_list_context) in spare_list {
                                        log::trace!(
                                            "Checking in model: {:?} for {:?}",
                                            spare_list_context.model.name.as_deref(),
                                            vertex.shared_state.as_deref()
                                        );
                                        for (_key, other_vertex) in
                                            spare_list_context.model.vertices.iter()
                                        {
                                            if other_vertex.shared_state.as_deref()
                                                == vertex.shared_state.as_deref()
                                            {
                                                list_of_same_shared_states.push((
                                                    spare_list_context.id.clone(),
                                                    other_vertex.id.clone().unwrap(),
                                                ));
                                                log::trace!(
                                                    "Adding shared state: {:?}",
                                                    list_of_same_shared_states.last()
                                                );
                                            }
                                        }
                                    }
                                    log::trace!(
                                        "Matching shared states: {}",
                                        list_of_same_shared_states.len()
                                    );

                                    // If we have shared states matches, let's pick one vertex to swicth to
                                    if list_of_same_shared_states.len() > 1 {
                                        let mut rng = rand::thread_rng();
                                        let index =
                                            rng.gen_range(0..list_of_same_shared_states.len());
                                        log::trace!("Random number is: {}", index);
                                        match list_of_same_shared_states.get(index) {
                                            Some(context_vertex) => {
                                                log::debug!(
                                                    "Switching to shared state: {}, {}",
                                                    context_vertex.0.clone(),
                                                    context_vertex.1.clone()
                                                );
                                                step(
                                                    Step {
                                                        context_id: context_vertex.0.clone(),
                                                        element_id: context_vertex.1.clone(),
                                                    },
                                                    &mut self.unvisited_elements,
                                                    &mut self.profile,
                                                );
                                                self.current_context_id =
                                                    Some(context_vertex.0.clone());
                                                self.contexts
                                                    .get_mut(&context_vertex.0.clone())
                                                    .unwrap()
                                                    .current_element_id =
                                                    Some(context_vertex.1.clone());
                                                return Some(context_vertex.1.clone());
                                            }
                                            None => {
                                                log::error!(
                                                    "Could not switch to another shared state."
                                                );
                                                self.status = MachineStatus::Ended;
                                                return None;
                                            }
                                        }
                                    }
                                }
                            }
                            None => {}
                        }

                        match context.model.out_edges(context.current_element_id.clone()) {
                            Some(list) => {
                                let mut rng = rand::thread_rng();
                                let index = rng.gen_range(0..list.len());
                                match list.get(index) {
                                    Some(i) => {
                                        context.current_element_id = Some(i.clone());
                                        step(
                                            Step {
                                                context_id: context.id.clone(),
                                                element_id: context
                                                    .current_element_id
                                                    .clone()
                                                    .unwrap(),
                                            },
                                            &mut self.unvisited_elements,
                                            &mut self.profile,
                                        );
                                        return Some(i.clone());
                                    }
                                    None => {
                                        log::error!(
                                            "Random selection of an edge resultes in failure"
                                        );
                                        self.status = MachineStatus::Ended;
                                        return None;
                                    }
                                }
                            }
                            None => {
                                self.status = MachineStatus::Ended;
                                return None;
                            }
                        }
                    }
                    self.status = MachineStatus::Ended;
                    return None;
                }
                None => {
                    self.status = MachineStatus::Ended;
                    return None;
                }
            }
        } else if self.status == MachineStatus::Ended {
            return None;
        }
        None
    }

    /**
     * Returns the context that has an element with an id that is
     * equal to the starting id of the machine.
     */
    fn verify_valid_start_context(&mut self) -> bool {
        if self.start_context_id.is_none() || self.start_element_id.is_none() {
            return false;
        }
        match self
            .contexts
            .get_mut(&self.start_context_id.clone().unwrap())
        {
            Some(context) => {
                if context.model.has_id(self.start_element_id.clone()) {
                    return true;
                }
                return false;
            }
            None => return false,
        }
    }

    pub fn load_models(&mut self, models: Models) -> Result<Option<String>, &'static str> {
        log::debug!("Loading {} models", models.models.len());
        for (key, model) in models.models {
            self.contexts.insert(
                key.clone(),
                Context {
                    id: key.clone(),
                    model: model.clone(),
                    generators: Vec::new(),
                    current_element_id: None,
                },
            );

            // The start_element_id can be defined in one or many models, but only one value should be used.
            // Graphwalker studio saves the same value in all models in a json file.
            if model.start_element_id.is_some() {
                log::debug!(
                    "Found start element: {:?}",
                    model.start_element_id.as_deref()
                );
                self.start_element_id = model.start_element_id;
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
            if context.model.has_id(self.start_element_id.clone()) {
                self.start_context_id = Some(key.clone());
            }
        }

        /*
         * Verify the corectness of starting element and context.
         */
        match self.start_element_id {
            Some(_) => match self.start_context_id {
                Some(_) => return Ok(self.start_context_id.clone()),
                None => Err("Could not determine whch model to start in"),
            },
            None => Err("Did not find the starting element in any model"),
        }
    }

    pub fn walk(&mut self) -> Result<(), &'static str> {
        loop {
            match self.next() {
                Some(_next_id) => match self.has_next() {
                    Some(has_next) => {
                        if !has_next {
                            break;
                        }
                    }
                    None => {
                        break;
                    }
                },
                None => {
                    break;
                }
            }
        }

        if self.status != MachineStatus::Ended {
            return Err("Walking the models did not end as expected.");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use io::json;
    use pretty_assertions::assert_eq;

    #[test]
    fn walk() {
        let mut machine = Machine::new();
        let res = machine.load_models(json::read::read(
            "/home/krikar//dev/graphwalker-rs/models/petclinic.json",
        ));
        assert!(res.is_ok());
        let res = machine.walk();
        assert!(res.is_ok(), "{:?}", Err::<(), Result<(), &str>>(res));
    }

    #[test]
    fn machine() {
        let mut machine = Machine::new();
        assert!(machine
            .load_models(json::read::read(
                "/home/krikar//dev/graphwalker-rs/crates/machine/tests/models/login.json",
            ))
            .is_ok());

        assert_eq!(machine.contexts.len(), 1);
        assert_eq!(
            machine.start_context_id.clone().unwrap(),
            "853429e2-0528-48b9-97b3-7725eafbb8b5".to_string()
        );
        assert_eq!(machine.start_element_id.clone().unwrap(), "e0".to_string());
        assert_eq!(machine.status, MachineStatus::NotStarted);

        let mut path = Vec::new();
        loop {
            match machine.next() {
                Some(next_id) => {
                    path.push(next_id);
                    assert_eq!(machine.status, MachineStatus::Running);

                    match machine.has_next() {
                        Some(has_next) => {
                            if !has_next {
                                break;
                            }
                        }
                        None => {
                            break;
                        }
                    }
                }
                None => {
                    assert_eq!(machine.status, MachineStatus::Ended);
                    break;
                }
            }
        }
    }
}
