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
extern crate r2d2;
extern crate r2d2_postgres;
extern crate postgres;
extern crate chan_signal;

mod error;
mod headers;
mod events;
mod proto_events;
mod gateway;
mod config;
mod logger;
mod cors;
mod app_registry;

use gateway::Gateway;
use app_registry::AppRegistry;
use config::Config;
use chan_signal::{notify, Signal};
use futures::sync::oneshot;

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

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
    let exit_signal = notify(&[Signal::INT, Signal::TERM]);

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

    let control = Arc::new(AtomicBool::new(true));
    let config = Arc::new(Config::parse(&config_file_location));
    let app_registry = Arc::new(AppRegistry::new(config.clone()));
    let mut threads: Vec<JoinHandle<_>> = Vec::new();
    let (server_tx, server_rx) = oneshot::channel();

    threads.push({
        let registry = app_registry.clone();
        let control = control.clone();
        thread::spawn(move || {
            info!("Starting the app registry thread...");
            registry.run_updater(control);
            info!("Exiting the app registry thread...");
        })
    });

    threads.push({
        let gateway = Gateway::new(config.clone(), app_registry.clone());
        thread::spawn(move || {
            info!("Starting the SDK gateway thread...");
            gateway.run(server_rx);
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
