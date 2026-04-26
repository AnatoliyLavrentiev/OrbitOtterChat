use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    errors::AppError,
    handlers::current_user::AuthUser,
    realtime::WsEvent,
    services::servers_service::{CreateInviteResult, CreateServerResult, ServersService},
    AppState,
};

#[derive(Deserialize)]
pub struct CreateServerBody {
    pub name: String,
    pub initial_channel_name: Option<String>,
    pub initial_channel_description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateServerBody {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
}

#[derive(Deserialize)]
pub struct JoinServerBody {
    pub invite_code: String,
}

#[derive(Deserialize)]
pub struct JoinByCodeBody {
    pub invite_code: String,
}

#[derive(Deserialize)]
pub struct UpdateRoleBody {
    pub role: crate::domain::permissions::Role,
}

#[derive(Deserialize)]
pub struct CreateInviteBody {
    pub max_uses: Option<i32>,
    pub expires_in_hours: Option<i64>,
}

#[derive(Deserialize)]
pub struct TransferOwnershipBody {
    pub new_owner_id: Uuid,
}

#[derive(Deserialize)]
pub struct BanMemberBody {
    pub user_id: Uuid,
    pub duration_hours: Option<i64>,
    pub reason: Option<String>,
}

#[derive(Deserialize)]
pub struct MemberPath {
    pub id: Uuid,
    pub user_id: Uuid,
}

#[derive(Serialize)]
pub struct MemberListItem {
    pub user_id: Uuid,
    pub role: crate::models::MemberRole,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub display_name_mode: String,
}

#[derive(Serialize)]
pub struct BanListItem {
    pub server_id: Uuid,
    pub user_id: Uuid,
    pub banned_by: Uuid,
    pub reason: Option<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub display_name_mode: Option<String>,
}

pub fn routes() -> Router<AppState> {
    let router: Router<AppState> = Router::new();

    router
        .route("/", post(create_server).get(list_servers))
        .route(
            "/{id}",
            get(get_server).put(update_server).delete(delete_server),
        )
        .route("/join", post(join_server_by_code))
        .route("/{id}/join", post(join_server))
        .route("/{id}/leave", delete(leave_server))
        .route("/{id}/members", get(list_members))
        .route(
            "/{id}/members/{user_id}",
            put(update_member_role).delete(kick_member),
        )
        .route("/{id}/invites", post(create_invite))
        .route("/{id}/ownership", put(transfer_ownership))
        .route("/{id}/bans", get(list_bans).post(ban_member))
        .route("/{id}/bans/{user_id}", delete(unban_member))
        .nest(
            "/{server_id}/channels",
            crate::handlers::channels::server_scoped_routes(),
        )
}

pub async fn create_server(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateServerBody>,
) -> Result<Json<CreateServerResult>, AppError> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("name is empty".into()));
    }
    if name.len() > 64 {
        return Err(AppError::BadRequest("name too long (max 64)".into()));
    }

    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let initial_channel_name = body
        .initial_channel_name
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let initial_channel_description = body
        .initial_channel_description
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());

    let res = ServersService::create_server(
        &mut conn,
        auth.id,
        name,
        initial_channel_name,
        initial_channel_description,
    )?;
    Ok(Json(res))
}

pub async fn list_servers(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<crate::models::Server>>, AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let servers = ServersService::list_my_servers(&mut conn, auth.id)?;
    Ok(Json(servers))
}

pub async fn get_server(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::models::Server>, AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let server = ServersService::get_server_details(&mut conn, auth.id, id)?;
    Ok(Json(server))
}

pub async fn update_server(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateServerBody>,
) -> Result<Json<crate::models::Server>, AppError> {
    if let Some(name) = &body.name {
        let n = name.trim();
        if n.is_empty() {
            return Err(AppError::BadRequest("name is empty".into()));
        }
        if n.len() > 64 {
            return Err(AppError::BadRequest("name too long (max 64)".into()));
        }
    }

    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let server = ServersService::update_server(
        &mut conn,
        auth.id,
        id,
        body.name.as_deref().map(str::trim),
        body.description.as_ref().map(|opt| opt.as_deref()),
    )?;

    Ok(Json(server))
}

pub async fn delete_server(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    ServersService::delete_server(&mut conn, auth.id, id)?;
    Ok(())
}

pub async fn join_server(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<JoinServerBody>,
) -> Result<(), AppError> {
    let code = body.invite_code.trim();
    if code.is_empty() {
        return Err(AppError::BadRequest("invite_code is empty".into()));
    }

    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let joined_id = ServersService::join_server_by_invite(&mut conn, auth.id, code)?;
    if joined_id != id {
        return Err(AppError::BadRequest(
            "invite_code is for another server".into(),
        ));
    }

    Ok(())
}

