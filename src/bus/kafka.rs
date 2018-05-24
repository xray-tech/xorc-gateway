use rdkafka::{
    config::ClientConfig,
    producer::FutureProducer,
};

use prost::Message;
use events::output::events::SdkEventBatch;
use error::GatewayError;
use futures::Future;
use context::Context;
use ::CONFIG;

pub struct Kafka {
    producer: FutureProducer,
}

impl Kafka {
    pub fn new() -> Kafka {
        info!("Connecting to Kafka...");

        let producer = ClientConfig::new()
            .set("bootstrap.servers", &CONFIG.kafka.brokers)
            .set("produce.offset.report", "true")
            .create()
            .expect("Producer creation error");

        Kafka {
            producer,
        }
    }

    pub fn publish(
        &self,
        event: &SdkEventBatch,
        context: &Context,
    ) -> impl Future<Item=(), Error=GatewayError>
    {
        let mut buf = Vec::new();
        event.encode(&mut buf).unwrap();

        self.producer.send_copy::<Vec<u8>, Vec<u8>>(
            CONFIG.kafka.topic.as_ref(),
            None,
            Some(&buf),
            Self::routing_key(context).as_ref(),
            None,
            1000,
        ).map_err(|_| GatewayError::ServiceUnavailable("Could not send to Kafka")).map(|_| ())
    }

    fn routing_key(context: &Context) -> Option<Vec<u8>> {
        context
            .device_id
            .as_ref()
            .map(|ref device_id| {
                let key = format!("{}|{}", context.app_id, device_id.cleartext);
                key.as_bytes().to_vec()
            })
    }
}
