use diesel::pg::PgConnection;
use diesel::prelude::*;
use uuid::Uuid;

use crate::models::{InviteUse, NewInviteUse};
use crate::schema::invite_uses;

pub fn create_invite_use(
    conn: &mut PgConnection,
    invite_id: Uuid,
    user_id: Uuid,
) -> QueryResult<InviteUse> {
    let new_use = NewInviteUse { invite_id, user_id };

    diesel::insert_into(invite_uses::table)
        .values(&new_use)
        .returning(InviteUse::as_returning())
        .get_result(conn)
}

pub fn count_uses_by_invite(conn: &mut PgConnection, invite_id: Uuid) -> QueryResult<i64> {
    use crate::schema::invite_uses::dsl;

    dsl::invite_uses
        .filter(dsl::invite_id.eq(invite_id))
        .count()
        .get_result(conn)
}

pub fn has_used_invite(
    conn: &mut PgConnection,
    invite_id: Uuid,
    user_id: Uuid,
) -> QueryResult<bool> {
    use crate::schema::invite_uses::dsl;

    diesel::select(diesel::dsl::exists(
        dsl::invite_uses
            .filter(dsl::invite_id.eq(invite_id))
            .filter(dsl::user_id.eq(user_id)),
    ))
    .get_result(conn)
}
