//! Encrypts the Bancho bots' IRC passwords for the database. The bots need the plaintext
//! to log in, so the passwords can't be hashed; with them encrypted, a database dump
//! alone doesn't give the bot accounts away. The key (BOT_SECRET_KEY) is only in the
//! environment.

use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305, Key, Nonce,
};

// A password as it's stored.
pub struct Sealed {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

pub struct BotSecret(ChaCha20Poly1305);

impl BotSecret {
    // From 64 hex digits (32 bytes); None for anything else.
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim();
        if hex.len() != 64 || !hex.is_ascii() {
            return None;
        }
        let mut key = [0u8; 32];
        for (i, byte) in key.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
        }
        Some(Self(ChaCha20Poly1305::new(Key::from_slice(&key))))
    }

    pub fn seal(&self, password: &str) -> Sealed {
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = self
            .0
            .encrypt(&nonce, password.as_bytes())
            .expect("encrypting a short password can't fail");
        Sealed {
            nonce: nonce.to_vec(),
            ciphertext,
        }
    }

    // None when it was sealed with another key, or was tampered with.
    pub fn open(&self, nonce: &[u8], ciphertext: &[u8]) -> Option<String> {
        if nonce.len() != 12 {
            return None;
        }
        let plain = self.0.decrypt(Nonce::from_slice(nonce), ciphertext).ok()?;
        String::from_utf8(plain).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

    #[test]
    fn seals_and_opens() {
        let secret = BotSecret::from_hex(KEY).unwrap();
        let sealed = secret.seal("hunter2");
        assert_ne!(sealed.ciphertext, b"hunter2");
        assert_eq!(
            secret.open(&sealed.nonce, &sealed.ciphertext).as_deref(),
            Some("hunter2")
        );
        // A fresh nonce each time.
        assert_ne!(secret.seal("hunter2").nonce, sealed.nonce);
    }

    #[test]
    fn another_key_cant_open_it() {
        let sealed = BotSecret::from_hex(KEY).unwrap().seal("hunter2");
        let other = BotSecret::from_hex(&"ab".repeat(32)).unwrap();
        assert_eq!(other.open(&sealed.nonce, &sealed.ciphertext), None);
    }

    #[test]
    fn needs_64_hex_digits() {
        assert!(BotSecret::from_hex("abcd").is_none());
        assert!(BotSecret::from_hex(&"zz".repeat(32)).is_none());
        assert!(BotSecret::from_hex(&format!(" {KEY}\n")).is_some());
    }
}
