# CI/CD

OrbitOtterChat uses GitHub Actions for quality gates, Docker delivery, optional server deployment, and Linux desktop bundle artifacts.

## CI Workflow

File: `.github/workflows/ci.yml`

Runs on every branch push and pull request to `main` or `master`.

Frontend job:
- `npm ci`
- `npm run lint`
- `npx tsc --noEmit`
- `npm test`
- `npm run build`

Backend job:
- PostgreSQL 16 service on port `5433`
- `cargo check --locked`
- `cargo fmt --check`
- `cargo test --locked`
- `cargo llvm-cov --locked --fail-under-lines 70`
- LCOV upload as `backend-lcov`

## CD Workflow

File: `.github/workflows/cd.yml`

Runs on:
- pushes to `main` and `master`
- tags matching `v*`
- manual `workflow_dispatch`

The workflow has three delivery paths:

- Quality gates: frontend and backend checks must pass before publishing.
- Docker delivery: backend and frontend images are built and pushed to GHCR.
- Desktop delivery: Linux `.deb`, `.rpm`, and `.AppImage` bundles are built for `v*` tags and manual runs.

## Docker Images

Images:

```text
ghcr.io/<owner>/rtc-backend
ghcr.io/<owner>/rtc-frontend
```

Tags:
- `sha-<commit_sha>` for immutable deployments
- branch names for branch builds
- Git tag names for `v*` releases
- `latest` only from the default branch

## Optional Server Deployment

Deploy runs only when all required secrets are configured. If any required deploy secret is missing, deploy steps are skipped with GitHub Actions notices.

Required secrets:

- `DEPLOY_HOST`
- `DEPLOY_PORT` optional, defaults to `22`
- `DEPLOY_USER`
- `DEPLOY_SSH_KEY`
- `DEPLOY_PATH`
- `GHCR_USERNAME`
- `GHCR_TOKEN`
- `POSTGRES_USER`
- `POSTGRES_PASSWORD`
- `POSTGRES_DB`
- `JWT_SECRET`

Deployment flow:

1. Upload `deploy/docker-compose.prod.yml` to the server.
2. Generate a runtime `.env` on the server.
3. Log in to GHCR.
4. Pull images.
5. Run `docker compose -f docker-compose.prod.yml up -d`.
6. Prune unused Docker images.

## Desktop Bundle Artifacts

The `desktop-bundles` job runs on `v*` tags and manual workflow runs.

It installs Linux Tauri dependencies:

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf rpm
```

It builds:

```bash
cd chatroom
npm ci
npm run tauri -- build
```

Uploaded artifacts:

- `OrbitOtterChat-deb`
- `OrbitOtterChat-rpm`
- `OrbitOtterChat-AppImage`

## Release Tag Convention

Use semantic version tags that start with `v`, for example:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Older local tags such as `deb-release-0.1.0`, `rpm-release-0.1.0`, and `appimage-release-0.1.0` do not match the current workflow trigger.

## School Project Mode

If there is no production server:

- keep CI enabled
- keep CD enabled
- do not configure deploy secrets

Docker images and desktop artifacts can still be produced while server deployment is skipped safely.