pub async fn join_server_by_code(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<JoinByCodeBody>,
) -> Result<Json<Uuid>, AppError> {
    let code = body.invite_code.trim();
    if code.is_empty() {
        return Err(AppError::BadRequest("invite_code is empty".into()));
    }

    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let server_id = ServersService::join_server_by_invite(&mut conn, auth.id, code)?;
    Ok(Json(server_id))
}

pub async fn leave_server(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    ServersService::leave_server(&mut conn, auth.id, id)?;
    Ok(())
}

pub async fn list_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<MemberListItem>>, AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let members = ServersService::list_members(&mut conn, auth.id, id)?;
    let member_ids: Vec<Uuid> = members.iter().map(|m| m.user_id).collect();
    let users = crate::repositories::users::find_users_by_ids(&mut conn, &member_ids)?;
    let mut by_id = std::collections::HashMap::new();
    for user in users {
        by_id.insert(user.id, user);
    }

    let mapped = members
        .into_iter()
        .map(|m| {
            if let Some(user) = by_id.get(&m.user_id) {
                MemberListItem {
                    user_id: m.user_id,
                    role: m.role,
                    username: user.username.clone(),
                    nickname: user.nickname.clone(),
                    avatar_url: user.avatar_url.clone(),
                    display_name_mode: user.display_name_mode.clone(),
                }
            } else {
                MemberListItem {
                    user_id: m.user_id,
                    role: m.role,
                    username: m.user_id.to_string(),
                    nickname: None,
                    avatar_url: None,
                    display_name_mode: "nickname".to_string(),
                }
            }
        })
        .collect();

    Ok(Json(mapped))
}

pub async fn update_member_role(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(MemberPath { id, user_id }): Path<MemberPath>,
    Json(body): Json<UpdateRoleBody>,
) -> Result<(), AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    ServersService::update_member_role(&mut conn, auth.id, id, user_id, body.role)?;
    Ok(())
}

pub async fn create_invite(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<CreateInviteBody>,
) -> Result<Json<CreateInviteResult>, AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if let Some(max_uses) = body.max_uses {
        if max_uses <= 0 {
            return Err(AppError::BadRequest("max_uses must be > 0".into()));
        }
    }
    if let Some(hours) = body.expires_in_hours {
        if hours <= 0 {
            return Err(AppError::BadRequest("expires_in_hours must be > 0".into()));
        }
    }

    let invite = ServersService::create_invite(
        &mut conn,
        auth.id,
        id,
        body.max_uses,
        body.expires_in_hours,
    )?;

    Ok(Json(invite))
}

pub async fn transfer_ownership(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<TransferOwnershipBody>,
) -> Result<(), AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    ServersService::transfer_ownership(&mut conn, auth.id, id, body.new_owner_id)?;
    Ok(())
}

pub async fn kick_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(MemberPath { id, user_id }): Path<MemberPath>,
) -> Result<(), AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    ServersService::kick_member(&mut conn, auth.id, id, user_id)?;
    Ok(())
}

pub async fn ban_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<BanMemberBody>,
) -> Result<(), AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let ban = ServersService::ban_member(
        &mut conn,
        auth.id,
        id,
        body.user_id,
        body.duration_hours,
        body.reason.as_deref(),
    )?;

    state.ws.publish(WsEvent::ServerBanApplied {
        server_id: id,
        user_id: body.user_id,
        banned_by: auth.id,
        reason: ban.reason.clone(),
        expires_at: ban.expires_at,
    });
    Ok(())
}

pub async fn unban_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(MemberPath { id, user_id }): Path<MemberPath>,
) -> Result<(), AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    ServersService::unban_member(&mut conn, auth.id, id, user_id)?;
    Ok(())
}

pub async fn list_bans(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<BanListItem>>, AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let bans = ServersService::list_bans(&mut conn, auth.id, id)?;
    let banned_ids: Vec<Uuid> = bans.iter().map(|b| b.user_id).collect();
    let users = crate::repositories::users::find_users_by_ids(&mut conn, &banned_ids)?;
    let mut by_id = std::collections::HashMap::new();
    for user in users {
        by_id.insert(user.id, user);
    }

    let mapped = bans
        .into_iter()
        .map(|ban| {
            let user = by_id.get(&ban.user_id);
            BanListItem {
                server_id: ban.server_id,
                user_id: ban.user_id,
                banned_by: ban.banned_by,
                reason: ban.reason,
                expires_at: ban.expires_at,
                created_at: ban.created_at,
                username: user.map(|u| u.username.clone()),
                nickname: user.and_then(|u| u.nickname.clone()),
                avatar_url: user.and_then(|u| u.avatar_url.clone()),
                display_name_mode: user.map(|u| u.display_name_mode.clone()),
            }
        })
        .collect();

    Ok(Json(mapped))
}
