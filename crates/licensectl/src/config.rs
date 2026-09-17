use serde::Deserialize;
use crate::{Error,Result};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config{
    pub schema_version:u32,pub product:String,pub server_url:String,pub state_dir:std::path::PathBuf,
    pub connect_timeout_seconds:u64,pub request_timeout_seconds:u64,pub max_attempts:u32
}
impl Config{
    pub fn read(path:&std::path::Path)->Result<Self>{
        let bytes=license_store::read_absolute(path,16384,false)?;
        let c:Self=toml::from_str(std::str::from_utf8(&bytes).map_err(|_|Error::config())?).map_err(|_|Error::config())?;
        license_core::identifier(&c.product)?;
        if c.schema_version!=1||!c.state_dir.is_absolute()||!(1..=10).contains(&c.connect_timeout_seconds)||!(1..=60).contains(&c.request_timeout_seconds)||!(1..=5).contains(&c.max_attempts){return Err(Error::config());}
        validate_url(&c.server_url)?;Ok(c)
    }
}
pub fn validate_url(raw:&str)->Result<reqwest::Url>{
    let u=reqwest::Url::parse(raw).map_err(|_|Error::config())?;
    if !u.username().is_empty()||u.password().is_some()||u.query().is_some()||u.fragment().is_some()||u.path()!="/"{return Err(Error::config());}
    let loopback=u.host_str().is_some_and(|h|h=="localhost"||h.trim_matches(['[',']']).parse::<std::net::IpAddr>().is_ok_and(|ip|ip.is_loopback()));
    if u.scheme()!="https"&&!(cfg!(feature="dev")&&u.scheme()=="http"&&loopback){return Err(Error::new(64,"HTTPS_REQUIRED"));}
    Ok(u)
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn url_policy(){for url in ["http://localhost.evil.test","http://192.168.0.1","https://u:p@example.org","https://example.org/path","https://example.org?x"]{assert!(validate_url(url).is_err());}assert!(validate_url("https://example.org").is_ok());assert_eq!(validate_url("http://127.0.0.1:8080").is_ok(),cfg!(feature="dev"));}
}
