use hmac::{Hmac,Mac};
use sha2::{Digest,Sha256};
use license_core::{Code,Result};
pub fn machine_id(raw:&str)->Result<String>{
    let v=raw.trim_matches(|c:char|c.is_ascii_whitespace()).to_ascii_lowercase();
    if v.len()!=32||!v.bytes().all(|b|b.is_ascii_hexdigit())||v.bytes().all(|b|b==b'0'){return Err(Code::IdentityUnavailable);}Ok(v)
}
pub fn system_uuid(raw:&str)->Result<String>{
    let v=raw.trim_matches(|c:char|c.is_ascii_whitespace()).to_ascii_lowercase();
    if v.len()!=36||v.bytes().enumerate().any(|(i,b)|if [8,13,18,23].contains(&i){b!=b'-'}else{!b.is_ascii_hexdigit()})
        ||v.bytes().filter(|b|*b!=b'-').all(|b|b==b'0')||v.bytes().filter(|b|*b!=b'-').all(|b|b==b'f'){return Err(Code::IdentityUnavailable);}Ok(v)
}
pub fn derive(product:&str,kind:&str,normalized:&str)->String{
    let key=Sha256::digest(format!("license-guard/fingerprint/v1/{product}").as_bytes());
    let mut mac=Hmac::<Sha256>::new_from_slice(&key).expect("SHA256 HMAC accepts any key length");
    mac.update(format!("{kind}:{normalized}").as_bytes());hex::encode(mac.finalize().into_bytes())
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn supplied_fingerprint(){
        let v:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/fingerprint-vector.json")).unwrap();
        for (kind,input,output) in [("machine-id","normalized_machine_id","machine_id_hash"),("system-uuid","normalized_system_uuid","system_uuid_hash")]{
            assert_eq!(derive("worker-suite",kind,v[input].as_str().unwrap()),v[output]);
        }
        assert!(machine_id("00000000000000000000000000000000").is_err());
        assert!(system_uuid("ffffffff-ffff-ffff-ffff-ffffffffffff").is_err());
        assert_eq!(machine_id(" 0123456789ABCDEF0123456789ABCDEF\n").unwrap(),"0123456789abcdef0123456789abcdef");
    }
}
