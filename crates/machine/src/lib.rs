use std::collections::HashMap;

use graph::{Model};

#[derive(Debug)]
struct StopCondition {}

#[derive(Debug)]
struct Generator {
    stop_conditions: Vec<StopCondition>,
}

#[derive(Debug)]
struct Context {
    pub id: String,
    model: Model,
    generators: Vec<Generator>,
    current_element_id: Option<u32>,
}

#[derive(Debug, PartialEq)]
enum MachineStatus {
    NotStarted,
    Running,
    Ended,
}

struct Machine {
    contexts: HashMap<String, Context>,
    current_context_id: Option<String>,
    start_context_id: Option<String>,
    start_element_id: Option<String>,
    status: MachineStatus,
}

impl Machine {
    fn new() -> Machine {
        Machine {
            contexts: HashMap::new(),
            current_context_id: None,
            start_context_id: None,
            start_element_id: None,
            status: MachineStatus::NotStarted,
        }
    }

    
    /**
     * Returns the next id of the element to be executed. If no more elements
     * found to be executed, None is returned.
     */
    fn next(&mut self) -> Option<String> {
        /*
         * If machine is not started, we pick the `start_id` as the first element
         * to be executed.
         * There can only be one starting point in a machine.
         */
        if self.status == MachineStatus::NotStarted {
            if self.verify_valid_start_context() {
                self.status = MachineStatus::Running;
                self.current_context_id = self.start_context_id;
                match self.contexts.get_mut(&self.start_context_id.unwrap()) {
                    Some(context) => {
                        context.current_element_id = self.start_element_id;
                        self.profile.steps.push(Step {
                            context_id: context.id,
                            element_id: context.current_element_id.unwrap(),
                        });
                        return Some(context.current_element_id.unwrap());
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
            match self.contexts.get_mut(&self.current_context_id.unwrap()) {
                Some(context) => {
                    /*
                     * Is the current element an edge, and does is exist in the mdoel?
                     */
                    if context
                        .model
                        .edges
                        .contains_key(&context.current_element_id.unwrap())
                    {
                        let e: &Edge = context
                            .model
                            .edges
                            .get(&context.current_element_id.unwrap())
                            .unwrap();
                        context.current_element_id = Some(e.dst_id);
                        self.profile.steps.push(Step {
                            context_id: context.id,
                            element_id: context.current_element_id.unwrap(),
                        });
                        return Some(e.dst_id);
                    }
                    /*
                     * Is the current element a vertex, and does is exist in the mdoel?
                     */
                    else if context
                        .model
                        .vertices
                        .contains_key(&context.current_element_id.unwrap())
                    {
                        /*
                         * First check if the current vertex is a shared vertex.
                         */
                        match context.model.vertices.get_mut(&context.current_element_id.unwrap()) {
                            Some(vertex) => {
                                if vertex.shared_id.is_some() {

                                    let mut list = Vec::<&Context>::new();
                                    for (_key, context) in self.contexts.iter_mut() {
                                        for (_key, other_vertex) in context.model.vertices.iter() {
                                            if other_vertex.shared_id.is_some() && other_vertex.shared_id.unwrap() == vertex.shared_id.unwrap() {
                                                list.push(context);
                                            }
                                        }
                                    }
                                }
                            }
                            None => {}
                        }

                        match context.model.out_edges(context.current_element_id) {
                            Some(list) => {
                                let mut rng = rand::thread_rng();
                                let index = rng.gen_range(0..list.len());
                                match list.get(index) {
                                    Some(i) => {
                                        context.current_element_id = Some(*i);
                                        self.profile.steps.push(Step {
                                            context_id: context.id,
                                            element_id: context.current_element_id.unwrap(),
                                        });
                                        return Some(*i);
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
        if self
            .contexts
            .get_key_value(&self.start_context_id.unwrap())
            .is_some()
        {
            match self.contexts.get_key_value(&self.start_context_id.unwrap()) {
                Some((_id, context)) => {
                    if context.model.has_id(self.start_element_id) {
                        return true;
                    }
                    return false;
                }
                None => return false,
            }
        }
        return false;
    }
}


#[cfg(test)]
mod tests {
    use io::json;

    use super::*;

    #[test]
    fn machine() {
        let mut machine = Machine::new();
        assert_eq!(machine.contexts.len(), 0);

        for model in json::read::read("tests/models/login.json").models {
            machine.contexts.insert(
                model.id.clone(),
                Context {
                    id: model.id.clone(),
                    model: model,
                    generators: Vec::new(),
                    current_element_id: None,
                },
            );
    
        }
        assert_eq!(machine.contexts.len(), 1);
        assert_eq!(machine.contexts.get("853429e2-0528-48b9-97b3-7725eafbb8b5").unwrap().id.clone(), "853429e2-0528-48b9-97b3-7725eafbb8b5");
        assert_eq!(machine.status, MachineStatus::NotStarted);


        loop {
            match machine.next() {
                Some(next_id) => {
                    println!("{}", next_id);
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