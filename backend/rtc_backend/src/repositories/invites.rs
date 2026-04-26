use diesel::pg::PgConnection;
use diesel::prelude::*;
use uuid::Uuid;

use chrono::{DateTime, Utc};

use crate::models::{Invite, NewInvite};
use crate::schema::invites;

pub fn create_invite(
    conn: &mut PgConnection,
    server_id: Uuid,
    code: &str,
    created_by: Uuid,
    expires_at: Option<DateTime<Utc>>,
    max_uses: Option<i32>,
) -> QueryResult<Invite> {
    let new_invite = NewInvite {
        server_id,
        code,
        created_by,
        expires_at,
        max_uses,
    };

    diesel::insert_into(invites::table)
        .values(&new_invite)
        .returning(Invite::as_returning())
        .get_result(conn)
}

pub fn find_by_code(conn: &mut PgConnection, code: &str) -> QueryResult<Invite> {
    use crate::schema::invites::dsl;

    dsl::invites
        .filter(dsl::code.eq(code))
        .select(Invite::as_select())
        .first(conn)
}

pub fn find_by_id(conn: &mut PgConnection, invite_id: Uuid) -> QueryResult<Invite> {
    use crate::schema::invites::dsl;

    dsl::invites
        .filter(dsl::id.eq(invite_id))
        .select(Invite::as_select())
        .first(conn)
}

pub fn increment_uses_count(conn: &mut PgConnection, invite_id: Uuid) -> QueryResult<Invite> {
    use crate::schema::invites::dsl;

    diesel::update(dsl::invites.filter(dsl::id.eq(invite_id)))
        .filter(
            dsl::max_uses
                .is_null()
                .or(dsl::uses_count.nullable().lt(dsl::max_uses)),
        )
        .set(dsl::uses_count.eq(dsl::uses_count + 1))
        .returning(Invite::as_returning())
        .get_result(conn)
}
