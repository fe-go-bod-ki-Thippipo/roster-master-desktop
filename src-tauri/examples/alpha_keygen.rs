use base64::{engine::general_purpose::STANDARD as B64, Engine};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{env, fs};

#[derive(Serialize)]
struct AlphaKeyFile {
    key_id: String,
    env: &'static str,
    private_key_b64: String,
    public_key_b64: String,
}

fn main() {
    let output = env::args().nth(1).unwrap_or_else(|| "alpha-central.rmkey".into());
    if !output.ends_with(".rmkey") {
        panic!("output must use .rmkey extension");
    }
    let sk = SigningKey::generate(&mut OsRng);
    let vk: VerifyingKey = sk.verifying_key();
    let public = vk.to_bytes();
    let digest = Sha256::digest(public);
    let key_id = digest[..8].iter().map(|b| format!("{b:02x}")).collect::<String>();
    let file = AlphaKeyFile {
        key_id,
        env: "ALPHA",
        private_key_b64: B64.encode(sk.to_bytes()),
        public_key_b64: B64.encode(public),
    };
    let json = serde_json::to_string_pretty(&file).expect("serialize key");
    fs::write(&output, json).expect("write .rmkey");
    println!("Created {output}. Keep this private file offline; never commit it.");
}
