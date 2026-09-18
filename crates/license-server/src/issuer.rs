use ed25519_dalek::SigningKey;
use license_core::{Claims, Result, envelope::Envelope};
pub struct Issuer {
    pub kid: String,
    key: SigningKey,
    #[cfg(test)]
    pub fail_signing: bool,
}
impl Issuer {
    pub fn new(kid: String, key: SigningKey) -> Result<Self> {
        license_core::identifier(&kid)?;
        let public = license_core::envelope::encode(key.verifying_key().as_bytes());
        if !cfg!(any(test, feature = "dev"))
            && (public == license_core::trust::FIXTURE_ISSUER
                || public == license_core::trust::FIXTURE_DEVICE)
        {
            return Err(license_core::Code::UnknownKey);
        }
        Ok(Self {
            kid,
            key,
            #[cfg(test)]
            fail_signing: false,
        })
    }
    pub fn load(kid: String, path: &std::path::Path) -> Result<Self> {
        let b = zeroize::Zeroizing::new(license_store::read_service(path, 32, true)?);
        let seed = zeroize::Zeroizing::new(
            <[u8; 32]>::try_from(b.as_slice()).map_err(|_| license_core::Code::IoError)?,
        );
        Self::new(kid, SigningKey::from_bytes(&seed))
    }
    pub fn issue(&self, claims: &Claims) -> Result<Envelope> {
        #[cfg(test)]
        if self.fail_signing {
            return Err(license_core::Code::InternalError);
        }
        claims.validate()?;
        license_core::crypto::sign(claims, license_core::LEASE_TYPE, &self.kid, &self.key)
    }
    pub fn public(&self) -> String {
        license_core::envelope::encode(self.key.verifying_key().as_bytes())
    }
}
