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

    fn push(&mut self, step: Step ) {
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
        }
    }

    pub fn get_profile(&self) -> Profile {
        self.profile.clone()
    }

    fn is_visited(&self, context_id: String, element_id: String) -> bool {
        if self
            .profile
            .steps
            .as_slice()
            .into_iter()
            .filter(|s| s.context_id == context_id && s.element_id == element_id)
            .collect::<Vec<_>>()
            .len()
            > 0
        {
            return true;
        } else {
            return false;
        }
    }

    /*
     * Calculates if the machine has covered the models given their stop conditions
     */
    fn get_fullfilment(&self) -> Option<f32> {
        let mut edges_count: f32 = 0.;
        let mut visited_edges_count: f32 = 0.;

        for context in &self.contexts {
            let context = &context.1;
            edges_count += context.model.edges.len() as f32;

            visited_edges_count += context
                .model
                .edges
                .iter()
                .filter(|(id, _edge)| {
                    self.is_visited(context.id.clone(), (**id.clone()).to_string())
                })
                .collect::<Vec<_>>()
                .len() as f32;
        }
        if edges_count == 0. {
            return None;
        }
        return Some(visited_edges_count / edges_count);
    }

    /**
     * Returns true if more elements exist to visit. The generator dictates
     * when a model in a context is fullfilled.
     * If the macine is not ready toi run, None is returned.
     */
    pub fn has_next(&mut self) -> Option<bool> {
        match self.get_fullfilment() {
            Some(full_filment) => {
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
            if self.verify_valid_start_context() {
                self.status = MachineStatus::Running;
                self.current_context_id = self.start_context_id.clone();
                match self
                    .contexts
                    .get_mut(&self.start_context_id.clone().unwrap())
                {
                    Some(context) => {
                        context.current_element_id = self.start_element_id.clone();
                        self.profile.push(Step {
                            context_id: context.id.clone(),
                            element_id: context.current_element_id.clone().unwrap(),
                        });
                        return Some(context.current_element_id.clone().unwrap());
                    }
                    None => return None,
                }
            } else {
                return None;
            }
        } else if self.status == MachineStatus::Running {
            if self.current_context_id.is_none() {
                self.status = MachineStatus::Ended;
                return None;
            }
            match self
                .contexts
                .get_mut(&self.current_context_id.clone().unwrap())
            {
                Some(context) => {
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
                        self.profile.push(Step {
                            context_id: context.id.clone(),
                            element_id: context.current_element_id.clone().unwrap(),
                        });
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
                                    // let mut list = Vec::<&Context>::new();
                                    // for (_key, context) in self.contexts {
                                    //     for (_key, other_vertex) in context.model.vertices.iter() {
                                    //         if other_vertex.shared_state.is_some() && other_vertex.shared_state.clone().unwrap() == vertex.shared_state.clone().unwrap() {
                                    //             list.push(&context);
                                    //         }
                                    //     }
                                    // }
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
                                        self.profile.push(Step {
                                            context_id: context.id.clone(),
                                            element_id: context.current_element_id.clone().unwrap(),
                                        });
                                        return Some(i.clone());
                                    }
                                    None => {
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
            if model.start_element_id.is_some() {
                self.start_element_id = model.start_element_id;
                self.start_context_id = model.id;
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
            "/home/krikar//dev/graphwalker-rs/models/simple.json",
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
