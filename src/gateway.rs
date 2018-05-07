use hyper::{
    Body, Method, Request, Response, Server, StatusCode,
    service::service_fn,
    rt::{self, Future},
    self,
};

use std::{
    sync::Arc,
    net::ToSocketAddrs,
};

use futures::{
    future,
    Stream,
};

use gelf::{
    Level,
};

use serde_json;
use config::Config;
use events::{SDKEventBatch, SDKResponse, EventResult, EventStatus};
use headers::DeviceHeaders;
use ::GLOG;

pub struct Gateway {
    config: Arc<Config>,
}

impl Gateway {
    fn service(req: Request<Body>) -> Box<Future<Item=Response<Body>, Error=hyper::Error> + Send>{
        match (req.method(), req.uri().path()) {
            (&Method::OPTIONS, "/") => {
                let _device_headers = DeviceHeaders::from(req.headers());

                // TODO: CORS
                Box::new(future::ok(Response::new("".into())))
            },
            (&Method::POST, "/") => {
                let device_headers = DeviceHeaders::from(req.headers());

                if device_headers.device_id.cleartext.is_none() {
                    let _ = GLOG.log_with_headers(
                        "Invalid Device ID",
                        Level::Error,
                        &device_headers,
                    );

                    let mut res = Response::new("Bad D360-Device-Id".into());
                    *res.status_mut() = StatusCode::BAD_REQUEST;

                    return Box::new(future::ok(res))
                }

                Box::new(req.into_body().concat2().map(move |body| {
                    if body.is_empty() {
                        let _ = GLOG.log_with_headers(
                            "Empty payload received from device",
                            Level::Error,
                            &device_headers
                        );

                        let mut res = Response::new("Empty payload".into());
                        *res.status_mut() = StatusCode::BAD_REQUEST;

                        return res
                    }

                    if let Ok(event) = serde_json::from_slice::<SDKEventBatch>(&body) {
                        let _ = GLOG.log_with_headers(
                            &format!("Received a batch of events"),
                            Level::Informational,
                            &device_headers
                        );

                        let results: Vec<EventResult> = event.events.iter().map(|e| {
                            EventResult::register(
                                &e.id,
                                EventStatus::Success,
                                &device_headers,
                            )
                        }).collect();

                        let body = serde_json::to_string(&SDKResponse::from(results)).unwrap();

                        Response::new(body.into())
                    } else {
                        let _ = GLOG.log_with_headers(
                            "Invalid JSON received",
                            Level::Error,
                            &device_headers
                        );

                        let mut res = Response::new("Invalid payload".into());
                        *res.status_mut() = StatusCode::BAD_REQUEST;

                        return res
                    }
                }))
            },
            (&Method::GET, "/watchdog") => {
                Box::new(future::ok(Response::new("".into())))
            },
            _ => {
                let mut res = Response::new(Body::empty());
                *res.status_mut() = StatusCode::NOT_FOUND;
                Box::new(future::ok(res))
            }
        }
    }

    pub fn new(config: Arc<Config>) -> Gateway {
        Gateway { config }
    }

    pub fn run(self) {
        let mut addr_iter = self.config.gateway.listen_address.to_socket_addrs().unwrap();
        let addr = addr_iter.next().unwrap();

        let server = Server::bind(&addr)
            .serve(|| service_fn(Self::service))
            .map_err(|e| println!("server error: {}", e));

        println!("Listening on http://{}", &addr);
        rt::run(server);
    }
}