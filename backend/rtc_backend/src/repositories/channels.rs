use chrono::Utc;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{Channel, NewChannel};
use crate::schema::channels;

pub fn create_channel(
    conn: &mut PgConnection,
    server_id: Uuid,
    name: &str,
    topic: Option<&str>,
    position: i32,
    created_by: Uuid,
) -> QueryResult<Channel> {
    let new_channel = NewChannel {
        server_id,
        name,
        topic,
        position,
        created_by,
    };

    diesel::insert_into(channels::table)
        .values(&new_channel)
        .returning(Channel::as_returning())
        .get_result(conn)
}

pub fn list_channels_by_server(
    conn: &mut PgConnection,
    server_id: Uuid,
) -> QueryResult<Vec<Channel>> {
    use crate::schema::channels::dsl;

    dsl::channels
        .filter(dsl::server_id.eq(server_id))
        .order(dsl::position.asc())
        .select(Channel::as_select())
        .load(conn)
}

pub fn delete_channel(conn: &mut PgConnection, channel_id: Uuid) -> QueryResult<usize> {
    use crate::schema::channels::dsl;

    diesel::delete(dsl::channels.filter(dsl::id.eq(channel_id))).execute(conn)
}

#[derive(Debug, Default, AsChangeset)]
#[diesel(table_name = channels)]
struct ChannelChangeset<'a> {
    name: Option<&'a str>,
    topic: Option<Option<&'a str>>,
    position: Option<i32>,
    updated_at: Option<chrono::DateTime<Utc>>,
}

pub fn update_channel(
    conn: &mut PgConnection,
    channel_id: Uuid,
    name: Option<&str>,
    topic: Option<Option<&str>>,
    position: Option<i32>,
) -> QueryResult<Channel> {
    use crate::schema::channels::dsl;

    let changes = ChannelChangeset {
        name,
        topic,
        position,
        updated_at: Some(Utc::now()),
    };

    diesel::update(dsl::channels.filter(dsl::id.eq(channel_id)))
        .set(changes)
        .returning(Channel::as_returning())
        .get_result(conn)
}

pub fn find_by_id(conn: &mut PgConnection, channel_id: Uuid) -> QueryResult<Channel> {
    use crate::schema::channels::dsl;

    dsl::channels
        .filter(dsl::id.eq(channel_id))
        .select(Channel::as_select())
        .first(conn)
}

pub fn get_or_create_dm_channel(
    conn: &mut PgConnection,
    user1: Uuid,
    user2: Uuid,
) -> QueryResult<Channel> {
    use crate::schema::channel_members::dsl as cm;
    use crate::schema::channels::dsl as ch;

    let user1_channels: Vec<Uuid> = cm::channel_members
        .filter(cm::user_id.eq(user1))
        .select(cm::channel_id)
        .load(conn)?;

    let user2_channels: Vec<Uuid> = cm::channel_members
        .filter(cm::user_id.eq(user2))
        .select(cm::channel_id)
        .load(conn)?;

    let shared: Vec<Uuid> = user1_channels
        .iter()
        .filter(|id| user2_channels.contains(id))
        .copied()
        .collect();

    for channel_id in &shared {
        let existing = ch::channels
            .filter(ch::id.eq(channel_id))
            .filter(ch::server_id.is_null())
            .select(Channel::as_select())
            .first(conn)
            .optional()?;

        if let Some(channel) = existing {
            return Ok(channel);
        }
    }

    let channel: Channel = diesel::insert_into(ch::channels)
        .values((
            ch::name.eq(format!("dm-{}-{}", user1, user2)),
            ch::position.eq(0),
            ch::created_by.eq(user1),
        ))
        .returning(Channel::as_returning())
        .get_result(conn)?;

    diesel::insert_into(cm::channel_members)
        .values(vec![
            (cm::channel_id.eq(channel.id), cm::user_id.eq(user1)),
            (cm::channel_id.eq(channel.id), cm::user_id.eq(user2)),
        ])
        .execute(conn)?;

    Ok(channel)
}

pub fn list_dm_channels_for_user(
    conn: &mut PgConnection,
    user_id: Uuid,
) -> QueryResult<Vec<Channel>> {
    use crate::schema::channel_members::dsl as cm;
    use crate::schema::channels::dsl as ch;

    cm::channel_members
        .inner_join(ch::channels.on(cm::channel_id.eq(ch::id)))
        .filter(cm::user_id.eq(user_id))
        .filter(ch::server_id.is_null())
        .select(Channel::as_select())
        .order(ch::updated_at.desc())
        .load(conn)
}

pub fn list_dm_member_ids(conn: &mut PgConnection, channel_id: Uuid) -> QueryResult<Vec<Uuid>> {
    use crate::schema::channel_members::dsl as cm;

    cm::channel_members
        .filter(cm::channel_id.eq(channel_id))
        .select(cm::user_id)
        .load(conn)
}

pub fn ensure_dm_member(
    conn: &mut PgConnection,
    channel_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    use crate::schema::channel_members::dsl as cm;

    let exists = cm::channel_members
        .filter(cm::channel_id.eq(channel_id))
        .filter(cm::user_id.eq(user_id))
        .select((cm::channel_id, cm::user_id))
        .first::<(Uuid, Uuid)>(conn)
        .optional()
        .map_err(AppError::Db)?;

    if exists.is_some() {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "You are not a member of this DM".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::Connection;
    use std::env;
    use uuid::Uuid;

    fn db_conn() -> Option<PgConnection> {
        dotenvy::dotenv().ok();
        let url = env::var("TEST_DATABASE_URL")
            .ok()
            .or_else(|| env::var("DATABASE_URL").ok())?;
        PgConnection::establish(&url).ok()
    }

    #[test]
    fn channel_crud() {
        let Some(mut conn) = db_conn() else {
            return;
        };
        conn.begin_test_transaction().unwrap();

        let owner = crate::repositories::users::create_user(
            &mut conn,
            &format!("{}@example.com", Uuid::new_v4().simple()),
            &format!("owner-{}", Uuid::new_v4().simple()),
            "password-hash",
        )
        .expect("create owner user");
        let server = crate::repositories::servers::create_server(
            &mut conn,
            owner.id,
            "chan test server",
            None,
        )
        .expect("create_server");

        let ch = create_channel(&mut conn, server.id, "general", Some("topic"), 0, owner.id)
            .expect("create_channel");

        let found = find_by_id(&mut conn, ch.id).expect("find_by_id");
        assert_eq!(found.id, ch.id);
        assert_eq!(found.name, "general");

        let updated = update_channel(&mut conn, ch.id, Some("random"), Some(None), Some(2))
            .expect("update_channel");

        assert_eq!(updated.name, "random");
        assert_eq!(updated.topic, None);
        assert_eq!(updated.position, 2);

        let list = list_channels_by_server(&mut conn, server.id).expect("list_channels_by_server");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, ch.id);

        let deleted = delete_channel(&mut conn, ch.id).expect("delete_channel");
        assert_eq!(deleted, 1);

        let err = find_by_id(&mut conn, ch.id).unwrap_err();
        assert_eq!(err, diesel::result::Error::NotFound);
    }
}
