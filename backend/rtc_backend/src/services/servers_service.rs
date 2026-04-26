use diesel::pg::PgConnection;
use diesel::Connection;
use uuid::Uuid;

use crate::domain::permissions::{can_leave_server, can_manage_roles, Role};
use crate::errors::AppError;
use crate::models::MemberRole;
use crate::models::Server;
use crate::repositories;

use chrono::Utc;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CreateServerResult {
    pub server_id: Uuid,
    pub invite_code: String,
}

#[derive(Debug, Serialize)]
pub struct CreateInviteResult {
    pub invite_code: String,
}

pub struct ServersService;

impl ServersService {
    pub fn create_server(
        conn: &mut PgConnection,
        owner_id: Uuid,
        name: &str,
        initial_channel_name: Option<&str>,
        initial_channel_description: Option<&str>,
    ) -> Result<CreateServerResult, AppError> {
        validate_server_name(name)?;
        let first_channel_name = initial_channel_name.unwrap_or("general").trim();
        validate_channel_name(first_channel_name)?;
        let first_channel_description = initial_channel_description
            .map(str::trim)
            .filter(|v| !v.is_empty());
        let invite_code = generate_invite_code();

        conn.transaction(|conn| {
            let server = repositories::servers::create_server(conn, owner_id, name, None)?;

            repositories::server_members::add_member(conn, server.id, owner_id, MemberRole::Owner)?;

            let _ = repositories::channels::create_channel(
                conn,
                server.id,
                first_channel_name,
                first_channel_description,
                0,
                owner_id,
            )?;

            repositories::invites::create_invite(
                conn,
                server.id,
                &invite_code,
                owner_id,
                None,
                None,
            )?;

            Ok(CreateServerResult {
                server_id: server.id,
                invite_code,
            })
        })
    }

    pub fn list_my_servers(
        conn: &mut PgConnection,
        user_id: Uuid,
    ) -> Result<Vec<Server>, AppError> {
        let servers = repositories::servers::list_for_user(conn, user_id)?;
        Ok(servers)
    }

    pub fn get_server_details(
        conn: &mut PgConnection,
        user_id: Uuid,
        server_id: Uuid,
    ) -> Result<crate::models::Server, AppError> {
        if !repositories::server_members::is_member(conn, server_id, user_id)? {
            return Err(AppError::Forbidden("not a member of this server".into()));
        }
        let server = repositories::servers::find_by_id(conn, server_id)
            .map_err(map_diesel_notfound_to_notfound)?;
        Ok(server)
    }

    pub fn join_server_by_invite(
        conn: &mut PgConnection,
        user_id: Uuid,
        invite_code: &str,
    ) -> Result<Uuid, AppError> {
        let invite_code = invite_code.trim();
        if invite_code.is_empty() {
            return Err(AppError::BadRequest("invite_code is empty".into()));
        }

        conn.transaction(|conn| {
            let invite = repositories::invites::find_by_code(conn, invite_code)
                .map_err(map_diesel_notfound_to_notfound)?;

            if let Some(expires_at) = invite.expires_at {
                if expires_at < Utc::now() {
                    return Err(AppError::BadRequest("invite expired".into()));
                }
            }

            repositories::servers::find_by_id(conn, invite.server_id)
                .map_err(map_diesel_notfound_to_notfound)?;

            if repositories::server_bans::is_active_ban(conn, invite.server_id, user_id)? {
                return Err(AppError::Forbidden(
                    "you are banned from this server".into(),
                ));
            }

            if repositories::server_members::is_member(conn, invite.server_id, user_id)? {
                return Err(AppError::Conflict("already a member".into()));
            }

            repositories::invites::increment_uses_count(conn, invite.id).map_err(|e| match e {
                DieselError::NotFound => AppError::BadRequest("invite exhausted".into()),
                other => AppError::from(other),
            })?;

            repositories::server_members::add_member(
                conn,
                invite.server_id,
                user_id,
                MemberRole::Member,
            )?;

            repositories::invite_uses::create_invite_use(conn, invite.id, user_id)?;

            Ok(invite.server_id)
        })
    }

