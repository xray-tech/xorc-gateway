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
use context::Context;
use tokio::runtime::{Builder as RuntimeBuilder};
use app_registry::AppRegistry;
use entity_storage::EntityStorage;
use cors::Cors;
use ::GLOG;

pub struct Gateway {
    config: Arc<Config>,
    cors: Arc<Option<Cors>>,
    app_registry: Arc<AppRegistry>,
    entity_storage: Arc<Option<EntityStorage>>,
}

impl Gateway {
    fn service(
        &self, 
        req: Request<Body>
    ) -> Box<Future<Item=Response<Body>, Error=GatewayError> + Send + 'static>
    {
        match (req.method(), req.uri().path()) {
            // SDK OPTIONS request
            (&Method::OPTIONS, "/") => {
                Box::new(self.handle_options())
            },
            // SDK events main path
            (&Method::POST, "/") => {
                Box::new(self.handle_sdk(req))
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

        ok(builder.body(buffer.into()).unwrap())
    }

    fn handle_options(
        &self
    ) -> impl Future<Item=Response<Body>, Error=GatewayError> + Send + 'static
    {
        let mut builder =
            if let Some(ref cors) = *self.cors {
                cors.response_builder_wildcard()
            } else {
                Response::builder()
            };

        builder.status(StatusCode::OK);
        ok(builder.body("".into()).unwrap())
    }

    fn get_context(
        event: Arc<SDKEventBatch>,
        header_map: Arc<HeaderMap>,
        entity_storage: Arc<Option<EntityStorage>>
    ) -> impl Future<Item=Context, Error=BlockingError> + 'static + Send
    {
        lazy(move || poll_fn(move || blocking(|| {
            match *entity_storage {
                Some(ref es) => {
                    let fun = || es.get_id_for_ifa(&event.environment.app_id, &event.device);

                    Context::new(
                        &*header_map,
                        &event.environment.app_id,
                        event.device.platform(),
                        fun
                    )
                },
                None => {
                    let fun = || None;

                    Context::new(
                        &*header_map,
                        &event.environment.app_id,
                        event.device.platform(),
                        fun
                    )
                }
            }
        })))
    }

    fn handle_event<'a>(
        body: Vec<u8>,
        cors: Arc<Option<Cors>>,
        app_registry: Arc<AppRegistry>,
        event: Arc<SDKEventBatch>,
        headers: Arc<HeaderMap>,
        entity_storage: Arc<Option<EntityStorage>>
    ) -> impl Future<Item=(String, Context), Error=(GatewayError, Option<Context>)> + 'static + Send
    {
        Self::get_context(event.clone(), headers.clone(), entity_storage.clone())
            .map_err(|_| {
                (GatewayError::ServiceUnavailable("Too many pending requests to Aerospike"), None)
            })
            .and_then(move |context| {
                if context.device_id.cleartext.is_none() {
                    return err((GatewayError::BadDeviceId, Some(context)))
                }

                if let Some(ref cors) = *cors {
                    if event.device.platform() == Platform::Web {
                        if !cors.valid_origin(&context.app_id, context.origin.as_ref().map(|x| &**x)) {
                            return err((GatewayError::UnknownOrigin, Some(context)))
                        }
                    }
                }

                let validation = app_registry.validate(
                    &event.environment.app_id,
                    &context,
                    &event.device.platform(),
                    &body,
                );

                match validation {
                    Ok(()) => {
                        let api_token = context.api_token.clone();
                        let ciphertext = context.device_id.ciphertext.clone();

                        let results: Vec<EventResult> = event.events.iter().map(|e| {
                            EventResult::register(
                                &e.id,
                                EventStatus::Success,
                                &api_token,
                                &ciphertext,
                            )
                        }).collect();

                        ok((
                            serde_json::to_string(&SDKResponse::from(results)).unwrap(),
                            context
                        ))
                    },
                    Err(e) => {
                        err((e, Some(context)))
                    }
                }
            })
    }

    fn handle_sdk(
        &self,
        req: Request<Body>
    ) -> impl Future<Item=Response<Body>, Error=GatewayError> + 'static + Send
    {
        let handle_event_cors = self.cors.clone();
        let ok_response_cors = self.cors.clone();
        let err_response_cors = self.cors.clone();
        let app_registry = self.app_registry.clone();
        let entity_storage = self.entity_storage.clone();
        let (head, body) = req.into_parts();
        let headers = Arc::new(head.headers);

        body
            .concat2()
            .or_else(|_| err((GatewayError::InternalServerError("body concat"), None)))
            .and_then(move |body| {
                if let Ok(event) = serde_json::from_slice::<SDKEventBatch>(&body).map(|e| Arc::new(e)) {
                    Either::A(Self::handle_event(
                        body.to_vec(),
                        handle_event_cors,
                        app_registry,
                        event,
                        headers.clone(),
                        entity_storage,
                    ))
                } else {
                    Either::B(err((GatewayError::InvalidPayload, None)))
                }
            })
            .then(move |res| {
                match res {
                    Ok((sdk_response, context)) => {
                        let json_body = serde_json::to_string(&sdk_response).unwrap();

                        let mut builder =
                            if let Some(ref cors) = *ok_response_cors {
                                cors.response_builder_origin(
                                    &context.app_id,
                                    context.origin.as_ref().map(|x| &**x),
                                    &context.platform
                                )
                            } else {
                                Response::builder()
                            };

                        builder.status(StatusCode::OK);
                        ok(builder.body(json_body.into()).unwrap())
                    },
                    Err((e, context)) => {
                        let _ = GLOG.log_error(&e, &context);
                        ok(error::into_response(e, context, &*err_response_cors))
                    },
                }
            })
    }
}
