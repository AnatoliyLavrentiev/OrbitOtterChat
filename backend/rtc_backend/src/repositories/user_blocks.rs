use diesel::pg::PgConnection;
use diesel::prelude::*;
use uuid::Uuid;

use crate::models::{NewUserBlock, UserBlock};

pub fn block_user(
    conn: &mut PgConnection,
    blocker_id: Uuid,
    blocked_id: Uuid,
) -> QueryResult<UserBlock> {
    use crate::schema::user_blocks::dsl;

    let row = NewUserBlock {
        blocker_id,
        blocked_id,
    };

    diesel::insert_into(dsl::user_blocks)
        .values(&row)
        .on_conflict((dsl::blocker_id, dsl::blocked_id))
        .do_nothing()
        .execute(conn)?;

    dsl::user_blocks
        .filter(dsl::blocker_id.eq(blocker_id))
        .filter(dsl::blocked_id.eq(blocked_id))
        .select(UserBlock::as_select())
        .first(conn)
}

pub fn unblock_user(
    conn: &mut PgConnection,
    blocker_id: Uuid,
    blocked_id: Uuid,
) -> QueryResult<usize> {
    use crate::schema::user_blocks::dsl;

    diesel::delete(
        dsl::user_blocks
            .filter(dsl::blocker_id.eq(blocker_id))
            .filter(dsl::blocked_id.eq(blocked_id)),
    )
    .execute(conn)
}

pub fn list_blocked_by_user(
    conn: &mut PgConnection,
    blocker_id: Uuid,
) -> QueryResult<Vec<UserBlock>> {
    use crate::schema::user_blocks::dsl;

    dsl::user_blocks
        .filter(dsl::blocker_id.eq(blocker_id))
        .order(dsl::created_at.desc())
        .select(UserBlock::as_select())
        .load(conn)
}

pub fn is_blocked_any_direction(
    conn: &mut PgConnection,
    user_a: Uuid,
    user_b: Uuid,
) -> QueryResult<bool> {
    use crate::schema::user_blocks::dsl;

    diesel::select(diesel::dsl::exists(
        dsl::user_blocks.filter(
            dsl::blocker_id
                .eq(user_a)
                .and(dsl::blocked_id.eq(user_b))
                .or(dsl::blocker_id.eq(user_b).and(dsl::blocked_id.eq(user_a))),
        ),
    ))
    .get_result(conn)
}
