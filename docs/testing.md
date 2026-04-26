# Testing Guide

## Requirements

- Docker and Docker Compose
- Node.js `>=20.9.0`
- npm
- Rust stable toolchain
- `jq` for API smoke commands
- Linux Tauri dependencies when building desktop bundles

## Database

Start the PostgreSQL service:

```bash
docker compose up -d db
docker compose ps
```

Backend tests read `TEST_DATABASE_URL` first and fall back to `DATABASE_URL`.

```bash
export TEST_DATABASE_URL=postgres://rtc_user:rtc_pass@localhost:5433/rtc
export DATABASE_URL=postgres://rtc_user:rtc_pass@localhost:5433/rtc
export JWT_SECRET=test-jwt-secret
```

## Backend Checks

```bash
cd backend/rtc_backend
cargo fmt --check
cargo check --locked
cargo test --locked
```

Useful focused tests:

```bash
cargo test auth_signup_me_logout_refresh_flow
cargo test roles_channels_and_message_permissions_flow
cargo test direct_messages_blocks_and_history_cleanup_flow
cargo test file_upload_creates_file_message_flow
cargo test message_pin_flow_allows_author_and_persists_pin_state
cargo test message_reaction_toggle_flow
```

## Backend Coverage

Install once:

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov
```

Run:

```bash
cd backend/rtc_backend
cargo llvm-cov --locked --fail-under-lines 70 --lcov --output-path coverage/lcov.info
cargo llvm-cov report --summary-only
```

CI uploads the LCOV report as `backend-lcov`.

## Frontend Checks

```bash
cd chatroom
npm ci
npm run lint
npx tsc --noEmit
npm test
npm run build
```

Current lint policy allows warnings but fails on errors.

## Desktop Bundle Check

Install Linux dependencies on Ubuntu/Debian:

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf rpm
```

Build packages:

```bash
cd chatroom
npm ci
npm run tauri -- build
```

Expected outputs:

```text
src-tauri/target/release/bundle/deb/*.deb
src-tauri/target/release/bundle/rpm/*.rpm
src-tauri/target/release/bundle/appimage/*.AppImage
```

Optional package inspection:

```bash
dpkg-deb --info src-tauri/target/release/bundle/deb/*.deb
rpm -qip src-tauri/target/release/bundle/rpm/*.rpm
file src-tauri/target/release/bundle/appimage/*.AppImage
```

## End-to-End Local Run

Start backend and database:

```bash
docker compose up -d --build db backend
docker compose logs -f backend
```

Start frontend:

```bash
cd chatroom
npm run dev
```

Open `http://localhost:3001`.

## API Smoke Check

```bash
curl -i http://localhost:3000/

curl -sS -X POST http://localhost:3000/auth/signup \
  -H "Content-Type: application/json" \
  -d '{"email":"test1@example.com","username":"test1","password":"Passw0rd!"}'

TOKENS=$(curl -sS -X POST http://localhost:3000/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"test1@example.com","password":"Passw0rd!"}')

ACCESS=$(echo "$TOKENS" | jq -r '.access_token')
curl -sS http://localhost:3000/me -H "Authorization: Bearer $ACCESS"
```

## Troubleshooting

- Backend DB errors: check `TEST_DATABASE_URL`, `DATABASE_URL`, and whether `db` is healthy on port `5433`.
- Frontend build rejects Node: install Node.js `>=20.9.0`.
- Tauri build cannot find `libsoup-3.0`, `gtk`, `javascriptcoregtk-4.1`, or `gio-2.0`: install the Linux Tauri dependencies above.
- AppImage tooling downloads fail: rerun with network access, because Tauri downloads Linux packaging helpers.
