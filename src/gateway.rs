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
    sync::Arc,
    error::Error,
    env,
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

use tokio_threadpool::blocking;
use bus;
use serde_json;
use error::{self, GatewayError};
use context::{Context, DeviceId};
use tokio::runtime::{Builder as RuntimeBuilder};
use encryption::{Cleartext, Ciphertext};
use prost::Message;
use metrics::*;

use ::{
    GLOG,
    APP_REGISTRY,
    CORS,
    CONFIG,
    IFA_MATCHING,
};

struct BusConnections {
    pub kafka: bus::Kafka,
}

pub struct Gateway {
    connections: Arc<BusConnections>,
}

impl Clone for Gateway {
    fn clone(&self) -> Gateway {
        Gateway {
            connections: self.connections.clone(),
        }
    }
}

impl Gateway {
    fn new() -> Gateway {
        let connections = Arc::new(BusConnections {
            kafka: bus::Kafka::new(),
        });

        Gateway { connections }
    }

    /// ROUTES
    ///
    /// - OPTIONS to /xray/events/xorc/sdk/v1 :: for CORS/web-push
    /// - POST to /xray/events/xorc/sdk/v1    :: SDK Events, sent to kafka/rmq
    /// - GET to /metrics                          :: Prometheus metrics
    fn service(
        &self,
        req: Request<Body>,
    ) -> Box<Future<Item=Response<Body>, Error=GatewayError> + Send + 'static>
    {
        match (req.method(), req.uri().path()) {
            // SDK OPTIONS request
            (&Method::OPTIONS, "/xray/events/xorc/sdk/v1") => {
                REQUEST_COUNTER.with_label_values(&[
                    "200",
                    "options",
                ]).inc();

                Box::new(Self::handle_options())
            },
            // SDK events main path
            (&Method::POST, "/xray/events/xorc/sdk/v1") => {
                let timer = RESPONSE_TIMES_HISTOGRAM.start_timer();

                Box::new(Self::handle_sdk(req, self.connections.clone()).then(|response| {
                    timer.observe_duration();
                    response
                }))
            },
            // Prometheus metrics
            (&Method::GET, "/metrics") => {
                Box::new(Self::handle_metrics())
            },
            _ => {
                REQUEST_COUNTER.with_label_values(&[
                    "404",
                    "not_found",
                ]).inc();

                let mut res = Response::new(Body::empty());
                *res.status_mut() = StatusCode::NOT_FOUND;
                Box::new(future::ok(res))
            }
        }
    }

    /// Run the service, keeps running until a signal is sent through rx
    pub fn run(rx: oneshot::Receiver<()>) {
        let port = match env::var("PORT") {
            Ok(val) => val,
            Err(_) => String::from("1337"),
        };

        let mut addr_iter = format!("0.0.0.0:{}", port).to_socket_addrs().unwrap();
        let addr = addr_iter.next().unwrap();

        let mut runtime = RuntimeBuilder::new()
            .name_prefix(CONFIG.gateway.process_name_prefix.clone())
            .core_threads(CONFIG.gateway.threads)
            .build().unwrap();

        let gateway = Self::new();

        let server = Server::bind(&addr)
            .serve(move || {
                let gw = gateway.clone();
                service_fn(move |req: Request<Body>| {
                    gw.service(req)
                })
            })
            .map_err(|e| error!(*GLOG, "Critical server error, exiting: {}", e));

        info!(
            *GLOG,
            "Running on {} threads. Listening on http://{}", CONFIG.gateway.threads,
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
        mut context: Context,
        event: SDKEventBatch,
        event_id: String,
    ) -> impl Future<Item=(Vec<EventResult>, Context, SDKEventBatch), Error=GatewayError>
    {
        let fetch_id = match context.device_id {
            Some(ref device_id) => {
                Either::A(ok(device_id.clone()))
            },
            _ => {
                let app_id = context.app_id.clone();
                let ifa = event.device.ifa.clone();
                let tracking_enabled = event.device.ifa_tracking_enabled;

                let get_id = lazy(move || poll_fn(move || blocking(|| {
                    let device_id = IFA_MATCHING
                        .get_id_for_ifa(&app_id, &ifa, tracking_enabled)
                        .map(move |device_id| {
                            let cleartext = Cleartext::from(device_id);
                            let ciphertext = Ciphertext::encrypt(&cleartext);

                            DeviceId { cleartext, ciphertext }
                        }).unwrap_or_else(|| {
                            DeviceId::generate()
                        });

                    let _ = IFA_MATCHING.put_id_for_ifa(
                        &app_id,
                        &device_id.cleartext,
                        &ifa,
                        tracking_enabled
                    );

                    device_id
                })));

                Either::B(get_id)
            }
        };

        fetch_id.map(|device_id: DeviceId| {
            let ciphertext = if context.device_id.is_none() {
                let ciphertext = device_id.ciphertext.clone();
                context.device_id = Some(device_id);

                ciphertext
            } else {
                device_id.ciphertext
            };

            if context.api_token.is_none() {
                context.api_token = APP_REGISTRY.token_for(&context.app_id);
            }

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
        let find_register_event = event.events.iter()
            .find(|ref e| e.is_register())
            .map(|e| e.id.clone());

        if let Some(event_id) = find_register_event {
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
        body: &[u8],
        event: SDKEventBatch,
        headers: &HeaderMap,
        connections: Arc<BusConnections>
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

        match APP_REGISTRY.validate(&event, &context, &body) {
            Ok(()) => {
                let response = Self::generate_event_results(context, event)
                    .map_err(|e| (e, None))
                    .and_then(move |(results, context, event)| {
                        let event_count = event.events.len();
                        let proto_event: output::events::SdkEventBatch =
                            event.into_proto(&context);

                        let mut payload = Vec::new();
                        proto_event.encode(&mut payload).unwrap();

                        connections
                            .kafka
                            .publish(&payload, &context)
                            .or_else(|e| { err((e, None)) })
                            .map(move |_| {
                                info!(
                                    *GLOG,
                                    "Successfully sent {} event(s) downstream", event_count;
                                    &context
                                );

                                EVENTS_COUNTER.inc_by(results.len() as f64);

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
        req: Request<Body>,
        connections: Arc<BusConnections>
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
                        body.as_ref(),
                        event,
                        &headers,
                        connections
                    ))
                } else {
                    Either::B(err((GatewayError::InvalidPayload, None)))
                }
            })
            .then(move |res| {
                match res {
                    Ok((json_body, context)) => {
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

                        builder.header(
                            header::CONTENT_TYPE,
                            "application/json"
                        );

                        REQUEST_COUNTER.with_label_values(&[
                            "200",
                            "sdk_events",
                        ]).inc();

                        builder.status(StatusCode::OK);
                        ok(builder.body(json_body.into()).unwrap())
                    },
                    Err((e, context)) => {
                        match context {
                            Some(ref context) =>
                                error!(*GLOG, "Response error: {}", e.description(); context),
                            _ => {
                                error!(*GLOG, "Response error: {}", e.description());
                            }
                        }

                        let response = error::into_response(&e, &context);

                        REQUEST_COUNTER.with_label_values(&[
                            response.status().as_str(),
                            "sdk_events",
                        ]).inc();

                        ok(response)
                    },
                }
            })
    }
}
