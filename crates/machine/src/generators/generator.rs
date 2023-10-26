use std::sync::{Arc, Mutex};

use evalexpr::eval_with_context_mut;
use graph::Edge;

use crate::{stop_condition::StopCondition, Context, Machine, Position, SharedState};

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
        context: Arc<Mutex<Context>>,
        shared_states: &Vec<SharedState>,
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
    stop_conditions: Vec<Arc<dyn StopCondition>>,
}

impl Generator for RandomGenerator {
    fn kind(&self) -> GeneratorKind {
        GeneratorKind::RandomGenerator
    }

    fn get_next_edge(
        &self,
        ctx: Arc<Mutex<Context>>,
        shared_states: &Vec<SharedState>,
        current_pos: Position,
    ) -> Result<Position, String> {
        if let Some(vertex) = ctx
            .lock()
            .unwrap()
            .model
            .vertices
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

            for e in ctx
                .lock()
                .unwrap()
                .model
                .out_edges(current_pos.element_id.clone())
            {
                let pos = Position {
                    model_id: current_pos.model_id.clone(),
                    element_id: e.id.clone(),
                };
                if self.is_selectable(ctx.clone(), &e) {
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
    fn is_selectable(&self, ctx: Arc<Mutex<Context>>, e: &Edge) -> bool {
        if let Some(guard) = e.guard.clone() {
            log::debug!("Edge has guard: {:?}", guard);

            match eval_with_context_mut(&guard, &mut ctx.lock().unwrap().eval_context) {
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
        random.stop_conditions.push(Arc::new(edge_coverage));

        assert_eq!(random.is_fullfilled(), true, "Should be false");
        assert_eq!(
            random.kind(),
            GeneratorKind::RandomGenerator,
            "Incorrect condition type found"
        );
    }
}
