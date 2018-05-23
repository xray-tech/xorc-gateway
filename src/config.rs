use std::fs::File;
use std::io::prelude::*;
use toml;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub kafka: KafkaConfig,
    pub rabbitmq: RabbitMqConfig,
    pub gateway: GatewayConfig,
    pub cors: Option<CorsConfig>,
    pub origins: Vec<OriginConfig>,
    pub test_apps: Vec<TestAppConfig>,
    pub postgres: Option<PostgresConfig>,
    pub aerospike: Option<AerospikeConfig>,
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

        toml::from_str(&config_toml).unwrap_or_else(|err| panic!("Error while reading config: [{}]", err))
    }
}

#[derive(Deserialize, Debug)]
pub struct GatewayConfig {
    pub address: String,
    pub threads: usize,
    pub process_name_prefix: String,
    pub default_token: String,
    pub allow_empty_signature: bool,
}

#[derive(Deserialize, Debug)]
pub struct OriginConfig {
    pub app_id: u32,
    pub allowed: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct TestAppConfig {
    pub app_id: i32,
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
pub struct PostgresConfig {
    pub uri: String,
    pub pool_size: u32,
    pub min_idle: u32,
    pub idle_timeout: u64,
    pub max_lifetime: u64,
}

#[derive(Deserialize, Debug)]
pub struct AerospikeConfig {
    pub nodes: String,
    pub namespace: String,
}

#[derive(Deserialize, Debug)]
pub struct KafkaConfig {
    pub topic: String,
    pub brokers: String,
}

#[derive(Deserialize, Debug)]
pub struct RabbitMqConfig {
    pub exchange: String,
    pub vhost: String,
    pub host: String,
    pub port: u16,
    pub login: String,
    pub password: String,
}
