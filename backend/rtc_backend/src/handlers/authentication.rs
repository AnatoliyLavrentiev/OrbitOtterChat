use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

use crate::{errors::AppError, repositories, security::tokens, services::hash_passwords, AppState};

#[derive(Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct SigninRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
}

pub fn routes() -> Router<AppState> {
    let router: Router<AppState> = Router::new();
    router
        .route("/signup", post(signup))
        .route("/login", post(sign_in))
        .route("/signin", post(sign_in))
        .route("/refresh", post(refresh))
        .route("/logout", post(logout))
}

pub async fn signup(
    State(state): State<AppState>,
    Json(req): Json<SignupRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let email = req.email.trim().to_lowercase();
    let username = req.username.trim().to_string();
    let password = req.password;

    if email.is_empty() || username.is_empty() || password.is_empty() {
        return Err(AppError::BadRequest("missing fields".into()));
    }

    let password_hash = hash_passwords::hash_passwd(&password)?;

    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let user = repositories::users::create_user(&mut conn, &email, &username, &password_hash)?;

    let access_token = state.jwt.sign_access_token(user.id)?;

    let refresh_raw = tokens::generate_refresh_token();
    let refresh_hash = tokens::hash_refresh_token(&refresh_raw);

    let days_valid: i64 = 30;

    repositories::refresh_tokens::create_refresh_token(
        &mut conn,
        user.id,
        &refresh_hash,
        days_valid,
        None,
        None,
    )?;

    Ok(Json(AuthResponse {
        access_token,
        refresh_token: refresh_raw,
    }))
}

pub async fn sign_in(
    State(state): State<AppState>,
    Json(req): Json<SigninRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let email = req.email.trim().to_lowercase();
    let password = req.password;

    if email.is_empty() || password.is_empty() {
        return Err(AppError::BadRequest("missing fields".into()));
    }

    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let user = repositories::users::find_user_by_email(&mut conn, &email)
        .map_err(|_| AppError::Unauthorized)?;

    let ok = hash_passwords::verify_passwd(&password, &user.password_hash);
    if !ok {
        return Err(AppError::Unauthorized);
    }

    let access_token = state.jwt.sign_access_token(user.id)?;

    let refresh_raw = tokens::generate_refresh_token();
    let refresh_hash = tokens::hash_refresh_token(&refresh_raw);

    let days_valid: i64 = 30;

    repositories::refresh_tokens::create_refresh_token(
        &mut conn,
        user.id,
        &refresh_hash,
        days_valid,
        None,
        None,
    )?;

    Ok(Json(AuthResponse {
        access_token,
        refresh_token: refresh_raw,
    }))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let refresh_raw = req.refresh_token.trim();
    if refresh_raw.is_empty() {
        return Err(AppError::BadRequest("missing refresh_token".into()));
    }

    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let days_valid: i64 = 30;

    let (new_row, new_raw) = repositories::refresh_tokens::rotate_refresh_token(
        &mut conn,
        refresh_raw,
        days_valid,
        None,
        None,
    )
    .map_err(|_| AppError::Unauthorized)?;

    let access_token = state.jwt.sign_access_token(new_row.user_id)?;

    Ok(Json(AuthResponse {
        access_token,
        refresh_token: new_raw,
    }))
}

pub async fn logout(
    State(state): State<AppState>,
    Json(req): Json<LogoutRequest>,
) -> Result<(), AppError> {
    let refresh_raw = req.refresh_token.trim();
    if refresh_raw.is_empty() {
        return Err(AppError::BadRequest("missing refresh_token".into()));
    }

    let refresh_hash = tokens::hash_refresh_token(refresh_raw);

    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    repositories::refresh_tokens::revoke_refresh_token_by_hash(&mut conn, &refresh_hash)
        .map_err(|_| AppError::Unauthorized)?;

    Ok(())
}
