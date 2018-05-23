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
        Either
    },
    Future,
    Stream,
    sync::oneshot,
};

use events::{
    input::{
        SDKDevice,
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

impl Gateway {
    /// ROUTES
    ///
    /// Define all the endpoints here.
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
            .pool_size(4);

        let mut runtime = RuntimeBuilder::new()
            .threadpool_builder(threadpool_builder)
            .build().unwrap();

        let server = Server::bind(&addr)
            .serve(move || {
                service_fn(move |req: Request<Body>| {
                    Self::service(req)
                })
            })
            .map_err(|e| println!("server error: {}", e));

        println!("Listening on http://{}", &addr);
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
        context: &Context,
        app_id: &str,
        device: &SDKDevice,
        event_id: &str
    ) -> EventResult
    {
        let ciphertext: Ciphertext = match context.device_id {
            Some(ref device_id) => {
                device_id.ciphertext.clone()
            },
            _ => {
                match &*ENTITY_STORAGE {
                    Some(ref storage) => {
                        storage.get_id_for_ifa(&app_id, &device)
                            .map(|device_id| {
                                let cleartext = Cleartext::from(device_id);
                                Ciphertext::encrypt(&cleartext)
                            })
                            .unwrap_or_else(|| {
                                DeviceId::generate().ciphertext
                            })
                    },
                    _ => {
                        DeviceId::generate().ciphertext
                    }
                }
            }
        };

        EventResult::register(
            event_id.to_string(),
            EventStatus::Success,
            context.api_token.clone(),
            ciphertext,
        )
    }

    /// SDK event handling is here
    fn handle_event(
        body: Vec<u8>,
        mut event: SDKEventBatch,
        headers: HeaderMap,
    ) -> impl Future<Item=(String, Context), Error=(GatewayError, Option<Context>)> + 'static + Send
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

        let validation = APP_REGISTRY.validate(
            &event.environment.app_id,
            &context,
            &event.device.platform(),
            &body,
        );

        match validation {
            Ok(()) => {
                if let Some(ref ip) = context.ip { event.device.set_ip_and_country(ip) }

                let results: Vec<EventResult> = event.events.iter().map(|e| {
                    if e.is_register() {
                        Self::create_new_device(
                            &context,
                            &event.environment.app_id,
                            &event.device,
                            &e.id,
                        )
                    } else {
                        EventResult::new(
                            e.id.clone(),
                            EventStatus::Success,
                        )
                    }
                }).collect();

                // TODO: SORT EVENTS
                let mut proto_event: output::events::SdkEventBatch =
                    event.into();

                if proto_event.event.len() == 0 {
                    warn!("Received a request without any events in it!");

                    Either::B(err((
                        GatewayError::InvalidPayload,
                        Some(context)
                    )))
                } else {
                    let kafka = KAFKA.publish(&proto_event, &context);
                    let rabbitmq = RABBITMQ.publish(&proto_event, &context);

                    let response = kafka.join(rabbitmq)
                        .or_else(|e| { err((e, None)) })
                        .map(move |_| {
                            (
                                serde_json::to_string(&SDKResponse::from(results)).unwrap(),
                                context
                            )
                        });

                    Either::A(response)
                }
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
                    Either::A(Self::handle_event(
                        body.to_vec(),
                        event,
                        headers,
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
