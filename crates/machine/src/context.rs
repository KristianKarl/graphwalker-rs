use graph::Model;

#[path = "generator/random.rs"]
pub mod random;

use random::Random;

#[derive(Debug, Clone)]
pub struct Context {
    pub id: String,
    pub model: Model,
    pub generators: Vec<Random>,
}
