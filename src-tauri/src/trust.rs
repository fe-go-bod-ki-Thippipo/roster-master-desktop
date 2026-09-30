use base64::{engine::general_purpose::STANDARD as B64, Engine};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;

pub const DOMAIN_PREFIX: &[u8] = b"RMPKG-PROVISION-v1\0";

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddedKey {
    pub key_id: String,
    pub env: String,
    pub public_key_b64: String,
}

const EMBEDDED_KEYS_JSON: &str = include_str!("../trust/alpha_central_keys.json");

pub fn embedded_keys() -> Result<Vec<EmbeddedKey>, String> {
    serde_json::from_str(EMBEDDED_KEYS_JSON).map_err(|_| "Embedded trust anchor ไม่ถูกต้อง".into())
}

pub fn trusted_key(key_id: &str) -> Result<EmbeddedKey, String> {
    embedded_keys()?
        .into_iter()
        .find(|k| k.key_id == key_id)
        .ok_or_else(|| "Package ลงนามด้วย key_id ที่ Unit ไม่เชื่อถือ".into())
}

fn verify_with_public_key(public_key_b64: &str, signature_b64: &str, payload: &[u8]) -> Result<(), String> {
    let pb = B64.decode(public_key_b64).map_err(|_| "Trusted public key ไม่ถูกต้อง")?;
    let pa: [u8; 32] = pb.try_into().map_err(|_| "Trusted public key length ไม่ถูกต้อง")?;
    let vk = VerifyingKey::from_bytes(&pa).map_err(|_| "Trusted public key ไม่ถูกต้อง")?;
    let sb = B64.decode(signature_b64).map_err(|_| "Signature ไม่ถูกต้อง")?;
    let sig = Signature::from_slice(&sb).map_err(|_| "Signature ไม่ถูกต้อง")?;
    let mut message = Vec::with_capacity(DOMAIN_PREFIX.len() + payload.len());
    message.extend_from_slice(DOMAIN_PREFIX);
    message.extend_from_slice(payload);
    vk.verify_strict(&message, &sig)
        .map_err(|_| "Package ไม่ได้ลงนามโดย Central ที่เชื่อถือ".into())
}

pub fn verify_envelope(key_id: &str, signature_b64: &str, payload: &[u8]) -> Result<EmbeddedKey, String> {
    let key = trusted_key(key_id)?;
    verify_with_public_key(&key.public_key_b64, signature_b64, payload)?;
    Ok(key)
}

#[cfg(test)]
pub fn verify_with_public_key_for_test(public_key_b64: &str, signature_b64: &str, payload: &[u8]) -> Result<(), String> {
    verify_with_public_key(public_key_b64, signature_b64, payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use rand_core::OsRng;

    #[test]
    fn domain_separation_and_strict_signature_are_enforced() {
        let sk = SigningKey::generate(&mut OsRng);
        let payload = br#"{"schema_version":2}"#;
        let mut message = DOMAIN_PREFIX.to_vec();
        message.extend_from_slice(payload);
        let sig = sk.sign(&message);
        let pk = B64.encode(sk.verifying_key().to_bytes());
        let sb = B64.encode(sig.to_bytes());
        assert!(verify_with_public_key_for_test(&pk, &sb, payload).is_ok());
        assert!(verify_with_public_key_for_test(&pk, &sb, br#"{"schema_version":3}"#).is_err());
    }
}
