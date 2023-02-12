use rss::validation::ValidationError;
use warp::{reject, Rejection, Reply};
use tracing::info;
use warp::body::BodyDeserializeError;
use reqwest::StatusCode;

/// Convert different crates' Error to MyError, used as warp's Rejection
#[derive(Debug)]
pub enum MyError {
    Validation(ValidationError),
    Reqwest(reqwest::Error),
}

impl reject::Reject for MyError{}

pub async fn return_error(r: Rejection) -> Result<impl Reply, Rejection> {
    info!("{:?}", r);
    if let Some(e) = r.find::<BodyDeserializeError>() {
        Ok(warp::reply::with_status(e.to_string(), StatusCode::UNPROCESSABLE_ENTITY))
    } else {
        Err(warp::reject())
    }
}
