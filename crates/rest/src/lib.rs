// https://github.com/andrewleverette/rust_warp_api/tree/master

#[path = "handlers.rs"]
pub mod handlers;

#[path = "routes.rs"]
pub mod routes;

use machine::Machine;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type MachineRt = Arc<Mutex<Machine>>;

pub fn init_machine(machine: Machine) -> MachineRt {
    Arc::new(Mutex::new(machine))
}

#[tokio::main]
pub async fn run_rest_service(machine: Machine) {
    let m = init_machine(machine);
    let graphwalker_routes = routes::graphwalker_routes(m);

    warp::serve(graphwalker_routes)
        .run(([127, 0, 0, 1], 3000))
        .await;
}
