use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{errors::AppError, handlers::current_user::AuthUser, AppState};

#[derive(Serialize)]
pub struct BlockListItem {
    pub user_id: Uuid,
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub display_name_mode: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_blocks))
        .route("/{target_user_id}", post(block_user).delete(unblock_user))
}

pub async fn block_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(target_user_id): Path<Uuid>,
) -> Result<(), AppError> {
    if target_user_id == auth.id {
        return Err(AppError::BadRequest("cannot block yourself".into()));
    }

    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    crate::repositories::users::find_user_by_id(&mut conn, target_user_id)
        .map_err(|_| AppError::NotFound("user not found".into()))?;

    crate::repositories::user_blocks::block_user(&mut conn, auth.id, target_user_id)?;
    Ok(())
}

pub async fn unblock_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(target_user_id): Path<Uuid>,
) -> Result<(), AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    crate::repositories::user_blocks::unblock_user(&mut conn, auth.id, target_user_id)?;
    Ok(())
}

pub async fn list_blocks(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<BlockListItem>>, AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let blocks = crate::repositories::user_blocks::list_blocked_by_user(&mut conn, auth.id)?;
    let blocked_ids: Vec<Uuid> = blocks.iter().map(|b| b.blocked_id).collect();
    let users = crate::repositories::users::find_users_by_ids(&mut conn, &blocked_ids)?;
    let mut by_id = std::collections::HashMap::new();
    for user in users {
        by_id.insert(user.id, user);
    }

    let mapped = blocks
        .into_iter()
        .map(|b| {
            let user = by_id.get(&b.blocked_id);
            BlockListItem {
                user_id: b.blocked_id,
                username: user.map(|u| u.username.clone()),
                nickname: user.and_then(|u| u.nickname.clone()),
                avatar_url: user.and_then(|u| u.avatar_url.clone()),
                display_name_mode: user.map(|u| u.display_name_mode.clone()),
                created_at: b.created_at,
            }
        })
        .collect();

    Ok(Json(mapped))
}
