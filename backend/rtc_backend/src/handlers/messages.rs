use crate::AppState;

use axum::{
    extract::{Multipart, Path, State},
    routing::{post, put},
    Json, Router,
};

use diesel::result::Error as DieselError;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::{
    domain::permissions::{can_delete_message, Role},
    errors::AppError,
    handlers::current_user::AuthUser,
    models::{MemberRole, Message},
    realtime::WsEvent,
    repositories,
};

pub fn channel_scoped_routes() -> Router<AppState> {
    let router: Router<AppState> = Router::new();
    router
        .route("/messages", post(send_message).get(list_channel_messages))
        .route("/messages/file", post(upload_file_message))
}

pub fn routes() -> Router<AppState> {
    let router: Router<AppState> = Router::new();
    router
        .route("/{id}", put(edit_message).delete(delete_message))
        .route("/{id}/pin", put(pin_message).delete(unpin_message))
        .route(
            "/{id}/reactions",
            post(toggle_message_reaction).get(list_message_reactions),
        )
}

#[derive(Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
}

#[derive(Deserialize)]
pub struct EditMessageRequest {
    pub content: String,
}

#[derive(Serialize)]
pub struct MessageResponse {
    pub message: Message,
}

#[derive(Deserialize)]
pub struct ToggleMessageReactionRequest {
    pub emoji: String,
}

#[derive(Serialize)]
pub struct MessageReactionSummary {
    pub emoji: String,
    pub count: i64,
    pub reacted: bool,
}

#[derive(Serialize)]
pub struct MessageReactionsResponse {
    pub reactions: Vec<MessageReactionSummary>,
}

const MAX_ATTACHMENT_SIZE_BYTES: usize = 10 * 1024 * 1024;

pub async fn send_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    let content = req.content.trim();

    if content.is_empty() {
        return Err(AppError::BadRequest("message is empty".into()));
    }

    let is_gif = content.starts_with("gif::");

    if is_gif {
        let gif_url = &content["gif::".len()..];
        if !gif_url.contains(".giphy.com/") {
            return Err(AppError::BadRequest("invalid gif url".into()));
        }
    } else if content.len() > 2000 {
        return Err(AppError::BadRequest("message too long (max 2000)".into()));
    }

    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let channel = repositories::channels::find_by_id(&mut conn, channel_id)
        .map_err(map_diesel_notfound_to_notfound("channel not found"))?;

    match channel.server_id {
        Some(server_id) => {
            ensure_member(&mut conn, server_id, auth.id)?;
            let mentioned_user_ids = if is_gif {
                Vec::new()
            } else {
                resolve_mentioned_users(&mut conn, server_id, content)?
            };

            let message =
                repositories::messages::create_message(&mut conn, channel_id, auth.id, content)?;
            let _ = repositories::message_mentions::replace_mentions_for_message(
                &mut conn,
                message.id,
                &mentioned_user_ids,
            )?;

            state.ws.publish(WsEvent::MessageNew {
                server_id: Some(server_id),
                channel_id,
                message_id: message.id,
                author_id: message.author_id,
                content: message.content.clone(),
                dm_member_ids: None,
            });

            Ok(Json(MessageResponse { message }))
        }
        None => {
            repositories::channels::ensure_dm_member(&mut conn, channel_id, auth.id)?;
            let dm_member_ids = repositories::channels::list_dm_member_ids(&mut conn, channel_id)?;
            if let Some(other_user_id) = dm_member_ids.iter().find(|&&id| id != auth.id).copied() {
                let blocked = repositories::user_blocks::is_blocked_any_direction(
                    &mut conn,
                    auth.id,
                    other_user_id,
                )?;
                if blocked {
                    return Err(AppError::Forbidden(
                        "direct messages are disabled because one user blocked the other".into(),
                    ));
                }
            }
            let message =
                repositories::messages::create_message(&mut conn, channel_id, auth.id, content)?;
            state.ws.publish(WsEvent::MessageNew {
                server_id: None,
                channel_id,
                message_id: message.id,
                author_id: message.author_id,
                content: message.content.clone(),
                dm_member_ids: Some(dm_member_ids),
            });

            Ok(Json(MessageResponse { message }))
        }
    }
}

