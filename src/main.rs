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
extern crate chan;
#[macro_use]
extern crate aerospike;

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
extern crate prometheus;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate postgres;
extern crate chan_signal;
extern crate blake2;
extern crate rdkafka;
extern crate lapin_futures;

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

use gateway::Gateway;
use entity_storage::EntityStorage;
use app_registry::AppRegistry;
use config::Config;
use chan_signal::{notify, Signal};
use futures::sync::oneshot;
use cors::Cors;

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    env,
};

/// We need one copy of these for every thread in the pool for maximum
/// performance.
thread_local! {
    pub static ENTITY_STORAGE: EntityStorage = EntityStorage::new();
    pub static KAFKA: bus::Kafka = bus::Kafka::new();
    pub static RABBITMQ: bus::RabbitMq = bus::RabbitMq::new();
}

/// Global non-IO services that can be raced from all the threads.
lazy_static! {
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
    let exit_signal = notify(&[Signal::INT, Signal::TERM]);
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

    chan_select! {
        exit_signal.recv() -> signal => {
            info!("Received signal: {:?}", signal);

            server_tx.send(()).unwrap();
            control.store(false, Ordering::Relaxed);

            for thread in threads {
                thread.thread().unpark();
                thread.join().unwrap();
            }
        },
    }
}
