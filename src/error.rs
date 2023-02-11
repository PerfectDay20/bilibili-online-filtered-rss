use rss::validation::ValidationError;
use warp::{reject, Rejection, Reply};
use tracing::info;
use warp::body::BodyDeserializeError;
use reqwest::StatusCode;

#[derive(Debug)]
pub struct InternalError {
    pub reason: String,
}

impl InternalError {
    pub fn new(reason: String) -> Self {
        Self { reason }
    }
}

impl reject::Reject for InternalError {}

impl From<ValidationError> for InternalError {
    fn from(e: ValidationError) -> Self {
        InternalError::new(e.to_string())
    }
}

impl From<reqwest::Error> for InternalError {
    fn from(e: reqwest::Error) -> Self {
        InternalError::new(e.to_string())
    }
}

pub async fn return_error(r: Rejection) -> Result<impl Reply, Rejection> {
    info!("{:?}", r);
    if let Some(e) = r.find::<BodyDeserializeError>() {
        Ok(warp::reply::with_status(e.to_string(), StatusCode::UNPROCESSABLE_ENTITY))
    } else {
        Err(warp::reject())
    }
}
