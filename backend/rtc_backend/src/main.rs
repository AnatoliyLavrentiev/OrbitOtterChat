mod db;
mod domain;
mod errors;
mod handlers;
mod models;
mod realtime;
mod repositories;
mod schema;
mod security;
mod services;

use crate::db::DbPool;
use crate::realtime::WsHub;
use crate::security::jwt::JwtConfig;

use axum::extract::DefaultBodyLimit;
use axum::routing::{any, delete, get, post};
use axum::{Extension, Router};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub jwt: JwtConfig,
    pub ws: WsHub,
}

async fn data() -> &'static str {
    "Hello, World!"
}

fn build_app(state: AppState, pool: DbPool) -> Router {
    Router::new()
        .route("/", get(data))
        .route(
            "/me",
            get(handlers::current_user::me).put(handlers::current_user::update_me),
        )
        .route("/dms", get(handlers::channels::list_my_dms))
        .route(
            "/dms/{target_user_id}",
            post(handlers::channels::create_or_get_dm),
        )
        .route(
            "/dms/channel/{channel_id}",
            delete(handlers::channels::delete_dm_history),
        )
        .nest("/blocks", handlers::blocks::routes())
        .route(
            "/me/avatar",
            axum::routing::post(handlers::current_user::upload_avatar),
        )
        .route("/server/{id}", get(handlers::servers::get_server))
        .nest("/auth", handlers::authentication::routes())
        .nest("/servers", handlers::servers::routes())
        .nest("/channels", handlers::channels::routes())
        .nest(
            "/channels/{channel_id}",
            handlers::messages::channel_scoped_routes(),
        )
        .nest("/messages", handlers::messages::routes())
        .route("/ws", any(handlers::web_socket::ws_handler))
        .nest_service("/uploads", ServeDir::new("./uploads"))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(DefaultBodyLimit::max(12 * 1024 * 1024))
        .layer(Extension(pool))
        .with_state(state)
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");
    let pool = db::create_pool(&database_url);

    let jwt = security::jwt::JwtConfig::new_from_env();
    let ws = WsHub::new(2048);

    let state = AppState {
        db: pool.clone(),
        jwt,
        ws,
    };

    let app = build_app(state, pool);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind");

    println!("Listening on: {:?}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .await
        .expect("Server failed to start");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::{Method, Request, StatusCode};
    use diesel::pg::PgConnection;
    use diesel::Connection;
    use diesel_migrations::{FileBasedMigrations, MigrationHarness};
    use serde_json::{json, Value};
    use tower::ServiceExt;
    use uuid::Uuid;

    fn test_db_url() -> Option<String> {
        dotenvy::dotenv().ok();
        std::env::var("TEST_DATABASE_URL")
            .ok()
            .or_else(|| std::env::var("DATABASE_URL").ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    fn setup_app() -> Option<Router> {
        let database_url = test_db_url()?;
        if PgConnection::establish(&database_url).is_err() {
            return None;
        }
        if std::env::var("JWT_SECRET").is_err() {
            std::env::set_var("JWT_SECRET", "test-jwt-secret");
        }

        let pool = db::create_pool_with_max_size(&database_url, 2);
        {
            let mut conn = pool.get().ok()?;
            let migrations = FileBasedMigrations::from_path("./migrations").ok()?;
            conn.run_pending_migrations(migrations).ok()?;
        }

        let state = AppState {
            db: pool.clone(),
            jwt: JwtConfig::new_from_env(),
            ws: WsHub::new(2048),
        };
        Some(build_app(state, pool))
    }

    async fn request_json(
        app: &Router,
        method: Method,
        path: &str,
        token: Option<&str>,
        payload: Option<Value>,
    ) -> (StatusCode, Value) {
        let body = payload
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_else(String::new);

        let mut builder = Request::builder().method(method).uri(path);
        if payload.is_some() {
            builder = builder.header("content-type", "application/json");
        }
        if let Some(t) = token {
            builder = builder.header("authorization", format!("Bearer {t}"));
        }

        let req = builder
            .body(Body::from(body))
            .expect("request should be created");
        let resp = app
            .clone()
            .oneshot(req)
            .await
            .expect("router should respond");
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("body should be read");
        let parsed = serde_json::from_slice::<Value>(&bytes).unwrap_or(Value::Null);
        (status, parsed)
    }

    async fn request_raw(
        app: &Router,
        method: Method,
        path: &str,
        token: Option<&str>,
        content_type: Option<&str>,
        body: impl Into<Body>,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(path);
        if let Some(content_type) = content_type {
            builder = builder.header("content-type", content_type);
        }
        if let Some(t) = token {
            builder = builder.header("authorization", format!("Bearer {t}"));
        }

        let req = builder
            .body(body.into())
            .expect("request should be created");
        let resp = app
            .clone()
            .oneshot(req)
            .await
            .expect("router should respond");
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("body should be read");
        let parsed = serde_json::from_slice::<Value>(&bytes).unwrap_or(Value::Null);
        (status, parsed)
    }

    fn unique(suffix: &str) -> String {
        format!("{}-{}", suffix, Uuid::new_v4().simple())
    }

    async fn signup_user(app: &Router, prefix: &str) -> (String, String, String, String) {
        let email = format!("{}@example.com", unique(prefix));
        let username = unique(&format!("{prefix}-user"));
        let (signup_status, signup_body) = request_json(
            app,
            Method::POST,
            "/auth/signup",
            None,
            Some(json!({
                "email": email,
                "username": username,
                "password": "Passw0rd!"
            })),
        )
        .await;
        assert_eq!(signup_status, StatusCode::OK);
        let access = signup_body
            .get("access_token")
            .and_then(Value::as_str)
            .expect("access token")
            .to_string();
        let (me_status, me_body) = request_json(app, Method::GET, "/me", Some(&access), None).await;
        assert_eq!(me_status, StatusCode::OK);
        let user_id = me_body
            .get("id")
            .and_then(Value::as_str)
            .expect("user id")
            .to_string();
        (access, user_id, email, username)
    }

    async fn create_server_for(app: &Router, access: &str, prefix: &str) -> (String, String) {
        let (create_status, create_body) = request_json(
            app,
            Method::POST,
            "/servers",
            Some(access),
            Some(json!({ "name": unique(prefix) })),
        )
        .await;
        assert_eq!(create_status, StatusCode::OK);
        let server_id = create_body
            .get("server_id")
            .and_then(Value::as_str)
            .expect("server_id")
            .to_string();
        let invite_code = create_body
            .get("invite_code")
            .and_then(Value::as_str)
            .expect("invite_code")
            .to_string();
        (server_id, invite_code)
    }

    #[tokio::test]
    async fn auth_signup_me_logout_refresh_flow() {
        let Some(app) = setup_app() else {
            return;
        };

        let email = format!("{}@example.com", unique("auth"));
        let username = unique("user");
        let password = "Passw0rd!";

        let (signup_status, signup_body) = request_json(
            &app,
            Method::POST,
            "/auth/signup",
            None,
            Some(json!({
                "email": email,
                "username": username,
                "password": password
            })),
        )
        .await;
        assert_eq!(signup_status, StatusCode::OK);

        let access = signup_body
            .get("access_token")
            .and_then(Value::as_str)
            .expect("access token must exist")
            .to_string();
        let refresh = signup_body
            .get("refresh_token")
            .and_then(Value::as_str)
            .expect("refresh token must exist")
            .to_string();

        let (me_status, me_body) =
            request_json(&app, Method::GET, "/me", Some(&access), None).await;
        assert_eq!(me_status, StatusCode::OK);
        assert!(me_body.get("id").and_then(Value::as_str).is_some());

        let (logout_status, _) = request_json(
            &app,
            Method::POST,
            "/auth/logout",
            None,
            Some(json!({ "refresh_token": refresh })),
        )
        .await;
        assert_eq!(logout_status, StatusCode::OK);

        let (refresh_status, _) = request_json(
            &app,
            Method::POST,
            "/auth/refresh",
            None,
            Some(json!({ "refresh_token": refresh })),
        )
        .await;
        assert_eq!(refresh_status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn owner_cannot_leave_server() {
        let Some(app) = setup_app() else {
            return;
        };

        let email = format!("{}@example.com", unique("owner"));
        let username = unique("owner-user");

        let (_, auth_body) = request_json(
            &app,
            Method::POST,
            "/auth/signup",
            None,
            Some(json!({
                "email": email,
                "username": username,
                "password": "Passw0rd!"
            })),
        )
        .await;
        let owner_access = auth_body
            .get("access_token")
            .and_then(Value::as_str)
            .expect("access token")
            .to_string();

        let (create_status, create_body) = request_json(
            &app,
            Method::POST,
            "/servers",
            Some(&owner_access),
            Some(json!({ "name": unique("server") })),
        )
        .await;
        assert_eq!(create_status, StatusCode::OK);
        let server_id = create_body
            .get("server_id")
            .and_then(Value::as_str)
            .expect("server_id exists");

        let (leave_status, _) = request_json(
            &app,
            Method::DELETE,
            &format!("/servers/{server_id}/leave"),
            Some(&owner_access),
            None,
        )
        .await;
        assert_eq!(leave_status, StatusCode::FORBIDDEN);

        let (channels_status, channels_body) = request_json(
            &app,
            Method::GET,
            &format!("/servers/{server_id}/channels"),
            Some(&owner_access),
            None,
        )
        .await;
        assert_eq!(channels_status, StatusCode::OK);
        let channels = channels_body.as_array().expect("channels list");
        assert!(!channels.is_empty());
        let has_general = channels.iter().any(|ch| {
            ch.get("name")
                .and_then(Value::as_str)
                .map(|n| n == "general")
                .unwrap_or(false)
        });
        assert!(has_general);
    }

    #[tokio::test]
    async fn roles_channels_and_message_permissions_flow() {
        let Some(app) = setup_app() else {
            return;
        };

        let owner_email = format!("{}@example.com", unique("flow-owner"));
        let owner_username = unique("flow-owner-user");
        let member_email = format!("{}@example.com", unique("flow-member"));
        let member_username = unique("flow-member-user");

        let (_, owner_auth) = request_json(
            &app,
            Method::POST,
            "/auth/signup",
            None,
            Some(json!({
                "email": owner_email,
                "username": owner_username,
                "password": "Passw0rd!"
            })),
        )
        .await;
        let owner_access = owner_auth
            .get("access_token")
            .and_then(Value::as_str)
            .expect("owner access")
            .to_string();

        let (create_status, create_body) = request_json(
            &app,
            Method::POST,
            "/servers",
            Some(&owner_access),
            Some(json!({ "name": unique("flow-server") })),
        )
        .await;
        assert_eq!(create_status, StatusCode::OK);
        let server_id = create_body
            .get("server_id")
            .and_then(Value::as_str)
            .expect("server_id");
        let invite_code = create_body
            .get("invite_code")
            .and_then(Value::as_str)
            .expect("invite_code");

        let (_, member_auth) = request_json(
            &app,
            Method::POST,
            "/auth/signup",
            None,
            Some(json!({
                "email": member_email,
                "username": member_username,
                "password": "Passw0rd!"
            })),
        )
        .await;
        let member_access = member_auth
            .get("access_token")
            .and_then(Value::as_str)
            .expect("member access")
            .to_string();

        let (join_status, _) = request_json(
            &app,
            Method::POST,
            "/servers/join",
            Some(&member_access),
            Some(json!({ "invite_code": invite_code })),
        )
        .await;
        assert_eq!(join_status, StatusCode::OK);

        let (member_me_status, member_me_body) =
            request_json(&app, Method::GET, "/me", Some(&member_access), None).await;
        assert_eq!(member_me_status, StatusCode::OK);
        let member_id = member_me_body
            .get("id")
            .and_then(Value::as_str)
            .expect("member id");

        let (promote_status, _) = request_json(
            &app,
            Method::PUT,
            &format!("/servers/{server_id}/members/{member_id}"),
            Some(&owner_access),
            Some(json!({ "role": "ADMIN" })),
        )
        .await;
        assert_eq!(promote_status, StatusCode::OK);

        let (create_owner_channel_status, create_owner_channel_body) = request_json(
            &app,
            Method::POST,
            &format!("/servers/{server_id}/channels"),
            Some(&owner_access),
            Some(json!({ "name": unique("owner-room") })),
        )
        .await;
        assert_eq!(create_owner_channel_status, StatusCode::OK);
        let owner_channel_id = create_owner_channel_body
            .get("channel")
            .and_then(|v| v.get("id"))
            .and_then(Value::as_str)
            .expect("owner channel id");

        let (create_admin_channel_status, _) = request_json(
            &app,
            Method::POST,
            &format!("/servers/{server_id}/channels"),
            Some(&member_access),
            Some(json!({ "name": unique("admin-room") })),
        )
        .await;
        assert_eq!(create_admin_channel_status, StatusCode::OK);

        let (member_message_status, member_message_body) = request_json(
            &app,
            Method::POST,
            &format!("/channels/{owner_channel_id}/messages"),
            Some(&member_access),
            Some(json!({ "content": "hello from member" })),
        )
        .await;
        assert_eq!(member_message_status, StatusCode::OK);
        let member_message_id = member_message_body
            .get("message")
            .and_then(|v| v.get("id"))
            .and_then(Value::as_str)
            .expect("member message id");

        let (owner_delete_member_msg_status, _) = request_json(
            &app,
            Method::DELETE,
            &format!("/messages/{member_message_id}"),
            Some(&owner_access),
            None,
        )
        .await;
        assert_eq!(owner_delete_member_msg_status, StatusCode::OK);

        let (owner_message_status, owner_message_body) = request_json(
            &app,
            Method::POST,
            &format!("/channels/{owner_channel_id}/messages"),
            Some(&owner_access),
            Some(json!({ "content": "hello from owner" })),
        )
        .await;
        assert_eq!(owner_message_status, StatusCode::OK);
        let owner_message_id = owner_message_body
            .get("message")
            .and_then(|v| v.get("id"))
            .and_then(Value::as_str)
            .expect("owner message id");

        let (demote_status, _) = request_json(
            &app,
            Method::PUT,
            &format!("/servers/{server_id}/members/{member_id}"),
            Some(&owner_access),
            Some(json!({ "role": "MEMBER" })),
        )
        .await;
        assert_eq!(demote_status, StatusCode::OK);

        let (member_delete_owner_msg_status, _) = request_json(
            &app,
            Method::DELETE,
            &format!("/messages/{owner_message_id}"),
            Some(&member_access),
            None,
        )
        .await;
        assert_eq!(member_delete_owner_msg_status, StatusCode::FORBIDDEN);

        let (member_create_channel_after_demote_status, _) = request_json(
            &app,
            Method::POST,
            &format!("/servers/{server_id}/channels"),
            Some(&member_access),
            Some(json!({ "name": unique("member-room") })),
        )
        .await;
        assert_eq!(
            member_create_channel_after_demote_status,
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn message_reaction_toggle_flow() {
        let Some(app) = setup_app() else {
            return;
        };

        let email = format!("{}@example.com", unique("react"));
        let username = unique("react-user");

        let (_, auth_body) = request_json(
            &app,
            Method::POST,
            "/auth/signup",
            None,
            Some(json!({
                "email": email,
                "username": username,
                "password": "Passw0rd!"
            })),
        )
        .await;
        let access = auth_body
            .get("access_token")
            .and_then(Value::as_str)
            .expect("access token")
            .to_string();
        let (create_status, create_body) = request_json(
            &app,
            Method::POST,
            "/servers",
            Some(&access),
            Some(json!({ "name": unique("react-server") })),
        )
        .await;
        assert_eq!(create_status, StatusCode::OK);
        let server_id = create_body
            .get("server_id")
            .and_then(Value::as_str)
            .expect("server_id exists");

        let (channels_status, channels_body) = request_json(
            &app,
            Method::GET,
            &format!("/servers/{server_id}/channels"),
            Some(&access),
            None,
        )
        .await;
        assert_eq!(channels_status, StatusCode::OK);
        let channel_id = channels_body
            .as_array()
            .and_then(|list| list.first())
            .and_then(|item| item.get("id"))
            .and_then(Value::as_str)
            .expect("channel id");

        let (message_status, message_body) = request_json(
            &app,
            Method::POST,
            &format!("/channels/{channel_id}/messages"),
            Some(&access),
            Some(json!({ "content": "react me" })),
        )
        .await;
        assert_eq!(message_status, StatusCode::OK);
        let message_id = message_body
            .get("message")
            .and_then(|v| v.get("id"))
            .and_then(Value::as_str)
            .expect("message id");

        let (toggle_on_status, toggle_on_body) = request_json(
            &app,
            Method::POST,
            &format!("/messages/{message_id}/reactions"),
            Some(&access),
            Some(json!({ "emoji": "👍" })),
        )
        .await;
        assert_eq!(toggle_on_status, StatusCode::OK);

        let reactions_on = toggle_on_body
            .get("reactions")
            .and_then(Value::as_array)
            .expect("reactions array");
        assert_eq!(reactions_on.len(), 1);
        assert_eq!(
            reactions_on[0].get("emoji").and_then(Value::as_str),
            Some("👍")
        );
        assert_eq!(
            reactions_on[0].get("count").and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            reactions_on[0].get("reacted").and_then(Value::as_bool),
            Some(true)
        );

        let (toggle_off_status, toggle_off_body) = request_json(
            &app,
            Method::POST,
            &format!("/messages/{message_id}/reactions"),
            Some(&access),
            Some(json!({ "emoji": "👍" })),
        )
        .await;
        assert_eq!(toggle_off_status, StatusCode::OK);
        let reactions_off = toggle_off_body
            .get("reactions")
            .and_then(Value::as_array)
            .expect("reactions array");
        assert!(reactions_off.is_empty());
    }

    #[tokio::test]
    async fn message_pin_flow_allows_author_and_persists_pin_state() {
        let Some(app) = setup_app() else {
            return;
        };

        let email = format!("{}@example.com", unique("pin"));
        let username = unique("pin-user");

        let (_, auth_body) = request_json(
            &app,
            Method::POST,
            "/auth/signup",
            None,
            Some(json!({
                "email": email,
                "username": username,
                "password": "Passw0rd!"
            })),
        )
        .await;
        let access = auth_body
            .get("access_token")
            .and_then(Value::as_str)
            .expect("access token")
            .to_string();
        let (me_status, me_body) =
            request_json(&app, Method::GET, "/me", Some(&access), None).await;
        assert_eq!(me_status, StatusCode::OK);
        let user_id = me_body
            .get("id")
            .and_then(Value::as_str)
            .expect("user id")
            .to_string();

        let (create_status, create_body) = request_json(
            &app,
            Method::POST,
            "/servers",
            Some(&access),
            Some(json!({ "name": unique("pin-server") })),
        )
        .await;
        assert_eq!(create_status, StatusCode::OK);
        let server_id = create_body
            .get("server_id")
            .and_then(Value::as_str)
            .expect("server_id exists");

        let (channels_status, channels_body) = request_json(
            &app,
            Method::GET,
            &format!("/servers/{server_id}/channels"),
            Some(&access),
            None,
        )
        .await;
        assert_eq!(channels_status, StatusCode::OK);
        let channel_id = channels_body
            .as_array()
            .and_then(|list| list.first())
            .and_then(|item| item.get("id"))
            .and_then(Value::as_str)
            .expect("channel id");

        let (message_status, message_body) = request_json(
            &app,
            Method::POST,
            &format!("/channels/{channel_id}/messages"),
            Some(&access),
            Some(json!({ "content": "pin me for later" })),
        )
        .await;
        assert_eq!(message_status, StatusCode::OK);
        let message_id = message_body
            .get("message")
            .and_then(|v| v.get("id"))
            .and_then(Value::as_str)
            .expect("message id");

        let (pin_status, pin_body) = request_json(
            &app,
            Method::PUT,
            &format!("/messages/{message_id}/pin"),
            Some(&access),
            None,
        )
        .await;
        assert_eq!(pin_status, StatusCode::OK);
        assert_eq!(
            pin_body
                .get("message")
                .and_then(|v| v.get("pinned_by"))
                .and_then(Value::as_str),
            Some(user_id.as_str())
        );
        assert!(pin_body
            .get("message")
            .and_then(|v| v.get("pinned_at"))
            .and_then(Value::as_str)
            .is_some());

        let (messages_status, messages_body) = request_json(
            &app,
            Method::GET,
            &format!("/channels/{channel_id}/messages"),
            Some(&access),
            None,
        )
        .await;
        assert_eq!(messages_status, StatusCode::OK);
        let pinned = messages_body
            .as_array()
            .and_then(|list| {
                list.iter()
                    .find(|item| item.get("id").and_then(Value::as_str) == Some(message_id))
            })
            .expect("pinned message in history");
        assert!(pinned.get("pinned_at").and_then(Value::as_str).is_some());

        let (unpin_status, unpin_body) = request_json(
            &app,
            Method::DELETE,
            &format!("/messages/{message_id}/pin"),
            Some(&access),
            None,
        )
        .await;
        assert_eq!(unpin_status, StatusCode::OK);
        assert!(unpin_body
            .get("message")
            .and_then(|v| v.get("pinned_at"))
            .is_some_and(Value::is_null));
        assert!(unpin_body
            .get("message")
            .and_then(|v| v.get("pinned_by"))
            .is_some_and(Value::is_null));
    }

    #[tokio::test]
    async fn profile_update_and_avatar_upload_validation_flow() {
        let Some(app) = setup_app() else {
            return;
        };
        let (access, _user_id, original_email, _username) = signup_user(&app, "profile").await;
        let (_, _other_id, other_email, _other_username) = signup_user(&app, "profile-other").await;

        let (empty_status, _) =
            request_json(&app, Method::PUT, "/me", Some(&access), Some(json!({}))).await;
        assert_eq!(empty_status, StatusCode::BAD_REQUEST);

        let (bad_mode_status, _) = request_json(
            &app,
            Method::PUT,
            "/me",
            Some(&access),
            Some(json!({ "display_name_mode": "display" })),
        )
        .await;
        assert_eq!(bad_mode_status, StatusCode::BAD_REQUEST);

        let (conflict_status, _) = request_json(
            &app,
            Method::PUT,
            "/me",
            Some(&access),
            Some(json!({ "email": other_email })),
        )
        .await;
        assert_eq!(conflict_status, StatusCode::CONFLICT);

        let new_username = unique("profile-renamed");
        let (update_status, update_body) = request_json(
            &app,
            Method::PUT,
            "/me",
            Some(&access),
            Some(json!({
                "email": original_email,
                "username": new_username,
                "nickname": "Demo Nick",
                "avatar_url": "https://example.com/avatar.png",
                "display_name_mode": "username"
            })),
        )
        .await;
        assert_eq!(update_status, StatusCode::OK);
        assert_eq!(
            update_body.get("nickname").and_then(Value::as_str),
            Some("Demo Nick")
        );
        assert_eq!(
            update_body.get("display_name_mode").and_then(Value::as_str),
            Some("username")
        );

        let boundary = format!("boundary-{}", Uuid::new_v4().simple());
        let bad_avatar_body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"avatar.txt\"\r\nContent-Type: text/plain\r\n\r\nnot-image\r\n--{boundary}--\r\n"
        );
        let (bad_avatar_status, _) = request_raw(
            &app,
            Method::POST,
            "/me/avatar",
            Some(&access),
            Some(&format!("multipart/form-data; boundary={boundary}")),
            bad_avatar_body,
        )
        .await;
        assert_eq!(bad_avatar_status, StatusCode::BAD_REQUEST);

        let boundary = format!("boundary-{}", Uuid::new_v4().simple());
        let good_avatar_body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"avatar.gif\"\r\nContent-Type: image/gif\r\n\r\nGIF89a\r\n--{boundary}--\r\n"
        );
        let (avatar_status, avatar_body) = request_raw(
            &app,
            Method::POST,
            "/me/avatar",
            Some(&access),
            Some(&format!("multipart/form-data; boundary={boundary}")),
            good_avatar_body,
        )
        .await;
        assert_eq!(avatar_status, StatusCode::OK);
        assert!(avatar_body
            .get("avatar_url")
            .and_then(Value::as_str)
            .is_some_and(|url| url.starts_with("/uploads/avatars/")));
    }

    #[tokio::test]
    async fn direct_messages_blocks_and_history_cleanup_flow() {
        let Some(app) = setup_app() else {
            return;
        };
        let (alice_access, _alice_id, _, _) = signup_user(&app, "dm-alice").await;
        let (bob_access, bob_id, _, _) = signup_user(&app, "dm-bob").await;

        let (dm_status, dm_body) = request_json(
            &app,
            Method::POST,
            &format!("/dms/{bob_id}"),
            Some(&alice_access),
            None,
        )
        .await;
        assert_eq!(dm_status, StatusCode::OK);
        let dm_channel_id = dm_body.get("id").and_then(Value::as_str).expect("dm id");

        let (send_status, _) = request_json(
            &app,
            Method::POST,
            &format!("/channels/{dm_channel_id}/messages"),
            Some(&alice_access),
            Some(json!({ "content": "private hello" })),
        )
        .await;
        assert_eq!(send_status, StatusCode::OK);

        let (bob_dms_status, bob_dms_body) =
            request_json(&app, Method::GET, "/dms", Some(&bob_access), None).await;
        assert_eq!(bob_dms_status, StatusCode::OK);
        assert!(bob_dms_body
            .as_array()
            .expect("dm list")
            .iter()
            .any(|channel| channel.get("id").and_then(Value::as_str) == Some(dm_channel_id)));

        let (block_status, _) = request_json(
            &app,
            Method::POST,
            &format!("/blocks/{bob_id}"),
            Some(&alice_access),
            None,
        )
        .await;
        assert_eq!(block_status, StatusCode::OK);

        let (blocks_status, blocks_body) =
            request_json(&app, Method::GET, "/blocks", Some(&alice_access), None).await;
        assert_eq!(blocks_status, StatusCode::OK);
        assert!(blocks_body
            .as_array()
            .expect("blocks")
            .iter()
            .any(|block| block.get("user_id").and_then(Value::as_str) == Some(bob_id.as_str())));

        let (blocked_dm_status, _) = request_json(
            &app,
            Method::POST,
            &format!("/dms/{bob_id}"),
            Some(&alice_access),
            None,
        )
        .await;
        assert_eq!(blocked_dm_status, StatusCode::FORBIDDEN);

        let (blocked_send_status, _) = request_json(
            &app,
            Method::POST,
            &format!("/channels/{dm_channel_id}/messages"),
            Some(&bob_access),
            Some(json!({ "content": "blocked reply" })),
        )
        .await;
        assert_eq!(blocked_send_status, StatusCode::FORBIDDEN);

        let (unblock_status, _) = request_json(
            &app,
            Method::DELETE,
            &format!("/blocks/{bob_id}"),
            Some(&alice_access),
            None,
        )
        .await;
        assert_eq!(unblock_status, StatusCode::OK);

        let (delete_history_status, _) = request_json(
            &app,
            Method::DELETE,
            &format!("/dms/channel/{dm_channel_id}"),
            Some(&alice_access),
            None,
        )
        .await;
        assert_eq!(delete_history_status, StatusCode::OK);

        let (messages_status, messages_body) = request_json(
            &app,
            Method::GET,
            &format!("/channels/{dm_channel_id}/messages"),
            Some(&alice_access),
            None,
        )
        .await;
        assert_eq!(messages_status, StatusCode::OK);
        assert!(messages_body.as_array().expect("messages").is_empty());
    }

    #[tokio::test]
    async fn server_moderation_invites_bans_and_unban_flow() {
        let Some(app) = setup_app() else {
            return;
        };
        let (owner_access, _owner_id, _, _) = signup_user(&app, "mod-owner").await;
        let (member_access, member_id, _, _) = signup_user(&app, "mod-member").await;
        let (outsider_access, outsider_id, _, _) = signup_user(&app, "mod-outsider").await;
        let (server_id, invite_code) = create_server_for(&app, &owner_access, "mod-server").await;

        let (join_status, _) = request_json(
            &app,
            Method::POST,
            "/servers/join",
            Some(&member_access),
            Some(json!({ "invite_code": invite_code })),
        )
        .await;
        assert_eq!(join_status, StatusCode::OK);

        let (member_invite_status, _) = request_json(
            &app,
            Method::POST,
            &format!("/servers/{server_id}/invites"),
            Some(&member_access),
            Some(json!({ "max_uses": 1 })),
        )
        .await;
        assert_eq!(member_invite_status, StatusCode::FORBIDDEN);

        let (owner_invite_status, owner_invite_body) = request_json(
            &app,
            Method::POST,
            &format!("/servers/{server_id}/invites"),
            Some(&owner_access),
            Some(json!({ "max_uses": 1, "expires_in_hours": 2 })),
        )
        .await;
        assert_eq!(owner_invite_status, StatusCode::OK);
        let limited_invite = owner_invite_body
            .get("invite_code")
            .and_then(Value::as_str)
            .expect("limited invite");

        let (outsider_join_status, _) = request_json(
            &app,
            Method::POST,
            "/servers/join",
            Some(&outsider_access),
            Some(json!({ "invite_code": limited_invite })),
        )
        .await;
        assert_eq!(outsider_join_status, StatusCode::OK);

        let (_, second_outsider_id, _, _) = signup_user(&app, "mod-outsider2").await;
        let second_outsider_access = {
            let (access, _, _, _) = signup_user(&app, "mod-outsider3").await;
            access
        };
        let (exhausted_join_status, _) = request_json(
            &app,
            Method::POST,
            "/servers/join",
            Some(&second_outsider_access),
            Some(json!({ "invite_code": limited_invite })),
        )
        .await;
        assert_eq!(exhausted_join_status, StatusCode::BAD_REQUEST);
        assert!(!second_outsider_id.is_empty());

        let (ban_status, _) = request_json(
            &app,
            Method::POST,
            &format!("/servers/{server_id}/bans"),
            Some(&owner_access),
            Some(json!({
                "user_id": member_id,
                "duration_hours": 1,
                "reason": "demo moderation"
            })),
        )
        .await;
        assert_eq!(ban_status, StatusCode::OK);

        let (bans_status, bans_body) = request_json(
            &app,
            Method::GET,
            &format!("/servers/{server_id}/bans"),
            Some(&owner_access),
            None,
        )
        .await;
        assert_eq!(bans_status, StatusCode::OK);
        assert!(bans_body
            .as_array()
            .expect("bans")
            .iter()
            .any(|ban| ban.get("user_id").and_then(Value::as_str) == Some(member_id.as_str())));

        let (banned_join_status, _) = request_json(
            &app,
            Method::POST,
            "/servers/join",
            Some(&member_access),
            Some(json!({ "invite_code": limited_invite })),
        )
        .await;
        assert_eq!(banned_join_status, StatusCode::FORBIDDEN);

        let (unban_status, _) = request_json(
            &app,
            Method::DELETE,
            &format!("/servers/{server_id}/bans/{member_id}"),
            Some(&owner_access),
            None,
        )
        .await;
        assert_eq!(unban_status, StatusCode::OK);

        let (kick_status, _) = request_json(
            &app,
            Method::DELETE,
            &format!("/servers/{server_id}/members/{outsider_id}"),
            Some(&owner_access),
            None,
        )
        .await;
        assert_eq!(kick_status, StatusCode::OK);
    }

    #[tokio::test]
    async fn channel_crud_and_message_editing_permissions_flow() {
        let Some(app) = setup_app() else {
            return;
        };
        let (owner_access, _owner_id, _, _) = signup_user(&app, "crud-owner").await;
        let (member_access, _member_id, _, _) = signup_user(&app, "crud-member").await;
        let (server_id, invite_code) = create_server_for(&app, &owner_access, "crud-server").await;

        let (join_status, _) = request_json(
            &app,
            Method::POST,
            "/servers/join",
            Some(&member_access),
            Some(json!({ "invite_code": invite_code })),
        )
        .await;
        assert_eq!(join_status, StatusCode::OK);

        let (create_status, create_body) = request_json(
            &app,
            Method::POST,
            &format!("/servers/{server_id}/channels"),
            Some(&owner_access),
            Some(json!({ "name": unique("crud-room"), "topic": "before", "position": 1 })),
        )
        .await;
        assert_eq!(create_status, StatusCode::OK);
        let channel_id = create_body
            .get("channel")
            .and_then(|value| value.get("id"))
            .and_then(Value::as_str)
            .expect("channel id");

        let (get_status, get_body) = request_json(
            &app,
            Method::GET,
            &format!("/channels/{channel_id}"),
            Some(&owner_access),
            None,
        )
        .await;
        assert_eq!(get_status, StatusCode::OK);
        assert_eq!(
            get_body
                .get("channel")
                .and_then(|value| value.get("topic"))
                .and_then(Value::as_str),
            Some("before")
        );

        let (member_update_status, _) = request_json(
            &app,
            Method::PUT,
            &format!("/channels/{channel_id}"),
            Some(&member_access),
            Some(json!({ "name": unique("nope") })),
        )
        .await;
        assert_eq!(member_update_status, StatusCode::FORBIDDEN);

        let (update_status, update_body) = request_json(
            &app,
            Method::PUT,
            &format!("/channels/{channel_id}"),
            Some(&owner_access),
            Some(json!({ "name": unique("crud-renamed"), "topic": null, "position": 3 })),
        )
        .await;
        assert_eq!(update_status, StatusCode::OK);
        assert_eq!(
            update_body
                .get("channel")
                .and_then(|value| value.get("position"))
                .and_then(Value::as_i64),
            Some(3)
        );

        let (send_status, send_body) = request_json(
            &app,
            Method::POST,
            &format!("/channels/{channel_id}/messages"),
            Some(&owner_access),
            Some(json!({ "content": "editable" })),
        )
        .await;
        assert_eq!(send_status, StatusCode::OK);
        let message_id = send_body
            .get("message")
            .and_then(|value| value.get("id"))
            .and_then(Value::as_str)
            .expect("message id");

        let (member_edit_status, _) = request_json(
            &app,
            Method::PUT,
            &format!("/messages/{message_id}"),
            Some(&member_access),
            Some(json!({ "content": "stolen edit" })),
        )
        .await;
        assert_eq!(member_edit_status, StatusCode::FORBIDDEN);

        let (owner_edit_status, owner_edit_body) = request_json(
            &app,
            Method::PUT,
            &format!("/messages/{message_id}"),
            Some(&owner_access),
            Some(json!({ "content": "edited by author" })),
        )
        .await;
        assert_eq!(owner_edit_status, StatusCode::OK);
        assert_eq!(
            owner_edit_body
                .get("message")
                .and_then(|value| value.get("content"))
                .and_then(Value::as_str),
            Some("edited by author")
        );

        let (delete_channel_status, _) = request_json(
            &app,
            Method::DELETE,
            &format!("/channels/{channel_id}"),
            Some(&owner_access),
            None,
        )
        .await;
        assert_eq!(delete_channel_status, StatusCode::OK);
    }

    #[tokio::test]
    async fn file_upload_creates_file_message_flow() {
        let Some(app) = setup_app() else {
            return;
        };
        let (owner_access, _owner_id, _, _) = signup_user(&app, "file-owner").await;
        let (server_id, _) = create_server_for(&app, &owner_access, "file-server").await;

        let (create_status, create_body) = request_json(
            &app,
            Method::POST,
            &format!("/servers/{server_id}/channels"),
            Some(&owner_access),
            Some(json!({ "name": unique("files"), "topic": "attachments", "position": 1 })),
        )
        .await;
        assert_eq!(create_status, StatusCode::OK);
        let channel_id = create_body
            .get("channel")
            .and_then(|value| value.get("id"))
            .and_then(Value::as_str)
            .expect("channel id");

        let bad_boundary = format!("bad-{}", Uuid::new_v4().simple());
        let bad_file_body = format!(
            "--{bad_boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"empty.txt\"\r\nContent-Type: text/plain\r\n\r\n\r\n--{bad_boundary}--\r\n"
        );
        let (bad_status, _) = request_raw(
            &app,
            Method::POST,
            &format!("/channels/{channel_id}/messages/file"),
            Some(&owner_access),
            Some(&format!("multipart/form-data; boundary={bad_boundary}")),
            bad_file_body,
        )
        .await;
        assert_eq!(bad_status, StatusCode::BAD_REQUEST);

        let boundary = format!("file-{}", Uuid::new_v4().simple());
        let file_body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"demo-notes.txt\"\r\nContent-Type: text/plain\r\n\r\nInvestor demo notes\r\n--{boundary}--\r\n"
        );
        let (upload_status, upload_body) = request_raw(
            &app,
            Method::POST,
            &format!("/channels/{channel_id}/messages/file"),
            Some(&owner_access),
            Some(&format!("multipart/form-data; boundary={boundary}")),
            file_body,
        )
        .await;
        assert_eq!(upload_status, StatusCode::OK);
        let content = upload_body
            .get("message")
            .and_then(|value| value.get("content"))
            .and_then(Value::as_str)
            .expect("file message content");
        assert!(content.starts_with("file::/uploads/attachments/"));
        assert!(content.contains("::demo-notes.txt::text/plain::19"));
    }
}
