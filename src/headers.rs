use uuid::Uuid;
use hyper::HeaderMap;
use ring::{aead, error};
use rand::{RngCore, thread_rng};
use std::env;
use gelf::Level;
use base64;

use ::GLOG;

lazy_static! {
    static ref OPENING_KEY: aead::OpeningKey =
        if let Ok(ref secret) = env::var("SECRET") {
            aead::OpeningKey::new(
                &aead::AES_256_GCM,
                &base64::decode_config(secret, base64::URL_SAFE_NO_PAD).unwrap(),
            ).unwrap()
        } else {
            panic!("No secret given, please set it in SECRET in base64 format (url safe, no pad)");
        };

    static ref SEALING_KEY: aead::SealingKey =
        if let Ok(ref secret) = env::var("SECRET") {
            aead::SealingKey::new(
                &aead::AES_256_GCM,
                &base64::decode_config(secret, base64::URL_SAFE_NO_PAD).unwrap(),
            ).unwrap()
        } else {
            panic!("No secret given, please set it in SECRET in base64 format (url safe, no pad)");
        };
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
    pub encrypted: Option<String>,
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
        assert_eq!(36, cleartext.len());

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

    fn decrypt_aead(encrypted: &str) -> Result<String, error::Unspecified> {
        let mut decoded = base64::decode(encrypted).map_err(|_| error::Unspecified)?;
        let (nonce, mut cipher) = decoded.split_at_mut(12);

        let decrypted_content = aead::open_in_place(
            &OPENING_KEY,
            &nonce,
            &[],
            0,
            &mut cipher,
        ).unwrap();

        Ok(String::from_utf8_lossy(&decrypted_content).into())
    }
}

impl<'a> From<&'a HeaderMap> for DeviceHeaders {
    fn from(headers: &HeaderMap) -> DeviceHeaders {
        let mut device_headers = DeviceHeaders {
            api_token: Self::get_value(headers, "D360-Api-Token"),
            device_id: DeviceId {
                encrypted: None,
                cleartext: None,
            },
            signature: Self::get_value(headers, "D360-Signature"),
            ip: Self::get_value(headers, "X-Real-IP"),
        };

        match Self::get_value(headers, "D360-Device-Id") {
            Some(encrypted) => {
                let decryption = Self::decrypt_aead(&encrypted);
                device_headers.device_id.encrypted = Some(encrypted);

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
                device_headers.device_id.encrypted = Some(Self::encrypt_aead(&cleartext));
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

