# CI/CD

OrbitOtterChat uses GitHub Actions for visible quality gates. Production backend deploys are handled outside this repository by Railway.

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

Backend deployment is managed by Railway. This repository CI workflow does not deploy, publish Docker images, or require deployment secrets.
