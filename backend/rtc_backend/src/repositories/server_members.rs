use diesel::pg::PgConnection;
use diesel::prelude::*;
use uuid::Uuid;

use crate::models::{MemberRole, NewServerMember, ServerMember};
use crate::schema::server_members;

pub fn add_member(
    conn: &mut PgConnection,
    server_id: Uuid,
    user_id: Uuid,
    role: MemberRole,
) -> QueryResult<usize> {
    let new_member = NewServerMember {
        server_id,
        user_id,
        role,
    };

    diesel::insert_into(server_members::table)
        .values(&new_member)
        .execute(conn)
}

pub fn remove_member(
    conn: &mut PgConnection,
    server_id: Uuid,
    user_id: Uuid,
) -> QueryResult<usize> {
    use crate::schema::server_members::dsl;

    diesel::delete(
        dsl::server_members
            .filter(dsl::server_id.eq(server_id))
            .filter(dsl::user_id.eq(user_id)),
    )
    .execute(conn)
}

pub fn get_role(
    conn: &mut PgConnection,
    server_id: Uuid,
    user_id: Uuid,
) -> QueryResult<MemberRole> {
    use crate::schema::server_members::dsl;

    dsl::server_members
        .filter(dsl::server_id.eq(server_id))
        .filter(dsl::user_id.eq(user_id))
        .select(dsl::role)
        .first(conn)
}

pub fn is_member(conn: &mut PgConnection, server_id: Uuid, user_id: Uuid) -> QueryResult<bool> {
    use crate::schema::server_members::dsl;

    diesel::select(diesel::dsl::exists(
        dsl::server_members
            .filter(dsl::server_id.eq(server_id))
            .filter(dsl::user_id.eq(user_id)),
    ))
    .get_result(conn)
}

pub fn update_role(
    conn: &mut PgConnection,
    server_id: Uuid,
    user_id: Uuid,
    new_role: MemberRole,
) -> QueryResult<usize> {
    use crate::schema::server_members::dsl;

    diesel::update(
        dsl::server_members
            .filter(dsl::server_id.eq(server_id))
            .filter(dsl::user_id.eq(user_id)),
    )
    .set(dsl::role.eq(new_role))
    .execute(conn)
}

pub fn list_members(conn: &mut PgConnection, server_id: Uuid) -> QueryResult<Vec<ServerMember>> {
    use crate::schema::server_members::dsl;

    dsl::server_members
        .filter(dsl::server_id.eq(server_id))
        .select(ServerMember::as_select())
        .load(conn)
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
    fn add_member_and_get_role_and_is_member_and_remove_member() {
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
        let server =
            crate::repositories::servers::create_server(&mut conn, owner.id, "test server", None)
                .expect("create_server");

        let user = crate::repositories::users::create_user(
            &mut conn,
            &format!("{}@example.com", Uuid::new_v4().simple()),
            &format!("member-{}", Uuid::new_v4().simple()),
            "password-hash",
        )
        .expect("create member user");
        let user_id = user.id;

        let inserted =
            add_member(&mut conn, server.id, user_id, MemberRole::Member).expect("add_member");
        assert_eq!(inserted, 1);

        let is_mem = is_member(&mut conn, server.id, user_id).expect("is_member");
        assert!(is_mem);

        let role = get_role(&mut conn, server.id, user_id).expect("get_role");
        assert_eq!(role, MemberRole::Member);

        let updated =
            update_role(&mut conn, server.id, user_id, MemberRole::Admin).expect("update_role");
        assert_eq!(updated, 1);

        let role2 = get_role(&mut conn, server.id, user_id).expect("get_role after update");
        assert_eq!(role2, MemberRole::Admin);

        let deleted = remove_member(&mut conn, server.id, user_id).expect("remove_member");
        assert_eq!(deleted, 1);

        let is_mem2 = is_member(&mut conn, server.id, user_id).expect("is_member after delete");
        assert!(!is_mem2);
    }
}
