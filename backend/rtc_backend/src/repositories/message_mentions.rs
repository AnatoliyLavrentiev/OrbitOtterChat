use diesel::pg::PgConnection;
use diesel::prelude::*;
use uuid::Uuid;

use crate::models::{MessageMention, NewMessageMention};
use crate::schema::message_mentions;

pub fn replace_mentions_for_message(
    conn: &mut PgConnection,
    message_id: Uuid,
    mentioned_user_ids: &[Uuid],
) -> QueryResult<Vec<MessageMention>> {
    use crate::schema::message_mentions::dsl;

    diesel::delete(dsl::message_mentions.filter(dsl::message_id.eq(message_id))).execute(conn)?;

    if mentioned_user_ids.is_empty() {
        return Ok(Vec::new());
    }

    let rows: Vec<NewMessageMention> = mentioned_user_ids
        .iter()
        .map(|user_id| NewMessageMention {
            message_id,
            user_id: *user_id,
        })
        .collect();

    diesel::insert_into(message_mentions::table)
        .values(&rows)
        .returning(MessageMention::as_returning())
        .get_results(conn)
}
