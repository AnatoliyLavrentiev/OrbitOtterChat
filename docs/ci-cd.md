# CI/CD

OrbitOtterChat uses GitHub Actions for quality gates, Railway production smoke checks, and desktop release publishing.

## Pipeline Overview

```text
push any branch / pull request -> CI

push main -> CI -> Railway deploys backend -> CD smoke check

push v* tag / manual Release run -> desktop bundles -> GitHub Release assets
```

Railway owns the actual backend deployment. GitHub Actions verifies source quality, checks the deployed backend, and publishes desktop packages.

## Workflows

| Workflow | File | Trigger | Purpose |
| --- | --- | --- | --- |
| CI | `.github/workflows/ci.yml` | Every branch push and pull requests to `main` or `master` | Frontend and backend quality gates |
| CD | `.github/workflows/cd.yml` | Successful CI workflow on `main`, or manual dispatch | Production Railway smoke check |
| Release | `.github/workflows/release.yml` | `v*` tag push, or manual dispatch with a tag | Build and upload desktop bundles |

## CI Workflow

File: `.github/workflows/ci.yml`

CI has two independent jobs: `Frontend` and `Backend`.

### Frontend Job

Working directory: `chatroom`

Commands:

```bash
npm ci
npm run lint
npx next typegen
npx tsc --noEmit
npm test
npm run build
```

What this checks:

- dependencies install from `chatroom/package-lock.json`
- ESLint rules for the Next.js app
- generated Next.js types
- TypeScript type safety
- Vitest unit tests
- production static Next export used by Tauri

Frontend tests run by `npm test`:

```text
chatroom/lib/fileMessage.test.ts
chatroom/lib/messageSearch.test.ts
chatroom/lib/releaseConfig.test.ts
chatroom/lib/runtimeEndpoints.test.ts
```

Current frontend test coverage includes:

- file message metadata parsing and size formatting
- message search filtering
- Tauri/static export release configuration
- API and WebSocket endpoint resolution, including the Railway backend URL

### Backend Job

Working directory: `backend/rtc_backend`

Service container:

```text
postgres:16
database: rtc
user: rtc_user
port on runner: 5433
```

Environment:

```text
DATABASE_URL=postgres://rtc_user:rtc_pass@localhost:5433/rtc
TEST_DATABASE_URL=postgres://rtc_user:rtc_pass@localhost:5433/rtc
JWT_SECRET=test-jwt-secret
```

Commands:

```bash
sudo apt-get update && sudo apt-get install -y libpq-dev pkg-config
cargo fmt --all --check
cargo check --locked
cargo test --locked
```

What this checks:

- Rust formatting
- dependency lockfile consistency
- backend compilation
- PostgreSQL-backed repository and API flows
- domain, service, auth token, JWT, and WebSocket behavior

Backend tests run by `cargo test --locked`:

```text
domain::permissions
handlers::web_socket
repositories::channels
repositories::server_members
security::jwt
security::tokens
services::hash_passwords
services::servers_service
main.rs integration flows
```

The integration flows include signup/login/current-user behavior, channel CRUD, roles and permissions, direct messages and blocks, message reactions and pins, file uploads, profile/avatar validation, invites, bans, and unban behavior.

## CD Workflow

File: `.github/workflows/cd.yml`

CD does not deploy the backend directly. Railway deploys the backend from this repository using `railway.toml`.

After CI succeeds on `main`, the CD workflow:

1. waits 45 seconds for Railway to finish deploying
2. requests `https://orbitotterchat-production.up.railway.app/`
3. retries up to 20 times with a 15 second delay
4. passes only when the backend returns HTTP `200`

Manual production smoke check:

```bash
gh workflow run CD
```

Equivalent local smoke check:

```bash
curl -i https://orbitotterchat-production.up.railway.app/
```

Expected result:

```text
HTTP/2 200
Hello, World!
```

## Release Workflow

File: `.github/workflows/release.yml`

Release builds Linux desktop packages from a tag and publishes them to GitHub Releases.

The release workflow:

1. checks out the release tag
2. installs Node.js and Rust
3. installs Linux Tauri build dependencies
4. runs `./scripts/build-desktop-release.sh`
5. creates or updates the GitHub Release for the tag
6. uploads `.deb`, `.rpm`, and `.AppImage` assets

The desktop release build uses:

```text
NEXT_PUBLIC_API_URL=https://orbitotterchat-production.up.railway.app
```

That means packaged desktop clients connect to:

```text
https://orbitotterchat-production.up.railway.app
wss://orbitotterchat-production.up.railway.app/ws
```

Create a new release:

```bash
git tag -a v0.1.1 -m "OrbitOtterChat v0.1.1"
git push origin v0.1.1
```

Rebuild an existing release manually:

```bash
gh workflow run Release --field tag=v0.1.0
```

Build the same desktop packages locally:

```bash
./scripts/build-desktop-release.sh
```

Generated local bundles:

```text
chatroom/src-tauri/target/release/bundle/deb/*.deb
chatroom/src-tauri/target/release/bundle/rpm/*.rpm
chatroom/src-tauri/target/release/bundle/appimage/*.AppImage
```

## Useful Commands

List workflows:

```bash
gh workflow list
```

List recent runs:

```bash
gh run list --limit 10
```

Watch a run:

```bash
gh run watch <run-id> --exit-status
```

View a failed run:

```bash
gh run view <run-id> --log-failed
```

Run frontend checks locally:

```bash
cd chatroom
npm ci
npm run lint
npx next typegen
npx tsc --noEmit
npm test
npm run build
```

Run backend checks locally:

```bash
cd backend/rtc_backend
cargo fmt --all --check
cargo check --locked
cargo test --locked
```

Run only repository backend tests:

```bash
cd backend/rtc_backend
cargo test --locked repositories::
```

Run only endpoint resolution frontend tests:

```bash
cd chatroom
npm test -- lib/runtimeEndpoints.test.ts
```

Check published release assets:

```bash
gh release view v0.1.0 --json url,tagName,assets
```

## Troubleshooting

### Backend CI Fails With `citext`

Error shape:

```text
SerializationError(FailedToLookupTypeError(... type_name: "citext"))
```

This means a PostgreSQL connection was opened before migrations created the `citext` extension. Backend tests should use the shared test helper in `backend/rtc_backend/src/test_support.rs`, which runs migrations before returning test connections.

### CD Smoke Fails

Check Railway first:

```bash
curl -i https://orbitotterchat-production.up.railway.app/
```

If local curl works but GitHub CD fails, Railway may still have been deploying. Re-run CD manually:

```bash
gh workflow run CD
```

### Release Upload Fails

Confirm the tag exists on GitHub:

```bash
git ls-remote --tags origin v0.1.0
```

Confirm the GitHub Release exists and inspect assets:

```bash
gh release view v0.1.0 --json url,assets
```

Re-run the release workflow:

```bash
gh workflow run Release --field tag=v0.1.0
```
