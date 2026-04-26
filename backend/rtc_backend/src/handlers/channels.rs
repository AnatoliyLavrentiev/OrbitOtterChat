use crate::AppState;

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Extension, Json, Router,
};

use diesel::result::Error as DieselError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::DbPool,
    domain::permissions::{can_create_channel, can_delete_channel, can_update_channel, Role},
    errors::AppError,
    handlers::current_user::AuthUser,
    models::{Channel, MemberRole},
    repositories,
};

pub fn server_scoped_routes() -> Router<AppState> {
    let router: Router<AppState> = Router::new();
    router.route("/", post(create_channel).get(list_server_channels))
}

pub fn routes() -> Router<AppState> {
    let router: Router<AppState> = Router::new();
    router.route(
        "/{id}",
        get(get_channel).put(update_channel).delete(delete_channel),
    )
}

#[derive(Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    pub topic: Option<String>,
    pub position: Option<i32>,
}

#[derive(Deserialize)]
pub struct UpdateChannelRequest {
    pub name: Option<String>,
    pub topic: Option<Option<String>>,
    pub position: Option<i32>,
}

#[derive(Serialize)]
pub struct ChannelResponse {
    pub channel: Channel,
}

pub async fn create_channel(
    Extension(pool): Extension<DbPool>,
    auth: AuthUser,
    Path(server_id): Path<Uuid>,
    Json(req): Json<CreateChannelRequest>,
) -> Result<Json<ChannelResponse>, AppError> {
    let mut conn = pool.get().map_err(|e| AppError::Internal(e.to_string()))?;

    ensure_member(&mut conn, server_id, auth.id)?;

    let role = get_user_role(&mut conn, server_id, auth.id)?;
    if !can_create_channel(role) {
        return Err(AppError::Forbidden("not allowed to create channel".into()));
    }

    let name = req.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("channel name is empty".into()));
    }
    if name.len() > 64 {
        return Err(AppError::BadRequest(
            "channel name too long (max 64)".into(),
        ));
    }

    let channel = repositories::channels::create_channel(
        &mut conn,
        server_id,
        name,
        req.topic.as_deref(),
        req.position.unwrap_or(0),
        auth.id,
    )?;

    Ok(Json(ChannelResponse { channel }))
}

pub async fn list_server_channels(
    Extension(pool): Extension<DbPool>,
    auth: AuthUser,
    Path(server_id): Path<Uuid>,
) -> Result<Json<Vec<Channel>>, AppError> {
    let mut conn = pool.get().map_err(|e| AppError::Internal(e.to_string()))?;

    ensure_member(&mut conn, server_id, auth.id)?;

    let channels = repositories::channels::list_channels_by_server(&mut conn, server_id)?;
    Ok(Json(channels))
}

pub async fn get_channel(
    Extension(pool): Extension<DbPool>,
    auth: AuthUser,
    Path(channel_id): Path<Uuid>,
) -> Result<Json<ChannelResponse>, AppError> {
    let mut conn = pool.get().map_err(|e| AppError::Internal(e.to_string()))?;

    let channel = repositories::channels::find_by_id(&mut conn, channel_id)
        .map_err(map_diesel_notfound_to_notfound("channel not found"))?;

    if let Some(server_id) = channel.server_id {
        ensure_member(&mut conn, server_id, auth.id)?;
    }

    Ok(Json(ChannelResponse { channel }))
}

pub async fn update_channel(
    Extension(pool): Extension<DbPool>,
    auth: AuthUser,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<UpdateChannelRequest>,
) -> Result<Json<ChannelResponse>, AppError> {
    let mut conn = pool.get().map_err(|e| AppError::Internal(e.to_string()))?;

    let existing = repositories::channels::find_by_id(&mut conn, channel_id)
        .map_err(map_diesel_notfound_to_notfound("channel not found"))?;

    if let Some(server_id) = existing.server_id {
        ensure_member(&mut conn, server_id, auth.id)?;
        let role = get_user_role(&mut conn, server_id, auth.id)?;
        if !can_update_channel(role) {
            return Err(AppError::Forbidden("not allowed to update channel".into()));
        }
    }

    if let Some(name) = &req.name {
        let n = name.trim();
        if n.is_empty() {
            return Err(AppError::BadRequest("channel name is empty".into()));
        }
        if n.len() > 64 {
            return Err(AppError::BadRequest(
                "channel name too long (max 64)".into(),
            ));
        }
    }
    if let Some(position) = req.position {
        if position < 0 {
            return Err(AppError::BadRequest("position must be >= 0".into()));
        }
    }

    let normalized_topic: Option<Option<String>> = req.topic.map(|topic| {
        topic
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToString::to_string)
    });

    let updated = repositories::channels::update_channel(
        &mut conn,
        channel_id,
        req.name.as_deref().map(str::trim),
        normalized_topic.as_ref().map(|v| v.as_deref()),
        req.position,
    )?;

    Ok(Json(ChannelResponse { channel: updated }))
}

