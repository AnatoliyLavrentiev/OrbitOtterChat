use axum::{extract::FromRequestParts, http::request::Parts};
use axum::{extract::Multipart, extract::State, Json};
use axum_extra::headers::{authorization::Bearer, Authorization};
use axum_extra::TypedHeader;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use jsonwebtoken::{decode, Algorithm, Validation};
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::{errors::AppError, security::jwt::AccessClaims, AppState};

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub display_name_mode: String,
}

#[derive(Deserialize)]
pub struct UpdateMeRequest {
    pub email: Option<String>,
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub display_name_mode: Option<String>,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let issuer = state.jwt.issuer.clone();
        let audience = state.jwt.audience.clone();
        let decoding = state.jwt.decoding.clone();

        async move {
            let TypedHeader(Authorization(bearer)) =
                TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
                    .await
                    .map_err(|_| AppError::Unauthorized)?;

            let token = bearer.token();

            let mut validation = Validation::new(Algorithm::HS256);
            validation.set_issuer(&[issuer.as_str()]);
            validation.set_audience(&[audience.as_str()]);

            let data = decode::<AccessClaims>(token, &decoding, &validation)
                .map_err(|_| AppError::Unauthorized)?;

            let user_id = Uuid::parse_str(&data.claims.sub).map_err(|_| AppError::Unauthorized)?;

            Ok(AuthUser { id: user_id })
        }
    }
}

pub async fn me(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<MeResponse>, AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let user = crate::repositories::users::find_user_by_id(&mut conn, auth.id)
        .map_err(|_| AppError::Unauthorized)?;

    Ok(Json(MeResponse {
        id: user.id,
        email: user.email,
        username: user.username,
        nickname: user.nickname,
        avatar_url: user.avatar_url,
        display_name_mode: user.display_name_mode,
    }))
}

pub async fn update_me(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<UpdateMeRequest>,
) -> Result<Json<MeResponse>, AppError> {
    let email = body
        .email
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let username = body
        .username
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let nickname =
        body.nickname
            .as_deref()
            .map(str::trim)
            .map(|v| if v.is_empty() { None } else { Some(v) });
    let avatar_url =
        body.avatar_url.as_deref().map(str::trim).map(
            |v| {
                if v.is_empty() {
                    None
                } else {
                    Some(v)
                }
            },
        );
    let display_name_mode = body.display_name_mode.as_deref().map(str::trim);
    if let Some(mode) = display_name_mode {
        if mode != "nickname" && mode != "username" {
            return Err(AppError::BadRequest(
                "display_name_mode must be 'nickname' or 'username'".into(),
            ));
        }
    }

    if email.is_none()
        && username.is_none()
        && nickname.is_none()
        && avatar_url.is_none()
        && display_name_mode.is_none()
    {
        return Err(AppError::BadRequest("nothing to update".into()));
    }

    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let user = crate::repositories::users::update_user_profile(
        &mut conn,
        auth.id,
        email,
        username,
        nickname,
        avatar_url,
        display_name_mode,
    )
    .map_err(|e| match e {
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
            AppError::Conflict("email or username already exists".into())
        }
        other => AppError::from(other),
    })?;

    Ok(Json(MeResponse {
        id: user.id,
        email: user.email,
        username: user.username,
        nickname: user.nickname,
        avatar_url: user.avatar_url,
        display_name_mode: user.display_name_mode,
    }))
}

pub async fn upload_avatar(
    State(state): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<MeResponse>, AppError> {
    let mut image: Option<(Vec<u8>, String)> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("invalid multipart data: {e}")))?
    {
        if field.name() != Some("avatar") {
            continue;
        }
        let content_type = field
            .content_type()
            .map(|v| v.to_string())
            .unwrap_or_default();
        if !content_type.starts_with("image/") {
            return Err(AppError::BadRequest("avatar must be an image file".into()));
        }
        let ext = if content_type.contains("png") {
            "png"
        } else if content_type.contains("jpeg") || content_type.contains("jpg") {
            "jpg"
        } else if content_type.contains("webp") {
            "webp"
        } else if content_type.contains("gif") {
            "gif"
        } else {
            return Err(AppError::BadRequest(
                "unsupported image type (png, jpg, webp, gif)".into(),
            ));
        };

        let bytes = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("failed to read avatar: {e}")))?;
        if bytes.is_empty() {
            return Err(AppError::BadRequest("avatar file is empty".into()));
        }
        if bytes.len() > 5 * 1024 * 1024 {
            return Err(AppError::BadRequest("avatar is too large (max 5MB)".into()));
        }
        image = Some((bytes.to_vec(), ext.to_string()));
        break;
    }

    let (bytes, ext) =
        image.ok_or_else(|| AppError::BadRequest("avatar file is required".into()))?;
    let file_name = format!("{}.{}", Uuid::new_v4().simple(), ext);
    let rel_path = format!("avatars/{file_name}");
    let full_dir = "uploads/avatars";
    let full_path = format!("{full_dir}/{file_name}");

    tokio::fs::create_dir_all(full_dir)
        .await
        .map_err(|e| AppError::Internal(format!("failed to create uploads directory: {e}")))?;
    tokio::fs::write(&full_path, bytes)
        .await
        .map_err(|e| AppError::Internal(format!("failed to store avatar: {e}")))?;

    let avatar_url = format!("/uploads/{rel_path}");

    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let user = crate::repositories::users::update_user_profile(
        &mut conn,
        auth.id,
        None,
        None,
        None,
        Some(Some(avatar_url.as_str())),
        None,
    )?;

    Ok(Json(MeResponse {
        id: user.id,
        email: user.email,
        username: user.username,
        nickname: user.nickname,
        avatar_url: user.avatar_url,
        display_name_mode: user.display_name_mode,
    }))
}
