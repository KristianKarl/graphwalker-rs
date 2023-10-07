use std::convert::Infallible;
use warp::{self, http::StatusCode};

use crate::MachineRt;

pub async fn has_next(machine: MachineRt) -> Result<Box<dyn warp::Reply>, Infallible> {
    let mut m = machine.lock().await;
    match m.has_next() {
        Some(has_next) => {
            return Ok(Box::new(warp::reply::json(&has_next)));
        }
        None => {}
    }
    Ok(Box::new(StatusCode::BAD_REQUEST))
}

pub async fn get_next(machine: MachineRt) -> Result<Box<dyn warp::Reply>, Infallible> {
    let mut m = machine.lock().await;
    if let Ok(next_pos) = m.next_step() {
        match next_pos {
            Some(position) => {
                return Ok(Box::new(warp::reply::json(&position)));
            }
            None => {}
        }
    }
    Ok(Box::new(StatusCode::BAD_REQUEST))
}
