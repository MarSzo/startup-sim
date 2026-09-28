//! The login API over HTTPS (its own thread; the game loop only reads the
//! tickets). JSON in and out:
//!
//! - `POST /api/register` `{nick, password}`, `POST /api/login` (same),
//!   `POST /api/refresh` `{refresh}`, `POST /api/logout` `{refresh}`,
//!   `POST /api/password` `{nick, password, new_password}` →
//!   `{ok, nick, ticket, refresh, character, error}`;
//! - `GET /api/cert`: the server's own certificate (PEM) when it made it
//!   itself, so the client can pin it on first contact (like SSH).
//!
//! Passwords never cross the plain UDP game protocol.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::auth::{Auth, AuthError, Granted};

/// Where the certificate comes from.
pub struct Tls {
    pub cert: PathBuf,
    pub key: PathBuf,
    /// Made by the server itself (clients pin it); false = a real one.
    pub self_signed: bool,
}

/// The server's own certificate in `dir` (made once, then reused).
pub fn self_signed(dir: &Path) -> Result<Tls, String> {
    let (cert, key) = (dir.join("cert.pem"), dir.join("key.pem"));
    if !cert.exists() || !key.exists() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let names = vec!["startup-sim".to_string(), "localhost".to_string(), "127.0.0.1".to_string(), "::1".to_string()];
        let made = rcgen::generate_simple_self_signed(names).map_err(|e| e.to_string())?;
        std::fs::write(&cert, made.cert.pem()).map_err(|e| e.to_string())?;
        std::fs::write(&key, made.signing_key.serialize_pem()).map_err(|e| e.to_string())?;
    }
    Ok(Tls { cert, key, self_signed: true })
}

#[derive(Clone)]
struct AppState {
    auth: Auth,
    /// PEM of our own certificate (None with a real one).
    cert: Option<String>,
}

#[derive(Deserialize)]
struct Creds {
    nick: String,
    password: String,
}

#[derive(Deserialize)]
struct NewPassword {
    nick: String,
    password: String,
    new_password: String,
}

#[derive(Deserialize)]
struct RefreshReq {
    refresh: String,
}

#[derive(Serialize, Default)]
struct Reply {
    ok: bool,
    nick: String,
    ticket: String,
    refresh: String,
    character: bool,
    error: String,
}

type Answer = (StatusCode, Json<Reply>);

fn answer(r: Result<Granted, AuthError>) -> Answer {
    match r {
        Ok(g) => (StatusCode::OK, Json(Reply { ok: true, nick: g.nick, ticket: g.ticket, refresh: g.refresh, character: g.character, error: String::new() })),
        Err(e) => {
            if let AuthError::Internal(msg) = &e {
                eprintln!("auth: {msg}");
            }
            let status = StatusCode::from_u16(e.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            (status, Json(Reply { error: e.message(), ..Default::default() }))
        }
    }
}

/// Argon2 is slow on purpose: off the async thread.
async fn blocking(f: impl FnOnce() -> Result<Granted, AuthError> + Send + 'static) -> Answer {
    answer(tokio::task::spawn_blocking(f).await.unwrap_or_else(|e| Err(AuthError::Internal(e.to_string()))))
}

async fn register(State(s): State<AppState>, ConnectInfo(from): ConnectInfo<SocketAddr>, Json(c): Json<Creds>) -> Answer {
    let ip = from.ip().to_string();
    let r = blocking(move || s.auth.register(&c.nick, &c.password, &ip)).await;
    if r.1.ok {
        println!("* auth: new account '{}'", r.1.nick);
    }
    r
}

async fn login(State(s): State<AppState>, ConnectInfo(from): ConnectInfo<SocketAddr>, Json(c): Json<Creds>) -> Answer {
    let ip = from.ip().to_string();
    blocking(move || s.auth.login(&c.nick, &c.password, &ip)).await
}

async fn password(State(s): State<AppState>, ConnectInfo(from): ConnectInfo<SocketAddr>, Json(c): Json<NewPassword>) -> Answer {
    let ip = from.ip().to_string();
    blocking(move || s.auth.change_password(&c.nick, &c.password, &c.new_password, &ip)).await
}

async fn refresh(State(s): State<AppState>, Json(r): Json<RefreshReq>) -> Answer {
    blocking(move || s.auth.refresh(&r.refresh)).await
}

async fn logout(State(s): State<AppState>, Json(r): Json<RefreshReq>) -> StatusCode {
    s.auth.logout(&r.refresh);
    StatusCode::NO_CONTENT
}

async fn cert(State(s): State<AppState>) -> (StatusCode, String) {
    match s.cert {
        Some(pem) => (StatusCode::OK, pem),
        None => (StatusCode::NOT_FOUND, String::new()),
    }
}

/// Start the HTTPS API on its own thread.
pub fn spawn(auth: Auth, bind: SocketAddr, tls: Tls) -> Result<(), String> {
    let cert_pem = if tls.self_signed { Some(std::fs::read_to_string(&tls.cert).map_err(|e| e.to_string())?) } else { None };
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    std::thread::Builder::new()
        .name("https".into())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => return eprintln!("https: {e}"),
            };
            rt.block_on(async move {
                let config = match axum_server::tls_rustls::RustlsConfig::from_pem_file(&tls.cert, &tls.key).await {
                    Ok(c) => c,
                    Err(e) => return eprintln!("https: certificate {}: {e}", tls.cert.display()),
                };
                let app = Router::new()
                    .route("/api/register", post(register))
                    .route("/api/login", post(login))
                    .route("/api/refresh", post(refresh))
                    .route("/api/password", post(password))
                    .route("/api/logout", post(logout))
                    .route("/api/cert", get(cert))
                    .with_state(AppState { auth, cert: cert_pem });
                if let Err(e) = axum_server::bind_rustls(bind, config).serve(app.into_make_service_with_connect_info::<SocketAddr>()).await {
                    eprintln!("https: {e}");
                }
            });
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}
