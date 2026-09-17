//! Duplicate detection precedes typed deserialization, including nested objects.
use crate::error::{Code, Result};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize, de::{self, DeserializeOwned, DeserializeSeed, MapAccess, SeqAccess, Visitor}};
use serde_json::Value;
use std::collections::HashSet;
pub const MAX_ENVELOPE: usize = 65536;
pub const MAX_PAYLOAD: usize = 49152;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope { pub protected: String, pub payload: String, pub signature: String }
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Header { pub typ: String, pub alg: String, pub kid: String }

struct Strict(usize);
impl<'de> DeserializeSeed<'de> for Strict {
    type Value = Value;
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> std::result::Result<Value, D::Error> {
        if self.0 > 16 { return Err(de::Error::custom("depth")); }
        d.deserialize_any(self)
    }
}
impl<'de> Visitor<'de> for Strict {
    type Value = Value;
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("strict JSON") }
    fn visit_bool<E: de::Error>(self,v:bool)->std::result::Result<Value,E>{Ok(Value::Bool(v))}
    fn visit_i64<E: de::Error>(self,v:i64)->std::result::Result<Value,E>{Ok(v.into())}
    fn visit_u64<E: de::Error>(self,v:u64)->std::result::Result<Value,E>{Ok(v.into())}
    fn visit_f64<E: de::Error>(self,_:f64)->std::result::Result<Value,E>{Err(E::custom("non-integer number"))}
    fn visit_str<E: de::Error>(self,v:&str)->std::result::Result<Value,E>{Ok(v.into())}
    fn visit_string<E: de::Error>(self,v:String)->std::result::Result<Value,E>{Ok(v.into())}
    fn visit_unit<E: de::Error>(self)->std::result::Result<Value,E>{Ok(Value::Null)}
    fn visit_none<E: de::Error>(self)->std::result::Result<Value,E>{Ok(Value::Null)}
    fn visit_seq<A:SeqAccess<'de>>(self,mut a:A)->std::result::Result<Value,A::Error>{
        let mut out=Vec::new();
        while let Some(v)=a.next_element_seed(Strict(self.0+1))? {out.push(v);}
        Ok(Value::Array(out))
    }
    fn visit_map<A:MapAccess<'de>>(self,mut a:A)->std::result::Result<Value,A::Error>{
        let mut out=serde_json::Map::new(); let mut seen=HashSet::new();
        while let Some(k)=a.next_key::<String>()? {
            if !seen.insert(k.clone()) {return Err(de::Error::custom("duplicate key"));}
            out.insert(k,a.next_value_seed(Strict(self.0+1))?);
        }
        Ok(Value::Object(out))
    }
}
pub fn strict<T:DeserializeOwned>(bytes:&[u8], limit:usize)->Result<T>{
    if bytes.is_empty() || bytes.len()>limit {return Err(Code::LicenseMalformed);}
    let mut de=serde_json::Deserializer::from_slice(bytes);
    let value=Strict(0).deserialize(&mut de).map_err(|_|Code::LicenseMalformed)?;
    de.end().map_err(|_|Code::LicenseMalformed)?;
    serde_json::from_value(value).map_err(|_|Code::LicenseMalformed)
}
pub fn encode(bytes:&[u8])->String {URL_SAFE_NO_PAD.encode(bytes)}
pub fn decode(s:&str,max:usize)->Result<Vec<u8>>{
    if s.is_empty() || s.len()>max.div_ceil(3)*4 {return Err(Code::LicenseMalformed);}
    let out=URL_SAFE_NO_PAD.decode(s).map_err(|_|Code::LicenseMalformed)?;
    if out.len()>max || encode(&out)!=s {return Err(Code::LicenseMalformed);}
    Ok(out)
}
pub fn fixed<const N:usize>(s:&str)->Result<[u8;N]>{
    decode(s,N)?.try_into().map_err(|_|Code::LicenseMalformed)
}
impl Envelope{
    pub fn parse(bytes:&[u8])->Result<Self>{
        let e:Self=strict(bytes,MAX_ENVELOPE)?;
        decode(&e.protected,1024)?;decode(&e.payload,MAX_PAYLOAD)?;fixed::<64>(&e.signature)?;
        Ok(e)
    }
    pub fn header(&self, typ:&str)->Result<Header>{
        let h:Header=strict(&decode(&self.protected,1024)?,1024)?;
        if h.typ!=typ || h.alg!="Ed25519" {return Err(Code::UnsupportedFormat);}
        crate::claims::identifier(&h.kid)?;
        Ok(h)
    }
    pub fn signing_bytes(&self)->Vec<u8>{
        format!("license-guard/v1\n{}.{}",self.protected,self.payload).into_bytes()
    }
    pub fn digest(&self)->String{crate::crypto::hash(format!("{}.{}.{}",self.protected,self.payload,self.signature).as_bytes())}
    pub fn payload<T:DeserializeOwned>(&self)->Result<T>{strict(&decode(&self.payload,MAX_PAYLOAD)?,MAX_PAYLOAD)}
}
