use crate::{Error, Result, config::Config};
use std::{io::Read, time::Duration};
pub struct Client {
    inner: reqwest::blocking::Client,
    base: reqwest::Url,
    attempts: u32,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ApiError {
    schema_version: u32,
    code: String,
    message: String,
    retryable: bool,
    request_id: String,
}
impl Client {
    pub fn new(c: &Config) -> Result<Self> {
        Ok(Self {
            inner: reqwest::blocking::Client::builder()
                .connect_timeout(Duration::from_secs(c.connect_timeout_seconds))
                .timeout(Duration::from_secs(c.request_timeout_seconds))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|_| Error::new(70, "HTTP_CLIENT"))?,
            base: crate::config::validate_url(&c.server_url)?,
            attempts: c.max_attempts,
        })
    }
    pub fn post(&self, path: &str, body: &[u8], token: Option<&str>) -> Result<Vec<u8>> {
        let url = self.base.join(path).map_err(|_| Error::config())?;
        let mut last = Error::new(75, "NETWORK_UNAVAILABLE");
        for attempt in 0..self.attempts {
            let mut request = self
                .inner
                .post(url.clone())
                .header("content-type", "application/json")
                .body(body.to_vec());
            if let Some(t) = token {
                request = request.bearer_auth(t);
            }
            let mut wait = None;
            match request.send() {
                Err(_) => {}
                Ok(response) => {
                    let status = response.status();
                    wait = response
                        .headers()
                        .get("retry-after")
                        .and_then(|s| s.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .map(|s| s.min(5));
                    if response.content_length().is_some_and(|n| n > 65536) {
                        return Err(Error::new(78, "RESPONSE_LIMIT"));
                    }
                    let mut bytes = Vec::new();
                    if response.take(65537).read_to_end(&mut bytes).is_err() {
                        last = Error::new(75, "RESPONSE_LOST");
                    } else {
                        if bytes.len() > 65536 {
                            return Err(Error::new(78, "RESPONSE_LIMIT"));
                        }
                        if status.is_success() {
                            return Ok(bytes);
                        }
                        let code = license_core::envelope::strict::<ApiError>(&bytes, 65536)
                            .ok()
                            .filter(|e| {
                                e.schema_version == 1
                                    && license_core::identifier(&e.code).is_ok()
                                    && e.message.len() <= 512
                                    && license_core::uuid4(&e.request_id).is_ok()
                                    && e.retryable == matches!(status.as_u16(), 429 | 503)
                            })
                            .map_or_else(|| "SERVER_DENIED".into(), |e| e.code);
                        if !matches!(status.as_u16(), 429 | 503) {
                            return Err(Error { exit: 78, code });
                        }
                        last = Error { exit: 75, code };
                    }
                }
            }
            if attempt + 1 < self.attempts {
                let jitter = license_core::crypto::random::<1>().map_err(Error::from)?[0] as u64;
                std::thread::sleep(Duration::from_millis(
                    wait.map_or(250 * (1 << attempt) + jitter, |s| s * 1000),
                ));
            }
        }
        Err(last)
    }
}
