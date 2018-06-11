use encryption::Cleartext;

use std::{
    time::Duration,
    thread,
    io
};

// OCD...
use aerospike::{
    Key,
    Error,
    Client,
    ErrorKind,
    ResultCode,
    ReadPolicy,
    WritePolicy,
    ClientPolicy,
};

use ::CONFIG;

use metrics::{
    AEROSPIKE_LATENCY_HISTOGRAM,
    AEROSPIKE_GET_COUNTER,
    AEROSPIKE_PUT_COUNTER
};

pub struct EntityStorage {
    namespace: String,
    client: Client
}

impl EntityStorage {
    pub fn new() -> EntityStorage {
        let client_policy = ClientPolicy {
            thread_pool_size: 16,
            ..Default::default()
        };

        let client = Client::new(&client_policy, &CONFIG.aerospike.nodes).unwrap();

        EntityStorage {
            namespace: CONFIG.aerospike.namespace.clone(),
            client: client
        }
    }

    fn as_key(
        &self,
        ifa: &str,
        app_id: &str,
    ) -> Key
    {
        as_key!(
            self.namespace.clone(),
            String::from("gw_known_ifas"),
            format!("{}@{}", ifa, app_id)
        )
    }

    pub fn get_id_for_ifa<'a>(
        &self,
        app_id: &str,
        ifa: &Option<String>,
        ifa_tracking_enabled: bool,
    ) -> Option<String>
    {
        match ifa {
            Some(ref ifa) if
                ifa_tracking_enabled == true &&
                ifa != "00000000-0000-0000-0000-000000000000" =>
            {
                let key = self.as_key(ifa, app_id);
                let mut back_off = Duration::from_millis(1);
                let timer = AEROSPIKE_LATENCY_HISTOGRAM.start_timer();

                for _ in 0..5 {
                    match self.client.get(&ReadPolicy::default(), &key, ["entity_id"]) {
                        Ok(record) => {
                            timer.observe_duration();
                            AEROSPIKE_GET_COUNTER.with_label_values(&["ok"]).inc();
                            return record.bins.get("entity_id").map(|v| v.as_string())
                        }
                        Err(Error(ErrorKind::ServerError(ResultCode::KeyNotFoundError), _)) => {
                            timer.observe_duration();
                            AEROSPIKE_GET_COUNTER.with_label_values(&["not_found"]).inc();
                            return None
                        }
                        Err(e) => {
                            AEROSPIKE_GET_COUNTER.with_label_values(&["error"]).inc();
                            warn!("Error reading known ifa, retrying: [{:?}]", e);
                            thread::park_timeout(back_off);
                            back_off += Duration::from_millis(1);
                        }
                    }
                }

                timer.observe_duration();

                error!("Could not read known ifa.");

                None
            },
            _ => None
        }
    }

    pub fn put_id_for_ifa<'a>(
        &self,
        app_id: &str,
        device_id: &Cleartext,
        ifa: &Option<String>,
        ifa_tracking_enabled: bool,
    ) -> Result<(), io::Error>
    {
        match ifa {
            Some(ref ifa) if
                ifa_tracking_enabled == true &&
                ifa != "00000000-0000-0000-0000-000000000000" =>
            {
                let bin = as_bin!(
                    "entity_id",
                    device_id.as_ref()
                );

                let key = self.as_key(ifa, app_id);
                let mut back_off = Duration::from_millis(1);
                let timer = AEROSPIKE_LATENCY_HISTOGRAM.start_timer();

                for _ in 0..5 {
                    match self.client.put(&WritePolicy::default(), &key, &[&bin]) {
                        Ok(_) => {
                            timer.observe_duration();
                            AEROSPIKE_PUT_COUNTER.with_label_values(&["ok"]).inc();
                            return Ok(())
                        },
                        Err(e) => {
                            AEROSPIKE_PUT_COUNTER.with_label_values(&["error"]).inc();
                            warn!("Error serializing known ifa, retrying: [{:?}]", e);
                            thread::park_timeout(back_off);
                            back_off += Duration::from_millis(1);
                        }
                    }
                }

                timer.observe_duration();

                panic!("Could not write known ifa. Aborting!")
            },
            _ => {
                Err(
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "IFA storage is not allowed or ifa was faulty"
                    )
                )
            }
        }
    }
}
