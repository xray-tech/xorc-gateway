use prometheus::{
    self,
    Encoder,
    TextEncoder
};

use http::{
    header,
};

use hyper::{
    Body, Method, Request, Response, Server, StatusCode, HeaderMap,
    service::service_fn,
    rt::Future,
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
use events::{SDKEventBatch, SDKResponse, EventResult, EventStatus, Platform};
use headers::DeviceHeaders;
use tokio::runtime::{Builder as RuntimeBuilder};
use tokio_threadpool;
use cors::Cors;
use ::GLOG;

pub struct Gateway {
    config: Arc<Config>,
    cors: Arc<Cors>,
}

type ResponseFuture = Box<Future<Item=Response<Body>, Error=hyper::Error> + Send + 'static>;

impl Gateway {
    pub fn new(config: Arc<Config>) -> Gateway {
        let cors = Arc::new(Cors::from(config.clone()));

        Gateway {
            config,
            cors,
        }
    }

    pub fn run(self) {
        let mut addr_iter = self.config.gateway.address.to_socket_addrs().unwrap();
        let addr = addr_iter.next().unwrap();

        let mut threadpool_builder = tokio_threadpool::Builder::new();
        threadpool_builder
            .name_prefix(self.config.gateway.process_name_prefix.clone())
            .pool_size(4);

        let mut runtime = RuntimeBuilder::new()
            .threadpool_builder(threadpool_builder)
            .build().unwrap();

        let gateway = Arc::new(self);

        let server = Server::bind(&addr)
            .serve(move || {
                let gw = gateway.clone();
                service_fn(move |req: Request<Body>| {
                    gw.service(req)
                })
            })
            .map_err(|e| println!("server error: {}", e));

        println!("Listening on http://{}", &addr);
        runtime.spawn(server);
        runtime.shutdown_on_idle().wait().unwrap();
    }

    fn service(&self, req: Request<Body>) -> ResponseFuture {
        match (req.method(), req.uri().path()) {
            (&Method::OPTIONS, "/") => {
                self.handle_options(req)
            },
            (&Method::POST, "/") => {
                self.handle_event(req)
            },
            (&Method::GET, "/watchdog") => {
                self.handle_metrics()
            },
            _ => {
                let mut res = Response::new(Body::empty());
                *res.status_mut() = StatusCode::NOT_FOUND;
                Box::new(future::ok(res))
            }
        }
    }

    fn handle_metrics(&self) -> ResponseFuture {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = vec![];

        encoder.encode(&metric_families, &mut buffer).unwrap();

        let mut builder = Response::builder();

        builder.header(
            header::CONTENT_TYPE,
            encoder.format_type()
        );

        let response = builder.body(buffer.into()).unwrap();

        Box::new(future::ok(response))
    }

    fn handle_options(&self, req: Request<Body>) -> ResponseFuture {
        let _device_headers = DeviceHeaders::from(req.headers());

        // TODO: CORS
        Box::new(future::ok(Response::new("".into())))
    }

    fn handle_event(&self, req: Request<Body>) -> ResponseFuture {
        let device_headers = DeviceHeaders::from(req.headers());

        if device_headers.device_id.cleartext.is_none() {
            let res = Self::respond_error(
                "Bad D360-Device-Id",
                &device_headers,
                StatusCode::BAD_REQUEST,
            );

            return Box::new(future::ok(res))
        }

        let cors = self.cors.clone();

        Box::new(req.into_body().concat2().map(move |body| {
            if body.is_empty() {
                return Self::respond_error(
                    "Empty payload",
                    &device_headers,
                    StatusCode::BAD_REQUEST,
                )
            }

            if let Ok(event) = serde_json::from_slice::<SDKEventBatch>(&body) {
                let mut builder = Response::builder();

                if event.device.platform() == Platform::Web {
                    let cors_headers = device_headers.origin.as_ref()
                        .and_then(|o| {
                            cors.headers_for(&event.environment.app_id, &o)
                        });

                    if let Some(headers) = cors_headers {
                        for (k, v) in headers.into_iter() {
                            builder.header(k, v);
                        }
                    } else {
                        return Self::respond_error(
                            "Unknown Origin",
                            &device_headers,
                            StatusCode::FORBIDDEN,
                        )
                    }
                }

                let results: Vec<EventResult> = event.events.iter().map(|e| {
                    EventResult::register(
                        &e.id,
                        EventStatus::Success,
                        &device_headers,
                    )
                }).collect();

                let body = serde_json::to_string(&SDKResponse::from(results)).unwrap();

                builder.body(body.into()).unwrap()
            } else {
                Self::respond_error(
                    "Invalid payload",
                    &device_headers,
                    StatusCode::BAD_REQUEST,
                )
            }
        }))
    }

    fn respond_error(
        error: &'static str,
        device_headers: &DeviceHeaders,
        status_code: StatusCode,
    ) -> Response<Body>
    {
        let _ = GLOG.log_with_headers(
            error,
            Level::Error,
            device_headers
        );

        let mut res = Response::new(error.into());
        *res.status_mut() = status_code;

        return res
    }
}
