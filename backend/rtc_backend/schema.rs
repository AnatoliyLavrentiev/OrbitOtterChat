
pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "member_role"))]
    pub struct MemberRole;
}

diesel::table! {
    channels (id) {
        id -> Uuid,
        server_id -> Uuid,
        name -> Text,
        topic -> Nullable<Text>,
        position -> Int4,
        created_by -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    invite_uses (invite_id, user_id) {
        invite_id -> Uuid,
        user_id -> Uuid,
        used_at -> Timestamptz,
    }
}

diesel::table! {
    invites (id) {
        id -> Uuid,
        server_id -> Uuid,
        code -> Text,
        created_by -> Uuid,
        expires_at -> Nullable<Timestamptz>,
        max_uses -> Nullable<Int4>,
        uses_count -> Int4,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    messages (id) {
        id -> Uuid,
        channel_id -> Uuid,
        author_id -> Uuid,
        content -> Text,
        created_at -> Timestamptz,
        edited_at -> Nullable<Timestamptz>,
        deleted_at -> Nullable<Timestamptz>,
        deleted_by -> Nullable<Uuid>,
        pinned_at -> Nullable<Timestamptz>,
        pinned_by -> Nullable<Uuid>,
    }
}

diesel::table! {
    refresh_tokens (id) {
        id -> Uuid,
        user_id -> Uuid,
        token_hash -> Text,
        created_at -> Timestamptz,
        expires_at -> Timestamptz,
        revoked_at -> Nullable<Timestamptz>,
        replaced_by -> Nullable<Uuid>,
        user_agent -> Nullable<Text>,
        ip -> Nullable<Inet>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::MemberRole;

    server_members (server_id, user_id) {
        server_id -> Uuid,
        user_id -> Uuid,
        role -> MemberRole,
        joined_at -> Timestamptz,
    }
}

diesel::table! {
    servers (id) {
        id -> Uuid,
        name -> Text,
        description -> Nullable<Text>,
        created_by -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        email -> Citext,
        username -> Citext,
        password_hash -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(channels -> servers (server_id));
diesel::joinable!(channels -> users (created_by));
diesel::joinable!(invite_uses -> invites (invite_id));
diesel::joinable!(invite_uses -> users (user_id));
diesel::joinable!(invites -> servers (server_id));
diesel::joinable!(invites -> users (created_by));
diesel::joinable!(messages -> channels (channel_id));
diesel::joinable!(refresh_tokens -> users (user_id));
diesel::joinable!(server_members -> servers (server_id));
diesel::joinable!(server_members -> users (user_id));
diesel::joinable!(servers -> users (created_by));

diesel::allow_tables_to_appear_in_same_query!(
    channels,
    invite_uses,
    invites,
    messages,
    refresh_tokens,
    server_members,
    servers,
    users,
);
