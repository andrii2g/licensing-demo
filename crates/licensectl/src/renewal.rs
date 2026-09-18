use crate::{Result, config::Config};
use license_core::{Action, Mode, policy::authenticate};
use license_store::SecureDir;
pub fn renew(c: &Config, dir: &SecureDir, scheduled: bool) -> Result<()> {
    if scheduled
        && let Ok(bytes) = dir.read("license.lic", 65536, false)
        && let Ok(v) = authenticate(&bytes, &crate::trust()?)
        && v.claims.mode == Mode::ManualOffline
        && v.claims.lease_valid_until > license_core::now()
    {
        println!("MANUAL_OFFLINE_NOOP");
        return Ok(());
    }
    crate::activation::execute(c, dir, Action::Renew, None)
}
