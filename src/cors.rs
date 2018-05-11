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
    pub fn new(config: Arc<Config>) -> Option<Cors> {
        config.cors.as_ref().map(|ref cors_config| {
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
                allowed_methods: cors_config.allowed_methods.clone(),
                allowed_headers: cors_config.allowed_headers.clone(),
            }
        })
    }

    pub fn headers_for(
        &self,
        app_id: &str,
        origin: &str
    ) -> Option<Vec<(HeaderName, HeaderValue)>>
    {
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

    pub fn wildcard_headers(&self) -> Vec<(HeaderName, HeaderValue)> {
        vec![
            (
                header::ACCESS_CONTROL_ALLOW_ORIGIN,
                "*".parse().unwrap()
            ),
            (
                header::ACCESS_CONTROL_ALLOW_METHODS,
                self.allowed_methods.parse().unwrap()
            ),
            (
                header::ACCESS_CONTROL_ALLOW_HEADERS,
                self.allowed_headers.parse().unwrap()
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::Config;
    use std::sync::Arc;

    #[test]
    fn new_with_no_cors_config() {
        let mut config = Config::parse("config/config.toml.tests");
        config.cors = None;

        assert!(Cors::new(Arc::new(config)).is_none());
    }

    #[test]
    fn new_with_cors_config() {
        let cors = Cors::new(Arc::new(Config::parse("config/config.toml.tests")));
        assert!(cors.is_some());
    }

    #[test]
    fn wildcard_headers() {
        let cors = Cors::new(Arc::new(Config::parse("config/config.toml.tests"))).unwrap();

        assert_eq!(
            vec![
                (
                    header::ACCESS_CONTROL_ALLOW_ORIGIN,
                    "*".parse().unwrap()
                ),
                (
                    header::ACCESS_CONTROL_ALLOW_METHODS,
                    "HERP,DERP".parse().unwrap()
                ),
                (
                    header::ACCESS_CONTROL_ALLOW_HEADERS,
                    "Content-Type, Content-Length".parse().unwrap()
                ),
            ],
            cors.wildcard_headers(),
        )
    }

    #[test]
    fn headers_for_existing_app_from_allowed_origin() {
        let cors = Cors::new(Arc::new(Config::parse("config/config.toml.tests"))).unwrap();

        assert_eq!(
            Some(vec![
                (
                    header::ACCESS_CONTROL_ALLOW_ORIGIN,
                    "https://reddit.com".parse().unwrap()
                ),
                (
                    header::ACCESS_CONTROL_ALLOW_METHODS,
                    "HERP,DERP".parse().unwrap()
                ),
                (
                    header::ACCESS_CONTROL_ALLOW_HEADERS,
                    "Content-Type, Content-Length".parse().unwrap()
                ),
            ]),
            cors.headers_for("2", "https://reddit.com")
        );
    }

    #[test]
    fn headers_for_existing_app_from_wrong_origin() {
        let cors = Cors::new(Arc::new(Config::parse("config/config.toml.tests"))).unwrap();

        assert!(cors.headers_for("2", "https://facebook.com").is_none());
    }

    #[test]
    fn headers_for_non_existing_app() {
        let cors = Cors::new(Arc::new(Config::parse("config/config.toml.tests"))).unwrap();

        assert!(cors.headers_for("3", "https://www.google.fi").is_none());
    }
}
