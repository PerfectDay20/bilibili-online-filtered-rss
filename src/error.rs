use reqwest::StatusCode;
use rss::validation::ValidationError;
use tracing::info;
use warp::body::BodyDeserializeError;
use warp::reject::MissingHeader;
use warp::{reject, Rejection, Reply};

/// Convert different crates' Error to MyError, used as warp's Rejection
#[derive(Debug)]
pub enum MyError {
    Validation(ValidationError),
    Reqwest(reqwest::Error),
    AuthNotSet,
    UnAuthorized,
}

impl reject::Reject for MyError {}

pub async fn return_error(r: Rejection) -> Result<impl Reply, Rejection> {
    info!("{:?}", r);

    if let Some(e) = r.find::<BodyDeserializeError>() {
        Ok(warp::reply::with_status(
            e.to_string(),
            StatusCode::UNPROCESSABLE_ENTITY,
        ))
    } else if let Some(MyError::AuthNotSet) = r.find::<MyError>() {
        Ok(warp::reply::with_status(
            "Update API is disabled. To enable it, set the CLI option auth_password".to_string(),
            StatusCode::FORBIDDEN,
        ))
    } else if let Some(MyError::UnAuthorized) = r.find::<MyError>() {
        Ok(warp::reply::with_status(
            "Authorization header is not equal to CLI option auth_password".to_string(),
            StatusCode::UNAUTHORIZED,
        ))
    } else if let Some(m) = r.find::<MissingHeader>() {
        Ok(warp::reply::with_status(
            m.to_string(),
            StatusCode::UNAUTHORIZED,
        ))
    } else {
        Err(warp::reject())
    }
}
