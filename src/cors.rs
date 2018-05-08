use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    iter::FromIterator,
};

use config::Config;
use http::{header::{self, HeaderName, HeaderValue}};

pub struct Cors {
    allowed_methods: String,
    allowed_headers: String,
    allowed_origins: HashMap<String, HashSet<String>>
}

impl Cors {
    pub fn headers_for(&self, app_id: &str, origin: &str) -> Option<Vec<(HeaderName, HeaderValue)>> {
        match self.allowed_origins.get(app_id) {
            Some(app_origins) if app_origins.contains(origin)  => {
                let mut headers = Vec::new();
                headers.push((
                    header::ACCESS_CONTROL_ALLOW_ORIGIN,
                    origin.to_string().parse().unwrap()
                ));

                headers.push((
                    header::ACCESS_CONTROL_ALLOW_METHODS,
                    self.allowed_methods.parse().unwrap()
                ));

                headers.push((
                    header::ACCESS_CONTROL_ALLOW_HEADERS,
                    self.allowed_headers.parse().unwrap()
                ));

                Some(headers)
            },
            _ => None
        }
    }
}

impl From<Arc<Config>> for Cors {
    fn from(config: Arc<Config>) -> Cors {
        let allowed_origins: HashMap<String, HashSet<String>> = config.origins
            .iter()
            .fold(HashMap::new(), |mut acc, origin| {
                acc.insert(
                    format!("{}", origin.app_id),
                    HashSet::from_iter(origin.allowed.iter().map(|s| s.to_string()))
                );

                acc
            });

        Cors {
            allowed_origins: allowed_origins,
            allowed_methods: config.cors.allowed_methods.clone(),
            allowed_headers: config.cors.allowed_headers.clone(),

        }
    }
}
