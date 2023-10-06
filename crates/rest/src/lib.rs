// https://github.com/andrewleverette/rust_warp_api/tree/master

mod routes;
mod handlers;

use machine::Machine;

#[tokio::main]
pub async fn run_rest_service(machine: Machine) {
    let graphwalker_routes = routes::graphwalker_routes(machine);

    warp::serve(graphwalker_routes)
        .run(([127, 0, 0, 1], 3000))
        .await;
}