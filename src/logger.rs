use slog::{self, Drain};
use slog_term::{TermDecorator, CompactFormat};
use slog_async::Async;
use slog_json::Json;
use std::{env, io};
use ::RUST_ENV;

pub struct Logger;

impl Logger {
    pub fn new() -> slog::Logger {
        let drain = match env::var("LOG_FORMAT") {
            Ok(ref val) if val == "json" => {
                let drain = Json::new(io::stderr()).add_default_keys().build().fuse();
                Async::new(drain).build().fuse()
            }
            _ => {
                let decorator = TermDecorator::new().stderr().build();
                let drain = CompactFormat::new(decorator).build().fuse();
                Async::new(drain).build().fuse()
            }
        };

        slog::Logger::root(
            drain,
            o!("application_name" => "xorc-gateway", "environment" => &*RUST_ENV)
        )
    }
}
