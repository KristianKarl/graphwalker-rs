use std::convert::Infallible;
use warp::{self, Filter};

use crate::handlers;

use machine::Machine;

pub fn graphwalker_routes(
    machine: Machine,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    has_next(machine.clone())
}

fn has_next(
    machine: Machine,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("hasNext")
        .and(warp::get())
        .and(with_machine(machine))
        .and_then(handlers::has_next)
}

fn with_machine(machine: Machine) -> impl Filter<Extract = (Machine,), Error = Infallible> + Clone {
    warp::any().map(move || machine.clone())
}