pub async fn delete_channel(
    Extension(pool): Extension<DbPool>,
    auth: AuthUser,
    Path(channel_id): Path<Uuid>,
) -> Result<(), AppError> {
    let mut conn = pool.get().map_err(|e| AppError::Internal(e.to_string()))?;

    let existing = repositories::channels::find_by_id(&mut conn, channel_id)
        .map_err(map_diesel_notfound_to_notfound("channel not found"))?;

    if let Some(server_id) = existing.server_id {
        ensure_member(&mut conn, server_id, auth.id)?;
        let role = get_user_role(&mut conn, server_id, auth.id)?;
        if !can_delete_channel(role) {
            return Err(AppError::Forbidden("not allowed to delete channel".into()));
        }
    }

    let deleted = repositories::channels::delete_channel(&mut conn, channel_id)?;
    if deleted == 0 {
        return Err(AppError::NotFound("channel not found".into()));
    }

    Ok(())
}

fn ensure_member(
    conn: &mut diesel::pg::PgConnection,
    server_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let is_member = repositories::server_members::is_member(conn, server_id, user_id)?;
    if !is_member {
        return Err(AppError::Forbidden("not a member of this server".into()));
    }
    Ok(())
}

fn get_user_role(
    conn: &mut diesel::pg::PgConnection,
    server_id: Uuid,
    user_id: Uuid,
) -> Result<Role, AppError> {
    let db_role = repositories::server_members::get_role(conn, server_id, user_id)
        .map_err(map_diesel_notfound_to_notfound("membership not found"))?;

    Ok(match db_role {
        MemberRole::Owner => Role::Owner,
        MemberRole::Admin => Role::Admin,
        MemberRole::Member => Role::Member,
    })
}

fn map_diesel_notfound_to_notfound(msg: &'static str) -> impl FnOnce(DieselError) -> AppError {
    move |e| match e {
        DieselError::NotFound => AppError::NotFound(msg.into()),
        other => AppError::Db(other),
    }
}

pub async fn list_my_dms(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<Channel>>, AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let channels = repositories::channels::list_dm_channels_for_user(&mut conn, auth.id)?;
    Ok(Json(channels))
}

pub async fn create_or_get_dm(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(target_user_id): Path<Uuid>,
) -> Result<Json<Channel>, AppError> {
    if target_user_id == auth.id {
        return Err(AppError::BadRequest(
            "cannot create direct message with yourself".into(),
        ));
    }

    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    crate::repositories::users::find_user_by_id(&mut conn, target_user_id)
        .map_err(|_| AppError::NotFound("target user not found".into()))?;

    let blocked = crate::repositories::user_blocks::is_blocked_any_direction(
        &mut conn,
        auth.id,
        target_user_id,
    )?;
    if blocked {
        return Err(AppError::Forbidden(
            "direct messages are disabled because one user blocked the other".into(),
        ));
    }

    let channel =
        repositories::channels::get_or_create_dm_channel(&mut conn, auth.id, target_user_id)?;

    Ok(Json(channel))
}

pub async fn delete_dm_history(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<Uuid>,
) -> Result<(), AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let channel = repositories::channels::find_by_id(&mut conn, channel_id)
        .map_err(map_diesel_notfound_to_notfound("channel not found"))?;

    if channel.server_id.is_some() {
        return Err(AppError::BadRequest(
            "this endpoint is only available for direct message channels".into(),
        ));
    }

    repositories::channels::ensure_dm_member(&mut conn, channel_id, auth.id)?;
    repositories::messages::hard_delete_messages_by_channel(&mut conn, channel_id)?;

    Ok(())
}
