use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, DeliveryFuture},
};

use prost::Message;
use events::output::events::SdkEventBatch;
use ::CONFIG;

pub struct Kafka {
    producer: FutureProducer,
}

impl Kafka {
    pub fn new() -> Kafka {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", &CONFIG.kafka.brokers)
            .set("produce.offset.report", "true")
            .create()
            .expect("Producer creation error");

        Kafka {
            producer,
        }
    }

    pub fn publish(&self, event: SdkEventBatch) -> DeliveryFuture {
        let mut buf = Vec::new();
        event.encode(&mut buf).unwrap();

        self.producer.send_copy::<Vec<u8>, ()>(
            CONFIG.kafka.topic.as_ref(),
            None,
            Some(&buf),
            None,
            None,
            1000,
        )
    }
}
