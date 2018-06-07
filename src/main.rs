#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;
#[allow(unused_imports)]
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate prost_derive;
#[macro_use]
extern crate indoc;
#[macro_use]
extern crate aerospike;
#[macro_use]
extern crate prometheus;

extern crate hex;
extern crate crossbeam;
extern crate hyper;
extern crate pretty_env_logger;
extern crate gelf;
extern crate env_logger;
extern crate ring;
extern crate serde;
extern crate chrono;
extern crate prost;
extern crate bytes;
extern crate toml;
extern crate futures;
extern crate base64;
extern crate uuid;
extern crate rand;
extern crate http;
extern crate tokio;
extern crate tokio_threadpool;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate postgres;
extern crate blake2;
extern crate rdkafka;
extern crate lapin_futures;
extern crate tokio_signal;
extern crate maxminddb;

mod entity_storage;
mod error;
mod context;
mod events;
mod gateway;
mod config;
mod logger;
mod cors;
mod app_registry;
mod encryption;
mod bus;
mod metrics;

use gateway::Gateway;
use entity_storage::EntityStorage;
use app_registry::AppRegistry;
use config::Config;
use futures::{sync::oneshot, Future, Stream};
use cors::Cors;
use tokio_signal::unix::{Signal, SIGINT};

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    env,
};

/// Global non-IO services that can be raced from all the threads.
lazy_static! {
    pub static ref GEOIP: maxminddb::Reader =
        match env::var("GEOIP") {
            Ok(geoip_location) => {
                maxminddb::Reader::open(&geoip_location).unwrap()
            },
            _ => {
                maxminddb::Reader::open("./resources/GeoLite2-Country.mmdb").unwrap()
            }
        };

    pub static ref ENTITY_STORAGE: EntityStorage = EntityStorage::new();
    pub static ref GLOG: logger::GelfLogger =
        logger::GelfLogger::new().unwrap();

    pub static ref CONFIG: Config =
        match env::var("CONFIG") {
            Ok(config_file_location) => {
                Config::parse(&config_file_location)
            },
            _ => {
                Config::parse("./config/config.toml.tests")
            }
        };

    pub static ref APP_REGISTRY: AppRegistry = AppRegistry::new();
    pub static ref CORS: Option<Cors> = Cors::new();
}

fn main() {
    let control = Arc::new(AtomicBool::new(true));

    let mut threads: Vec<JoinHandle<_>> = Vec::new();
    let (server_tx, server_rx) = oneshot::channel();

    threads.push({
        let control = control.clone();
        thread::spawn(move || {
            info!("Starting the app registry thread...");
            APP_REGISTRY.run_updater(control);
            info!("Exiting the app registry thread...");
        })
    });

    threads.push({
        thread::spawn(move || {
            info!("Starting the SDK gateway thread...");
            Gateway::run(server_rx);
            info!("Exiting the SDK gateway thread...");
        })
    });

    let _ = Signal::new(SIGINT).flatten_stream().into_future().and_then(|_| {
        if let Err(error) = server_tx.send(()) {
            error!(
                "There was an error sending the server shutdown signal: [{:?}]",
                error
            );
        };

        control.store(false, Ordering::Relaxed);

        for thread in threads {
            thread.thread().unpark();
            thread.join().unwrap();
        }

        Ok(())
    }).wait();
}
