use uuid::Uuid;
use hyper::HeaderMap;
use ring::{aead, error};
use rand::{RngCore, thread_rng};
use std::env;
use gelf::Level;
use base64;

use ::GLOG;

lazy_static! {
    static ref SECRET: Vec<u8> =
        if let Ok(ref secret) = env::var("SECRET") {
            base64::decode_config(secret, base64::URL_SAFE_NO_PAD).unwrap()
        } else {
            warn!("No secret given, please set it in SECRET in base64 format (url safe, no pad)");

            vec![129, 164, 171, 19, 88, 96, 172, 49, 218, 122, 106, 79, 226, 124,
                 112, 233, 172, 165, 64, 54, 31, 139, 249, 226, 199, 148, 8, 27,
                 76, 91, 164, 146]
        };

    static ref OPENING_KEY: aead::OpeningKey =
        aead::OpeningKey::new(&aead::AES_256_GCM, &SECRET).unwrap();

    static ref SEALING_KEY: aead::SealingKey =
        aead::SealingKey::new(&aead::AES_256_GCM, &SECRET).unwrap();
}

#[derive(Debug)]
pub struct DeviceHeaders {
    pub api_token: Option<String>,
    pub device_id: DeviceId,
    pub signature: Option<String>,
    pub ip: Option<String>,
}

#[derive(Debug)]
pub struct DeviceId {
    pub ciphertext: Option<String>,
    pub cleartext: Option<String>,
}

impl DeviceHeaders {
    fn get_value(headers: &HeaderMap, key: &'static str) -> Option<String> {
        headers
            .get(key)
            .and_then(|h| h.to_str().ok())
            .map(|s| String::from(s))
    }

    fn encrypt_aead(cleartext: &str) -> String {
        let mut nonce = [0u8; 12];
        thread_rng().fill_bytes(&mut nonce);

        let mut ciphertext = [0u8; 52];

        for (i, c) in cleartext.as_bytes().iter().enumerate() {
            ciphertext[i] = *c;
        }

        aead::seal_in_place(
            &SEALING_KEY,
            &nonce,
            &[],
            &mut ciphertext,
            16,
        ).unwrap();

        let mut payload = [0u8; 64];

        for (i, c) in nonce.iter().enumerate() {
            payload[i] = *c;
        }

        for (i, c) in ciphertext.iter().enumerate() {
            payload[i + 12] = *c;
        }

        base64::encode(payload.as_ref())
    }

    fn decrypt_aead(ciphertext: &str) -> Result<String, error::Unspecified> {
        let mut decoded = base64::decode(ciphertext).map_err(|_| error::Unspecified)?;
        let (nonce, mut cipher) = decoded.split_at_mut(12);

        let decrypted_content = aead::open_in_place(
            &OPENING_KEY,
            &nonce,
            &[],
            0,
            &mut cipher,
        )?;

        Ok(String::from_utf8_lossy(&decrypted_content).into())
    }
}

impl<'a> From<&'a HeaderMap> for DeviceHeaders {
    fn from(headers: &HeaderMap) -> DeviceHeaders {
        let mut device_headers = DeviceHeaders {
            api_token: Self::get_value(headers, "D360-Api-Token"),
            device_id: DeviceId {
                ciphertext: None,
                cleartext: None,
            },
            signature: Self::get_value(headers, "D360-Signature"),
            ip: Self::get_value(headers, "X-Real-IP"),
        };

        match Self::get_value(headers, "D360-Device-Id") {
            Some(ciphertext) => {
                let decryption = Self::decrypt_aead(&ciphertext);
                device_headers.device_id.ciphertext = Some(ciphertext);

                if let Ok(cleartext) = decryption {
                    device_headers.device_id.cleartext = Some(cleartext);
                } else {
                    let _ = GLOG.log_with_headers(
                        "Invalid Device ID",
                        Level::Error,
                        &device_headers,
                    );
                };
            },
            _ => {
                let mut uuid = [0u8; 16];
                thread_rng().fill_bytes(&mut uuid);

                let cleartext = Uuid::new_v4().hyphenated().to_string();
                device_headers.device_id.ciphertext = Some(Self::encrypt_aead(&cleartext));
                device_headers.device_id.cleartext = Some(cleartext);

                let _ = GLOG.log_with_headers(
                    "Created a new Device ID",
                    Level::Informational,
                    &device_headers,
                );
            },
        };

        device_headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::HeaderMap;
    use http::header::HeaderValue;

    #[test]
    fn test_empty_ip_address() {
        let header_map = HeaderMap::new();
        let device_headers = DeviceHeaders::from(&header_map);

        assert!(device_headers.ip.is_none());
    }

    #[test]
    fn test_existing_ip_address() {
        let mut header_map = HeaderMap::new();
        let ip = "1.1.1.1";

        header_map.insert(
            "X-Real-IP",
            HeaderValue::from_static(ip),
        );

        let device_headers = DeviceHeaders::from(&header_map);

        assert_eq!(device_headers.ip, Some(ip.to_string()));
    }

    #[test]
    fn test_empty_api_token() {
        let header_map = HeaderMap::new();
        let device_headers = DeviceHeaders::from(&header_map);

        assert!(device_headers.api_token.is_none());
    }

    #[test]
    fn test_existing_api_token() {
        let mut header_map = HeaderMap::new();
        let token = "some-token";

        header_map.insert(
            "D360-Api-Token",
            HeaderValue::from_static(token),
        );

        let device_headers = DeviceHeaders::from(&header_map);

        assert_eq!(device_headers.api_token, Some(token.to_string()));
    }

    #[test]
    fn test_empty_signature() {
        let header_map = HeaderMap::new();
        let device_headers = DeviceHeaders::from(&header_map);

        assert!(device_headers.signature.is_none());
    }

    #[test]
    fn test_existing_signature() {
        let mut header_map = HeaderMap::new();
        let signature = "some-signature";

        header_map.insert(
            "D360-Signature",
            HeaderValue::from_static(signature),
        );

        let device_headers = DeviceHeaders::from(&header_map);

        assert_eq!(device_headers.signature, Some(signature.to_string()));
    }

    #[test]
    fn test_empty_device_id() {
        let header_map = HeaderMap::new();
        let device_headers = DeviceHeaders::from(&header_map);
        let device_id = device_headers.device_id;

        assert!(device_id.ciphertext.is_some());
        assert!(device_id.cleartext.is_some());
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

        let device_headers = DeviceHeaders::from(&header_map);
        let device_id = device_headers.device_id;

        assert_eq!(device_id.ciphertext, Some(cipher.to_string()));
        assert_eq!(device_id.cleartext, Some(clear.to_string()));
    }

    #[test]
    fn test_faulty_device_id() {
        let mut header_map = HeaderMap::new();
        let cipher = "THIS_IS_FAKED";

        header_map.insert(
            "D360-Device-Id",
            HeaderValue::from_static(cipher),
        );

        let device_headers = DeviceHeaders::from(&header_map);
        let device_id = device_headers.device_id;

        assert_eq!(device_id.ciphertext, Some(cipher.to_string()));
        assert!(device_id.cleartext.is_none());
    }
}
