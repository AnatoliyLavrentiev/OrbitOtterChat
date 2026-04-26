use diesel::pg::PgConnection;
use diesel::prelude::*;

use chrono::{Duration, Utc};
use ipnetwork::IpNetwork;
use uuid::Uuid;

use crate::models::{NewRefreshToken, RefreshToken};
use crate::schema::refresh_tokens;

pub fn create_refresh_token(
    conn: &mut PgConnection,
    user_id: Uuid,
    token_hash: &str,
    days_valid: i64,
    user_agent: Option<&str>,
    ip: Option<std::net::IpAddr>,
) -> QueryResult<RefreshToken> {
    let expires_at = Utc::now() + Duration::days(days_valid);

    let ip: Option<IpNetwork> = ip.map(IpNetwork::from);

    let new_token = NewRefreshToken {
        user_id,
        token_hash,
        expires_at,
        user_agent,
        ip,
    };

    diesel::insert_into(refresh_tokens::table)
        .values(&new_token)
        .returning(RefreshToken::as_returning())
        .get_result(conn)
}

pub fn revoke_refresh_token_by_hash(
    conn: &mut PgConnection,
    token_hash: &str,
) -> QueryResult<RefreshToken> {
    use crate::schema::refresh_tokens::dsl;

    diesel::update(dsl::refresh_tokens.filter(dsl::token_hash.eq(token_hash)))
        .set(dsl::revoked_at.eq(Some(Utc::now())))
        .returning(RefreshToken::as_returning())
        .get_result(conn)
}

pub fn find_active_by_hash(conn: &mut PgConnection, token_hash: &str) -> QueryResult<RefreshToken> {
    use crate::schema::refresh_tokens::dsl;

    let now = Utc::now();

    dsl::refresh_tokens
        .filter(dsl::token_hash.eq(token_hash))
        .filter(dsl::revoked_at.is_null())
        .filter(dsl::expires_at.gt(now))
        .select(RefreshToken::as_select())
        .first(conn)
}

pub fn rotate_refresh_token(
    conn: &mut PgConnection,
    refresh_token_raw: &str,
    days_valid: i64,
    user_agent: Option<&str>,
    ip: Option<std::net::IpAddr>,
) -> QueryResult<(RefreshToken, String)> {
    use crate::schema::refresh_tokens::dsl;

    let now = Utc::now();

    let current_hash = crate::security::tokens::hash_refresh_token(refresh_token_raw);

    let current: RefreshToken = dsl::refresh_tokens
        .filter(dsl::token_hash.eq(&current_hash))
        .select(RefreshToken::as_select())
        .first(conn)?;

    if current.revoked_at.is_some() {
        return Err(diesel::result::Error::NotFound);
    }

    if current.expires_at <= now {
        return Err(diesel::result::Error::NotFound);
    }

    let new_raw = crate::security::tokens::generate_refresh_token();
    let new_hash = crate::security::tokens::hash_refresh_token(&new_raw);

    let expires_at = now + Duration::days(days_valid);
    let ip: Option<IpNetwork> = ip.map(IpNetwork::from);

    let new_row = diesel::insert_into(refresh_tokens::table)
        .values(&NewRefreshToken {
            user_id: current.user_id,
            token_hash: &new_hash,
            expires_at,
            user_agent,
            ip,
        })
        .returning(RefreshToken::as_returning())
        .get_result(conn)?;

    diesel::update(dsl::refresh_tokens.filter(dsl::id.eq(current.id)))
        .set((
            dsl::revoked_at.eq(Some(now)),
            dsl::replaced_by.eq(Some(new_row.id)),
        ))
        .execute(conn)?;

    Ok((new_row, new_raw))
}