    pub fn create_invite(
        conn: &mut PgConnection,
        actor_id: Uuid,
        server_id: Uuid,
        max_uses: Option<i32>,
        expires_in_hours: Option<i64>,
    ) -> Result<CreateInviteResult, AppError> {
        conn.transaction(|conn| {
            let actor_role = Self::get_user_role(conn, server_id, actor_id)?;
            if !matches!(actor_role, Role::Owner | Role::Admin) {
                return Err(AppError::Forbidden(
                    "only admin/owner can create invites".into(),
                ));
            }

            let invite_code = generate_invite_code();
            let expires_at = expires_in_hours.map(|h| Utc::now() + chrono::Duration::hours(h));

            repositories::invites::create_invite(
                conn,
                server_id,
                &invite_code,
                actor_id,
                expires_at,
                max_uses,
            )?;

            Ok(CreateInviteResult { invite_code })
        })
    }

    pub fn list_members(
        conn: &mut PgConnection,
        actor_id: Uuid,
        server_id: Uuid,
    ) -> Result<Vec<crate::models::ServerMember>, AppError> {
        let _ = Self::get_user_role(conn, server_id, actor_id)?;
        Ok(repositories::server_members::list_members(conn, server_id)?)
    }

    pub fn join_server_in_server(
        conn: &mut PgConnection,
        user_id: Uuid,
        server_id: Uuid,
        invite_code: &str,
    ) -> Result<(), AppError> {
        conn.transaction(|conn| {
            let invite = repositories::invites::find_by_code(conn, invite_code)
                .map_err(map_diesel_notfound_to_notfound)?;

            if invite.server_id != server_id {
                return Err(AppError::BadRequest(
                    "invite does not belong to this server".into(),
                ));
            }

            drop(invite);
            let _ = Self::join_server_by_invite(conn, user_id, invite_code)?;
            Ok(())
        })
    }

    pub fn leave_server(
        conn: &mut PgConnection,
        user_id: Uuid,
        server_id: Uuid,
    ) -> Result<(), AppError> {
        conn.transaction(|conn| {
            let role = Self::get_user_role(conn, server_id, user_id)?;

            if !can_leave_server(role) {
                return Err(AppError::Forbidden("owner cannot leave server".into()));
            }

            let deleted = repositories::server_members::remove_member(conn, server_id, user_id)?;
            if deleted == 0 {
                return Err(AppError::NotFound("membership not found".into()));
            }

            Ok(())
        })
    }

    pub fn update_server(
        conn: &mut PgConnection,
        actor_id: Uuid,
        server_id: Uuid,
        name: Option<&str>,
        description: Option<Option<&str>>,
    ) -> Result<crate::models::Server, AppError> {
        Self::validate_server_name_opt(name)?;

        conn.transaction(|conn| {
            let actor_role = Self::get_user_role(conn, server_id, actor_id)?;
            if !matches!(actor_role, Role::Owner | Role::Admin) {
                return Err(AppError::Forbidden(
                    "only admin/owner can update server".into(),
                ));
            }
            let server = repositories::servers::update_server(conn, server_id, name, description)
                .map_err(map_diesel_notfound_to_notfound)?;
            Ok(server)
        })
    }

    fn validate_server_name_opt(name: Option<&str>) -> Result<(), AppError> {
        if let Some(name) = name {
            validate_server_name(name)?;
        }
        Ok(())
    }

    pub fn delete_server(
        conn: &mut PgConnection,
        actor_id: Uuid,
        server_id: Uuid,
    ) -> Result<(), AppError> {
        conn.transaction(|conn| {
            let actor_role = Self::get_user_role(conn, server_id, actor_id)?;
            if actor_role != Role::Owner {
                return Err(AppError::Forbidden("only owner can delete server".into()));
            }

            let deleted = repositories::servers::delete_server(conn, server_id)?;
            if deleted == 0 {
                return Err(AppError::NotFound("server not found".into()));
            }
            Ok(())
        })
    }

