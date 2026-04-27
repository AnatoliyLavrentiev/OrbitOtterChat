# CD Automation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add GitHub Actions CD coverage for Railway production smoke checks and automated desktop releases.

**Architecture:** Railway remains the backend deploy system. GitHub Actions verifies production after `main` pushes and publishes desktop bundles when a `v*` release tag is pushed or the release workflow is manually dispatched.

**Tech Stack:** GitHub Actions, Bash, curl, npm, Tauri, GitHub Releases.

---

### Task 1: Production Backend Smoke CD

**Files:**
- Create: `.github/workflows/cd.yml`

- [ ] **Step 1: Add smoke workflow**

Create a workflow that runs after CI completes on `main`, waits briefly for Railway, then curls `https://orbitotterchat-production.up.railway.app/` until it returns `200`.

- [ ] **Step 2: Validate workflow formatting**

Run: `git diff --check`

Expected: exit code 0.

### Task 2: Desktop Release Workflow

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Add release workflow**

Create a workflow that runs on `v*` tags or manual dispatch, installs Linux Tauri dependencies, runs `./scripts/build-desktop-release.sh`, and uploads `.deb`, `.rpm`, and `.AppImage` files to the matching GitHub Release.

- [ ] **Step 2: Validate workflow formatting**

Run: `git diff --check`

Expected: exit code 0.

### Task 3: Documentation

**Files:**
- Modify: `docs/ci-cd.md`

- [ ] **Step 1: Document CD behavior**

Explain the CI workflow, Railway smoke CD workflow, and desktop release workflow.

- [ ] **Step 2: Verify repository status**

Run: `git diff -- .github/workflows/cd.yml .github/workflows/release.yml docs/ci-cd.md`

Expected: only CD workflow and documentation changes.
