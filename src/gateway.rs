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
    net::ToSocketAddrs,
};

use futures::{
    future::{
        self,
        ok,
        err,
        lazy,
        poll_fn,
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
    output,
};
use tokio_threadpool::{
    self,
    blocking,
};

use serde_json;
use error::{self, GatewayError};
use context::{Context, DeviceId};
use tokio::runtime::{Builder as RuntimeBuilder};
use encryption::{Cleartext, Ciphertext};

use ::{
    GLOG,
    APP_REGISTRY,
    CORS,
    CONFIG,
    ENTITY_STORAGE,
    KAFKA,
    RABBITMQ,
};

pub struct Gateway {}

type ErrorWithContext = (GatewayError, Option<Context>);

impl Gateway {
    /// ROUTES
    ///
    /// - OPTIONS to /     :: for CORS/web-push
    /// - POST to /        :: SDK Events, sent to kafka/rmq
    /// - GET to /watchdog :: Prometheus metrics
    fn service(
        req: Request<Body>
    ) -> Box<Future<Item=Response<Body>, Error=GatewayError> + Send + 'static>
    {
        match (req.method(), req.uri().path()) {
            // SDK OPTIONS request
            (&Method::OPTIONS, "/") => {
                Box::new(Self::handle_options())
            },
            // SDK events main path
            (&Method::POST, "/") => {
                Box::new(Self::handle_sdk(req))
            },
            // Prometheus metrics
            (&Method::GET, "/watchdog") => {
                Box::new(Self::handle_metrics())
            },
            _ => {
                let mut res = Response::new(Body::empty());
                *res.status_mut() = StatusCode::NOT_FOUND;
                Box::new(future::ok(res))
            }
        }
    }

    /// Run the service, keeps running until a signal is sent through rx
    pub fn run(rx: oneshot::Receiver<()>) {
        let mut addr_iter = CONFIG.gateway.address.to_socket_addrs().unwrap();
        let addr = addr_iter.next().unwrap();

        let mut threadpool_builder = tokio_threadpool::Builder::new();
        threadpool_builder
            .name_prefix(CONFIG.gateway.process_name_prefix.clone())
            .pool_size(CONFIG.gateway.threads);

        let mut runtime = RuntimeBuilder::new()
            .threadpool_builder(threadpool_builder)
            .build().unwrap();

        let server = Server::bind(&addr)
            .serve(move || {
                service_fn(move |req: Request<Body>| {
                    Self::service(req)
                })
            })
            .map_err(|e| println!("Critical server error, exiting: {}", e));

        println!(
            "Running on {} threads. Listening on http://{}",
            CONFIG.gateway.threads,
            &addr
        );

        runtime.spawn(server.select2(rx).then(move |_| Ok(())));
        runtime.shutdown_on_idle().wait().unwrap();
    }

    /// Prometheus endpoint
    fn handle_metrics(
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

    /// OPTIONS requests
    fn handle_options(
    ) -> impl Future<Item=Response<Body>, Error=GatewayError> + Send + 'static
    {
        let mut builder =
            if let Some(ref cors) = *CORS {
                cors.response_builder_wildcard()
            } else {
                Response::builder()
            };

        builder.status(StatusCode::OK);
        ok(builder.body("".into()).unwrap())
    }

    fn create_new_device(
        context: Context,
        event: SDKEventBatch,
        event_id: String,
    ) -> impl Future<Item=(Vec<EventResult>, Context, SDKEventBatch), Error=GatewayError>
    {
        let ciphertext = match context.device_id {
            Some(ref device_id) => {
                let ciphertext = device_id.ciphertext.clone();

                Either::A(ok(ciphertext))
            },
            _ => {
                let app_id = context.app_id.clone();
                let ifa = event.device.ifa.clone();
                let tracking_enabled = event.device.ifa_tracking_enabled;

                let get_id = lazy(move || poll_fn(move || blocking(|| {
                    let device_id = ENTITY_STORAGE.with(|s| {
                        s.get_id_for_ifa(&app_id, &ifa, tracking_enabled)
                    }).map(move |device_id| {
                        let cleartext = Cleartext::from(device_id);
                        let ciphertext = Ciphertext::encrypt(&cleartext);

                        DeviceId { cleartext, ciphertext }
                    }).unwrap_or_else(|| {
                        DeviceId::generate()
                    });

                    let _ = ENTITY_STORAGE.with(|s| {
                        s.put_id_for_ifa(
                            &app_id,
                            &device_id.cleartext,
                            &ifa,
                            tracking_enabled
                        )
                    });

                    device_id.ciphertext
                })));

                Either::B(get_id)
            }
        };

        ciphertext.map(|ciphertext: Ciphertext| {
            let results = vec![EventResult::register(
                event_id,
                EventStatus::Success,
                context.api_token.clone(),
                ciphertext,
            )];

            (results, context, event)
        }).map_err(|_| GatewayError::ServiceUnavailable("Aerospike is acting slow today"))
    }

    fn generate_event_results(
        context: Context,
        event: SDKEventBatch,
    ) -> impl Future<Item=(Vec<EventResult>, Context, SDKEventBatch), Error=GatewayError>
    {
        if let Some(event_id) = event.events.iter().find(|ref e| e.is_register()).map(|e| e.id.clone()) {
            Either::A(Self::create_new_device(context, event, event_id))
        } else {
            let results = event.events.iter().map(|e| {
                EventResult::new(
                    e.id.clone(),
                    EventStatus::Success,
                )
            }).collect();

            Either::B(ok((results, context, event)))
        }
    }

    /// SDK event handling is here
    fn handle_event(
        body: Vec<u8>,
        mut event: SDKEventBatch,
        headers: HeaderMap,
    ) -> impl Future<Item=(String, Context), Error=ErrorWithContext> + 'static + Send
    {
        let context = Context::new(
            &headers,
            &event.environment.app_id,
            event.device.platform(),
        );

        if let Some(ref cors) = *CORS {
            if event.device.platform() == Platform::Web {
                let app_id = &event.environment.app_id;
                let origin = headers.get(header::ORIGIN).and_then(|h| h.to_str().ok());

                if !cors.valid_origin(app_id, origin) {
                    return Either::B(err((GatewayError::UnknownOrigin, Some(context))))
                }
            }
        };

        match APP_REGISTRY.validate(&event, &context, &body) {
            Ok(()) => {
                if let Some(ref ip) = context.ip { event.device.set_ip_and_country(ip) }

                let response = Self::generate_event_results(context, event)
                    .map_err(|e| (e, None))
                    .and_then(move |(results, context, event)| {
                        let proto_event: output::events::SdkEventBatch =
                            event.into();

                        let kafka = KAFKA.with(|k| {
                            k.publish(&proto_event, &context)
                        });

                        let rabbitmq = RABBITMQ.with(|r| {
                            r.publish(&proto_event, &context)
                        });

                        kafka.join(rabbitmq)
                            .or_else(|e| { err((e, None)) })
                            .map(move |_| {
                                (
                                    serde_json::to_string(&SDKResponse::from(results)).unwrap(),
                                    context
                                )
                            })
                    });

                Either::A(response)
            },
            Err(e) => {
                Either::B(err((e, Some(context))))
            }
        }
    }

    /// The request level SDK event handling
    fn handle_sdk(
        req: Request<Body>
    ) -> impl Future<Item=Response<Body>, Error=GatewayError> + 'static + Send
    {
        let (head, body) = req.into_parts();
        let headers = head.headers;

        body
            .concat2()
            .or_else(|_| err((GatewayError::InternalServerError("body concat"), None)))
            .and_then(move |body| {
                if let Ok(event) = serde_json::from_slice::<SDKEventBatch>(&body) {
                    let event_handling =
                        Self::handle_event(
                            body.to_vec(),
                            event,
                            headers,
                        );

                    Either::A(event_handling)
                } else {
                    Either::B(err((GatewayError::InvalidPayload, None)))
                }
            })
            .then(move |res| {
                match res {
                    Ok((sdk_response, context)) => {
                        let json_body = serde_json::to_string(&sdk_response).unwrap();

                        let mut builder =
                            if let Some(ref cors) = *CORS {
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
                        ok(error::into_response(e, context))
                    },
                }
            })
    }
}
