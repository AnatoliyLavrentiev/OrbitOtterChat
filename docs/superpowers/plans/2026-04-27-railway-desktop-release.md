# Railway Desktop Release Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build OrbitOtterChat desktop bundles that automatically use the Railway backend.

**Architecture:** Use the existing `NEXT_PUBLIC_API_URL` endpoint mechanism at build time. Add a root-level release wrapper that exports the Railway URL and invokes the existing Tauri build.

**Tech Stack:** Bash, Next.js static export, Tauri, Vitest.

---

### Task 1: Endpoint Coverage

**Files:**
- Modify: `chatroom/lib/runtimeEndpoints.test.ts`

- [ ] **Step 1: Add Railway endpoint assertions**

Add this test:

```ts
it("maps the Railway release backend to HTTPS API and WSS websocket URLs", () => {
  const railwayUrl = "https://orbitotterchat-production.up.railway.app";

  expect(resolveApiBaseUrl(railwayUrl, "tauri:", "localhost")).toBe(railwayUrl);
  expect(resolveWsBaseUrl(railwayUrl, "tauri:", "localhost")).toBe(
    "wss://orbitotterchat-production.up.railway.app",
  );
});
```

- [ ] **Step 2: Run the focused test**

Run: `cd chatroom && npm test -- lib/runtimeEndpoints.test.ts`

Expected: PASS after implementation.

### Task 2: Release Wrapper

**Files:**
- Create: `scripts/build-desktop-release.sh`

- [ ] **Step 1: Add wrapper script**

Create this executable script:

```bash
#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHATROOM_DIR="$ROOT_DIR/chatroom"
RAILWAY_BACKEND_URL="${NEXT_PUBLIC_API_URL:-https://orbitotterchat-production.up.railway.app}"

export NEXT_PUBLIC_API_URL="$RAILWAY_BACKEND_URL"

echo "Building OrbitOtterChat desktop release"
echo "Backend: $NEXT_PUBLIC_API_URL"

cd "$CHATROOM_DIR"
npm ci
npm run tauri -- build
```

- [ ] **Step 2: Mark it executable**

Run: `chmod +x scripts/build-desktop-release.sh`

### Task 3: Documentation

**Files:**
- Modify: `README.md`
- Modify: `chatroom/README.md`

- [ ] **Step 1: Document the wrapper**

Add `./scripts/build-desktop-release.sh` as the recommended release command and mention the Railway backend URL.

- [ ] **Step 2: Run verification**

Run: `cd chatroom && npm test -- lib/runtimeEndpoints.test.ts lib/releaseConfig.test.ts`

Expected: PASS.
