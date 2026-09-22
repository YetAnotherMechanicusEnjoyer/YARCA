use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
use hex::{decode, encode};
use rand::{Rng, rng};

pub fn encrypt(plaintext: &str, key: &[u8; 32]) -> (String, String) {
    let mut rng = rng();
    let nonce_bytes: [u8; 12] = rng.random();
    let cipher = Aes256Gcm::new_from_slice(key).expect("Cipher failed.");
    let nonce = Nonce::from_slice(&nonce_bytes);

    let cipher_text = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .expect("Encryption failed.");
    (encode(nonce_bytes), encode(&cipher_text))
}

pub fn decrypt(nonce_hex: &str, ciphertext_hex: &str, key: &[u8; 32]) -> Option<String> {
    let cipher = Aes256Gcm::new_from_slice(key).expect("Cipher failed.");

    let nonce_bytes = decode(nonce_hex).ok()?;
    let ciphertext_bytes = decode(ciphertext_hex).ok()?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    match cipher.decrypt(nonce, ciphertext_bytes.as_slice()) {
        Ok(plaintext_bytes) => String::from_utf8(plaintext_bytes).ok(),
        Err(_) => None,
    }
}
