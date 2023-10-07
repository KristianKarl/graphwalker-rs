// https://github.com/andrewleverette/rust_warp_api/tree/master

mod routes;
mod handlers;

use std::sync::Arc;
use tokio::sync::Mutex;
use machine::Machine;

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

#[cfg(test)]
mod tests {
    use super::*;

    // 1.
    #[tokio::test]
    async fn get_next() {
        let m = init_machine(Machine::new());
        let graphwalker_routes = routes::graphwalker_routes(m);

        let res = warp::test::request() // 2.
            .method("GET")
            .path("/hasNext")
            .reply(&graphwalker_routes)
            .await;

        assert_eq!(res.status(), 400, "Should return 400 Bad request, since the machine is not loaded with anything meaningful");
    }
}