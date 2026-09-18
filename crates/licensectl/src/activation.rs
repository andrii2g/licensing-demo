use crate::{client::Client, config::Config, *};
use license_core::{
    envelope::{Envelope, strict},
    *,
};
use license_store::{Device, SecureDir};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    schema_version: u32,
    action: Action,
    server_url: String,
    envelope: String,
}
pub fn execute(
    c: &Config,
    dir: &SecureDir,
    action: Action,
    token: Option<&str>,
) -> crate::Result<()> {
    let device = if action == Action::Activate {
        Device::load_or_create(dir)?
    } else {
        Device::load(dir)?
    };
    let http = Client::new(c)?;
    let trust = crate::trust()?;
    for recovery in 0..2 {
        let bytes = match dir.read("pending-operation.json", 70000, true) {
            Ok(bytes) => {
                let p: Pending = strict(&bytes, 70000)?;
                let e = Envelope::parse(p.envelope.as_bytes())?;
                let r = DeviceRequest::parse(&e)?;
                if p.schema_version != 1
                    || p.action != action
                    || p.server_url != c.server_url
                    || r.action != action
                    || r.product != c.product
                    || r.installation_id != device.identity.installation_id
                    || r.installation_public_key != device.identity.installation_public_key
                {
                    return Err(Error::new(64, "PENDING_OPERATION_CONFLICT"));
                }
                crypto::verify_signature(&e, device.key.verifying_key().as_bytes())?;
                p.envelope.into_bytes()
            }
            Err(Code::LicenseMissing) => {
                let req = ChallengeRequest {
                    schema_version: 1,
                    action,
                    installation_id: device.identity.installation_id.clone(),
                    product: c.product.clone(),
                };
                let ch: Challenge = strict(
                    &http.post(
                        "/v1/challenges",
                        &serde_json::to_vec(&req).map_err(|_| Error::config())?,
                        token,
                    )?,
                    65536,
                )?;
                ch.validate()?;
                let inventory = if action == Action::Retire {
                    None
                } else {
                    Some(license_host::collect(&c.product, now())?)
                };
                let req = DeviceRequest {
                    schema_version: 1,
                    action,
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    challenge_id: ch.challenge_id,
                    nonce: ch.nonce,
                    installation_id: device.identity.installation_id.clone(),
                    product: c.product.clone(),
                    installation_public_key: device.identity.installation_public_key.clone(),
                    inventory,
                };
                let e = crypto::sign(
                    &req,
                    REQUEST_TYPE,
                    &device.identity.installation_id,
                    &device.key,
                )?;
                let exact = serde_json::to_string(&e).map_err(|_| Error::config())?;
                let pending = Pending {
                    schema_version: 1,
                    action,
                    server_url: c.server_url.clone(),
                    envelope: exact.clone(),
                };
                dir.atomic(
                    "pending-operation.json",
                    &serde_json::to_vec(&pending).map_err(|_| Error::config())?,
                    true,
                )?;
                exact.into_bytes()
            }
            Err(e) => return Err(e.into()),
        };
        let route = match action {
            Action::Activate => "/v1/activations".into(),
            Action::Renew => format!(
                "/v1/installations/{}/renew",
                device.identity.installation_id
            ),
            Action::Retire => format!(
                "/v1/installations/{}/retire",
                device.identity.installation_id
            ),
        };
        let response = match http.post(&route, &bytes, token) {
            Ok(v) => v,
            Err(e) if recovery == 0 && e.code == "CHALLENGE_INVALID" => {
                dir.remove("pending-operation.json")?;
                continue;
            }
            Err(e) => return Err(e),
        };
        if action == Action::Retire {
            let r: RetireResponse = strict(&response, 65536)?;
            if r.schema_version != 1 || r.installation_id != device.identity.installation_id {
                return Err(Error::new(78, "RESPONSE_IDENTITY"));
            }
            timestamp(r.retired_at)?;
            timestamp(r.reserved_until)?;
            dir.atomic("retirement.json", &response, false)?;
            dir.remove("pending-operation.json")?;
            println!(
                "{}",
                String::from_utf8(response).map_err(|_| Error::config())?
            );
            return Ok(());
        }
        let context = crate::context(c, device.identity.clone())?;
        match license_store::install::install(dir, &response, &trust, &context) {
            Ok(v) => {
                dir.remove("pending-operation.json")?;
                println!(
                    "{}",
                    serde_json::to_string(&ValidationResult::accepted(&v, context.now))
                        .map_err(|_| Error::config())?
                );
                return Ok(());
            }
            Err(Code::LicenseExpired) if recovery == 0 => {
                dir.remove("pending-operation.json")?;
            }
            Err(e) => return Err(e.into()),
        }
    }
    Err(Error::new(75, "RECOVERY_RETRY_REQUIRED"))
}
