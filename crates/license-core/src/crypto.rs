use crate::{
    Code, Result,
    envelope::{Envelope, Header, encode, fixed},
};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::Serialize;
use sha2::{Digest, Sha256};
pub fn hash(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}
pub fn random<const N: usize>() -> Result<[u8; N]> {
    let mut v = [0; N];
    getrandom::fill(&mut v).map_err(|_| Code::InternalError)?;
    Ok(v)
}
pub fn verify_signature(envelope: &Envelope, key: &[u8; 32]) -> Result<()> {
    let key = VerifyingKey::from_bytes(key).map_err(|_| Code::InvalidSignature)?;
    if key.is_weak() {
        return Err(Code::InvalidSignature);
    }
    key.verify_strict(
        &envelope.signing_bytes(),
        &Signature::from_bytes(&fixed::<64>(&envelope.signature)?),
    )
    .map_err(|_| Code::InvalidSignature)
}
pub fn sign<T: Serialize>(value: &T, typ: &str, kid: &str, key: &SigningKey) -> Result<Envelope> {
    let header = Header {
        typ: typ.into(),
        alg: "Ed25519".into(),
        kid: kid.into(),
    };
    let mut e = Envelope {
        protected: encode(&serde_json::to_vec(&header).map_err(|_| Code::InternalError)?),
        payload: encode(&serde_json::to_vec(value).map_err(|_| Code::InternalError)?),
        signature: String::new(),
    };
    e.signature = encode(&key.sign(&e.signing_bytes()).to_bytes());
    Ok(e)
}
