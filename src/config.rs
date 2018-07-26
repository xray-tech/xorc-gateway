use std::fs::File;
use std::io::prelude::*;
use toml;
use ::RUST_ENV;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub kafka: KafkaConfig,
    pub gateway: GatewayConfig,
    pub cors: Option<CorsConfig>,
    pub origins: Vec<OriginConfig>,
    pub test_apps: Option<Vec<TestAppConfig>>,
    pub cassandra: CassandraConfig,
}

impl Config {
    pub fn parse(path: &str) -> Config {
        let mut config_toml = String::new();

        let mut file = match File::open(&path) {
            Ok(file) => file,
            Err(err) => {
                panic!("Error while reading config file: [{}]", err);
            }
        };

        file.read_to_string(&mut config_toml)
            .unwrap_or_else(|err| panic!("Error while reading config: [{}]", err));

        let config: Config = toml::from_str(&config_toml)
            .unwrap_or_else(|err| {
                panic!("Error while reading config: [{}]", err)
            });

        if &*RUST_ENV != "development" {
            if config.gateway.allow_empty_signature {
                panic!("Cannot allow empty signatures outside of development environment.")
            }

            if config.cassandra.manage_apps {
                panic!("Cannot allow manage_apps to be false outside of development environment.")
            }
        }

        config
    }
}

#[derive(Deserialize, Debug)]
pub struct GatewayConfig {
    pub threads: usize,
    pub process_name_prefix: String,
    pub default_token: String,
    pub allow_empty_signature: bool,
}

#[derive(Deserialize, Debug)]
pub struct OriginConfig {
    pub app_id: String,
    pub allowed: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct TestAppConfig {
    pub app_id: String,
    pub token: Option<String>,
    pub secret_ios: Option<String>,
    pub secret_android: Option<String>,
    pub secret_web: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct CorsConfig {
    pub allowed_methods: String,
    pub allowed_headers: String,
}

#[derive(Deserialize, Debug)]
pub struct CassandraConfig {
    pub keyspace: String,
    pub contact_points: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub manage_apps: bool,
}

#[derive(Deserialize, Debug)]
pub struct KafkaConfig {
    pub topic: String,
    pub brokers: String,
}
