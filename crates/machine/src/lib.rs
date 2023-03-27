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
    id: String,
    model: Model,
    generators: Vec<Generator>,
    current_element_id: Option<u32>,
}

#[derive(Debug)]
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
    }
}