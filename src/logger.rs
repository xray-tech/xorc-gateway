use log::LevelFilter;
use gelf::{Error, Logger, Message, UdpBackend, Level};
use std::env;
use env_logger;
use headers::DeviceHeaders;

#[derive(Debug)]
pub enum LogAction {
    ConsumerCreate,
    ConsumerRestart,
    ConsumerStart,
    ConsumerDelete,
    NotificationResult,
}

pub struct GelfLogger {
    connection: Option<Logger>,
    filter: LevelFilter,
}

impl GelfLogger {
    pub fn new() -> Result<GelfLogger, Error> {
        let log_level_filter = match env::var("RUST_LOG") {
            Ok(val) => match val.as_ref() {
                "info" => LevelFilter::Info,
                "debug" => LevelFilter::Debug,
                "warn" => LevelFilter::Warn,
                "error" => LevelFilter::Error,
                _ => LevelFilter::Info,
            },
            _ => LevelFilter::Info,
        };

        if let Ok(ref host) = env::var("RUST_GELF") {
            let mut logger = Logger::new(Box::new(UdpBackend::new(host)?))?;
            let mut env_logger = Logger::new(Box::new(UdpBackend::new(host)?))?;

            logger.set_default_metadata(String::from("application_name"), String::from("sdk-gateway"));
            env_logger.set_default_metadata(String::from("application_name"), String::from("sdk-gateway"));

            if let Ok(environment) = env::var("RUST_ENV") {
                logger.set_default_metadata(
                    String::from("environment"),
                    String::from(environment.clone()),
                );
                env_logger.set_default_metadata(
                    String::from("environment"),
                    String::from(environment.clone()),
                );
            } else {
                logger
                    .set_default_metadata(String::from("environment"), String::from("development"));
                env_logger
                    .set_default_metadata(String::from("environment"), String::from("development"));
            };

            let filter = match env::var("RUST_LOG") {
                Ok(val) => match val.as_ref() {
                    "info" => Level::Informational,
                    "debug" => Level::Debug,
                    "warn" => Level::Warning,
                    "error" => Level::Error,
                    _ => Level::Informational,
                },
                _ => Level::Informational,
            };

            env_logger.install(filter)?;

            Ok(GelfLogger {
                connection: Some(logger),
                filter: log_level_filter,
            })
        } else {
            env_logger::init();

            Ok(GelfLogger {
                connection: None,
                filter: log_level_filter,
            })
        }
    }

    pub fn log_with_headers(&self, title: &str, level: Level, headers: &DeviceHeaders) -> Result<(), Error> {
        let mut msg = Message::new(title.to_string());
        msg.set_level(level);

        if let Some(ref api_token) = headers.api_token {
            msg.set_metadata("api_token", format!("{}", api_token))?;
        };

        if let Some(ref encrypted) = headers.device_id.ciphertext {
            msg.set_metadata("encrypted_device_id", format!("{}", encrypted))?;
        };

        if let Some(ref cleartext) = headers.device_id.cleartext {
            msg.set_metadata("device_id", format!("{}", cleartext))?;
        };

        if let Some(ref signature) = headers.signature {
            msg.set_metadata("signature", format!("{}", signature))?;
        };

        if let Some(ref ip) = headers.ip {
            msg.set_metadata("ip", format!("{}", ip))?;
        };

        self.log_message(msg);

        Ok(())
    }

    pub fn log_message(&self, msg: Message) {
        match self.connection {
            Some(ref connection) => connection.log_message(msg),
            None => {
                let level = match msg.level() {
                    Level::Emergency | Level::Alert | Level::Critical | Level::Error => LevelFilter::Error,
                    Level::Warning => LevelFilter::Warn,
                    Level::Notice | Level::Informational => LevelFilter::Info,
                    Level::Debug => LevelFilter::Debug,
                };

                if self.filter <= level {
                    let metadata = msg.all_metadata()
                        .iter()
                        .fold(Vec::new(), |mut acc, (k, v)| {
                            acc.push(format!("{}: {}", k, v));
                            acc
                        })
                        .join(", ");

                    println!("[{}] {}: ({})", level, msg.short_message(), metadata);
                }
            }
        }
    }
}
