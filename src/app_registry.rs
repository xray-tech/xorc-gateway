use std::{
    io,
    thread,
    time::Duration,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    collections::HashMap,
};

use base64;
use config::Config;
use ring::{hmac, digest};
use error::Error;
use events::input::Platform;
use headers::DeviceHeaders;
use crossbeam::sync::ArcCell;
use r2d2;
use r2d2_postgres::{PostgresConnectionManager, TlsMode};
use hex;
use ::GLOG;

pub struct Application {
    pub id: i32,
    pub token: Option<String>,
    pub ios_secret: Option<hmac::VerificationKey>,
    pub android_secret: Option<hmac::VerificationKey>,
    pub web_secret: Option<hmac::VerificationKey>,
}

pub struct AppRegistry {
    allow_empty_signature: bool,
    apps: ArcCell<HashMap<String, Application>>,
    pool: Option<r2d2::Pool<PostgresConnectionManager>>,
    config: Arc<Config>,
}

lazy_static! {
    static ref APPS_QUERY: &'static str =
        indoc!("
            SELECT id, sdk_token,
                i.sdk_api_secret AS ios_secret,
                a.sdk_api_secret AS android_secret,
                w.sdk_api_secret AS web_secret
            FROM applications
            LEFT JOIN ios_applications i
                ON i.application_id = applications.id
            LEFT JOIN android_applications a
                ON a.application_id = applications.id
            LEFT JOIN web_applications w
                ON w.application_id = applications.id
            WHERE deleted_at IS NULL
        ");
}

impl AppRegistry {
    pub fn new(config: Arc<Config>) -> AppRegistry {
        if let Some(psql_config) = config.clone().postgres.as_ref() {
            info!("Apps loaded form PostgreSQL CRM database.");

            let manager = PostgresConnectionManager::new(
                psql_config.uri.as_str(),
                TlsMode::None
            ).expect("Couldn't connect to PostgreSQL");

            let pool = r2d2::Builder::new()
                .max_size(psql_config.pool_size)
                .min_idle(Some(psql_config.min_idle))
                .idle_timeout(Some(Duration::from_millis(psql_config.idle_timeout)))
                .max_lifetime(Some(Duration::from_millis(psql_config.max_lifetime)))
                .build(manager).expect("Couldn't create a PostgreSQL connection pool");

            let registry = AppRegistry {
                allow_empty_signature: config.gateway.allow_empty_signature,
                config: config,
                pool: Some(pool),
                apps: ArcCell::new(Arc::new(HashMap::new())),
            };

            registry.update_apps().unwrap();

            registry
        } else {
            warn!("Apps loaded form configuration file. Development only!");

            let apps = config.test_apps
                .iter()
                .fold(HashMap::new(), |mut acc, test_app| {
                    let ios_secret = test_app
                        .secret_ios
                        .clone()
                        .map(|s| s.as_bytes().to_vec());

                    let android_secret = test_app
                        .secret_android
                        .clone()
                        .map(|s| s.as_bytes().to_vec());

                    let web_secret = test_app
                        .secret_web
                        .clone()
                        .map(|s| s.as_bytes().to_vec());

                    let app = Self::create_app(
                        test_app.app_id,
                        test_app.token.clone(),
                        ios_secret,
                        android_secret,
                        web_secret,
                    );

                    acc.insert(format!("{}", test_app.app_id), app);

                    acc
                });

            AppRegistry {
                allow_empty_signature: config.gateway.allow_empty_signature,
                config: config,
                pool: None,
                apps: ArcCell::new(Arc::new(apps)),
            }
        }
    }

    pub fn validate(
        &self,
        app_id: &str,
        device_headers: &DeviceHeaders,
        platform: &Platform,
        raw_data: &[u8],
    ) -> Result<(), Error>
    {
        let apps = self.apps.get();

        let app = apps
            .get(app_id)
            .ok_or(Error::AppDoesNotExist)?;

        let valid_token = app
            .token
            .as_ref()
            .unwrap_or(&self.config.gateway.default_token);

        let sent_token = device_headers
            .api_token
            .as_ref()
            .ok_or(Error::MissingToken)?;

        if sent_token != valid_token { return Err(Error::InvalidToken) }

        if self.allow_empty_signature {
            warn!("Skipped signature checks because of configuration. Use only on development!");
            return Ok(())
        }

        let signature = device_headers
            .signature
            .as_ref()
            .ok_or(Error::MissingSignature)?;

        let platform_key = match platform {
            Platform::Ios     => app.ios_secret.as_ref(),
            Platform::Android => app.android_secret.as_ref(),
            Platform::Web     => app.web_secret.as_ref(),
            _                 => None,
        }.ok_or(Error::AppDoesNotExist)?;

        let decoded_signature = base64::decode(signature.as_bytes())
            .map_err(|_| Error::InvalidSignature)?;

        hmac::verify(
            &platform_key,
            raw_data,
            &decoded_signature,
        ).map_err(|_| Error::InvalidSignature)
    }

    pub fn run_updater(&self, control: Arc<AtomicBool>) {
        while control.load(Ordering::Relaxed) {
            if let Err(e) = self.update_apps() {
                error!(
                    "Error updating application data from PostgreSQL, ignoring: [{:?}]",
                    e
                );
            };

            thread::park_timeout(Duration::from_secs(60));
        }
    }

    fn create_key(
        app_id: i32,
        column: &'static str,
        s: &Vec<u8>
    ) -> Option<hmac::VerificationKey> {
        hex::decode(s).and_then(|decoded| {
            Ok(hmac::VerificationKey::new(
                &digest::SHA512,
                &decoded,
            ))
        }).or_else(|e| {
            error!(
                "Error converting {} for app {}",
                column,
                app_id
            );

            Err(e)
        }).ok()
    }

    fn create_app(
        id: i32,
        token: Option<String>,
        ios_secret: Option<Vec<u8>>,
        android_secret: Option<Vec<u8>>,
        web_secret: Option<Vec<u8>>,
    ) -> Application
    {
        let ios_key = ios_secret
            .as_ref()
            .and_then(|s| Self::create_key(id, "ios_secret", s));

        let android_key = android_secret
            .as_ref()
            .and_then(|s| Self::create_key(id, "android_secret", s));

        let web_key = web_secret
            .as_ref()
            .and_then(|s| Self::create_key(id, "web_secret", s));

        Application {
            id: id,
            token: token,
            ios_secret: ios_key,
            android_secret: android_key,
            web_secret: web_key,
        }
    }

    fn update_apps(&self) -> Result<(), io::Error> {
        if let Some(ref pool) = self.pool {
            let connection = pool.get()
                .map_err(|_| io::Error::new(io::ErrorKind::ConnectionAborted, "pool död"))?;

            let apps = connection.query(&APPS_QUERY, &[]).map(|rows| {
                rows.iter().fold(HashMap::new(), |mut acc, row| {
                    let id = row.get("id");

                    let app = Self::create_app(
                        id,
                        row.get("sdk_token"),
                        row.get("ios_secret"),
                        row.get("android_secret"),
                        row.get("web_secret"),
                    );

                    let _ = GLOG.log_app_update(&app);
                    acc.insert(format!("{}", id), app);

                    acc
                })
            }).map_err(|_| io::Error::new(io::ErrorKind::ConnectionAborted, "query död"))?;

            self.swap_apps(apps);
        } else {
            warn!("No PostgreSQL connection defined, registry update dysfunctional");
        }

        Ok(())
    }

    fn swap_apps(&self, apps: HashMap<String, Application>) {
        self.apps.set(Arc::new(apps));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use error::Error;
    use hyper::HeaderMap;
    use http::header::HeaderValue;
    use headers::DeviceHeaders;
    use events::Platform;

    const TOKEN: &'static str =
        "46732a28cd445366c6c8dcbd57500af4e69597c8ebe224634d6ccab812275c9c";
    const IOS_SECRET: &'static str =
        "1b66af517dd60807aeff8b4582d202ef500085bc0cec92bc3e67f0c58d6203b5";
    const ANDROID_SECRET: &'static str =
        "d685e53ae50c945e5ae4f36170d7213360a25ed91b91a647574aa384d2b6f901";
    const WEB_SECRET: &'static str =
        "4c553960fdc2a82f90b84f6ef188e836818fcee2c43a6c32bd6c91f41772657f";

    lazy_static! {
        static ref CONFIG: Arc<Config> =
            Arc::new(Config::parse("config/config.toml.tests"));

        static ref APP_REGISTRY: AppRegistry =
            AppRegistry::new(CONFIG.clone());
    }

    #[test]
    fn test_app_creation_empty_secrets() {
        let app = AppRegistry::create_app(420, None, None, None, None);

        assert_eq!(420, app.id);

        assert!(app.token.is_none());
        assert!(app.ios_secret.is_none());
        assert!(app.android_secret.is_none());
        assert!(app.web_secret.is_none());
    }

    #[test]
    fn test_app_creation_with_token() {
        let app = AppRegistry::create_app(
            420,
            Some(TOKEN.to_string()),
            None,
            None,
            None
        );

        assert_eq!(Some(TOKEN.to_string()), app.token);
    }

    #[test]
    fn test_app_creation_with_secrets() {
        let app = AppRegistry::create_app(
            420,
            None,
            Some(IOS_SECRET.as_bytes().to_vec()),
            Some(ANDROID_SECRET.as_bytes().to_vec()),
            Some(WEB_SECRET.as_bytes().to_vec()),
        );

        assert!(app.ios_secret.is_some());
        assert!(app.android_secret.is_some());
        assert!(app.web_secret.is_some());
    }

    /// Testing the validation of iOS signature against the sent data. The
    /// signature is generated from the `test-sdk.py` test script:
    ///
    /// ```python3
    /// import hmac
    /// import base64
    ///
    /// data = "kulli"
    /// secret = bytearray.fromhex('1b66af517dd60807aeff8b4582d202ef500085bc0cec92bc3e67f0c58d6203b5')
    /// base64.b64encode(hmac.new(secret, data.encode('utf-8'), "SHA512").digest())
    ///
    /// >> b'8iq7J8PjWZvkfzPDa0HbfwnlbNWTK6giMO2Z1vsUhToMY62rSJtdIHkFaMY+UDIWRjCbf+c5le3AAHVUlDJDRg=='
    /// ```
    ///
    /// AppRegistry loads the application settings from the config file.
    #[test]
    fn test_validate_ios_valid_data() {
        let mut header_map = HeaderMap::new();

        header_map.insert(
            "D360-Signature",
            HeaderValue::from_static(
                "8iq7J8PjWZvkfzPDa0HbfwnlbNWTK6giMO2Z1vsUhToMY62rSJtdIHkFaMY+UDIWRjCbf+c5le3AAHVUlDJDRg=="
            ),
        );
        header_map.insert(
            "D360-Api-Token",
            HeaderValue::from_static(TOKEN),
        );

        let device_headers = DeviceHeaders::from(&header_map);

        let validation = APP_REGISTRY.validate(
            "1",
            &device_headers,
            &Platform::Ios,
            "kulli".as_bytes()
        );

        assert_eq!(Ok(()), validation);
    }

    /// Testing the validation of Android signature against the sent data. The
    /// signature is generated from the `test-sdk.py` test script:
    ///
    /// ```python3
    /// import hmac
    /// import base64
    ///
    /// data = "kulli"
    /// secret = bytearray.fromhex('d685e53ae50c945e5ae4f36170d7213360a25ed91b91a647574aa384d2b6f901')
    /// base64.b64encode(hmac.new(secret, data.encode('utf-8'), "SHA512").digest())
    ///
    /// >>> b'2dTSkXn6Z+DCYpXNKgRV2oA+wHhvig98A0eXfKpxgDndXTAxYDfAxGrCmbU+AHL9O+zajCLBKZzqmitPnQJeGA=='
    /// ```
    ///
    /// AppRegistry loads the application settings from the config file.
    #[test]
    fn test_validate_android_valid_data() {
        let mut header_map = HeaderMap::new();

        header_map.insert(
            "D360-Signature",
            HeaderValue::from_static(
                "2dTSkXn6Z+DCYpXNKgRV2oA+wHhvig98A0eXfKpxgDndXTAxYDfAxGrCmbU+AHL9O+zajCLBKZzqmitPnQJeGA=="
            ),
        );

        header_map.insert(
            "D360-Api-Token",
            HeaderValue::from_static(TOKEN),
        );

        let device_headers = DeviceHeaders::from(&header_map);
        let validation = APP_REGISTRY.validate(
            "1",
            &device_headers,
            &Platform::Android,
            "kulli".as_bytes()
        );

        assert_eq!(Ok(()), validation);
    }

    /// Testing the validation of Web signature against the sent data. The
    /// signature is generated from the `test-sdk.py` test script:
    ///
    /// ```python3
    /// import hmac
    /// import base64
    ///
    /// data = "kulli"
    /// secret = bytearray.fromhex('4c553960fdc2a82f90b84f6ef188e836818fcee2c43a6c32bd6c91f41772657f')
    /// base64.b64encode(hmac.new(secret, data.encode('utf-8'), "SHA512").digest())
    /// >>> b'iamp0NMGsLvLTsoTSRRKQn4uTThETrkdk7hjCX0jqDXdjNyOv/tRK9C9cnPhi4IIvP4Fj/kP/5L8waXx3fokOg=='
    /// ```
    ///
    /// AppRegistry loads the application settings from the config file.
    #[test]
    fn test_validate_web_valid_data() {
        let mut header_map = HeaderMap::new();

        header_map.insert(
            "D360-Signature",
            HeaderValue::from_static(
                "iamp0NMGsLvLTsoTSRRKQn4uTThETrkdk7hjCX0jqDXdjNyOv/tRK9C9cnPhi4IIvP4Fj/kP/5L8waXx3fokOg=="
            ),
        );

        header_map.insert(
            "D360-Api-Token",
            HeaderValue::from_static(TOKEN),
        );

        let device_headers = DeviceHeaders::from(&header_map);
        let validation = APP_REGISTRY.validate(
            "1",
            &device_headers,
            &Platform::Web,
            "kulli".as_bytes()
        );

        assert_eq!(Ok(()), validation);
    }

    #[test]
    fn test_validate_invalid_app() {
        let mut header_map = HeaderMap::new();

        header_map.insert(
            "D360-Signature",
            HeaderValue::from_static(
                "iamp0NMGsLvLTsoTSRRKQn4uTThETrkdk7hjCX0jqDXdjNyOv/tRK9C9cnPhi4IIvP4Fj/kP/5L8waXx3fokOg=="
            ),
        );
        header_map.insert(
            "D360-Api-Token",
            HeaderValue::from_static(TOKEN),
        );

        let device_headers = DeviceHeaders::from(&header_map);

        assert_eq!(
            Err(Error::AppDoesNotExist),
            APP_REGISTRY.validate(
                "2",
                &device_headers,
                &Platform::Web,
                "kulli".as_bytes()
            )
        );
    }

    #[test]
    fn test_validate_missing_token() {
        let mut header_map = HeaderMap::new();

        header_map.insert(
            "D360-Signature",
            HeaderValue::from_static(
                "iamp0NMGsLvLTsoTSRRKQn4uTThETrkdk7hjCX0jqDXdjNyOv/tRK9C9cnPhi4IIvP4Fj/kP/5L8waXx3fokOg=="
            ),
        );

        let device_headers = DeviceHeaders::from(&header_map);

        assert_eq!(
            Err(Error::MissingToken),
            APP_REGISTRY.validate(
                "1",
                &device_headers,
                &Platform::Web,
                "kulli".as_bytes()
            )
        );
    }

    #[test]
    fn test_validate_invalid_token() {
        let mut header_map = HeaderMap::new();

        header_map.insert(
            "D360-Signature",
            HeaderValue::from_static(
                "iamp0NMGsLvLTsoTSRRKQn4uTThETrkdk7hjCX0jqDXdjNyOv/tRK9C9cnPhi4IIvP4Fj/kP/5L8waXx3fokOg=="
            ),
        );
        header_map.insert(
            "D360-Api-Token",
            HeaderValue::from_static("pylly"),
        );

        let device_headers = DeviceHeaders::from(&header_map);

        assert_eq!(
            Err(Error::InvalidToken),
            APP_REGISTRY.validate(
                "1",
                &device_headers,
                &Platform::Web,
                "kulli".as_bytes()
            )
        );
    }

    #[test]
    fn test_validate_missing_signature_if_not_allowed() {
        let mut header_map = HeaderMap::new();

        header_map.insert(
            "D360-Api-Token",
            HeaderValue::from_static(TOKEN),
        );

        let device_headers = DeviceHeaders::from(&header_map);
        let validation = APP_REGISTRY.validate(
            "1",
            &device_headers,
            &Platform::Web,
            "kulli".as_bytes()
        );

        assert_eq!(Err(Error::MissingSignature), validation);
    }

    #[test]
    fn test_validate_missing_signature_if_allowed() {
        let mut config = Config::parse("config/config.toml.tests");
        config.gateway.allow_empty_signature = true;
        let app_registry = AppRegistry::new(Arc::new(config));
        let mut header_map = HeaderMap::new();

        header_map.insert(
            "D360-Api-Token",
            HeaderValue::from_static(TOKEN),
        );

        let device_headers = DeviceHeaders::from(&header_map);
        let validation = app_registry.validate(
            "1",
            &device_headers,
            &Platform::Web,
            "kulli".as_bytes()
        );

        assert_eq!(Ok(()), validation);
    }

    #[test]
    fn test_validate_no_secret_set_for_platform() {
        let mut header_map = HeaderMap::new();

        header_map.insert(
            "D360-Signature",
            HeaderValue::from_static(
                "iamp0NMGsLvLTsoTSRRKQn4uTThETrkdk7hjCX0jqDXdjNyOv/tRK9C9cnPhi4IIvP4Fj/kP/5L8waXx3fokOg=="
            ),
        );
        header_map.insert(
            "D360-Api-Token",
            HeaderValue::from_static(TOKEN),
        );

        let device_headers = DeviceHeaders::from(&header_map);
        let validation = APP_REGISTRY.validate(
            "1",
            &device_headers,
            &Platform::Unknown,
            "kulli".as_bytes()
        );

        assert_eq!(Err(Error::AppDoesNotExist), validation);
    }

    #[test]
    fn test_validate_web_invalid_signature() {
        let mut header_map = HeaderMap::new();

        header_map.insert(
            "D360-Signature",
            HeaderValue::from_static(
                "iamp0NMGsLvLTsoTSRRKQn4uTThETrkdk7hjCX0jqDXdjNyOv/tRK9C9cnPhi4IIvP4Fj/kP/5L8waXx3fokOg=="
            ),
        );

        header_map.insert(
            "D360-Api-Token",
            HeaderValue::from_static(TOKEN),
        );

        let device_headers = DeviceHeaders::from(&header_map);
        let validation = APP_REGISTRY.validate(
            "1",
            &device_headers,
            &Platform::Android,
            "kulli".as_bytes()
        );

        assert_eq!(Err(Error::InvalidSignature), validation);
    }
}
