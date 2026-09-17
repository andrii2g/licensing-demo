use crate::{Result,Code,envelope::fixed};
use std::collections::BTreeMap;
pub const FIXTURE_ISSUER:&str="ebVWLo_mVPlAeLES6KmLp5AfhTrmlb7X4OORC60ElmQ";
pub const FIXTURE_DEVICE:&str="5_FioQvsVZr-oZXk3OhLaVaNXSywlj60RsBoXisX8vA";
#[derive(Clone,Default)]
pub struct Trust(BTreeMap<String,[u8;32]>);
impl Trust{
    pub fn new(pairs:Vec<(String,String)>)->Result<Self>{
        let mut out=BTreeMap::new();
        for (kid,key) in pairs {
            crate::claims::identifier(&kid)?;
            if !cfg!(any(test,feature="dev")) && (kid.starts_with("TEST-")||key==FIXTURE_ISSUER||key==FIXTURE_DEVICE){return Err(Code::UnknownKey);}
            let bytes=fixed::<32>(&key)?;
            let parsed=ed25519_dalek::VerifyingKey::from_bytes(&bytes).map_err(|_|Code::UnknownKey)?;
            if parsed.is_weak()||out.insert(kid,bytes).is_some(){return Err(Code::UnknownKey);}
        }
        Ok(Self(out))
    }
    pub fn key(&self,kid:&str)->Result<&[u8;32]>{self.0.get(kid).ok_or(Code::UnknownKey)}
    pub fn compiled()->Result<Self>{
        let pairs:Vec<(String,String)>=crate::envelope::strict(include_bytes!(concat!(env!("OUT_DIR"),"/trust.json")),16384)?;
        Self::new(pairs)
    }
}
