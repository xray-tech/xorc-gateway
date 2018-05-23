use lapin_futures::{
    client::{
        Client,
        ConnectionOptions,
    },
    channel::*,
};

use std::{
    u32,
    thread,
    net::{
        SocketAddr,
        ToSocketAddrs,
    },
};

use futures::{
    Future,
    future::lazy,
};

use tokio;
use tokio::net::TcpStream;
use context::{Context, DeviceId};
use events::output::events::SdkEventBatch;
use prost::Message;

use error::GatewayError;
use ::CONFIG;

pub struct RabbitMq {
    channel: Channel<TcpStream>,
}

static RULE_ENGINE_PARTITIONS: u32 = 256;

impl RabbitMq {
    pub fn new() -> RabbitMq {
        info!("Connecting to RabbitMq...");

        let address: SocketAddr = format!(
            "{}:{}",
            CONFIG.rabbitmq.host,
            CONFIG.rabbitmq.port
        ).to_socket_addrs().unwrap().next().unwrap();

        let connection_options = ConnectionOptions {
            username: CONFIG.rabbitmq.login.clone(),
            password: CONFIG.rabbitmq.password.clone(),
            vhost: CONFIG.rabbitmq.vhost.clone(),
            ..Default::default()
        };

        let connecting = TcpStream::connect(&address)
            .and_then(move |stream| Client::connect(stream, &connection_options))
            .and_then(|(client, heartbeat_future_fn)| {
                let heartbeat_client = client.clone();

                thread::Builder::new()
                    .name("heartbeat thread".to_string())
                    .spawn(move || {
                        tokio::run(lazy(move || {
                            heartbeat_future_fn(&heartbeat_client)
                                .map(|s| {
                                    info!("Producer heartbeat thread exited cleanly ({:?})", s);
                                })
                                .map_err(|e| {
                                    error!("Producer heartbeat thread crashed, going down... ({:?})", e);
                                })
                        }))
                    })
                    .unwrap();

                client.create_channel()
            });

        RabbitMq {
            channel: connecting.wait().unwrap(),
        }
    }

    pub fn publish(
        &self,
        event: &SdkEventBatch,
        context: &Context,
    ) -> impl Future<Item=(), Error=GatewayError>
    {
        let mut buf = vec![];
        event.encode(&mut buf).unwrap();

        let routing_key = Self::routing_key(context.device_id.as_ref());

        self.channel.basic_publish(
            CONFIG.rabbitmq.exchange.as_ref(),
            routing_key.as_ref(),
            &buf,
            &BasicPublishOptions {
                mandatory: false,
                immediate: false,
                ..Default::default()
            },
            BasicProperties::default(),
        ).map_err(|_| GatewayError::ServiceUnavailable("Could not send to RabbitMq")).map(|_| ())
    }

    // Take the four first characters of the last part of the device id,
    // convert it to a number and modulo with the number of partitions to
    // get a random partition and use always the same partition for the same
    // device id.
    fn routing_key(device_id: Option<&DeviceId>) -> String {
        device_id
            .and_then(|ref device_id| device_id.cleartext.as_ref().get(24..28))
            .and_then(|part| u32::from_str_radix(part, 16).ok())
            .map(|random_number| format!("{}", random_number % RULE_ENGINE_PARTITIONS))
            .unwrap_or_else(|| {
                warn!("The device id doesn't look like a proper id: [{:?}]", device_id);
                String::from("0")
            })
    }
}