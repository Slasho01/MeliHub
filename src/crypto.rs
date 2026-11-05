//! Cifrado en reposo de datos sensibles (tokens de Mercado Libre). AES-256-GCM
//! con la clave derivada de `TOKEN_ENC_KEY`. Si no hay clave, es passthrough
//! (texto plano), y `descifrar` reconoce el texto plano antiguo por el prefijo.

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, Nonce};
use base64::{engine::general_purpose::STANDARD, Engine};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

const PREFIJO: &str = "enc:";
static CIPHER: OnceLock<Option<Aes256Gcm>> = OnceLock::new();

fn cipher() -> &'static Option<Aes256Gcm> {
    CIPHER.get_or_init(|| {
        let k = std::env::var("TOKEN_ENC_KEY").ok().filter(|s| !s.trim().is_empty())?;
        let key_bytes = Sha256::digest(k.as_bytes()); // 32 bytes
        Some(Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes)))
    })
}

/// Cifra un texto. Sin `TOKEN_ENC_KEY`, devuelve el texto tal cual.
pub fn cifrar(plain: &str) -> String {
    let Some(c) = cipher() else { return plain.to_string() };
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    match c.encrypt(&nonce, plain.as_bytes()) {
        Ok(ct) => {
            let mut blob = nonce.to_vec();
            blob.extend_from_slice(&ct);
            format!("{PREFIJO}{}", STANDARD.encode(blob))
        }
        Err(_) => plain.to_string(),
    }
}

/// Descifra un texto. Si no tiene el prefijo (texto plano antiguo), lo devuelve igual.
pub fn descifrar(s: &str) -> String {
    let Some(rest) = s.strip_prefix(PREFIJO) else { return s.to_string() };
    let Some(c) = cipher() else { return s.to_string() };
    let Ok(blob) = STANDARD.decode(rest) else { return s.to_string() };
    if blob.len() < 12 {
        return s.to_string();
    }
    let (nonce_bytes, ct) = blob.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    match c.decrypt(nonce, ct) {
        Ok(pt) => String::from_utf8_lossy(&pt).to_string(),
        Err(_) => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_y_passthrough() {
        std::env::set_var("TOKEN_ENC_KEY", "clave-de-prueba-123");
        let original = "APP_USR-token-secreto-xyz";
        let enc = cifrar(original);
        assert!(enc.starts_with("enc:"), "debe ir cifrado con prefijo");
        assert_ne!(enc, original);
        assert_eq!(descifrar(&enc), original, "descifra al original");
        // Texto plano antiguo (sin prefijo) se devuelve igual.
        assert_eq!(descifrar("token-plano-viejo"), "token-plano-viejo");
    }
}
