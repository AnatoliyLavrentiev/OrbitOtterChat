# Architecture

OrbitOtterChat is split into a Rust backend, a Next.js frontend, and a Tauri desktop shell.

## Runtime Components

- PostgreSQL stores users, servers, memberships, channels, messages, refresh tokens, invites, bans, blocks, reactions, mentions, and pins.
- Rust Axum backend exposes HTTP APIs and the `/ws` WebSocket endpoint.
- Next.js frontend renders the web chat UI.
- Tauri packages the frontend as the OrbitOtterChat desktop app.
- Docker Compose runs PostgreSQL, backend, and frontend for local and production deployments.

## Backend Layers

- `handlers/*`: HTTP and WebSocket boundaries, request parsing, auth extraction, response mapping.
- `services/*`: business workflows, permission orchestration, role checks, invite and moderation rules.
- `repositories/*`: Diesel queries and persistence primitives.
- `domain/*`: pure policy functions, especially role/permission logic.
- `security/*`: JWT and token helpers.

## Request Flow

1. Handler validates input and extracts authentication context.
2. Handler calls a service or repository function.
3. Service checks membership, role, block, ban, or ownership rules.
4. Repository reads or writes PostgreSQL through Diesel.
5. Handler serializes success data or maps failures to `AppError`.

## Realtime Flow

1. Client opens `GET /ws?token&server_id&channel_id`.
2. Backend validates the token and membership.
3. `WsHub` tracks connected users, presence, and status.
4. Events are filtered by server, channel, or DM membership.
5. Clients receive message, reaction, typing, status, and moderation events.

See `docs/socket-spec.md` for event payloads.

## Feature Areas

- Authentication: signup, login, refresh, logout, current user.
- Profile: username, nickname, display-name mode, avatar URL, avatar upload.
- Servers: create, update, delete, invites, membership, roles, ownership transfer.
- Moderation: kick, temporary ban, permanent ban, unban, ban list.
- Channels: create, list, update, delete, channel-scoped messages.
- Messages: create, list, edit, delete, search, pins, mentions, reactions, GIF payloads, file uploads.
- Direct messages: create/open DM, block/unblock users, delete local DM history.
- Presence: online/away/invisible status and typing indicators.

## Persistence

Schema changes live in `backend/rtc_backend/migrations`.

Uploads are stored on disk and mounted as Docker volume `backend_uploads` in Compose. Database rows store message content and upload/avatar references.

## Permission Model

Roles:

- `OWNER`
- `ADMIN`
- `MEMBER`

General rules:

- Owner controls ownership transfer and highest-risk server actions.
- Owner/Admin can manage channels, invites, moderation, and member roles within role hierarchy limits.
- Members can send messages and manage their own content where allowed.
- DM access is constrained by block policy and DM membership.

## Error Strategy

`AppError` maps failures to HTTP status classes:

- `400` invalid input
- `401` authentication failure
- `403` permission, membership, ban, or block failure
- `404` missing entities
- `409` conflict constraints
- `500` internal or database failure

## Delivery

- Web delivery: Docker frontend image serving Next.js on port `3001`.
- API delivery: Docker backend image serving Axum on port `3000`.
- Desktop delivery: Tauri bundles the same frontend into `.deb`, `.rpm`, and `.AppImage` packages.

## Testability

- Pure permission tests live close to domain code.
- Repository and integration tests use PostgreSQL.
- WebSocket routing tests validate event filtering.
- Frontend unit tests cover message search and file-message parsing.
- CI enforces backend coverage at 70% line coverage.
