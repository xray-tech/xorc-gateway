use std::fs::File;
use std::io::prelude::*;
use toml;

#[derive(Deserialize, Debug)]
pub struct Config {
    //pub postgres: PostgresConfig,
    //pub kafka: KafkaConfig,
    pub gateway: GatewayConfig,
    pub cors: CorsConfig,
    pub origins: Vec<OriginConfig>
}

impl Config {
    pub fn parse(path: String) -> Config {
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
}

#[derive(Deserialize, Debug)]
pub struct OriginConfig {
    pub app_id: u32,
    pub allowed: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct CorsConfig {
    pub allowed_methods: String,
    pub allowed_headers: String,
}

/*
#[derive(Deserialize, Debug)]
pub struct KafkaConfig {
    pub input_topic: String,
    pub config_topic: String,
    pub output_topic: String,
    pub retry_topic: String,
    pub group_id: String,
    pub brokers: String,
}

#[derive(Deserialize, Debug)]
pub struct PostgresConfig {
    pub uri: String,
    pub pool_size: u32,
    pub min_idle: u32,
    pub idle_timeout: u64,
    pub max_lifetime: u64,
}

*/