    pub fn update_member_role(
        conn: &mut PgConnection,
        actor_id: Uuid,
        server_id: Uuid,
        target_user_id: Uuid,
        new_role: Role,
    ) -> Result<(), AppError> {
        conn.transaction(|conn| {
            let actor_role = Self::get_user_role(conn, server_id, actor_id)?;
            if !can_manage_roles(actor_role) {
                return Err(AppError::Forbidden("only owner can manage roles".into()));
            }

            if matches!(new_role, Role::Owner) {
                return Err(AppError::BadRequest(
                    "use transfer_ownership to set Owner".into(),
                ));
            }

            let target_role = Self::get_user_role(conn, server_id, target_user_id)?;
            if target_role == Role::Owner {
                return Err(AppError::BadRequest(
                    "cannot change Owner role; use transfer_ownership".into(),
                ));
            }

            repositories::server_members::update_role(
                conn,
                server_id,
                target_user_id,
                role_to_member_role(new_role),
            )?;

            Ok(())
        })
    }

    pub fn kick_member(
        conn: &mut PgConnection,
        actor_id: Uuid,
        server_id: Uuid,
        target_user_id: Uuid,
    ) -> Result<(), AppError> {
        if actor_id == target_user_id {
            return Err(AppError::BadRequest("cannot kick yourself".into()));
        }

        conn.transaction(|conn| {
            let actor_role = Self::get_user_role(conn, server_id, actor_id)?;
            if !matches!(actor_role, Role::Owner | Role::Admin) {
                return Err(AppError::Forbidden(
                    "only admin/owner can kick members".into(),
                ));
            }

            let target_role = Self::get_user_role(conn, server_id, target_user_id)?;
            if target_role == Role::Owner {
                return Err(AppError::Forbidden("owner cannot be kicked".into()));
            }
            if actor_role == Role::Admin && target_role == Role::Admin {
                return Err(AppError::Forbidden(
                    "admin cannot kick another admin".into(),
                ));
            }

            let deleted =
                repositories::server_members::remove_member(conn, server_id, target_user_id)?;
            if deleted == 0 {
                return Err(AppError::NotFound("membership not found".into()));
            }
            Ok(())
        })
    }

    pub fn ban_member(
        conn: &mut PgConnection,
        actor_id: Uuid,
        server_id: Uuid,
        target_user_id: Uuid,
        duration_hours: Option<i64>,
        reason: Option<&str>,
    ) -> Result<crate::models::ServerBan, AppError> {
        if actor_id == target_user_id {
            return Err(AppError::BadRequest("cannot ban yourself".into()));
        }

        if let Some(hours) = duration_hours {
            if hours <= 0 {
                return Err(AppError::BadRequest("duration_hours must be > 0".into()));
            }
        }

        conn.transaction(|conn| {
            let actor_role = Self::get_user_role(conn, server_id, actor_id)?;
            if !matches!(actor_role, Role::Owner | Role::Admin) {
                return Err(AppError::Forbidden(
                    "only admin/owner can ban members".into(),
                ));
            }

            if repositories::server_members::is_member(conn, server_id, target_user_id)? {
                let target_role = Self::get_user_role(conn, server_id, target_user_id)?;
                if target_role == Role::Owner {
                    return Err(AppError::Forbidden("owner cannot be banned".into()));
                }
                if actor_role == Role::Admin && target_role == Role::Admin {
                    return Err(AppError::Forbidden("admin cannot ban another admin".into()));
                }
                let _ =
                    repositories::server_members::remove_member(conn, server_id, target_user_id)?;
            }

            let expires_at = duration_hours.map(|h| Utc::now() + chrono::Duration::hours(h));
            let ban = repositories::server_bans::upsert_ban(
                conn,
                server_id,
                target_user_id,
                actor_id,
                reason,
                expires_at,
            )?;
            Ok(ban)
        })
    }

    pub fn unban_member(
        conn: &mut PgConnection,
        actor_id: Uuid,
        server_id: Uuid,
        target_user_id: Uuid,
    ) -> Result<(), AppError> {
        conn.transaction(|conn| {
            let actor_role = Self::get_user_role(conn, server_id, actor_id)?;
            if !matches!(actor_role, Role::Owner | Role::Admin) {
                return Err(AppError::Forbidden(
                    "only admin/owner can unban members".into(),
                ));
            }
            let deleted = repositories::server_bans::unban(conn, server_id, target_user_id)?;
            if deleted == 0 {
                return Err(AppError::NotFound("ban not found".into()));
            }
            Ok(())
        })
    }

