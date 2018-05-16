use headers::DeviceHeaders;
use http::response;
use gelf::Level;
use ::GLOG;

use hyper::{
    Response,
    Body,
    StatusCode,
};

#[derive(Debug, PartialEq)]
pub enum Error {
    AppDoesNotExist,
    InvalidToken,
    MissingToken,
    MissingSignature,
    InvalidSignature,
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
