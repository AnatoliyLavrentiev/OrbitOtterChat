use chrono::Utc;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use uuid::Uuid;

use crate::models::{Message, NewMessage};
use crate::schema::messages;

pub fn create_message(
    conn: &mut PgConnection,
    channel_id: Uuid,
    author_id: Uuid,
    content: &str,
) -> QueryResult<Message> {
    let new_msg = NewMessage {
        channel_id,
        author_id,
        content,
    };

    diesel::insert_into(messages::table)
        .values(&new_msg)
        .returning(Message::as_returning())
        .get_result(conn)
}

pub fn find_by_id(conn: &mut PgConnection, message_id: Uuid) -> QueryResult<Message> {
    use crate::schema::messages::dsl;

    dsl::messages
        .filter(dsl::id.eq(message_id))
        .select(Message::as_select())
        .first(conn)
}

pub fn list_messages_by_channel(
    conn: &mut PgConnection,
    channel_id: Uuid,
    limit: i64,
    offset: i64,
) -> QueryResult<Vec<Message>> {
    use crate::schema::messages::dsl;

    dsl::messages
        .filter(dsl::channel_id.eq(channel_id))
        .filter(dsl::deleted_at.is_null())
        .order(dsl::created_at.asc())
        .limit(limit)
        .offset(offset)
        .select(Message::as_select())
        .load(conn)
}

pub fn update_message_content(
    conn: &mut PgConnection,
    message_id: Uuid,
    new_content: &str,
) -> QueryResult<Message> {
    use crate::schema::messages::dsl;

    diesel::update(
        dsl::messages
            .filter(dsl::id.eq(message_id))
            .filter(dsl::deleted_at.is_null()),
    )
    .set((
        dsl::content.eq(new_content),
        dsl::edited_at.eq(Some(Utc::now())),
    ))
    .returning(Message::as_returning())
    .get_result(conn)
}

pub fn soft_delete_message(
    conn: &mut PgConnection,
    message_id: Uuid,
    deleted_by: Uuid,
) -> QueryResult<usize> {
    use crate::schema::messages::dsl;

    diesel::update(dsl::messages.filter(dsl::id.eq(message_id)))
        .set((
            dsl::deleted_at.eq(Some(Utc::now())),
            dsl::deleted_by.eq(Some(deleted_by)),
        ))
        .execute(conn)
}

pub fn hard_delete_messages_by_channel(
    conn: &mut PgConnection,
    channel_id: Uuid,
) -> QueryResult<usize> {
    use crate::schema::messages::dsl;

    diesel::delete(dsl::messages.filter(dsl::channel_id.eq(channel_id))).execute(conn)
}

pub fn pin_message(
    conn: &mut PgConnection,
    message_id: Uuid,
    pinned_by: Uuid,
) -> QueryResult<Message> {
    use crate::schema::messages::dsl;

    diesel::update(
        dsl::messages
            .filter(dsl::id.eq(message_id))
            .filter(dsl::deleted_at.is_null()),
    )
    .set((
        dsl::pinned_at.eq(Some(Utc::now())),
        dsl::pinned_by.eq(Some(pinned_by)),
    ))
    .returning(Message::as_returning())
    .get_result(conn)
}

pub fn unpin_message(conn: &mut PgConnection, message_id: Uuid) -> QueryResult<Message> {
    use crate::schema::messages::dsl;

    diesel::update(
        dsl::messages
            .filter(dsl::id.eq(message_id))
            .filter(dsl::deleted_at.is_null()),
    )
    .set((
        dsl::pinned_at.eq::<Option<chrono::DateTime<Utc>>>(None),
        dsl::pinned_by.eq::<Option<Uuid>>(None),
    ))
    .returning(Message::as_returning())
    .get_result(conn)
}
