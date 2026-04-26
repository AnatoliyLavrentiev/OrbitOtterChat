# OrbitOtterChat

OrbitOtterChat is a real-time chat application for the T-DEV-600-REN_13 project.

It ships as:
- a web application served by Next.js
- a Linux desktop application built with Tauri (`.deb`, `.rpm`, `.AppImage`)
- a Rust backend API with WebSocket realtime events
- Docker images for deployment

## Features

- Email/password signup, login, refresh, logout, and current-user profile.
- Server workspaces with owner/admin/member roles.
- Channel creation, update, deletion, and membership-aware message access.
- Direct messages with block/unblock policy and DM history cleanup.
- Real-time WebSocket presence, status, typing indicators, and message events.
- Message create, edit, delete, search, pins, reactions, mentions, GIF messages, and file uploads.
- Invite codes, join-by-invite, member role updates, ownership transfer, kick, temporary ban, permanent ban, and unban.
- Profile settings with username, nickname, display-name mode, avatar URL, and local avatar upload.
- English/French UI language switch.

## Project Structure

- `backend/rtc_backend` - Rust Axum API, Diesel repositories, PostgreSQL migrations.
- `chatroom` - Next.js frontend and Tauri desktop shell.
- `docs` - architecture, testing, socket, and CI/CD documentation.
- `deploy/docker-compose.prod.yml` - production compose file used by CD.
- `.github/workflows` - CI and CD workflows.

## Prerequisites

For Docker-only development:
- Docker and Docker Compose.

For local development and desktop bundles:
- Node.js `>=20.9.0`
- npm
- Rust stable toolchain
- PostgreSQL client libraries (`libpq-dev`, `pkg-config`)
- Tauri Linux dependencies:
  - `libwebkit2gtk-4.1-dev`
  - `libgtk-3-dev`
  - `libayatana-appindicator3-dev`
  - `librsvg2-dev`
  - `patchelf`
  - `rpm`

Ubuntu/Debian setup:

```bash
sudo apt-get update
sudo apt-get install -y libpq-dev pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf rpm
```

## Run With Docker

From the repository root:

```bash
docker compose build
docker compose up
```

Services:
- Backend API: `http://localhost:3000`
- Frontend web app: `http://localhost:3001`
- PostgreSQL: `localhost:5433`

Stop services:

```bash
docker compose down
```

Stop services and remove persisted database/upload volumes:

```bash
docker compose down -v
```

## Run Locally

Start the database:

```bash
docker compose up -d db
```

Create `backend/rtc_backend/.env`:

```env
DATABASE_URL=postgres://rtc_user:rtc_pass@localhost:5433/rtc
RUST_LOG=info
JWT_SECRET=change_me_in_production
```

Run the backend:

```bash
cd backend/rtc_backend
cargo run
```

Run the frontend:

```bash
cd chatroom
npm ci
npm run dev
```

Open `http://localhost:3001`.

## Build Desktop Packages

```bash
cd chatroom
npm ci
npm run tauri -- build
```

Generated Linux bundles:

```text
chatroom/src-tauri/target/release/bundle/deb/*.deb
chatroom/src-tauri/target/release/bundle/rpm/*.rpm
chatroom/src-tauri/target/release/bundle/appimage/*.AppImage
```

## Quality Checks

Frontend:

```bash
cd chatroom
npm run lint
npx tsc --noEmit
npm test
npm run build
```

Backend:

```bash
cd backend/rtc_backend
cargo fmt --check
cargo check --locked
cargo test --locked
```

## API Smoke Check

```bash
curl -i http://localhost:3000/

curl -sS -X POST http://localhost:3000/auth/signup \
  -H "Content-Type: application/json" \
  -d '{"email":"u1@example.com","username":"u1","password":"Passw0rd!"}'

TOKENS=$(curl -sS -X POST http://localhost:3000/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"u1@example.com","password":"Passw0rd!"}')

ACCESS=$(echo "$TOKENS" | jq -r '.access_token')
curl -sS http://localhost:3000/me -H "Authorization: Bearer $ACCESS"
```

## CI/CD

- CI runs frontend lint/typecheck/tests/build and backend fmt/check/tests/coverage.
- CD builds and pushes Docker images to GHCR, then deploys over SSH when deploy secrets are configured.
- CD also builds OrbitOtterChat Linux desktop bundles on `v*` tags and manual workflow runs.

See [docs/ci-cd.md](docs/ci-cd.md) for details.

## More Documentation

- [Architecture](docs/architecture.md)
- [Testing](docs/testing.md)
- [CI/CD](docs/ci-cd.md)
- [WebSocket events](docs/socket-spec.md)
