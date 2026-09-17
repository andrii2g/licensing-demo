#![forbid(unsafe_code)]
pub mod fingerprint;
pub mod linux;
use license_core::{Inventory,Result};
pub type HostSnapshot=Inventory;
pub trait HostProvider {fn collect(&self,product:&str,now:i64)->Result<HostSnapshot>;}
pub struct LiveHost;
impl HostProvider for LiveHost{
    fn collect(&self,product:&str,now:i64)->Result<HostSnapshot>{linux::collect(product,now)}
}
/// Isolated development builds only. Production always uses the live provider.
pub fn collect(product:&str,now:i64)->Result<HostSnapshot>{
    #[cfg(feature="dev")]
    if let Some(path)=std::env::var_os("LICENSE_GUARD_DEV_INVENTORY"){
        let bytes=linux::bounded(&std::path::PathBuf::from(path),49152)?;
        let mut i:Inventory=license_core::envelope::strict(bytes.as_bytes(),49152)?;
        i.collected_at=now;i.validate()?;return Ok(i);
    }
    LiveHost.collect(product,now)
}
