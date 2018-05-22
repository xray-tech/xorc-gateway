use log::LevelFilter;
use gelf::{Error, Logger, Message, UdpBackend, Level};
use std::env;
use env_logger;
use context::Context;
use app_registry::Application;
use error::GatewayError;

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

    pub fn log_error(
        &self,
        error: &GatewayError,
        context: &Option<Context>
    ) -> Result<(), Error>
    {
        match error {
            GatewayError::AppDoesNotExist => {
                self.log_with_context(
                    "Unknown app",
                    Level::Error,
                    context,
                )
            },
            GatewayError::InvalidToken => {
                self.log_with_context(
                    "Invalid token",
                    Level::Error,
                    context,
                )
            },
            GatewayError::MissingToken => {
                self.log_with_context(
                    "Missing token",
                    Level::Error,
                    context,
                )
            },
            GatewayError::MissingSignature => {
                self.log_with_context(
                    "Missing signature",
                    Level::Error,
                    context,
                )
            },
            GatewayError::InvalidSignature => {
                self.log_with_context(
                    "Invalid signature",
                    Level::Error,
                    context,
                )
            },
            GatewayError::UnknownOrigin => {
                self.log_with_context(
                    "Unknown Origin",
                    Level::Error,
                    context
                )
            },
            GatewayError::BadDeviceId => {
                self.log_with_context(
                    "Bad D360-Device-Id",
                    Level::Error,
                    context
                )
            },
            GatewayError::InternalServerError(reason) => {
                self.log_with_context(
                    &format!("Internal Server Error: {}", reason),
                    Level::Error,
                    context
                )
            },
            GatewayError::ServiceUnavailable(reason) => {
                self.log_with_context(
                    &format!("Service unavailable: {}", reason),
                    Level::Error,
                    context
                )
            },
            GatewayError::InvalidPayload => {
                self.log_with_context(
                    "Invalid payload",
                    Level::Error,
                    context
                )
            }
        }
    }

    pub fn log_with_context(
        &self,
        title: &str,
        level: Level,
        context: &Option<Context>
    ) -> Result<(), Error>
    {
        let mut msg = Message::new(title.to_string());
        msg.set_level(level);

        if let Some(ref context) = context {
            if let Some(ref api_token) = context.api_token {
                msg.set_metadata("api_token", format!("{}", api_token))?;
            };

            if let Some(ref device_id) = context.device_id {
                msg.set_metadata("encrypted_device_id", format!("{}", device_id.ciphertext))?;
                msg.set_metadata("device_id", format!("{}", device_id.cleartext))?;
            }

            if let Some(ref signature) = context.signature {
                msg.set_metadata("signature", format!("{}", signature))?;
            };

            if let Some(ref ip) = context.ip {
                msg.set_metadata("ip", format!("{}", ip))?;
            };
        }

        self.log_message(msg);

        Ok(())
    }

    pub fn log_without_headers(
        &self,
        title: &str,
        level: Level,
    )
    {
        let mut msg = Message::new(title.to_string());
        msg.set_level(level);
        self.log_message(msg);
    }

    pub fn log_app_update(&self, app: &Application) -> Result<(), Error> {
        let mut msg = Message::new("Application data update".to_string());
        msg.set_level(Level::Informational);

        msg.set_metadata("app_id", format!("{}", app.id))?;

        if app.token.is_some() {
            msg.set_metadata("has_token", "true".to_string())?;
        } else {
            msg.set_metadata("has_token", "false".to_string())?;
        }

        if app.ios_secret.is_some() {
            msg.set_metadata("ios_enabled", "true".to_string())?;
        } else {
            msg.set_metadata("ios_enabled", "false".to_string())?;
        }

        if app.android_secret.is_some() {
            msg.set_metadata("android_enabled", "true".to_string())?;
        } else {
            msg.set_metadata("android_enabled", "false".to_string())?;
        }

        if app.web_secret.is_some() {
            msg.set_metadata("web_enabled", "true".to_string())?;
        } else {
            msg.set_metadata("web_enabled", "false".to_string())?;
        }

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