pub async fn upload_file_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<Json<MessageResponse>, AppError> {
    let mut upload: Option<(String, String, Vec<u8>)> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("invalid multipart data: {e}")))?
    {
        if field.name() != Some("file") {
            continue;
        }

        let original_name = field.file_name().unwrap_or("attachment").to_string();
        let content_type = field
            .content_type()
            .map(str::to_string)
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let bytes = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("failed to read file: {e}")))?;

        if bytes.is_empty() {
            return Err(AppError::BadRequest("file is empty".into()));
        }
        if bytes.len() > MAX_ATTACHMENT_SIZE_BYTES {
            return Err(AppError::BadRequest("file is too large (max 10MB)".into()));
        }

        upload = Some((
            sanitize_attachment_filename(&original_name),
            content_type,
            bytes.to_vec(),
        ));
        break;
    }

    let (filename, content_type, bytes) =
        upload.ok_or_else(|| AppError::BadRequest("file is required".into()))?;

    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let channel = repositories::channels::find_by_id(&mut conn, channel_id)
        .map_err(map_diesel_notfound_to_notfound("channel not found"))?;

    let (server_id, dm_member_ids) = match channel.server_id {
        Some(server_id) => {
            ensure_member(&mut conn, server_id, auth.id)?;
            (Some(server_id), None)
        }
        None => {
            repositories::channels::ensure_dm_member(&mut conn, channel_id, auth.id)?;
            let dm_member_ids = repositories::channels::list_dm_member_ids(&mut conn, channel_id)?;
            if let Some(other_user_id) = dm_member_ids.iter().find(|&&id| id != auth.id).copied() {
                let blocked = repositories::user_blocks::is_blocked_any_direction(
                    &mut conn,
                    auth.id,
                    other_user_id,
                )?;
                if blocked {
                    return Err(AppError::Forbidden(
                        "direct messages are disabled because one user blocked the other".into(),
                    ));
                }
            }
            (None, Some(dm_member_ids))
        }
    };

    let stored_name = format!("{}-{}", Uuid::new_v4().simple(), filename);
    let rel_path = format!("attachments/{stored_name}");
    let full_dir = "uploads/attachments";
    let full_path = format!("{full_dir}/{stored_name}");

    tokio::fs::create_dir_all(full_dir)
        .await
        .map_err(|e| AppError::Internal(format!("failed to create uploads directory: {e}")))?;
    let mut file = tokio::fs::File::create(&full_path)
        .await
        .map_err(|e| AppError::Internal(format!("failed to store file: {e}")))?;
    file.write_all(&bytes)
        .await
        .map_err(|e| AppError::Internal(format!("failed to store file: {e}")))?;

    let content = format!(
        "file::/uploads/{rel_path}::{}::{}::{}",
        filename,
        sanitize_file_mime(&content_type),
        bytes.len()
    );
    let message = repositories::messages::create_message(&mut conn, channel_id, auth.id, &content)?;

    state.ws.publish(WsEvent::MessageNew {
        server_id,
        channel_id,
        message_id: message.id,
        author_id: message.author_id,
        content: message.content.clone(),
        dm_member_ids,
    });

    Ok(Json(MessageResponse { message }))
}

pub async fn list_channel_messages(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<Uuid>,
) -> Result<Json<Vec<Message>>, AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let channel = repositories::channels::find_by_id(&mut conn, channel_id)
        .map_err(map_diesel_notfound_to_notfound("channel not found"))?;

    if let Some(server_id) = channel.server_id {
        ensure_member(&mut conn, server_id, auth.id)?;
    } else {
        repositories::channels::ensure_dm_member(&mut conn, channel_id, auth.id)?;
    }

    let limit: i64 = 50;
    let offset: i64 = 0;

    let messages =
        repositories::messages::list_messages_by_channel(&mut conn, channel_id, limit, offset)?;

    Ok(Json(messages))
}

pub async fn edit_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(message_id): Path<Uuid>,
    Json(req): Json<EditMessageRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    let content = req.content.trim();
    if content.is_empty() {
        return Err(AppError::BadRequest("message is empty".into()));
    }
    if content.len() > 2000 {
        return Err(AppError::BadRequest("message too long (max 2000)".into()));
    }

    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let existing = repositories::messages::find_by_id(&mut conn, message_id)
        .map_err(map_diesel_notfound_to_notfound("message not found"))?;

    let channel = repositories::channels::find_by_id(&mut conn, existing.channel_id)
        .map_err(map_diesel_notfound_to_notfound("channel not found"))?;

    if let Some(server_id) = channel.server_id {
        ensure_member(&mut conn, server_id, auth.id)?;
    } else {
        repositories::channels::ensure_dm_member(&mut conn, channel.id, auth.id)?;
    }

    if existing.author_id != auth.id {
        return Err(AppError::Forbidden("only author can edit message".into()));
    }

    let mentioned_user_ids = if let Some(server_id) = channel.server_id {
        resolve_mentioned_users(&mut conn, server_id, content)?
    } else {
        vec![]
    };

    let message = repositories::messages::update_message_content(&mut conn, message_id, content)?;
    let _ = repositories::message_mentions::replace_mentions_for_message(
        &mut conn,
        message.id,
        &mentioned_user_ids,
    )?;
    Ok(Json(MessageResponse { message }))
}

