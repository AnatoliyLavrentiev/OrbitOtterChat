use chrono::{DateTime, Utc};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use uuid::Uuid;

use crate::models::{NewServerBan, ServerBan};
use crate::schema::server_bans;

pub fn upsert_ban(
    conn: &mut PgConnection,
    server_id: Uuid,
    user_id: Uuid,
    banned_by: Uuid,
    reason: Option<&str>,
    expires_at: Option<DateTime<Utc>>,
) -> QueryResult<ServerBan> {
    use crate::schema::server_bans::dsl;

    let row = NewServerBan {
        server_id,
        user_id,
        banned_by,
        reason,
        expires_at,
    };

    diesel::insert_into(server_bans::table)
        .values(&row)
        .on_conflict((dsl::server_id, dsl::user_id))
        .do_update()
        .set((
            dsl::banned_by.eq(banned_by),
            dsl::reason.eq(reason),
            dsl::expires_at.eq(expires_at),
            dsl::created_at.eq(Utc::now()),
        ))
        .returning(ServerBan::as_returning())
        .get_result(conn)
}

pub fn unban(conn: &mut PgConnection, server_id: Uuid, user_id: Uuid) -> QueryResult<usize> {
    use crate::schema::server_bans::dsl;

    diesel::delete(
        dsl::server_bans
            .filter(dsl::server_id.eq(server_id))
            .filter(dsl::user_id.eq(user_id)),
    )
    .execute(conn)
}

pub fn is_active_ban(conn: &mut PgConnection, server_id: Uuid, user_id: Uuid) -> QueryResult<bool> {
    use crate::schema::server_bans::dsl;

    let now = Utc::now();

    diesel::select(diesel::dsl::exists(
        dsl::server_bans
            .filter(dsl::server_id.eq(server_id))
            .filter(dsl::user_id.eq(user_id))
            .filter(dsl::expires_at.is_null().or(dsl::expires_at.gt(now))),
    ))
    .get_result(conn)
}

pub fn list_bans(conn: &mut PgConnection, server_id: Uuid) -> QueryResult<Vec<ServerBan>> {
    use crate::schema::server_bans::dsl;

    dsl::server_bans
        .filter(dsl::server_id.eq(server_id))
        .select(ServerBan::as_select())
        .load(conn)
}
