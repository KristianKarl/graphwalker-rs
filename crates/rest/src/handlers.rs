use machine::MachineStatus;
use std::convert::Infallible;
use warp::{self, http::StatusCode};

use crate::MachineRt;

pub async fn has_next(machine: MachineRt) -> Result<Box<dyn warp::Reply>, Infallible> {
    let m = machine.lock().await;

    if !m.is_all_fullfilled() && m.status == MachineStatus::Running {
        log::debug!("hasNext: {:?}", true);
        return Ok(Box::new(warp::reply::json(&true)));
    }

    log::debug!("hasNext: {:?}", false);
    Ok(Box::new(warp::reply::json(&false)))
}

pub async fn get_next(machine: MachineRt) -> Result<Box<dyn warp::Reply>, Infallible> {
    let mut m = machine.lock().await;

    let result = m.step();
    match result {
        Ok(step) => {
            log::debug!("getNext: {:?}", step);
            Ok(Box::new(warp::reply::json(&step)))
        }

        Err(err) => {
            log::error!("getNext: {:?}", err);
            Ok(Box::new(StatusCode::BAD_REQUEST))
        }
    }
}
