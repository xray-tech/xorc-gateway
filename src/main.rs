#[macro_use] extern crate serde_derive;
#[macro_use] extern crate log;
#[macro_use] extern crate lazy_static;

#[allow(unused_imports)]
#[macro_use]
extern crate serde_json;

#[macro_use]
extern crate prost_derive;

extern crate hyper;
extern crate pretty_env_logger;
extern crate gelf;
extern crate env_logger;
extern crate ring;
extern crate serde;
extern crate chrono;
extern crate prost;
extern crate bytes;
extern crate argparse;
extern crate toml;
extern crate futures;
extern crate base64;
extern crate uuid;
extern crate rand;
extern crate http;
extern crate tokio;
extern crate tokio_threadpool;
extern crate prometheus;

mod headers;
mod events;
mod proto_events;
mod gateway;
mod config;
mod logger;
mod cors;

use gateway::Gateway;
use config::Config;
use std::sync::Arc;
use argparse::{
    ArgumentParser,
    Store,
};

lazy_static! {
    pub static ref GLOG: logger::GelfLogger =
        logger::GelfLogger::new().unwrap();
}

fn main() {
    let mut config_file_location = String::from("./config/config.toml");

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("SDK Gateway");
        ap.refer(&mut config_file_location).add_option(
            &["-c", "--config"],
            Store,
            "Config file (default: config.toml)",
        );
        ap.parse_args_or_exit();
    }

    let config = Arc::new(Config::parse(config_file_location));
    let gateway = Gateway::new(config.clone());
    gateway.run()
}
