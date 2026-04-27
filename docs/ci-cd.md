# CI/CD

OrbitOtterChat uses GitHub Actions for quality gates, production smoke checks, and desktop release publishing. Production backend deployment is handled by Railway.

## CI Workflow

File: `.github/workflows/ci.yml`

Runs on every branch push and pull request to `main` or `master`.

Frontend job:
- `npm ci`
- `npm run lint`
- `npx next typegen`
- `npx tsc --noEmit`
- `npm test`
- `npm run build`

Backend job:
- PostgreSQL 16 service on port `5433`
- `cargo fmt --check`
- `cargo check --locked`
- `cargo test --locked`

## Deployment

Backend deployment is managed by Railway from the repository's `railway.toml` configuration. GitHub Actions does not push backend containers itself.

File: `.github/workflows/cd.yml`

Runs after the `CI` workflow completes successfully on `main`, and can also be started manually.

CD job:
- waits briefly for Railway to finish deploying the latest `main`
- checks `https://orbitotterchat-production.up.railway.app/`
- fails if the production backend does not return HTTP `200`

## Desktop Releases

File: `.github/workflows/release.yml`

Runs on `v*` tag pushes and manual workflow runs.

Release job:
- checks out the release tag
- installs Node.js, Rust, and Linux Tauri dependencies
- runs `./scripts/build-desktop-release.sh`
- uploads `.deb`, `.rpm`, and `.AppImage` bundles to the matching GitHub Release

The release build uses:

```text
NEXT_PUBLIC_API_URL=https://orbitotterchat-production.up.railway.app
```

Manual release rerun:

```bash
gh workflow run Release --field tag=v0.1.0
```
