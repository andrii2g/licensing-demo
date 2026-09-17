#![forbid(unsafe_code)]
pub mod auth;
pub mod config;
pub mod issuer;
pub mod repository;
pub mod service;
pub mod routes;
pub mod limits;
use axum::{http::StatusCode,response::{IntoResponse,Response},Json};
#[derive(Debug,Clone)]
pub struct ApiError{pub status:u16,pub code:&'static str}
impl ApiError{pub fn new(status:u16,code:&'static str)->Self{Self{status,code}}}
impl std::fmt::Display for ApiError{fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.write_str(self.code)}}
impl std::error::Error for ApiError{}
impl From<license_core::Code> for ApiError{fn from(_:license_core::Code)->Self{Self::new(400,"MALFORMED_REQUEST")}}
impl From<rusqlite::Error> for ApiError{
    fn from(e:rusqlite::Error)->Self{
        if matches!(&e,rusqlite::Error::SqliteFailure(err,_) if matches!(err.code,rusqlite::ErrorCode::DatabaseBusy|rusqlite::ErrorCode::DatabaseLocked)){Self::new(503,"DATABASE_BUSY")}else{Self::new(503,"DATABASE_ERROR")}
    }
}
impl IntoResponse for ApiError{
    fn into_response(self)->Response{
        (StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),Json(serde_json::json!({
            "schema_version":1,"code":self.code,"message":self.code,
            "retryable":matches!(self.status,429|503),"request_id":uuid::Uuid::new_v4().to_string()
        }))).into_response()
    }
}
pub type Result<T>=std::result::Result<T,ApiError>;
pub fn json<T:serde::Serialize>(v:&T)->Result<String>{serde_json::to_string(v).map_err(|_|ApiError::new(500,"INTERNAL_ERROR"))}
#[cfg(test)] mod tests;