pub async fn delete_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(message_id): Path<Uuid>,
) -> Result<(), AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let existing = repositories::messages::find_by_id(&mut conn, message_id)
        .map_err(map_diesel_notfound_to_notfound("message not found"))?;

    let channel = repositories::channels::find_by_id(&mut conn, existing.channel_id)
        .map_err(map_diesel_notfound_to_notfound("channel not found"))?;
    let role = if let Some(server_id) = channel.server_id {
        ensure_member(&mut conn, server_id, auth.id)?;
        get_user_role(&mut conn, server_id, auth.id)?
    } else {
        repositories::channels::ensure_dm_member(&mut conn, channel.id, auth.id)?;
        Role::Member
    };

    if !can_delete_message(role, existing.author_id == auth.id) {
        return Err(AppError::Forbidden(
            "not allowed to delete this message".into(),
        ));
    }

    let deleted = repositories::messages::soft_delete_message(&mut conn, message_id, auth.id)?;
    if deleted == 0 {
        return Err(AppError::NotFound("message not found".into()));
    }
    let dm_member_ids = if channel.server_id.is_none() {
        Some(repositories::channels::list_dm_member_ids(
            &mut conn, channel.id,
        )?)
    } else {
        None
    };
    state.ws.publish(WsEvent::MessageDeleted {
        server_id: channel.server_id,
        channel_id: channel.id,
        message_id,
        deleted_by: auth.id,
        dm_member_ids,
    });
    Ok(())
}

pub async fn pin_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(message_id): Path<Uuid>,
) -> Result<Json<MessageResponse>, AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let existing = repositories::messages::find_by_id(&mut conn, message_id)
        .map_err(map_diesel_notfound_to_notfound("message not found"))?;

    ensure_can_pin_message(&mut conn, &existing, auth.id)?;

    let message = repositories::messages::pin_message(&mut conn, message_id, auth.id)?;
    Ok(Json(MessageResponse { message }))
}

pub async fn unpin_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(message_id): Path<Uuid>,
) -> Result<Json<MessageResponse>, AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let existing = repositories::messages::find_by_id(&mut conn, message_id)
        .map_err(map_diesel_notfound_to_notfound("message not found"))?;

    ensure_can_pin_message(&mut conn, &existing, auth.id)?;

    let message = repositories::messages::unpin_message(&mut conn, message_id)?;
    Ok(Json(MessageResponse { message }))
}

pub async fn list_message_reactions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(message_id): Path<Uuid>,
) -> Result<Json<MessageReactionsResponse>, AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let message = repositories::messages::find_by_id(&mut conn, message_id)
        .map_err(map_diesel_notfound_to_notfound("message not found"))?;

    if message.deleted_at.is_some() {
        return Err(AppError::NotFound("message not found".into()));
    }

    let channel = repositories::channels::find_by_id(&mut conn, message.channel_id)
        .map_err(map_diesel_notfound_to_notfound("channel not found"))?;
    if let Some(server_id) = channel.server_id {
        ensure_member(&mut conn, server_id, auth.id)?;
    } else {
        repositories::channels::ensure_dm_member(&mut conn, channel.id, auth.id)?;
    }

    let reactions = build_message_reaction_summary(&mut conn, message_id, auth.id)?;
    Ok(Json(MessageReactionsResponse { reactions }))
}

