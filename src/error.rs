use rss::validation::ValidationError;
use warp::reject;

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