    pub fn list_bans(
        conn: &mut PgConnection,
        actor_id: Uuid,
        server_id: Uuid,
    ) -> Result<Vec<crate::models::ServerBan>, AppError> {
        let actor_role = Self::get_user_role(conn, server_id, actor_id)?;
        if !matches!(actor_role, Role::Owner | Role::Admin) {
            return Err(AppError::Forbidden("only admin/owner can view bans".into()));
        }
        Ok(repositories::server_bans::list_bans(conn, server_id)?)
    }

    pub fn transfer_ownership(
        conn: &mut PgConnection,
        actor_id: Uuid,
        server_id: Uuid,
        new_owner_id: Uuid,
    ) -> Result<(), AppError> {
        if actor_id == new_owner_id {
            return Err(AppError::BadRequest("new_owner must be different".into()));
        }

        conn.transaction(|conn| {
            let actor_role = Self::get_user_role(conn, server_id, actor_id)?;
            if actor_role != Role::Owner {
                return Err(AppError::Forbidden(
                    "only owner can transfer ownership".into(),
                ));
            }

            let _new_owner_role = Self::get_user_role(conn, server_id, new_owner_id)?;

            repositories::server_members::update_role(
                conn,
                server_id,
                actor_id,
                MemberRole::Admin,
            )?;
            repositories::server_members::update_role(
                conn,
                server_id,
                new_owner_id,
                MemberRole::Owner,
            )?;

            Ok(())
        })
    }

    fn get_user_role(
        conn: &mut PgConnection,
        server_id: Uuid,
        user_id: Uuid,
    ) -> Result<Role, AppError> {
        let db_role = repositories::server_members::get_role(conn, server_id, user_id)
            .map_err(map_diesel_notfound_to_notfound)?;

        Ok(member_role_to_role(db_role))
    }
}

fn validate_server_name(name: &str) -> Result<(), AppError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("server name is empty".into()));
    }
    if name.len() > 64 {
        return Err(AppError::BadRequest(
            "server name is too long (max 64)".into(),
        ));
    }
    Ok(())
}

fn validate_channel_name(name: &str) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::BadRequest("channel name is empty".into()));
    }
    if name.len() > 64 {
        return Err(AppError::BadRequest(
            "channel name is too long (max 64)".into(),
        ));
    }
    Ok(())
}

fn generate_invite_code() -> String {
    let raw = Uuid::new_v4().to_string();
    raw[..8].to_string()
}

fn member_role_to_role(r: MemberRole) -> Role {
    match r {
        MemberRole::Owner => Role::Owner,
        MemberRole::Admin => Role::Admin,
        MemberRole::Member => Role::Member,
    }
}

fn role_to_member_role(role: Role) -> MemberRole {
    match role {
        Role::Owner => MemberRole::Owner,
        Role::Admin => MemberRole::Admin,
        Role::Member => MemberRole::Member,
    }
}

use diesel::result::Error as DieselError;

fn map_diesel_notfound_to_notfound(err: DieselError) -> AppError {
    match err {
        DieselError::NotFound => AppError::NotFound("record not found".into()),
        other => AppError::from(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_server_name_rejects_empty_and_too_long() {
        assert!(validate_server_name("").is_err());
        assert!(validate_server_name("   ").is_err());
        assert!(validate_server_name(&"a".repeat(65)).is_err());
    }

    #[test]
    fn validate_server_name_accepts_valid_name() {
        assert!(validate_server_name("General").is_ok());
        assert!(validate_server_name("  Team Chat  ").is_ok());
    }

    #[test]
    fn generated_invite_code_is_8_ascii_hex_chars() {
        let code = generate_invite_code();
        assert_eq!(code.len(), 8);
        assert!(code.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn role_mappings_are_consistent() {
        assert_eq!(member_role_to_role(MemberRole::Owner), Role::Owner);
        assert_eq!(member_role_to_role(MemberRole::Admin), Role::Admin);
        assert_eq!(member_role_to_role(MemberRole::Member), Role::Member);
        assert_eq!(role_to_member_role(Role::Owner), MemberRole::Owner);
        assert_eq!(role_to_member_role(Role::Admin), MemberRole::Admin);
        assert_eq!(role_to_member_role(Role::Member), MemberRole::Member);
    }
}
