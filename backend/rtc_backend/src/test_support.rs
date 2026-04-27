use diesel::pg::PgConnection;
use diesel::Connection;
use diesel_migrations::{FileBasedMigrations, MigrationHarness};

pub(crate) fn test_db_url() -> Option<String> {
    dotenvy::dotenv().ok();
    std::env::var("TEST_DATABASE_URL")
        .ok()
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub(crate) fn migrated_test_connection() -> Option<PgConnection> {
    let database_url = test_db_url()?;
    let mut conn = PgConnection::establish(&database_url).ok()?;
    run_migrations(&mut conn)?;
    Some(conn)
}

pub(crate) fn run_migrations<Conn>(conn: &mut Conn) -> Option<()>
where
    Conn: MigrationHarness<diesel::pg::Pg>,
{
    let migrations = FileBasedMigrations::from_path("./migrations").ok()?;
    conn.run_pending_migrations(migrations).ok()?;
    Some(())
}
