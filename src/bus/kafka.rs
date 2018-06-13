use rdkafka::{
    config::ClientConfig,
    producer::FutureProducer,
};

use futures::{
    Future,
    future::{ok, err},
};

use error::GatewayError;
use context::Context;
use ::CONFIG;

use metrics::KAFKA_LATENCY_HISTOGRAM;

pub struct Kafka {
    producer: FutureProducer,
}

impl Kafka {
    pub fn new() -> Kafka {
        info!("Connecting to Kafka...");

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
        payload: &Vec<u8>,
        context: &Context,
    ) -> impl Future<Item=(), Error=GatewayError>
    {
        let send_event = self.producer.send_copy::<Vec<u8>, Vec<u8>>(
            CONFIG.kafka.topic.as_ref(),
            None,
            Some(payload),
            Self::routing_key(context).as_ref(),
            None,
            1000
        );

        let timer = KAFKA_LATENCY_HISTOGRAM.start_timer();
        send_event.then(|res| {
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
