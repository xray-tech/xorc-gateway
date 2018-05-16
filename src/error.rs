use headers::DeviceHeaders;
use http::response;
use gelf::Level;
use ::GLOG;
use std::{error::Error, fmt};

use hyper::{
    Response,
    Body,
    StatusCode,
};

#[derive(Debug, PartialEq)]
pub enum GatewayError {
    AppDoesNotExist,
    InvalidToken,
    MissingToken,
    MissingSignature,
    InvalidSignature,
    InternalServerError(&'static str),
    ServiceUnavailable,
}

impl fmt::Display for GatewayError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error handling a request")
    }
}

impl Error for GatewayError {
    fn description(&self) -> &str {
        match self {
            GatewayError::AppDoesNotExist => "Application does not exist",
            GatewayError::InvalidToken => "The given SDK token was invalid",
            GatewayError::MissingToken => "No SDK token given",
            GatewayError::MissingSignature => "The signature header was missing",
            GatewayError::InvalidSignature => "The signature header was invalid",
            GatewayError::InternalServerError(reason) => reason,
            GatewayError::ServiceUnavailable => "The service is currently overloaded",
        }
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

pub fn unknown_app(
    device_headers: &DeviceHeaders,
    mut builder: response::Builder
) -> Response<Body>
{
    let _ = GLOG.log_with_headers(
        "Unknown app",
        Level::Error,
        &device_headers,
    );

    builder.status(StatusCode::FORBIDDEN);
    builder.body("Unknown app".into()).unwrap()
}

pub fn missing_token(
    device_headers: &DeviceHeaders,
    mut builder: response::Builder
) -> Response<Body>
{
    let _ = GLOG.log_with_headers(
        "Missing token",
        Level::Error,
        &device_headers,
    );

    builder.status(StatusCode::PRECONDITION_FAILED);
    builder.body("Missing D360-Api-Token".into()).unwrap()
}

pub fn missing_signature(
    device_headers: &DeviceHeaders,
    mut builder: response::Builder
) -> Response<Body>
{
    let _ = GLOG.log_with_headers(
        "Missing signature",
        Level::Error,
        &device_headers,
    );

    builder.status(StatusCode::PRECONDITION_FAILED);
    builder.body("missing signature".into()).unwrap()
}

pub fn invalid_signature(
    device_headers: &DeviceHeaders,
    mut builder: response::Builder
) -> Response<Body>
{
    let _ = GLOG.log_with_headers(
        "Invalid signature",
        Level::Error,
        &device_headers,
    );

    builder.status(StatusCode::PRECONDITION_FAILED);

    builder.body(
        "Invalid signature, check your secret key".into()
    ).unwrap()
}

pub fn invalid_token(
    device_headers: &DeviceHeaders,
    mut builder: response::Builder
) -> Response<Body>
{
    let _ = GLOG.log_with_headers(
        "Invalid token",
        Level::Error,
        &device_headers,
    );

    builder.status(StatusCode::PRECONDITION_FAILED);

    builder.body(
        "Invalid token".into()
    ).unwrap()
}

pub fn unknown_origin(
    device_headers: &DeviceHeaders,
    mut builder: response::Builder
) -> Response<Body>
{
    let _ = GLOG.log_with_headers(
        "Unknown Origin",
        Level::Error,
        &device_headers
    );
    builder.status(StatusCode::FORBIDDEN);
    builder.body("Unknown Origin".into()).unwrap()
}

pub fn bad_device_id(
    device_headers: &DeviceHeaders,
    mut builder: response::Builder
) -> Response<Body>
{
    let _ = GLOG.log_with_headers(
        "Bad D360-Device-Id",
        Level::Error,
        &device_headers
    );
    builder.status(StatusCode::BAD_REQUEST);
    builder.body("Bad D360-Device-Id".into()).unwrap()
}

pub fn internal_server_error(
    reason: &str,
) -> Response<Body>
{
    let mut builder = response::Builder::new();

    let _ = GLOG.log_without_headers(
        reason,
        Level::Error,
    );

    builder.status(StatusCode::INTERNAL_SERVER_ERROR);
    builder.body("Internal server error".into()).unwrap()
}

pub fn service_unavailable(
) -> Response<Body>
{
    let mut builder = response::Builder::new();

    let _ = GLOG.log_without_headers(
        "Service unavailable",
        Level::Error,
    );

    builder.status(StatusCode::SERVICE_UNAVAILABLE);
    builder.body("Service unavailable".into()).unwrap()
}

pub fn invalid_payload(
    mut builder: response::Builder
) -> Response<Body>
{
    let _ = GLOG.log_without_headers(
        "Invalid payload",
        Level::Error,
    );
    builder.status(StatusCode::BAD_REQUEST);
    builder.body("Empty payload".into()).unwrap()
}
