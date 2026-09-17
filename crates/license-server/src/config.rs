use serde::Deserialize;
use crate::{Result,ApiError};
#[derive(Clone,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema_version:u32,pub listen:String,pub database_path:String,pub issuer_key_path:String,pub issuer_key_id:String,
    pub challenge_ttl_seconds:u32,pub operation_cache_seconds:u32,pub max_outstanding_challenges_per_installation:u32,
    pub request_body_limit_bytes:usize,pub sqlite_busy_timeout_milliseconds:u64,
    pub requests_per_ip_per_minute:u32,pub requests_per_installation_per_minute:u32,
    pub trusted_proxy_addresses:Vec<String>
}
impl Config{
    pub fn read(path:&std::path::Path)->Result<Self>{
        let bytes=license_store::read_absolute(path,16384,true)?;
        let c:Self=toml::from_str(std::str::from_utf8(&bytes).map_err(|_|ApiError::new(400,"CONFIG_ERROR"))?).map_err(|_|ApiError::new(400,"CONFIG_ERROR"))?;
        c.validate()?;Ok(c)
    }
    pub fn validate(&self)->Result<()>{
        if self.schema_version!=1||self.challenge_ttl_seconds==0||self.challenge_ttl_seconds>300||self.operation_cache_seconds<86400
            ||self.max_outstanding_challenges_per_installation==0||self.max_outstanding_challenges_per_installation>10
            ||self.request_body_limit_bytes==0||self.request_body_limit_bytes>131072||self.requests_per_ip_per_minute==0||self.requests_per_installation_per_minute==0
            ||self.sqlite_busy_timeout_milliseconds>10000 {return Err(ApiError::new(400,"CONFIG_ERROR"));}
        let addr:self::Socket= self.listen.parse().map_err(|_|ApiError::new(400,"CONFIG_ERROR"))?;
        // Reference API deliberately binds loopback behind the TLS reverse proxy.
        if !addr.ip().is_loopback(){return Err(ApiError::new(400,"LOOPBACK_PROXY_REQUIRED"));}
        for p in [&self.database_path,&self.issuer_key_path]{if !std::path::Path::new(p).is_absolute(){return Err(ApiError::new(400,"CONFIG_ERROR"));}}
        license_core::identifier(&self.issuer_key_id)?;Ok(())
    }
}
type Socket=std::net::SocketAddr;
