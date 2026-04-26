use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::users;
#[derive(Debug, Clone, Queryable, Selectable, Serialize)]
#[diesel(table_name = users)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub display_name_mode: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable, Deserialize)]
#[diesel(table_name = users)]
pub struct NewUser<'a> {
    pub email: &'a str,
    pub username: &'a str,
    pub password_hash: &'a str,
}

use crate::schema::user_blocks;
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = user_blocks)]
pub struct UserBlock {
    pub blocker_id: Uuid,
    pub blocked_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = user_blocks)]
pub struct NewUserBlock {
    pub blocker_id: Uuid,
    pub blocked_id: Uuid,
}

use crate::schema::refresh_tokens;
#[derive(Debug, Queryable, Selectable, Serialize)]
#[diesel(table_name = refresh_tokens)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub replaced_by: Option<Uuid>,
    pub user_agent: Option<String>,
    pub ip: Option<IpNetwork>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = refresh_tokens)]
pub struct NewRefreshToken<'a> {
    pub user_id: Uuid,
    pub token_hash: &'a str,
    pub expires_at: DateTime<Utc>,
    pub user_agent: Option<&'a str>,
    pub ip: Option<IpNetwork>,
}

use crate::schema::servers;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = servers)]
pub struct Server {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable, Deserialize)]
#[diesel(table_name = servers)]
pub struct NewServer<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub created_by: Uuid,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = servers)]
pub struct ServerChangeset<'a> {
    pub name: Option<&'a str>,

    pub description: Option<Option<&'a str>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, DbEnum, Serialize, Deserialize)]
#[ExistingTypePath = "crate::schema::sql_types::MemberRole"]
pub enum MemberRole {
    #[db_rename = "OWNER"]
    Owner,
    #[db_rename = "ADMIN"]
    Admin,
    #[db_rename = "MEMBER"]
    Member,
}

use crate::schema::server_members;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = server_members)]
pub struct ServerMember {
    pub server_id: Uuid,
    pub user_id: Uuid,
    pub role: MemberRole,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Insertable, Deserialize)]
#[diesel(table_name = server_members)]
pub struct NewServerMember {
    pub server_id: Uuid,
    pub user_id: Uuid,
    pub role: MemberRole,
}

use crate::schema::server_bans;
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = server_bans)]
pub struct ServerBan {
    pub server_id: Uuid,
    pub user_id: Uuid,
    pub banned_by: Uuid,
    pub reason: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = server_bans)]
pub struct NewServerBan<'a> {
    pub server_id: Uuid,
    pub user_id: Uuid,
    pub banned_by: Uuid,
    pub reason: Option<&'a str>,
    pub expires_at: Option<DateTime<Utc>>,
}

use crate::schema::invites;
#[derive(Debug, Queryable, Selectable, Serialize)]
#[diesel(table_name = invites)]
pub struct Invite {
    pub id: Uuid,
    pub server_id: Uuid,
    pub code: String,
    pub created_by: Uuid,
    pub expires_at: Option<DateTime<Utc>>,
    pub max_uses: Option<i32>,
    pub uses_count: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable, Deserialize)]
#[diesel(table_name = invites)]
pub struct NewInvite<'a> {
    pub server_id: Uuid,
    pub code: &'a str,
    pub created_by: Uuid,
    pub expires_at: Option<DateTime<Utc>>,
    pub max_uses: Option<i32>,
}

use crate::schema::invite_uses;
#[derive(Debug, Queryable, Selectable, Serialize)]
#[diesel(table_name = invite_uses)]
pub struct InviteUse {
    pub invite_id: Uuid,
    pub user_id: Uuid,
    pub used_at: DateTime<Utc>,
}

#[derive(Debug, Insertable, Deserialize)]
#[diesel(table_name = invite_uses)]
pub struct NewInviteUse {
    pub invite_id: Uuid,
    pub user_id: Uuid,
}

use crate::schema::channels;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = channels)]
pub struct Channel {
    pub id: Uuid,
    pub server_id: Option<Uuid>,
    pub name: String,
    pub topic: Option<String>,
    pub position: i32,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable, Deserialize)]
#[diesel(table_name = channels)]
pub struct NewChannel<'a> {
    pub server_id: Uuid,
    pub name: &'a str,
    pub topic: Option<&'a str>,
    pub position: i32,
    pub created_by: Uuid,
}

use crate::schema::messages;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = messages)]
pub struct Message {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<Uuid>,
    pub pinned_at: Option<DateTime<Utc>>,
    pub pinned_by: Option<Uuid>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = messages)]
pub struct NewMessage<'a> {
    pub channel_id: Uuid,
    pub author_id: Uuid,
    pub content: &'a str,
}

use crate::schema::message_mentions;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = message_mentions)]
pub struct MessageMention {
    pub message_id: Uuid,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = message_mentions)]
pub struct NewMessageMention {
    pub message_id: Uuid,
    pub user_id: Uuid,
}

use crate::schema::message_reactions;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = message_reactions)]
pub struct MessageReaction {
    pub message_id: Uuid,
    pub user_id: Uuid,
    pub emoji: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = message_reactions)]
pub struct NewMessageReaction<'a> {
    pub message_id: Uuid,
    pub user_id: Uuid,
    pub emoji: &'a str,
}
