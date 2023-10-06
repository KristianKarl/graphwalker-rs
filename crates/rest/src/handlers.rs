use std::convert::Infallible;
use warp::{self, http::StatusCode};

use machine::Machine;

pub async fn has_next(mut machine: Machine) -> Result<Box<dyn warp::Reply>, Infallible> {
    match machine.has_next() {
        Some(has_next) => {
            return Ok(Box::new(warp::reply::json(&has_next)));
        }
        None => {}
    }
    Ok(Box::new(StatusCode::BAD_REQUEST))
}
