use prometheus::{CounterVec, Histogram};

lazy_static! {
    pub static ref REQUEST_COUNTER: CounterVec = register_counter_vec!(
        "http_requests_total",
        "Total number of HTTP requests made.",
        &["status", "endpoint"]
    ).unwrap();

    pub static ref RESPONSE_TIMES_HISTOGRAM: Histogram = register_histogram!(
        "http_request_latency_seconds",
        "The HTTP request latencies in seconds"
    ).unwrap();
}
