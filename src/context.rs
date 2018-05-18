use uuid::Uuid;
use hyper::HeaderMap;
use rand::{RngCore, thread_rng};
use http::{header::{self, AsHeaderName}};
use encryption::{Ciphertext, Cleartext};
use events::input::Platform;
use std::net::IpAddr;

#[derive(Debug, PartialEq, Clone)]
pub struct Context {
    pub app_id: String,
    pub platform: Platform,
    pub api_token: Option<String>,
    pub device_id: DeviceId,
    pub signature: Option<String>,
    pub ip: Option<IpAddr>,
    pub origin: Option<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct DeviceId {
    pub ciphertext: Option<Ciphertext>,
    pub cleartext: Option<Cleartext>,
}

impl Context {
    /// Creates a structure of device information from request headers. If
    /// `D360-Device-Id` is not included in the request, uses the given closure
    /// to find a device id for the user. If the header exists, it's expected to
    /// be encrypted using AES-256-GCM AEAD encryption.
    ///
    /// Possibilities with the incoming device-id:
    ///
    /// - Exists and valid: unencrypted and stored to the struct and we should
    ///   continue
    /// - Exists but invalid: cleartext not stored to the struct, user should
    ///   get an error
    /// - Empty: try using the given closure to fetch the id, then encrypting
    ///   it. If closure doesn't give any id for the device, we generate one using
    ///   UUID version4 (random)
    pub fn new<F>(
        headers: &HeaderMap,
        app_id: &str,
        platform: Platform,
        fetch_device_id: F,
    ) -> Context
    where
        F: FnOnce() -> Option<String>
    {
        let device_id = match Self::get_value(&headers, "D360-Device-Id") {
            Some(ciphertext) => {
                let ciphertext = Ciphertext::from(ciphertext);
                let cleartext = Cleartext::decrypt(&ciphertext);

                DeviceId {
                    ciphertext: Some(ciphertext),
                    cleartext: cleartext.ok(),
                }
            }
            _ => {
                match fetch_device_id() {
                    Some(entity_id) => {
                        let cleartext = Cleartext::from(entity_id);
                        let ciphertext = Ciphertext::encrypt(&cleartext);

                        DeviceId {
                            ciphertext: Some(ciphertext),
                            cleartext: Some(cleartext),
                        }
                    },
                    _ => {
                        Self::create_new_id()
                    }
                }
            }
        };

        let ip_addr: Option<IpAddr> = headers
            .get("X-Real-IP")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse().ok());

        Context {
            api_token: Self::get_value(&headers, "D360-Api-Token"),
            app_id: String::from(app_id),
            platform: platform,
            device_id: device_id,
            signature: Self::get_value(&headers, "D360-Signature"),
            ip: ip_addr,
            origin: Self::get_value(&headers, header::ORIGIN)
        }
    }

    fn get_value<K>(
        headers: &HeaderMap,
        key: K
    ) -> Option<String>
    where
        K: AsHeaderName
    {
        headers
            .get(key)
            .and_then(|h| h.to_str().ok())
            .map(|s| String::from(s))
    }

