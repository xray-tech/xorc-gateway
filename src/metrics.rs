use prometheus::{CounterVec, Counter, Histogram};

lazy_static! {
    pub static ref APP_UPDATE_COUNTER: Counter = register_counter!(
        "gateway_application_updates",
        "Total number of application updates"
    ).unwrap();

    pub static ref EVENTS_COUNTER: Counter = register_counter!(
        "events_total",
        "Total number of SDK events sent"
    ).unwrap();

    pub static ref REQUEST_COUNTER: CounterVec = register_counter_vec!(
        "http_requests_total",
        "Total number of HTTP requests made.",
        &["status", "endpoint"]
    ).unwrap();

    pub static ref AEROSPIKE_GET_COUNTER: CounterVec = register_counter_vec!(
        "aerospike_get_total",
        "Total number of gets to Aerospike",
        &["status"]
    ).unwrap();

    pub static ref AEROSPIKE_PUT_COUNTER: CounterVec = register_counter_vec!(
        "aerospike_put_total",
        "Total number of puts to Aerospike",
        &["status"]
    ).unwrap();

    pub static ref RESPONSE_TIMES_HISTOGRAM: Histogram = register_histogram!(
        "http_request_latency_seconds",
        "The HTTP request latencies in seconds"
    ).unwrap();

    pub static ref KAFKA_LATENCY_HISTOGRAM: Histogram = register_histogram!(
        "kafka_latency_seconds",
        "The HTTP request latencies in seconds"
    ).unwrap();

    pub static ref RABBITMQ_LATENCY_HISTOGRAM: Histogram = register_histogram!(
        "rabbitmq_latency_seconds",
        "The HTTP request latencies in seconds"
    ).unwrap();

    pub static ref AEROSPIKE_LATENCY_HISTOGRAM: Histogram = register_histogram!(
        "aerospike_latency_seconds",
        "The HTTP request latencies in seconds"
    ).unwrap();
}
