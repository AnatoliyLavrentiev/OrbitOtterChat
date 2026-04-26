use diesel::pg::PgConnection;
use diesel::result::{DatabaseErrorKind, Error as DieselError};

use crate::errors::AppError;
use crate::models::{RefreshToken, User};
use crate::repositories;

#[derive(Debug)]
pub struct LoginResult {
    pub user: User,
    pub refresh_token_raw: String,
}

#[derive(Debug)]
pub struct RefreshResult {
    pub user_id: uuid::Uuid,
    pub refresh_token_raw: String,
}

pub struct AuthService {
    refresh_days_valid: i64,
}

impl AuthService {
    pub fn new(refresh_days_valid: i64) -> Self {
        Self { refresh_days_valid }
    }

    pub fn login_stub_by_email(
        &self,
        conn: &mut PgConnection,
        email: &str,
        user_agent: Option<&str>,
        ip: Option<std::net::IpAddr>,
    ) -> Result<LoginResult, AppError> {
        let user = repositories::users::find_user_by_email(conn, email)
            .map_err(map_diesel_notfound_to_unauthorized)?;

        let refresh_raw = crate::security::tokens::generate_refresh_token();
        let token_hash = crate::security::tokens::hash_refresh_token(&refresh_raw);

        repositories::refresh_tokens::create_refresh_token(
            conn,
            user.id,
            &token_hash,
            self.refresh_days_valid,
            user_agent,
            ip,
        )
        .map_err(map_diesel_to_app_error)?;

        Ok(LoginResult {
            user,
            refresh_token_raw: refresh_raw,
        })
    }

    pub fn refresh(
        &self,
        conn: &mut PgConnection,
        refresh_token_raw: &str,
        user_agent: Option<&str>,
        ip: Option<std::net::IpAddr>,
    ) -> Result<RefreshResult, AppError> {
        let (new_row, new_raw) = repositories::refresh_tokens::rotate_refresh_token(
            conn,
            refresh_token_raw,
            self.refresh_days_valid,
            user_agent,
            ip,
        )
        .map_err(map_diesel_notfound_to_unauthorized)?;

        Ok(RefreshResult {
            user_id: new_row.user_id,
            refresh_token_raw: new_raw,
        })
    }

    pub fn logout(
        &self,
        conn: &mut PgConnection,
        refresh_token_raw: &str,
    ) -> Result<RefreshToken, AppError> {
        let token_hash = crate::security::tokens::hash_refresh_token(refresh_token_raw);

        let revoked = repositories::refresh_tokens::revoke_refresh_token_by_hash(conn, &token_hash)
            .map_err(map_diesel_notfound_to_unauthorized)?;

        Ok(revoked)
    }

    pub fn signup_stub(
        &self,
        conn: &mut PgConnection,
        email: &str,
        username: &str,
        password_hash: &str,
    ) -> Result<User, AppError> {
        repositories::users::create_user(conn, email, username, password_hash)
            .map_err(map_diesel_to_app_error)
    }
}

fn map_diesel_notfound_to_unauthorized(err: DieselError) -> AppError {
    match err {
        DieselError::NotFound => AppError::Unauthorized,
        other => map_diesel_to_app_error(other),
    }
}

fn map_diesel_to_app_error(err: DieselError) -> AppError {
    match err {
        DieselError::DatabaseError(kind, info) => match kind {
            DatabaseErrorKind::UniqueViolation => AppError::BadRequest(info.message().to_string()),
            _ => AppError::Internal(info.message().to_string()),
        },

        DieselError::NotFound => AppError::NotFound("record not found".into()),

        other => AppError::Internal(other.to_string()),
    }
}
