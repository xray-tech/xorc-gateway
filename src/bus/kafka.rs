use rdkafka::{
    config::ClientConfig,
    producer::{
        FutureProducer,
        future_producer::FutureRecord,
    },
};

use futures::{
    Future,
    future::{ok, err},
};

use error::GatewayError;
use context::Context;
use ::{CONFIG, GLOG};

use metrics::KAFKA_LATENCY_HISTOGRAM;

pub struct Kafka {
    producer: FutureProducer,
}

impl Kafka {
    pub fn new() -> Kafka {
        info!(*GLOG, "Connecting to Kafka...");

        let producer = ClientConfig::new()
            .set("bootstrap.servers", &CONFIG.kafka.brokers)
            .set("produce.offset.report", "true")
            .set("request.required.acks", "0")
            .create()
            .expect("Producer creation error");

        Kafka {
            producer,
        }
    }

    pub fn publish(
        &self,
        payload: &[u8],
        context: &Context,
    ) -> impl Future<Item=(), Error=GatewayError>
    {
        let routing_key = Self::routing_key(context);

        let record: FutureRecord<Vec<u8>, [u8]> = FutureRecord {
            topic: CONFIG.kafka.topic.as_ref(),
            partition: None,
            payload: Some(payload),
            key: routing_key.as_ref(),
            timestamp: None,
            headers: None,
        };

        let timer = KAFKA_LATENCY_HISTOGRAM.start_timer();

        self.producer.send(record, 1000).then(|res| {
            timer.observe_duration();

            match res {
                Ok(_) =>
                    ok(()),
                Err(_) =>
                    err(GatewayError::ServiceUnavailable("Could not send to kafka")),
            }
        })
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
