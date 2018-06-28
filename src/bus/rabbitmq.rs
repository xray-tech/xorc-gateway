use lapin_futures::{
    client::{
        Client,
        ConnectionOptions,
    },
    channel::*,
};

use std::{
    u32,
    thread::{JoinHandle, self},
    net::{
        SocketAddr,
        ToSocketAddrs,
    },
};

use futures::{
    Future,
    Stream,
    future::{ok, err},
};

use tokio_threadpool;
use tokio::{net::TcpStream, runtime};
use context::{Context, DeviceId};
use error::GatewayError;
use tokio_signal::unix::{Signal, SIGINT};
use ::CONFIG;
use metrics::RABBITMQ_LATENCY_HISTOGRAM;

static RULE_ENGINE_PARTITIONS: u32 = 256;

pub struct RabbitMq {
    channel: Channel<TcpStream>,
    handle: Option<JoinHandle<()>>
}

impl Drop for RabbitMq {
    fn drop(&mut self) {
        self.channel.close(200, "Bye");
        self.handle.take().unwrap().join().unwrap();
    }
}

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

        let stream = TcpStream::connect(&address).wait().unwrap();

        let (client, heartbeat) =
            Client::connect(stream, connection_options).wait().unwrap();

        let handle =
            thread::spawn(move || {
                info!("Starting the heartbeat thread");
                let signal = Signal::new(SIGINT).flatten_stream().into_future();

                let mut threadpool_builder = tokio_threadpool::Builder::new();
                threadpool_builder
                    .name_prefix("rabbitmq_heartbeat")
                    .pool_size(1);

                let mut runtime = runtime::Builder::new()
                    .threadpool_builder(threadpool_builder)
                    .build().unwrap();

                runtime.spawn(heartbeat.select2(signal).then(move |_| Ok(())));
                runtime.shutdown_on_idle().wait().unwrap();
            });


        let channel = client.create_channel().wait().unwrap();

        RabbitMq { channel, handle: Some(handle) }
    }

    pub fn publish(
        &self,
        payload: Vec<u8>,
        context: &Context,
    ) -> impl Future<Item=(), Error=GatewayError>
    {
        let routing_key = Self::routing_key(context.device_id.as_ref());

        let send_event = self.channel.basic_publish(
            CONFIG.rabbitmq.exchange.as_ref(),
            routing_key.as_ref(),
            payload,
            BasicPublishOptions {
                mandatory: false,
                immediate: false,
                ..Default::default()
            },
            BasicProperties::default(),
        );

        let timer = RABBITMQ_LATENCY_HISTOGRAM.start_timer();
        send_event.then(|res| {
            timer.observe_duration();

            match res {
                Ok(_) =>
                    ok(()),
                Err(_) =>
                    err(GatewayError::ServiceUnavailable("Could not send to rabbitmq")),
            }
        })
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
