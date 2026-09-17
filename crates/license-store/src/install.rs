use crate::SecureDir;
use license_core::{*,envelope::strict};
use serde::{Serialize,Deserialize};
#[derive(Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct HighWater {sequence:u64,digest:String}
pub fn install(dir:&SecureDir,bytes:&[u8],trust:&Trust,ctx:&Context)->Result<VerifiedLease>{
    let candidate=verify(bytes,trust,ctx)?;
    let mut sequence=0;let mut digest=String::new();
    match dir.read("install-state.json",4096,true){
        Ok(b)=>{let high:HighWater=strict(&b,4096)?;sequence=high.sequence;digest=high.digest;},
        Err(Code::LicenseMissing)=>{},Err(e)=>return Err(e)
    }
    match dir.read("license.lic",65536,false){
        Ok(b)=>{
            // Expired current authorization still supplies an authenticated sequence.
            let current=policy::authenticate(&b,trust)?;
            if current.claims.installation_id!=ctx.identity.installation_id{return Err(Code::InstallationMismatch);}
            if current.claims.sequence<sequence||(current.claims.sequence==sequence&&current.digest!=digest){return Err(Code::LeaseRollback);}
            sequence=current.claims.sequence;digest=current.digest;
        },
        Err(Code::LicenseMissing)=>{},Err(e)=>return Err(e)
    }
    if candidate.claims.sequence<sequence||(candidate.claims.sequence==sequence&&candidate.digest!=digest){return Err(Code::LeaseRollback);}
    dir.atomic("license.lic",bytes,false)?;
    let high=HighWater{sequence:candidate.claims.sequence,digest:candidate.digest.clone()};
    dir.atomic("install-state.json",&serde_json::to_vec(&high).map_err(|_|Code::InternalError)?,true)?;
    Ok(candidate)
}
