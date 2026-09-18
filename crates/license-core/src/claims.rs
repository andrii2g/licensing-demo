use crate::{Result, envelope::fixed, error::require};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;
pub const MAX_TIME: i64 = 4102444800;
pub const MAX_SEQUENCE: u64 = 9007199254740991;
pub const LEASE_TYPE: &str = "license-guard-lease-v1";
pub const REQUEST_TYPE: &str = "license-guard-request-v1";
// deserialize_with makes nullable members required instead of silently defaulting absent fields.
pub fn nullable<'de, D, T>(d: D) -> std::result::Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(d)
}
pub fn json_object<'de, D, T>(d: D) -> std::result::Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let map = serde_json::Map::<String, serde_json::Value>::deserialize(d)?;
    serde_json::from_value(serde_json::Value::Object(map)).map_err(serde::de::Error::custom)
}
pub fn optional_object<'de, D, T>(d: D) -> std::result::Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let value = Option::<serde_json::Map<String, serde_json::Value>>::deserialize(d)?;
    value
        .map(|m| {
            serde_json::from_value(serde_json::Value::Object(m)).map_err(serde::de::Error::custom)
        })
        .transpose()
}
pub fn object_array<'de, D, T>(d: D) -> std::result::Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    Vec::<serde_json::Map<String, serde_json::Value>>::deserialize(d)?
        .into_iter()
        .map(|m| {
            serde_json::from_value(serde_json::Value::Object(m)).map_err(serde::de::Error::custom)
        })
        .collect()
}
pub fn identifier(s: &str) -> Result<()> {
    require(
        !s.is_empty()
            && s.len() <= 64
            && s.as_bytes()[0].is_ascii_alphanumeric()
            && s.bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c)),
    )
}
pub fn uuid4(s: &str) -> Result<()> {
    let u = uuid::Uuid::parse_str(s).map_err(|_| crate::Code::LicenseMalformed)?;
    require(
        u.get_version_num() == 4 && u.get_variant() == uuid::Variant::RFC4122 && u.to_string() == s,
    )
}
pub fn hash_string(s: &str) -> Result<()> {
    require(
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
    )
}
pub fn text(s: &str, max: usize) -> Result<()> {
    require(!s.is_empty() && s.chars().count() <= max && !s.contains('\0'))
}
pub fn features(v: &[String], min: usize, max: usize) -> Result<()> {
    require(v.len() >= min && v.len() <= max && v.iter().collect::<HashSet<_>>().len() == v.len())?;
    for s in v {
        identifier(s)?;
    }
    Ok(())
}
pub fn timestamp(t: i64) -> Result<()> {
    require((0..=MAX_TIME).contains(&t))
}
pub fn cpu_count(n: Option<u32>) -> Result<()> {
    require(n.is_none_or(|x| (1..=1048576).contains(&x)))
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Renewable,
    ManualOffline,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingPolicy {
    #[serde(rename = "linux-host-v1")]
    Host,
    #[serde(rename = "linux-machine-v1")]
    Machine,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub policy: BindingPolicy,
    pub machine_id_hash: String,
    #[serde(deserialize_with = "nullable")]
    pub system_uuid_hash: Option<String>,
}
impl Binding {
    pub fn validate(&self) -> Result<()> {
        hash_string(&self.machine_id_hash)?;
        match (&self.policy, &self.system_uuid_hash) {
            (BindingPolicy::Host, Some(v)) => hash_string(v),
            (BindingPolicy::Machine, None) => Ok(()),
            _ => Err(crate::Code::LicenseMalformed),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claims {
    pub schema_version: u32,
    pub issuer: String,
    pub license_id: String,
    pub customer_id: String,
    pub product: String,
    pub installation_id: String,
    pub installation_public_key_sha256: String,
    pub sequence: u64,
    pub issued_at: i64,
    pub not_before: i64,
    pub lease_valid_until: i64,
    pub entitlement_expires_at: i64,
    pub mode: Mode,
    pub features: Vec<String>,
    #[serde(deserialize_with = "json_object")]
    pub binding: Binding,
    #[serde(deserialize_with = "nullable")]
    pub max_logical_processors: Option<u32>,
}
impl Claims {
    pub fn validate(&self) -> Result<()> {
        require(self.schema_version == 1 && self.issuer == "license-guard")?;
        for x in [&self.license_id, &self.customer_id, &self.product] {
            identifier(x)?;
        }
        uuid4(&self.installation_id)?;
        hash_string(&self.installation_public_key_sha256)?;
        require((1..=MAX_SEQUENCE).contains(&self.sequence))?;
        for t in [
            self.issued_at,
            self.not_before,
            self.lease_valid_until,
            self.entitlement_expires_at,
        ] {
            timestamp(t)?;
        }
        require(
            self.not_before <= self.issued_at
                && self.issued_at < self.lease_valid_until
                && self.lease_valid_until <= self.entitlement_expires_at,
        )?;
        require(
            self.mode != Mode::ManualOffline
                || self.lease_valid_until == self.entitlement_expires_at,
        )?;
        features(&self.features, 1, 64)?;
        self.binding.validate()?;
        cpu_count(self.max_logical_processors)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub schema_version: u32,
    pub installation_id: String,
    pub installation_public_key: String,
}
impl Identity {
    pub fn validate(&self) -> Result<()> {
        require(self.schema_version == 1)?;
        uuid4(&self.installation_id)?;
        let k = fixed::<32>(&self.installation_public_key)?;
        let key = ed25519_dalek::VerifyingKey::from_bytes(&k)
            .map_err(|_| crate::Code::LicenseMalformed)?;
        require(!key.is_weak())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cpu {
    #[serde(deserialize_with = "nullable")]
    pub model: Option<String>,
    #[serde(deserialize_with = "nullable")]
    pub sockets: Option<u32>,
    #[serde(deserialize_with = "nullable")]
    pub cores: Option<u32>,
    #[serde(deserialize_with = "nullable")]
    pub logical_processors: Option<u32>,
    #[serde(deserialize_with = "nullable")]
    pub available_logical_processors: Option<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Os {
    pub id: String,
    #[serde(deserialize_with = "nullable")]
    pub version: Option<String>,
    pub kernel: String,
    pub architecture: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NicKind {
    Physical,
    Virtual,
    Unknown,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Nic {
    pub name: String,
    pub mac: String,
    pub kind: NicKind,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Virtualization {
    Physical,
    Vmware,
    Other,
    Unknown,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    pub schema_version: u32,
    pub collected_at: i64,
    pub hostname: String,
    #[serde(deserialize_with = "object_array")]
    pub network_interfaces: Vec<Nic>,
    #[serde(deserialize_with = "json_object")]
    pub cpu: Cpu,
    #[serde(deserialize_with = "json_object")]
    pub os: Os,
    pub virtualization: Virtualization,
    pub machine_id_hash: String,
    #[serde(deserialize_with = "nullable")]
    pub system_uuid_hash: Option<String>,
}
impl Inventory {
    pub fn validate(&self) -> Result<()> {
        require(self.schema_version == 1)?;
        timestamp(self.collected_at)?;
        text(&self.hostname, 253)?;
        require(self.network_interfaces.len() <= 64)?;
        for n in &self.network_interfaces {
            text(&n.name, 64)?;
            let b = n
                .mac
                .split(':')
                .map(|s| {
                    require(
                        s.len() == 2
                            && s.bytes()
                                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
                    )?;
                    u8::from_str_radix(s, 16).map_err(|_| crate::Code::LicenseMalformed)
                })
                .collect::<Result<Vec<_>>>()?;
            require(b.len() == 6 && b.iter().any(|x| *x != 0) && b[0] & 1 == 0)?;
        }
        if let Some(v) = &self.cpu.model {
            text(v, 256)?;
        }
        for n in [
            self.cpu.sockets,
            self.cpu.cores,
            self.cpu.logical_processors,
            self.cpu.available_logical_processors,
        ] {
            cpu_count(n)?;
        }
        text(&self.os.id, 64)?;
        text(&self.os.kernel, 256)?;
        text(&self.os.architecture, 64)?;
        if let Some(v) = &self.os.version {
            text(v, 256)?;
        }
        hash_string(&self.machine_id_hash)?;
        if let Some(v) = &self.system_uuid_hash {
            hash_string(v)?;
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Activate,
    Renew,
    Retire,
}
impl Action {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Activate => "activate",
            Self::Renew => "renew",
            Self::Retire => "retire",
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeRequest {
    pub schema_version: u32,
    pub action: Action,
    pub installation_id: String,
    pub product: String,
}
impl ChallengeRequest {
    pub fn validate(&self) -> Result<()> {
        require(self.schema_version == 1)?;
        uuid4(&self.installation_id)?;
        identifier(&self.product)
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Challenge {
    pub schema_version: u32,
    pub challenge_id: String,
    pub nonce: String,
    pub expires_at: i64,
}
impl Challenge {
    pub fn validate(&self) -> Result<()> {
        require(self.schema_version == 1)?;
        uuid4(&self.challenge_id)?;
        fixed::<32>(&self.nonce)?;
        timestamp(self.expires_at)
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceRequest {
    pub schema_version: u32,
    pub action: Action,
    pub operation_id: String,
    pub challenge_id: String,
    pub nonce: String,
    pub installation_id: String,
    pub product: String,
    pub installation_public_key: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_object"
    )]
    pub inventory: Option<Inventory>,
}
impl DeviceRequest {
    pub fn parse(e: &crate::envelope::Envelope) -> Result<Self> {
        let value: serde_json::Value = e.payload()?;
        let req: Self =
            serde_json::from_value(value.clone()).map_err(|_| crate::Code::LicenseMalformed)?;
        require(req.schema_version == 1)?;
        uuid4(&req.operation_id)?;
        uuid4(&req.challenge_id)?;
        uuid4(&req.installation_id)?;
        fixed::<32>(&req.nonce)?;
        fixed::<32>(&req.installation_public_key)?;
        identifier(&req.product)?;
        match req.action {
            Action::Retire => require(value.get("inventory").is_none())?,
            _ => req
                .inventory
                .as_ref()
                .ok_or(crate::Code::LicenseMalformed)?
                .validate()?,
        }
        Ok(req)
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetireResponse {
    pub schema_version: u32,
    pub installation_id: String,
    pub retired_at: i64,
    pub reserved_until: i64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entitlement {
    pub license_id: String,
    pub customer_id: String,
    pub product: String,
    pub mode: Mode,
    pub binding_policy: BindingPolicy,
    pub expires_at: i64,
    pub max_installations: u32,
    pub lease_seconds: u32,
    #[serde(deserialize_with = "nullable")]
    pub max_logical_processors: Option<u32>,
    pub features: Vec<String>,
}
impl Entitlement {
    pub fn validate(&self) -> Result<()> {
        for x in [&self.license_id, &self.customer_id, &self.product] {
            identifier(x)?;
        }
        timestamp(self.expires_at)?;
        require(self.expires_at > 0 && self.max_installations > 0 && self.lease_seconds > 0)?;
        features(&self.features, 1, 64)?;
        cpu_count(self.max_logical_processors)
    }
}
