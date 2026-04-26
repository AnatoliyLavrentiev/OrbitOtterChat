use diesel::pg::PgConnection;
use diesel::prelude::*;
use uuid::Uuid;

use crate::models::{NewServer, Server, ServerChangeset};
use crate::schema::servers;

pub fn create_server(
    conn: &mut PgConnection,
    created_by: Uuid,
    name: &str,
    description: Option<&str>,
) -> QueryResult<Server> {
    let new_server = NewServer {
        name,
        description,
        created_by,
    };

    diesel::insert_into(servers::table)
        .values(&new_server)
        .returning(Server::as_returning())
        .get_result(conn)
}

pub fn find_by_id(conn: &mut PgConnection, server_id: Uuid) -> QueryResult<Server> {
    use crate::schema::servers::dsl;

    dsl::servers
        .filter(dsl::id.eq(server_id))
        .select(Server::as_select())
        .first(conn)
}

pub fn update_description(
    conn: &mut PgConnection,
    server_id: Uuid,
    description: Option<&str>,
) -> QueryResult<Server> {
    use crate::schema::servers::dsl;

    diesel::update(dsl::servers.filter(dsl::id.eq(server_id)))
        .set(dsl::description.eq(description))
        .returning(Server::as_returning())
        .get_result(conn)
}

pub fn list_for_user(conn: &mut PgConnection, user_id: Uuid) -> QueryResult<Vec<Server>> {
    use crate::schema::{server_members, servers};

    servers::table
        .inner_join(server_members::table.on(server_members::server_id.eq(servers::id)))
        .filter(server_members::user_id.eq(user_id))
        .select(Server::as_select())
        .load(conn)
}

pub fn delete_server(conn: &mut PgConnection, server_id: Uuid) -> QueryResult<usize> {
    use crate::schema::servers::dsl;
    diesel::delete(dsl::servers.filter(dsl::id.eq(server_id))).execute(conn)
}

pub fn update_server(
    conn: &mut PgConnection,
    server_id: Uuid,
    name: Option<&str>,
    description: Option<Option<&str>>,
) -> QueryResult<Server> {
    use crate::schema::servers::dsl;

    let changes = ServerChangeset { name, description };

    diesel::update(dsl::servers.filter(dsl::id.eq(server_id)))
        .set(changes)
        .returning(Server::as_returning())
        .get_result(conn)
}
