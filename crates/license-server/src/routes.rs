use crate::*;
use axum::{
    Router,
    body::to_bytes,
    extract::{ConnectInfo, Path, Request, State},
    http::{HeaderMap, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use license_core::{Action, ChallengeRequest};
use std::sync::{Arc, Mutex};
pub struct App {
    pub db: Mutex<rusqlite::Connection>,
    pub issuer: issuer::Issuer,
    pub config: config::Config,
    pub permits: Arc<tokio::sync::Semaphore>,
    pub db_permits: Arc<tokio::sync::Semaphore>,
    pub limiter: Mutex<limits::Limiter>,
}
impl App {
    pub fn new(
        db: rusqlite::Connection,
        issuer: issuer::Issuer,
        config: config::Config,
    ) -> Arc<Self> {
        Arc::new(Self {
            db: Mutex::new(db),
            issuer,
            config,
            permits: Arc::new(tokio::sync::Semaphore::new(16)),
            db_permits: Arc::new(tokio::sync::Semaphore::new(16)),
            limiter: Mutex::new(limits::Limiter::default()),
        })
    }
}
pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/v1/challenges", post(challenge))
        .route("/v1/activations", post(activate))
        .route("/v1/installations/{id}/renew", post(renew))
        .route("/v1/installations/{id}/retire", post(retire))
        .route("/health/live", get(|| async { "ok" }))
        .route("/health/ready", get(ready))
        .layer(middleware::from_fn_with_state(app.clone(), limit))
        .with_state(app)
}
async fn limit(State(app): State<Arc<App>>, req: Request, next: Next) -> Response {
    let peer = req
        .extensions()
        .get::<ConnectInfo<std::net::SocketAddr>>()
        .map(|p| p.0.ip().to_string())
        .unwrap_or_else(|| "local-test".into());
    let allowed = app
        .limiter
        .lock()
        .is_ok_and(|mut l| l.allow(format!("ip:{peer}"), app.config.requests_per_ip_per_minute));
    if !allowed {
        return ApiError::new(429, "RATE_LIMIT").into_response();
    }
    // Holding permit through body read, DB operation and response bounds concurrent allocations.
    let Ok(permit) = app.permits.clone().try_acquire_owned() else {
        return ApiError::new(503, "BUSY").into_response();
    };
    let response = tokio::time::timeout(std::time::Duration::from_secs(25), next.run(req))
        .await
        .unwrap_or_else(|_| ApiError::new(503, "TIMEOUT").into_response());
    drop(permit);
    response
}
fn token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|s| s.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(str::to_string)
}
async fn body(app: &App, req: Request) -> Result<(Option<String>, Vec<u8>)> {
    if !req
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|s| s.split(';').next() == Some("application/json"))
    {
        return Err(ApiError::new(400, "CONTENT_TYPE"));
    }
    let token = token(req.headers());
    let b = to_bytes(req.into_body(), app.config.request_body_limit_bytes)
        .await
        .map_err(|_| ApiError::new(400, "BODY_LIMIT"))?;
    Ok((token, b.to_vec()))
}
fn installation_limit(app: &App, id: &str) -> Result<()> {
    if !app
        .limiter
        .lock()
        .map_err(|_| ApiError::new(503, "BUSY"))?
        .allow(
            format!("id:{id}"),
            app.config.requests_per_installation_per_minute,
        )
    {
        return Err(ApiError::new(429, "RATE_LIMIT"));
    }
    Ok(())
}
async fn challenge(State(app): State<Arc<App>>, req: Request) -> Result<impl IntoResponse> {
    let (t, b) = body(&app, req).await?;
    let r: ChallengeRequest = license_core::envelope::strict(&b, 131072)?;
    r.validate()?;
    installation_limit(&app, &r.installation_id)?;
    let permit = app
        .db_permits
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::new(503, "BUSY"))?;
    let response = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let mut db = app.db.lock().map_err(|_| ApiError::new(503, "BUSY"))?;
        service::challenge(&mut db, &r, t.as_deref(), license_core::now(), &app.config)
    })
    .await
    .map_err(|_| ApiError::new(500, "INTERNAL_ERROR"))??;
    Ok(axum::Json(response))
}
async fn mutation(
    app: Arc<App>,
    req: Request,
    action: Action,
    id: Option<String>,
) -> Result<impl IntoResponse> {
    let (t, b) = body(&app, req).await?;
    let env = license_core::envelope::Envelope::parse(&b)?;
    let r = license_core::DeviceRequest::parse(&env)?;
    installation_limit(&app, &r.installation_id)?;
    let permit = app
        .db_permits
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::new(503, "BUSY"))?;
    let response = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let mut db = app.db.lock().map_err(|_| ApiError::new(503, "BUSY"))?;
        service::mutate(
            &mut db,
            &app.issuer,
            &b,
            service::Mutation {
                action,
                route_id: id.as_deref(),
                token: t.as_deref(),
                now: license_core::now(),
                cache_seconds: app.config.operation_cache_seconds,
            },
        )
    })
    .await
    .map_err(|_| ApiError::new(500, "INTERNAL_ERROR"))??;
    Ok(([(header::CONTENT_TYPE, "application/json")], response))
}
async fn activate(State(a): State<Arc<App>>, r: Request) -> Result<impl IntoResponse> {
    mutation(a, r, Action::Activate, None).await
}
async fn renew(
    State(a): State<Arc<App>>,
    Path(id): Path<String>,
    r: Request,
) -> Result<impl IntoResponse> {
    mutation(a, r, Action::Renew, Some(id)).await
}
async fn retire(
    State(a): State<Arc<App>>,
    Path(id): Path<String>,
    r: Request,
) -> Result<impl IntoResponse> {
    mutation(a, r, Action::Retire, Some(id)).await
}
async fn ready(State(a): State<Arc<App>>) -> Result<&'static str> {
    let permit = a
        .db_permits
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::new(503, "BUSY"))?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let db = a.db.lock().map_err(|_| ApiError::new(503, "BUSY"))?;
        db.query_row("SELECT 1", [], |r| r.get::<_, i64>(0))?;
        Ok::<_, ApiError>("ok")
    })
    .await
    .map_err(|_| ApiError::new(503, "BUSY"))?
}
