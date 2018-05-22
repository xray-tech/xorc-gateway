use context::Context;
use http::response;
use std::{error::Error, fmt};

use hyper::{
    Response,
    Body,
    StatusCode,
};

use ::CORS;

#[derive(Debug, PartialEq)]
pub enum GatewayError {
    AppDoesNotExist,
    InvalidToken,
    MissingToken,
    MissingSignature,
    InvalidSignature,
    UnknownOrigin,
    BadDeviceId,
    InvalidPayload,
    InternalServerError(&'static str),
    ServiceUnavailable(&'static str),
}

impl fmt::Display for GatewayError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error handling a request")
    }
}

impl Error for GatewayError {
    fn description(&self) -> &str {
        match self {
            GatewayError::AppDoesNotExist =>
                "Application does not exist",
            GatewayError::InvalidToken =>
                "The given SDK token was invalid",
            GatewayError::MissingToken =>
                "No SDK token given",
            GatewayError::MissingSignature =>
                "The signature header was missing",
            GatewayError::InvalidSignature =>
                "The signature header was invalid",
            GatewayError::UnknownOrigin =>
                "The ORIGIN didn't match to the CORS configuration",
            GatewayError::BadDeviceId =>
                "There is something fishy in the device id encryption",
            GatewayError::InvalidPayload =>
                "The request JSON was faulty",
            GatewayError::InternalServerError(reason) =>
                reason,
            GatewayError::ServiceUnavailable(reason) =>
                reason,
        }
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

fn response_builder_for(context: &Option<Context>) -> response::Builder {
    if let Some(ref cors) = *CORS {
        if let Some(context) = context {
            return cors.response_builder_origin(
                &context.app_id,
                context.origin.as_ref().map(|s| &**s),
                &context.platform
            )
        }
    }

    Response::builder()
}

pub fn into_response(
    error: GatewayError,
    context: Option<Context>,
) -> Response<Body> {
    let mut builder = response_builder_for(&context);

    match error {
        GatewayError::AppDoesNotExist => {
            builder.status(StatusCode::FORBIDDEN);
            builder.body("Unknown app".into()).unwrap()
        },
        GatewayError::InvalidToken => {
            builder.status(StatusCode::PRECONDITION_FAILED);
            builder.body("Invalid D360-Api-Token".into()).unwrap()
        },
        GatewayError::MissingToken => {
            builder.status(StatusCode::PRECONDITION_FAILED);
            builder.body("Missing D360-Api-Token".into()).unwrap()
        },
        GatewayError::MissingSignature => {
            builder.status(StatusCode::PRECONDITION_FAILED);
            builder.body("Missing D360-Signature".into()).unwrap()
        },
        GatewayError::InvalidSignature => {
            builder.status(StatusCode::PRECONDITION_FAILED);
            builder.body("Invalid D360-Signature".into()).unwrap()
        },
        GatewayError::UnknownOrigin => {
            builder.status(StatusCode::FORBIDDEN);
            builder.body("Unknown Origin".into()).unwrap()
        },
        GatewayError::BadDeviceId => {
            builder.status(StatusCode::BAD_REQUEST);
            builder.body("Bad D360-Device-Id".into()).unwrap()
        },
        GatewayError::InvalidPayload => {
            builder.status(StatusCode::BAD_REQUEST);
            builder.body("Invalid payload".into()).unwrap()
        },
        GatewayError::InternalServerError(_) => {
            builder.status(StatusCode::INTERNAL_SERVER_ERROR);
            builder.body("Invalid Server Error".into()).unwrap()
        },
        GatewayError::ServiceUnavailable(_) => {
            builder.status(StatusCode::SERVICE_UNAVAILABLE);
            builder.body("Service unavailable piri kulli kokaiini".into()).unwrap()
        },
    }
}
