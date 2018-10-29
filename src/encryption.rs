use ring::{aead, error};
use rand::{RngCore, thread_rng};
use std::{env, fmt};
use base64;
use serde::{
    ser::Serialize,
    Serializer,
};

use ::{RUST_ENV};

lazy_static! {
    static ref SECRET: Vec<u8> =
        if let Ok(ref secret) = env::var("SECRET") {
            base64::decode_config(secret, base64::URL_SAFE_NO_PAD).unwrap()
        } else {
            if &*RUST_ENV != "development" {
                panic!("Please set SECRET environment variable.")
            }

            vec![129, 164, 171, 19, 88, 96, 172, 49, 218, 122, 106, 79, 226, 124,
                 112, 233, 172, 165, 64, 54, 31, 139, 249, 226, 199, 148, 8, 27,
                 76, 91, 164, 146]
        };

    static ref OPENING_KEY: aead::OpeningKey =
        aead::OpeningKey::new(&aead::AES_256_GCM, &SECRET).unwrap();

    static ref SEALING_KEY: aead::SealingKey =
        aead::SealingKey::new(&aead::AES_256_GCM, &SECRET).unwrap();
}

#[derive(Debug, PartialEq, Clone)]
pub struct Ciphertext {
    value: String,
}

impl Ciphertext {
    /// Encrypt a device ID with AES 256 GCM encryption.
    pub fn encrypt(cleartext: &Cleartext) -> Ciphertext {
        // Always different and random
        let mut nonce = [0u8; 12];
        thread_rng().fill_bytes(&mut nonce);

        // 36 characters for the data, 16 for suffix
        let mut ciphertext = [0u8; 52];

        // Cleartext id in the beginning
        for (i, c) in cleartext.as_ref().as_bytes().iter().enumerate() {
            ciphertext[i] = *c;
        }

        // Seal with the nonce, setting 16 characters as suffix, encrypting with
        // `SECRET`
        aead::seal_in_place(
            &SEALING_KEY,
            &nonce,
            &[],
            &mut ciphertext,
            16,
        ).unwrap();

        // First 12 characters for nonce, the last 52 for the ciphertext
        let mut payload = [0u8; 64];

        for (i, c) in nonce.iter().enumerate() {
            payload[i] = *c;
        }

        for (i, c) in ciphertext.iter().enumerate() {
            payload[i + 12] = *c;
        }

        Ciphertext {
            value: base64::encode(payload.as_ref()),
        }
    }
}

impl From<String> for Ciphertext {
    fn from(value: String) -> Ciphertext {
        Ciphertext { value }
    }
}

impl<'a> From<&'a str> for Ciphertext {
    fn from(value: &'a str) -> Ciphertext {
        Ciphertext { value: value.to_string() }
    }
}

impl Into<String> for Ciphertext {
    fn into(self) -> String {
        self.value
    }
}

impl AsRef<str> for Ciphertext {
    fn as_ref(&self) -> &str {
        self.value.as_ref()
    }
}

impl fmt::Display for Ciphertext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl Serialize for Ciphertext {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_ref())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Cleartext {
    value: String
}

impl Cleartext {
    /// Decrypt a device ID with AES 256 GCM encryption.
    pub fn decrypt(
        ciphertext: &Ciphertext
    ) -> Result<Cleartext, error::Unspecified>
    {
        // 64 characters of encrypted data, first 12 for the nonce, last for the device id
        let mut decoded = base64::decode(ciphertext.as_ref()).map_err(|_| error::Unspecified)?;
        let (nonce, mut cipher) = decoded.split_at_mut(12);

        // Open with the nonce we generated with `encrypt` and the secret key
        let decrypted_content = aead::open_in_place(
            &OPENING_KEY,
            &nonce,
            &[],
            0,
            &mut cipher,
        )?;

        let value: String = String::from_utf8_lossy(&decrypted_content).into();

        Ok(Cleartext { value })
    }

    pub fn into_string(self) -> String {
        self.value
    }
}

impl From<String> for Cleartext {
    fn from(value: String) -> Cleartext {
        Cleartext { value }
    }
}

impl<'a> From<&'a str> for Cleartext {
    fn from(value: &'a str) -> Cleartext {
        Cleartext { value: value.to_string() }
    }
}

impl Into<String> for Cleartext {
    fn into(self) -> String {
        self.value
    }
}

impl AsRef<str> for Cleartext {
    fn as_ref(&self) -> &str {
        self.value.as_ref()
    }
}

impl fmt::Display for Cleartext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl Serialize for Cleartext {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_ref())
    }
}