    fn create_new_id() -> DeviceId {
        let mut uuid = [0u8; 16];
        thread_rng().fill_bytes(&mut uuid);

        let cleartext = Cleartext::from(
            Uuid::new_v4().hyphenated().to_string()
        );

        let ciphertext = Ciphertext::encrypt(&cleartext);

        DeviceId {
            ciphertext: Some(ciphertext),
            cleartext: Some(cleartext),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::HeaderMap;
    use http::header::HeaderValue;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_empty_ip_address() {
        let header_map = HeaderMap::new();
        let context = Context::new(&header_map, "123", Platform::Ios, || None);

        assert!(context.ip.is_none());
    }

    #[test]
    fn test_existing_ipv4_address() {
        let mut header_map = HeaderMap::new();
        let ip = "1.1.1.1";

        header_map.insert(
            "x-real-ip",
            HeaderValue::from_static(ip),
        );

        let context = Context::new(&header_map, "123", Platform::Ios, || None);

        assert_eq!(context.ip, Some(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
    }

    #[test]
    fn test_existing_ipv6_address() {
        let mut header_map = HeaderMap::new();
        let ip = "::1";

        header_map.insert(
            "x-real-ip",
            HeaderValue::from_static(ip),
        );

        let context = Context::new(&header_map, "123", Platform::Ios, || None);

        assert_eq!(context.ip, Some(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1))));
    }

    #[test]
    fn test_empty_api_token() {
        let header_map = HeaderMap::new();
        let context = Context::new(&header_map, "123", Platform::Ios, || None);

        assert!(context.api_token.is_none());
    }

    #[test]
    fn test_existing_api_token() {
        let mut header_map = HeaderMap::new();
        let token = "some-token";

        header_map.insert(
            "d360-api-token",
            HeaderValue::from_static(token),
        );

        let context = Context::new(&header_map, "123", Platform::Ios, || None);

        assert_eq!(context.api_token, Some(token.to_string()));
    }

    #[test]
    fn test_empty_signature() {
        let header_map = HeaderMap::new();
        let context = Context::new(&header_map, "123", Platform::Ios, || None);

        assert!(context.signature.is_none());
    }

    #[test]
    fn test_existing_signature() {
        let mut header_map = HeaderMap::new();
        let signature = "some-signature";

        header_map.insert(
            "D360-Signature",
            HeaderValue::from_static(signature),
        );

        let context = Context::new(&header_map, "123", Platform::Ios, || None);

        assert_eq!(context.signature, Some(signature.to_string()));
    }

    #[test]
    fn test_empty_origin() {
        let header_map = HeaderMap::new();
        let context = Context::new(&header_map, "123", Platform::Ios, || None);

        assert!(context.origin.is_none());
    }

    #[test]
    fn test_existing_origin() {
        let mut header_map = HeaderMap::new();
        let origin = "http://google.com";

        header_map.insert(
            "Origin",
            HeaderValue::from_static(origin),
        );

        let context = Context::new(&header_map, "123", Platform::Ios, || None);

        assert_eq!(context.origin, Some(origin.to_string()));
    }

    #[test]
    fn test_lowercase_origin() {
        let mut header_map = HeaderMap::new();
        let origin = "http://google.com";

        header_map.insert(
            "origin",
            HeaderValue::from_static(origin),
        );

        let context = Context::new(&header_map, "123", Platform::Ios, || None);

        assert_eq!(context.origin, Some(origin.to_string()));
    }

    #[test]
    fn test_empty_device_id() {
        let header_map = HeaderMap::new();
        let context = Context::new(&header_map, "123", Platform::Ios, || None);
        let device_id = context.device_id;

        assert!(device_id.ciphertext.is_some());
        assert!(device_id.cleartext.is_some());
    }

    #[test]
    fn test_empty_device_id_with_storage() {
        let header_map = HeaderMap::new();
        let context = Context::new(&header_map, "123", Platform::Ios, || Some("foobar".to_string()));
        let device_id = context.device_id;
        let cleartext = Cleartext::from("foobar");

        assert!(device_id.ciphertext.is_some());
        assert_eq!(device_id.cleartext, Some(cleartext));
    }

    #[test]
    fn test_existing_device_id() {
        let mut header_map = HeaderMap::new();
        let cipher = "PNslnKKJkbq8Nv5/C0CcoK7hnFsdltcW3yK/I0QYJ7bUX8EHx2/NX0r8OkJHC5lzY/cBwZ3FeeFmRRpxof+rtw==";
        let clear = "8f7f5c07-5eb2-4695-870c-065d886cdc9e";

        header_map.insert(
            "D360-Device-Id",
            HeaderValue::from_static(cipher),
        );

        let context = Context::new(&header_map, "123", Platform::Ios, || None);
        let device_id = context.device_id;

        assert_eq!(device_id.ciphertext, Some(Ciphertext::from(cipher)));
        assert_eq!(device_id.cleartext, Some(Cleartext::from(clear)));
    }

    #[test]
    fn test_faulty_device_id() {
        let mut header_map = HeaderMap::new();
        let cipher = "THIS_IS_FAKED";

        header_map.insert(
            "D360-Device-Id",
            HeaderValue::from_static(cipher),
        );

        let context = Context::new(&header_map, "123", Platform::Ios, || None);
        let device_id = context.device_id;

        assert_eq!(device_id.ciphertext, Some(Ciphertext::from(cipher)));
        assert!(device_id.cleartext.is_none());
    }
}
