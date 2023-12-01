use std::{cell::RefCell, rc::Rc};

use evalexpr::eval_with_context_mut;
use graph::Edge;

use crate::{stop_condition::StopCondition, Context, Position, SharedState};

#[derive(Debug, PartialEq)]
pub enum GeneratorKind {
    RandomGenerator,
}

impl core::fmt::Display for GeneratorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match *self {
            GeneratorKind::RandomGenerator => write!(f, "RandomGenerator"),
        }
    }
}

pub trait Generator {
    fn kind(&self) -> GeneratorKind;
    fn get_next_edge(
        &self,
        shared_states: &[SharedState],
        current_pos: Position,
    ) -> Result<Position, String>;
    fn is_fullfilled(&self) -> bool;
}

// https://users.rust-lang.org/t/derive-debug-not-playing-well-with-dyn/52398
impl<'a> core::fmt::Debug for dyn Generator + 'a {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.kind())
    }
}

#[derive(Debug, Default)]
pub struct RandomGenerator {
    stop_conditions: Vec<Rc<dyn StopCondition>>,
    context: Rc<RefCell<Context>>,
}

impl Generator for RandomGenerator {
    fn kind(&self) -> GeneratorKind {
        GeneratorKind::RandomGenerator
    }

    fn get_next_edge(
        &self,
        shared_states: &[SharedState],
        current_pos: Position,
    ) -> Result<Position, String> {
        let c = self.context.borrow_mut();
        if let Some(vertex) = c
            .model
            .vertices
            .read()
            .unwrap()
            .get(&current_pos.element_id)
        {
            // Build a list of candidates of edges to select
            // Look for shared_states
            let mut candidates: Vec<Position> = Vec::new();
            if let Some(name) = vertex.shared_state.clone() {
                for shared_state in shared_states {
                    if shared_state.name == name {
                        candidates.clone_from(&shared_state.positions);
                        break;
                    }
                }

                // Remove the current vertex from the candidate list, since we are already at it.
                let index = candidates
                    .iter()
                    .position(|x| *x == current_pos.clone())
                    .unwrap();
                candidates.remove(index);
            }

            for e in c.model.out_edges(&current_pos.element_id) {
                let pos = Position {
                    model_id: current_pos.model_id.clone(),
                    element_id: e.id.clone(),
                };
                if self.is_selectable(&e) {
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
            return Ok(candidates[random_index].clone());
        }

        // If reached this code, there is something fishy going on
        let msg = format!(
            "Could not find vertex nor edge matching the current position: {:?}",
            current_pos
        );
        log::warn!("{}", msg);
        Err(msg)
    }

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
    pub fn new(_ctx: Rc<RefCell<Context>>) -> Self {
        Self {
            context: Rc::default(),
            stop_conditions: Vec::default(),
        }
    }

    fn is_selectable(&self, e: &Edge) -> bool {
        if let Some(guard) = e.guard.clone() {
            log::debug!("Edge has guard: {:?}", guard);

            let mut c = self.context.borrow_mut();
            match eval_with_context_mut(&guard, &mut c.eval_context) {
                Ok(value) => match value.as_boolean() {
                    Ok(res) => {
                        log::debug!("The guard evaluated to: {:?}", res);
                        return res;
                    }
                    Err(err) => {
                        let msg =
                            format!("Evaluating guard {:?}, failed with error: {:?}", guard, err);
                        log::error!("{}", msg);
                        return true;
                    }
                },
                Err(err) => {
                    let msg = format!("Evaluating guard {:?}, failed with error: {:?}", guard, err);
                    log::error!("{}", msg);
                    return true;
                }
            }
        }
        log::trace!("No guard");
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    use crate::stop_condition::EdgeCoverage;

    #[test]
    fn random() {
        let mut random = RandomGenerator::default();

        let edge_coverage = EdgeCoverage::new(0f32);
        random.stop_conditions.push(Rc::new(edge_coverage));

        assert_eq!(random.is_fullfilled(), true, "Should be false");
        assert_eq!(
            random.kind(),
            GeneratorKind::RandomGenerator,
            "Incorrect condition type found"
        );
    }
}
