use encryption::Cleartext;
use uuid::Uuid;
use r2d2;

use ::CONFIG;

use std::{
    io
};

use cdrs::{
    authenticators::NoneAuthenticator,
    compression::Compression,
    query::{QueryBuilder, Query},
    frame::Frame,
    transport::TransportTcp,
    types::{value::Value, ByName},
    cluster::{LoadBalancingStrategy, LoadBalancer, ClusterConnectionManager},
};

use metrics::{
    SCYLLADB_LATENCY_HISTOGRAM,
    SCYLLADB_REQUEST_COUNTER,
};

type CassandraPool =
    r2d2::Pool<ClusterConnectionManager<NoneAuthenticator, TransportTcp>>;

pub struct IfaMatching {
    pool: CassandraPool,
}

impl IfaMatching {
    pub fn new() -> IfaMatching {
        let config = &CONFIG.cassandra;

        let cluster = config
            .contact_points
            .split(",")
            .map(|addr| TransportTcp::new(addr).unwrap())
            .collect();

        let load_balancer = LoadBalancer::new(cluster, LoadBalancingStrategy::RoundRobin);

        let manager = ClusterConnectionManager::new(
            load_balancer,
            NoneAuthenticator,
            Compression::None
        );

        let pool = r2d2::Pool::builder()
            .max_size(15)
            .build(manager)
            .unwrap();

        IfaMatching { pool, }
    }

    pub fn get_id_for_ifa(
        &self,
        app_id: &str,
        ifa: &Option<String>,
        ifa_tracking_enabled: bool,
    ) -> Option<String>
    {
        let ifa = Self::parse_ifa(ifa, ifa_tracking_enabled)?;
        let app_id = Uuid::parse_str(app_id).ok()?;

        let values = vec![
            app_id.into(),
            ifa.into(),
        ];

        let query = QueryBuilder::new(
            format!(
                "SELECT entity_id FROM {}.gw_known_ifas WHERE app_id=? AND ifa=?",
                CONFIG.cassandra.keyspace
            )
        ).values(values).finalize();

        let frame = self.run_query(query)
            .or_else(|e| {
                error!("Could not read IFA from ScyllaDB: {:?}", e);
                SCYLLADB_REQUEST_COUNTER.with_label_values(&["get", "error"]).inc();
                Err(e)
            })
            .ok()?;

        let body = frame.get_body().ok()?;
        let rows = body.into_rows()?;

        let entity_id: Option<Uuid> = rows.first()
            .and_then(|row| row.r_by_name("entity_id").ok());

        match entity_id {
            Some(entity_id) => {
                SCYLLADB_REQUEST_COUNTER.with_label_values(&["get", "ok"]).inc();
                Some(entity_id.hyphenated().to_string())
            }
            None => {
                SCYLLADB_REQUEST_COUNTER.with_label_values(&["get", "not_found"]).inc();
                None
            }
        }
    }

    pub fn put_id_for_ifa(
        &self,
        app_id: &str,
        device_id: &Cleartext,
        ifa: &Option<String>,
        ifa_tracking_enabled: bool,
    ) -> Result<(), io::Error>
    {
        let ifa = Self::parse_ifa(ifa, ifa_tracking_enabled)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "IFA storage is not allowed or ifa was faulty"
                )
            })?;

        let app_id = Uuid::parse_str(app_id)
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Could not write to IFA storage with faulty app_id: {:?}", e)
                )
            })?;

        let entity_id = Uuid::parse_str(device_id.as_ref())
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Could not write to IFA storage with faulty entity_id: {:?}", e)
                )
            })?;

        let values = vec![
            app_id.into(),
            ifa.into(),
            entity_id.into()
        ];

        let query = QueryBuilder::new(
            format!(
                "INSERT INTO {}.gw_known_ifas (app_id, ifa, entity_id) VALUES (?, ?, ?)",
                CONFIG.cassandra.keyspace
            )
        ).values(values).finalize();

        match self.run_query(query) {
            Ok(_) => {
                SCYLLADB_REQUEST_COUNTER.with_label_values(&["put", "ok"]).inc();
                Ok(())
            },
            Err(error) => {
                SCYLLADB_REQUEST_COUNTER.with_label_values(&["put", "error"]).inc();

                Err(
                    io::Error::new(
                        io::ErrorKind::Interrupted,
                        format!("Couldn't write to ScyllaDB: {:?}", error)
                    )
                )
            }
        }
    }

    fn run_query(&self, query: Query) -> Result<Frame, io::Error> {
        let conn = self.pool.get()
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Interrupted,
                    format!("Could not get a ScyllaDB connection from the pool: {:?}", e)
                )
            })?;

        let timer = SCYLLADB_LATENCY_HISTOGRAM.start_timer();
        let result = conn.query(query, false, false);
        timer.observe_duration();

        result.map_err(|e| {
            io::Error::new(
                io::ErrorKind::Interrupted,
                format!("Could not connect to ScyllaDB: {:?}", e)
            )
        })
    }

    fn parse_ifa(ifa: &Option<String>, ifa_tracking_enabled: bool) -> Option<Uuid> {
        ifa.as_ref()
            .and_then(|ref ifa| Uuid::parse_str(ifa).ok())
            .and_then(|ifa| {
                if ifa_tracking_enabled == true && ifa != Uuid::nil() {
                    Some(ifa)
                } else {
                    None
                }
            })
    }

}
