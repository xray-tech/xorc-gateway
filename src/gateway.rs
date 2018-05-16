use prometheus::{
    self,
    Encoder,
    TextEncoder
};

use http::{header, HeaderMap};

use hyper::{
    Body, Method, Request, Response, Server, StatusCode,
    service::service_fn,
};

use std::{
    sync::Arc,
    net::ToSocketAddrs,
};

use futures::{
    future::{
        self,
        lazy,
        poll_fn,
        ok,
        err,
        Either
    },
    Future,
    Stream,
    sync::oneshot,
};

use events::{
    input::{
        SDKEventBatch,
        SDKResponse,
        EventResult,
        EventStatus,
        Platform,
    },
};
use tokio_threadpool::{
    self,
    blocking,
    BlockingError,
};

use serde_json;
use config::Config;
use error::{self, GatewayError};
use headers::DeviceHeaders;
use tokio::runtime::{Builder as RuntimeBuilder};
use app_registry::AppRegistry;
use entity_storage::EntityStorage;
use cors::Cors;

pub struct Gateway {
    config: Arc<Config>,
    cors: Arc<Option<Cors>>,
    app_registry: Arc<AppRegistry>,
    entity_storage: Arc<Option<EntityStorage>>,
}

type ResponseFuture = Box<Future<Item=Response<Body>, Error=GatewayError> + Send + 'static>;

impl Gateway {
    fn service(&self, req: Request<Body>) -> ResponseFuture {
        match (req.method(), req.uri().path()) {
            // SDK OPTIONS request
            (&Method::OPTIONS, "/") => {
                Box::new(future::ok(self.handle_options()))
            },
            // SDK events main path
            (&Method::POST, "/") => {
                self.handle_sdk(req)
            },
            // Prometheus metrics
            (&Method::GET, "/watchdog") => {
                Box::new(self.handle_metrics())
            },
            _ => {
                let mut res = Response::new(Body::empty());
                *res.status_mut() = StatusCode::NOT_FOUND;
                Box::new(future::ok(res))
            }
        }
    }

    pub fn new(
        config: Arc<Config>,
        app_registry: Arc<AppRegistry>,
        entity_storage: Arc<Option<EntityStorage>>,
    ) -> Gateway
    {
        let cors = Arc::new(Cors::new(config.clone()));

        Gateway {
            config,
            cors,
            app_registry,
            entity_storage,
        }
    }

    pub fn run(self, rx: oneshot::Receiver<()>) {
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
        runtime.spawn(server.select2(rx).then(move |_| Ok(())));
        runtime.shutdown_on_idle().wait().unwrap();
    }

    fn handle_metrics(
        &self,
    ) -> impl Future<Item=Response<Body>, Error=GatewayError> + 'static + Send
    {
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

        future::ok(response)
    }

    fn handle_options(&self) -> Response<Body> {
        let mut builder = Response::builder();
        builder.status(StatusCode::OK);

        if let Some(ref cors) = *self.cors {
            for (k, v) in cors.wildcard_headers().into_iter() {
                builder.header(k, v);
            }
        }

        builder.body("".into()).unwrap()
    }

    fn get_device_headers(
        event: Arc<SDKEventBatch>,
        header_map: Arc<HeaderMap>,
        entity_storage: Arc<Option<EntityStorage>>
    ) -> impl Future<Item=DeviceHeaders, Error=BlockingError> + 'static + Send
    {
        lazy(move || poll_fn(move || blocking(|| {
            match *entity_storage {
                Some(ref entity_storage) => {
                    DeviceHeaders::new(
                        &*header_map,
                        || entity_storage.get_id_for_ifa(
                            &event.environment.app_id,
                            &event.device,
                        )
                    )
                },
                _ => {
                    DeviceHeaders::new(
                        &*header_map,
                        || None,
                    )
                }
            }
        })))
    }

    fn handle_event(
        body: Vec<u8>,
        cors: Arc<Option<Cors>>,
        app_registry: Arc<AppRegistry>,
        event: Arc<SDKEventBatch>,
        headers: Arc<HeaderMap>,
        entity_storage: Arc<Option<EntityStorage>>
    ) -> impl Future<Item=Response<Body>, Error=GatewayError> + 'static + Send
    {
        let mut builder = Response::builder();

        Self::get_device_headers(event.clone(), headers.clone(), entity_storage.clone())
            .map_err(|_| GatewayError::ServiceUnavailable)
            .map(move |device_headers| {
                if device_headers.device_id.cleartext.is_none() {
                    return error::bad_device_id(&device_headers, builder)
                }

                if let Some(ref cors) = *cors {
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
                            return error::unknown_origin(&device_headers, builder)
                        }
                    }
                }

                let validation = app_registry.validate(
                    &event.environment.app_id,
                    &device_headers,
                    &event.device.platform(),
                    &body,
                );

                match validation
                {
                    Err(GatewayError::AppDoesNotExist)  => error::unknown_app(&device_headers, builder),
                    Err(GatewayError::MissingToken)     => error::missing_token(&device_headers, builder),
                    Err(GatewayError::MissingSignature) => error::missing_signature(&device_headers, builder),
                    Err(GatewayError::InvalidSignature) => error::invalid_signature(&device_headers, builder),
                    Err(GatewayError::InvalidToken)     => error::invalid_token(&device_headers, builder),
                    _ => {
                        let results: Vec<EventResult> = event.events.iter().map(|e| {
                            EventResult::register(
                                &e.id,
                                EventStatus::Success,
                                &device_headers,
                            )
                        }).collect();

                        let body = serde_json::to_string(&SDKResponse::from(results)).unwrap();

                        builder.body(body.into()).unwrap()
                    },
                }
            })
    }

    fn handle_sdk(
        &self,
        req: Request<Body>
    ) -> ResponseFuture
    {
        let cors = self.cors.clone();
        let app_registry = self.app_registry.clone();
        let entity_storage = self.entity_storage.clone();
        let (head, body) = req.into_parts();
        let headers = Arc::new(head.headers);

        let handling = body
            .concat2()
            .map_err(|_| GatewayError::InternalServerError("body concat"))
            .and_then(move |body| {
                if let Ok(event) = serde_json::from_slice::<SDKEventBatch>(&body).map(|e| Arc::new(e)) {
                    Either::A(Self::handle_event(
                        body.to_vec(),
                        cors,
                        app_registry,
                        event,
                        headers,
                        entity_storage,
                    ))
                } else {
                    Either::B(ok(error::invalid_payload(Response::builder())))
                }
            })
            .then(|res| {
                match res {
                    Err(GatewayError::InternalServerError(reason)) => {
                        ok(error::internal_server_error(reason))
                    },
                    Err(GatewayError::ServiceUnavailable) => {
                        ok(error::service_unavailable())
                    },
                    Err(e) => {
                        err(e)
                    },
                    Ok(res) => ok(res)
                }
            });

        Box::new(handling)
    }
}
