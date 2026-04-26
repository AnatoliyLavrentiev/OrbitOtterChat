use diesel::dsl::count_star;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use uuid::Uuid;

use crate::models::NewMessageReaction;
use crate::schema::message_reactions;

pub fn toggle_reaction(
    conn: &mut PgConnection,
    message_id: Uuid,
    user_id: Uuid,
    emoji: &str,
) -> QueryResult<bool> {
    use crate::schema::message_reactions::dsl;

    let deleted = diesel::delete(
        dsl::message_reactions
            .filter(dsl::message_id.eq(message_id))
            .filter(dsl::user_id.eq(user_id))
            .filter(dsl::emoji.eq(emoji)),
    )
    .execute(conn)?;

    if deleted > 0 {
        return Ok(false);
    }

    let new_reaction = NewMessageReaction {
        message_id,
        user_id,
        emoji,
    };

    diesel::insert_into(message_reactions::table)
        .values(&new_reaction)
        .execute(conn)?;

    Ok(true)
}

pub fn list_reaction_counts(
    conn: &mut PgConnection,
    message_id: Uuid,
) -> QueryResult<Vec<(String, i64)>> {
    use crate::schema::message_reactions::dsl;

    dsl::message_reactions
        .filter(dsl::message_id.eq(message_id))
        .group_by(dsl::emoji)
        .order(dsl::emoji.asc())
        .select((dsl::emoji, count_star()))
        .load(conn)
}

pub fn list_user_emojis(
    conn: &mut PgConnection,
    message_id: Uuid,
    user_id: Uuid,
) -> QueryResult<Vec<String>> {
    use crate::schema::message_reactions::dsl;

    dsl::message_reactions
        .filter(dsl::message_id.eq(message_id))
        .filter(dsl::user_id.eq(user_id))
        .select(dsl::emoji)
        .load(conn)
}