pub async fn toggle_message_reaction(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(message_id): Path<Uuid>,
    Json(req): Json<ToggleMessageReactionRequest>,
) -> Result<Json<MessageReactionsResponse>, AppError> {
    let emoji = req.emoji.trim();
    if emoji.is_empty() {
        return Err(AppError::BadRequest("emoji is empty".into()));
    }
    if emoji.chars().count() > 16 {
        return Err(AppError::BadRequest("emoji too long (max 16 chars)".into()));
    }

    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let message = repositories::messages::find_by_id(&mut conn, message_id)
        .map_err(map_diesel_notfound_to_notfound("message not found"))?;

    if message.deleted_at.is_some() {
        return Err(AppError::NotFound("message not found".into()));
    }

    let channel = repositories::channels::find_by_id(&mut conn, message.channel_id)
        .map_err(map_diesel_notfound_to_notfound("channel not found"))?;
    if let Some(server_id) = channel.server_id {
        ensure_member(&mut conn, server_id, auth.id)?;
    } else {
        repositories::channels::ensure_dm_member(&mut conn, channel.id, auth.id)?;
    }

    let _ =
        repositories::message_reactions::toggle_reaction(&mut conn, message_id, auth.id, emoji)?;
    let reactions = build_message_reaction_summary(&mut conn, message_id, auth.id)?;
    let dm_member_ids = if channel.server_id.is_none() {
        Some(repositories::channels::list_dm_member_ids(
            &mut conn, channel.id,
        )?)
    } else {
        None
    };
    state.ws.publish(WsEvent::MessageReactionsUpdated {
        server_id: channel.server_id,
        channel_id: channel.id,
        message_id,
        dm_member_ids,
    });

    Ok(Json(MessageReactionsResponse { reactions }))
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

fn ensure_can_pin_message(
    conn: &mut diesel::pg::PgConnection,
    message: &Message,
    user_id: Uuid,
) -> Result<(), AppError> {
    if message.deleted_at.is_some() {
        return Err(AppError::NotFound("message not found".into()));
    }

    let channel = repositories::channels::find_by_id(conn, message.channel_id)
        .map_err(map_diesel_notfound_to_notfound("channel not found"))?;

    if let Some(server_id) = channel.server_id {
        ensure_member(conn, server_id, user_id)?;
        let role = get_user_role(conn, server_id, user_id)?;
        if !can_delete_message(role, message.author_id == user_id) {
            return Err(AppError::Forbidden(
                "not allowed to pin this message".into(),
            ));
        }
    } else {
        repositories::channels::ensure_dm_member(conn, channel.id, user_id)?;
    }

    Ok(())
}

fn map_diesel_notfound_to_notfound(msg: &'static str) -> impl FnOnce(DieselError) -> AppError {
    move |e| match e {
        DieselError::NotFound => AppError::NotFound(msg.into()),
        other => AppError::Db(other),
    }
}

fn sanitize_attachment_filename(filename: &str) -> String {
    let sanitized: String = filename
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches(['.', '_', '-']).to_string();
    if trimmed.is_empty() {
        "attachment".to_string()
    } else {
        trimmed.chars().take(120).collect()
    }
}

fn sanitize_file_mime(mime: &str) -> String {
    let sanitized: String = mime
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.' | '+'))
        .collect();
    if sanitized.is_empty() {
        "application/octet-stream".to_string()
    } else {
        sanitized.chars().take(120).collect()
    }
}

fn build_message_reaction_summary(
    conn: &mut diesel::pg::PgConnection,
    message_id: Uuid,
    user_id: Uuid,
) -> Result<Vec<MessageReactionSummary>, AppError> {
    let counts = repositories::message_reactions::list_reaction_counts(conn, message_id)?;
    let my_emojis: HashSet<String> =
        repositories::message_reactions::list_user_emojis(conn, message_id, user_id)?
            .into_iter()
            .collect();

    Ok(counts
        .into_iter()
        .map(|(emoji, count)| MessageReactionSummary {
            reacted: my_emojis.contains(&emoji),
            emoji,
            count,
        })
        .collect())
}

fn extract_mention_handles(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for token in content.split_whitespace() {
        let Some(raw) = token.strip_prefix('@') else {
            continue;
        };
        let handle = raw
            .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
            .to_lowercase();
        if handle.len() < 3 {
            continue;
        }
        if !handle
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            continue;
        }
        if seen.insert(handle.clone()) {
            out.push(handle);
        }
    }
    out
}

fn resolve_mentioned_users(
    conn: &mut diesel::pg::PgConnection,
    server_id: Uuid,
    content: &str,
) -> Result<Vec<Uuid>, AppError> {
    let handles = extract_mention_handles(content);
    if handles.is_empty() {
        return Ok(Vec::new());
    }

    let members = repositories::server_members::list_members(conn, server_id)?;
    let user_ids: Vec<Uuid> = members.iter().map(|m| m.user_id).collect();
    let users = repositories::users::find_users_by_ids(conn, &user_ids)?;

    let mut by_handle: HashMap<String, Uuid> = HashMap::new();
    for user in users {
        by_handle.insert(user.username.to_lowercase(), user.id);
        if let Some(nickname) = user.nickname {
            if !nickname.trim().is_empty() {
                by_handle.insert(nickname.to_lowercase(), user.id);
            }
        }
    }

    let mut resolved = Vec::new();
    let mut resolved_seen = HashSet::new();
    for handle in handles {
        let Some(user_id) = by_handle.get(&handle).copied() else {
            continue;
        };
        if resolved_seen.insert(user_id) {
            resolved.push(user_id);
        }
    }
    Ok(resolved)
}
