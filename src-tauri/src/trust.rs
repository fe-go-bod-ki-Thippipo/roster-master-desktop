use base64::{engine::general_purpose::STANDARD as B64,Engine};
use ed25519_dalek::{Signature,Verifier,VerifyingKey};

pub fn verify(public_key_b64:&str,signature_b64:&str,payload:&[u8])->Result<(),String>{
 let pb=B64.decode(public_key_b64).map_err(|_|"Trusted public key ไม่ถูกต้อง")?;
 let pa:[u8;32]=pb.try_into().map_err(|_|"Trusted public key length ไม่ถูกต้อง")?;
 let vk=VerifyingKey::from_bytes(&pa).map_err(|_|"Trusted public key ไม่ถูกต้อง")?;
 let sb=B64.decode(signature_b64).map_err(|_|"Signature ไม่ถูกต้อง")?;
 let sig=Signature::from_slice(&sb).map_err(|_|"Signature ไม่ถูกต้อง")?;
 vk.verify(payload,&sig).map_err(|_|"Package ไม่ได้ลงนามโดย Central ที่เชื่อถือ".to_string())
}
