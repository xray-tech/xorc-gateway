use config::AerospikeConfig;
use events::input::SDKDevice;
use encryption::Cleartext;

use std::{
    time::Duration,
    thread,
    io
};

// OCD...
use aerospike::{
    Error,
    Client,
    ErrorKind,
    ResultCode,
    ReadPolicy,
    WritePolicy,
    ClientPolicy,
};

pub struct EntityStorage {
    namespace: String,
    client: Client
}

impl EntityStorage {
    pub fn new(config: &AerospikeConfig) -> EntityStorage {
        let client_policy = ClientPolicy {
            thread_pool_size: 16,
            ..Default::default()
        };

        let client = Client::new(&client_policy, &config.nodes).unwrap();

        EntityStorage {
            namespace: config.namespace.clone(),
            client: client
        }
    }

    pub fn get_id_for_ifa<'a>(
        &self,
        app_id: &str,
        device: &'a SDKDevice
    ) -> Option<String>
    {
        match device.ifa {
            Some(ref ifa) if
                device.ifa_tracking_enabled == true &&
                ifa != "00000000-0000-0000-0000-000000000000" =>
            {
                let key = as_key!(
                    self.namespace.clone(),
                    String::from("gw_known_ifas"),
                    format!("{}@{}", ifa, app_id)
                );

                let mut back_off = Duration::from_millis(1);

                for _ in 0..5 {
                    match self.client.get(&ReadPolicy::default(), &key, ["entity_id"]) {
                        Ok(record) =>
                            return record.bins.get("entity_id").map(|v| v.as_string()),
                        Err(Error(ErrorKind::ServerError(ResultCode::KeyNotFoundError), _)) =>
                            return None,
                        Err(e) => {
                            warn!("Error reading known ifa, retrying: [{:?}]", e);
                            thread::park_timeout(back_off);
                            back_off += Duration::from_millis(1);
                        }
                    }
                }

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
        device: &'a SDKDevice
    ) -> Result<(), io::Error>
    {
        match device.ifa {
            Some(ref ifa) if
                device.ifa_tracking_enabled == true &&
                ifa != "00000000-0000-0000-0000-000000000000" =>
            {
                let key = as_key!(
                    self.namespace.clone(),
                    String::from("gw_known_ifas"),
                    format!("{}@{}", ifa, app_id)
                );

                let bin = as_bin!(
                    "entity_id",
                    device_id.as_ref()
                );

                let mut back_off = Duration::from_millis(1);

                for _ in 0..5 {
                    match self.client.put(&WritePolicy::default(), &key, &[&bin]) {
                        Ok(_) =>
                            return Ok(()),
                        Err(e) => {
                            warn!("Error serializing known ifa, retrying: [{:?}]", e);
                            thread::park_timeout(back_off);
                            back_off += Duration::from_millis(1);
                        }
                    }
                }

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
