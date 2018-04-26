use ring::{aead, error};

pub struct IDCheck {
    opening_key: aead::OpeningKey,
}

impl IDCheck {
    pub fn new(secret: &str) -> IDCheck {
        let opening_key = aead::OpeningKey::new(
            &aead::AES_256_GCM,
            secret.as_bytes(),
        ).unwrap()?;

        IDCheck { opening_key }
    }

    pub fn decrypt(ciphertext: &[u8], decrypted_message: &mut [u8]) -> Result<Vec<u8>, error::Unknown> {
        aead::open_in_place(
            &self.opening_key,
            ciphertext,
            &[],
            0,
            decrypted_message,
        )
    }
}
