use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::models::{NewUser, User};
use crate::schema::users;

pub fn create_user(
    conn: &mut PgConnection,
    email: &str,
    username: &str,
    password_hash: &str,
) -> QueryResult<User> {
    let new_user = NewUser {
        email,
        username,
        password_hash,
    };

    diesel::insert_into(users::table)
        .values(&new_user)
        .returning(User::as_returning())
        .get_result(conn)
}

pub fn find_user_by_email(conn: &mut PgConnection, target_email: &str) -> QueryResult<User> {
    users::table
        .filter(users::email.eq(target_email))
        .select(User::as_select())
        .first(conn)
}

pub fn find_user_by_id(conn: &mut PgConnection, user_id: uuid::Uuid) -> QueryResult<User> {
    users::table
        .filter(users::id.eq(user_id))
        .select(User::as_select())
        .first(conn)
}

pub fn find_users_by_ids(
    conn: &mut PgConnection,
    user_ids: &[uuid::Uuid],
) -> QueryResult<Vec<User>> {
    users::table
        .filter(users::id.eq_any(user_ids))
        .select(User::as_select())
        .load(conn)
}

pub fn update_user_profile(
    conn: &mut PgConnection,
    user_id: uuid::Uuid,
    email: Option<&str>,
    username: Option<&str>,
    nickname: Option<Option<&str>>,
    avatar_url: Option<Option<&str>>,
    display_name_mode: Option<&str>,
) -> QueryResult<User> {
    use crate::schema::users::dsl;
    use chrono::Utc;

    #[derive(AsChangeset)]
    #[diesel(table_name = crate::schema::users)]
    struct UserProfileChangeset<'a> {
        email: Option<&'a str>,
        username: Option<&'a str>,
        nickname: Option<Option<&'a str>>,
        avatar_url: Option<Option<&'a str>>,
        display_name_mode: Option<&'a str>,
        updated_at: chrono::DateTime<Utc>,
    }

    let changes = UserProfileChangeset {
        email,
        username,
        nickname,
        avatar_url,
        display_name_mode,
        updated_at: Utc::now(),
    };

    diesel::update(dsl::users.filter(dsl::id.eq(user_id)))
        .set(changes)
        .returning(User::as_returning())
        .get_result(conn)
}
