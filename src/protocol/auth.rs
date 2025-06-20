use anyhow::Result;
use base64::Engine;
use bytes::{Buf as _, BufMut as _, Bytes, BytesMut};
use log::error;
use ring_compat::{
  signature::{
    Signer, Verifier,
    ed25519::{Signature, SigningKey, VerifyingKey},
  },
};

pub const PROTOCOL_REV: u32 = 1;

pub fn generate_keypair() -> ([u8; 32], [u8; 32]) {
  let ed25519_seed: [u8; 32] = rand::random();
  let keypair = SigningKey::from_bytes(&ed25519_seed);
  let pubkey: [u8; 32] = keypair.verifying_key().0;
  let privkey: [u8; 32] = keypair.to_bytes();
  (pubkey, privkey)
}

fn derive_pubkey(privkey: &[u8; 32]) -> [u8; 32] {
  let keypair = SigningKey::from_bytes(privkey);
  keypair.verifying_key().0
}

fn sign(data: &[u8], privkey: &[u8; 32]) -> Result<[u8; 64]> {
  let keypair: SigningKey = SigningKey::from_bytes(privkey);
  let signature = keypair.try_sign(data).map_err(|e| anyhow::anyhow!(e))?;
  let sig_bytes: [u8; 64] = signature.to_bytes();
  Ok(sig_bytes)
}

fn verify(data: &[u8], pubkey: [u8; 32], signature: &[u8; 64]) -> bool {
  let verifying_key = VerifyingKey(pubkey);
  let sig = Signature::from_bytes(signature);
  verifying_key.verify(data, &sig).is_ok()
}

#[derive(Debug, Clone)]
pub struct AuthRequest {
  pub rev: u32,
  pub timestamp: u64,
  pub nonce: [u8; 16],
  pub pubkey: [u8; 32],
  pub signature: [u8; 64],
}

impl AuthRequest {
  pub fn new(privkey: [u8; 32]) -> Result<Self> {
    let timestamp = std::time::UNIX_EPOCH.elapsed()?.as_secs();
    let nonce = rand::random::<[u8; 16]>();
    let mut buf = BytesMut::with_capacity(4 + 8 + 16 + 32);
    let pubkey = derive_pubkey(&privkey);
    buf.put_u32_le(PROTOCOL_REV);
    buf.put_u64_le(timestamp);
    buf.put_slice(&nonce);
    buf.put_slice(&pubkey);
    let signature = sign(&buf, &privkey)?;
    Ok(Self {
      rev: PROTOCOL_REV,
      timestamp,
      nonce,
      pubkey,
      signature,
    })
  }

  pub fn verify(&self) -> bool {
    if self.rev != PROTOCOL_REV {
      return false; // Unsupported protocol revision
    }
    let Ok(timestamp) = std::time::UNIX_EPOCH.elapsed() else {
      error!("Failed to get current timestamp");
      return false; // Failed to get current timestamp
    };
    let timestamp = timestamp.as_secs();
    if i64::abs(self.timestamp as i64 - timestamp as i64) > 3 {
      return false; // Allow a 3 seconds clock skew
    }
    let mut buf = BytesMut::with_capacity(4 + 8 + 16 + 32);
    buf.put_u32_le(self.rev);
    buf.put_u64_le(self.timestamp);
    buf.put_slice(&self.nonce);
    buf.put_slice(&self.pubkey);
    verify(&buf, self.pubkey, &self.signature)
  }

  pub fn encode(&self) -> String {
    let mut buf = BytesMut::with_capacity(4 + 8 + 16 + 32 + 64);
    buf.put_u32_le(self.rev);
    buf.put_u64_le(self.timestamp);
    buf.put_slice(&self.nonce);
    buf.put_slice(&self.pubkey);
    buf.put_slice(&self.signature);
    base64::engine::general_purpose::STANDARD.encode(buf)
  }

  pub fn decode(encoded: &str) -> Result<Self> {
    let decoded = base64::engine::general_purpose::STANDARD.decode(encoded)
      .map_err(|e| anyhow::anyhow!("Failed to decode auth request: {}", e))?;
    if decoded.len() != 4 + 8 + 16 + 32 + 64 {
      return Err(anyhow::anyhow!("Invalid auth request length"));
    }
    let mut buf = Bytes::from(decoded);
    let rev = buf.get_u32_le();
    let timestamp = buf.get_u64_le();
    let mut nonce = [0u8; 16] ;
    buf.split_to(16).copy_to_slice(&mut nonce);
    let mut pubkey = [0u8; 32];
    buf.split_to(32).copy_to_slice(&mut pubkey);
    let mut signature = [0u8; 64];
    buf.split_to(64).copy_to_slice(&mut signature);

    Ok(Self {
      rev,
      timestamp,
      nonce,
      pubkey,
      signature,
    })
  }

  pub fn encoded_pubkey(&self) -> String {
    base64::engine::general_purpose::STANDARD.encode(&self.pubkey)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_auth_request() {
    let (pubkey, privkey) = generate_keypair();
    assert_eq!(derive_pubkey(&privkey), pubkey);

    let req = AuthRequest::new(privkey).unwrap();
    let encoded = req.encode();
    println!("Encoded: {:?}", encoded);
    let decoded = AuthRequest::decode(&encoded).unwrap();
    assert!(decoded.verify());

    println!("pubkey: {:?}", decoded.encoded_pubkey());
  }
}