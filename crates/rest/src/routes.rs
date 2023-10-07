use std::convert::Infallible;
use warp::{self, Filter};

use crate::handlers;
use crate::MachineRt;

pub fn graphwalker_routes(
    machine: MachineRt,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        has_next(machine.clone())
        .or(get_next(machine))
}

fn has_next(
    machine: MachineRt,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("hasNext")
        .and(warp::get())
        .and(with_machine(machine))
        .and_then(handlers::has_next)
}

fn get_next(
    machine: MachineRt,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("getNext")
        .and(warp::get())
        .and(with_machine(machine))
        .and_then(handlers::get_next)
}

fn with_machine(machine: MachineRt) -> impl Filter<Extract = (MachineRt,), Error = Infallible> + Clone {
    warp::any().map(move || machine.clone())
}
