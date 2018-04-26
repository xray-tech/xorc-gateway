#[macro_use] extern crate serde_derive;

extern crate hyper;
extern crate pretty_env_logger;
extern crate ring;
extern crate serde;
extern crate serde_json;
extern crate protobuf;
extern crate chrono;

mod events;
mod proto_events;

use hyper::{Body, Method, Request, Response, Server, StatusCode};
use hyper::service::service_fn_ok;
use hyper::rt::Future;

fn sdk_gateway(req: Request<Body>) -> Response<Body> {
    match (req.method(), req.uri().path()) {
        (&Method::OPTIONS, "/") => {
            Response::new("".into())
        },
        (&Method::POST, "/") => {
            Response::new(req.into_body())
        },
        (&Method::GET, "/watchdog") => {
            Response::new("".into())
        },
        _ => {
            let mut res = Response::new(Body::empty());
            *res.status_mut() = StatusCode::NOT_FOUND;
            res
        }
    }
}

fn main() {
    pretty_env_logger::init();

    let addr = ([0, 0, 0, 0], 1337).into();

    let server = Server::bind(&addr)
        .serve(|| service_fn_ok(sdk_gateway))
        .map_err(|e| eprintln!("server error: {}", e));

    println!("Listening on http://{}", addr);
    hyper::rt::run(server);
}
