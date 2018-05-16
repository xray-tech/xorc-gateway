use prometheus::{
    self,
    Encoder,
    TextEncoder
};

use http::{header, HeaderMap};

use hyper::{
    Body, Method, Request, Response, Server, StatusCode,
    service::service_fn,
    self,
};

use std::{
    sync::Arc,
    net::ToSocketAddrs,
};

use futures::{
    future::self,
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
};

use serde_json;
use config::Config;
use error::{self, Error};
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

type ResponseFuture = Box<Future<Item=Response<Body>, Error=hyper::Error> + Send + 'static>;

impl Gateway {
    fn service(&self, req: Request<Body>) -> ResponseFuture {
        match (req.method(), req.uri().path()) {
            // SDK OPTIONS request
            (&Method::OPTIONS, "/") => {
                Box::new(future::ok(self.handle_options()))
            },
            // SDK events main path
            (&Method::POST, "/") => {
                Box::new(self.handle_event(req))
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

    fn handle_metrics(&self) -> impl Future<Item=Response<Body>, Error=hyper::Error> {
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
        event: &SDKEventBatch,
        header_map: &HeaderMap,
        entity_storage: Arc<Option<EntityStorage>>,
    ) -> DeviceHeaders
    {
        match *entity_storage {
            Some(ref entity_storage) => {
                DeviceHeaders::new(
                    header_map,
                    || entity_storage.get_id_for_ifa(
                        &event.environment.app_id,
                        &event.device,
                    )
                )
            },
            _ => {
                DeviceHeaders::new(
                    header_map,
                    || None,
                )
            }
        }
    }

    fn handle_event(
        &self,
        req: Request<Body>
    ) -> impl Future<Item=Response<Body>, Error=hyper::Error>
    {
        let mut builder = Response::builder();
        let cors = self.cors.clone();
        let app_registry = self.app_registry.clone();
        let entity_storage = self.entity_storage.clone();
        let (head, body) = req.into_parts();

        body.concat2().map(move |ref body| {
            if let Ok(event) = serde_json::from_slice::<SDKEventBatch>(&body).map(|e| Arc::new(e)) {
                let device_headers = Self::get_device_headers(&event, &head.headers, entity_storage);

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
                    Err(Error::AppDoesNotExist)  => error::unknown_app(&device_headers, builder),
                    Err(Error::MissingToken)     => error::missing_token(&device_headers, builder),
                    Err(Error::MissingSignature) => error::missing_signature(&device_headers, builder),
                    Err(Error::InvalidSignature) => error::invalid_signature(&device_headers, builder),
                    Err(Error::InvalidToken)     => error::invalid_token(&device_headers, builder),
                    Ok(()) => {
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
            } else {
                error::invalid_payload(builder)
            }
        })
    }
}
