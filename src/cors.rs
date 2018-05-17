use std::{
    collections::{HashMap, HashSet},
    iter::FromIterator,
};
use http::{
    response::Builder,
    Response,
    header::{self, HeaderValue}
};

use events::input::Platform;

use ::CONFIG;

pub struct Cors {
    allowed_methods: String,
    allowed_headers: String,
    allowed_origins: HashMap<String, HashSet<String>>
}

impl Cors {
    pub fn new() -> Option<Cors> {
        CONFIG.cors.as_ref().map(|ref cors_config| {
            let allowed_origins: HashMap<String, HashSet<String>> = CONFIG.origins
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

    pub fn valid_origin(
        &self,
        app_id: &str,
        origin: Option<&str>,
    ) -> bool
    {
        if let Some(origin) = origin {
            match self.allowed_origins.get(app_id) {
                Some(app_origins) if app_origins.contains(origin) => true,
                _ => false
            }
        } else {
            false
        }
    }

    pub fn response_builder_origin(
        &self,
        app_id: &str,
        origin: Option<&str>,
        platform: &Platform
    ) -> Builder
    {
        let mut builder = Response::builder();

        if self.valid_origin(app_id, origin) && (platform == &Platform::Web) {
            builder.header(
                header::ACCESS_CONTROL_ALLOW_ORIGIN,
                HeaderValue::from_str(origin.unwrap()).unwrap(),
            );

            builder.header(
                header::ACCESS_CONTROL_ALLOW_METHODS,
                HeaderValue::from_str(self.allowed_methods.as_ref()).unwrap(),
            );

            builder.header(
                header::ACCESS_CONTROL_ALLOW_HEADERS,
                HeaderValue::from_str(self.allowed_headers.as_ref()).unwrap(),
            );
        }

        builder
    }

    pub fn response_builder_wildcard(
        &self,
    ) -> Builder
    {
        let mut builder = Response::builder();

        builder.header(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            HeaderValue::from_static("*"),
        );

        builder.header(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_str(self.allowed_methods.as_ref()).unwrap(),
        );

        builder.header(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_str(self.allowed_headers.as_ref()).unwrap(),
        );

        builder
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use events::input::Platform;

    #[test]
    fn new_with_cors_config() {
        let cors = Cors::new();
        assert!(cors.is_some());
    }

    #[test]
    fn wildcard_headers() {
        let cors = Cors::new().unwrap();

        let response = cors.response_builder_wildcard().body("").unwrap();

        let headers = response.headers();

        assert_eq!(
            "*",
            headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap()
        );

        assert_eq!(
            "HERP,DERP",
            headers.get(header::ACCESS_CONTROL_ALLOW_METHODS).unwrap()
        );

        assert_eq!(
            "Content-Type, Content-Length",
            headers.get(header::ACCESS_CONTROL_ALLOW_HEADERS).unwrap()
        );
    }

    #[test]
    fn headers_for_existing_app_from_allowed_origin() {
        let cors = Cors::new().unwrap();

        let response = cors.response_builder_origin(
            "2",
            Some("https://reddit.com"),
            &Platform::Web
        ).body("").unwrap();

        let headers = response.headers();

        assert_eq!(
            "https://reddit.com",
            headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap()
        );

        assert_eq!(
            "HERP,DERP",
            headers.get(header::ACCESS_CONTROL_ALLOW_METHODS).unwrap()
        );

        assert_eq!(
            "Content-Type, Content-Length",
            headers.get(header::ACCESS_CONTROL_ALLOW_HEADERS).unwrap()
        );

    }

    #[test]
    fn headers_for_existing_app_from_wrong_origin() {
        let cors = Cors::new().unwrap();

        let response = cors.response_builder_origin(
            "2",
            Some("https://facebook.com"),
            &Platform::Web
        ).body("").unwrap();

        let headers = response.headers();

        assert!(headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none());
    }

    #[test]
    fn headers_for_non_existing_app() {
        let cors = Cors::new().unwrap();

        let response = cors.response_builder_origin(
            "3",
            Some("https://reddit.com"),
            &Platform::Web
        ).body("").unwrap();

        let headers = response.headers();

        assert!(headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none());
    }
}
