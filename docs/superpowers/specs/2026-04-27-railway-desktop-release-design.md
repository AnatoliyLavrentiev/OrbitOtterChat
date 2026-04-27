# Railway Desktop Release Design

## Goal

Build desktop release bundles that automatically connect to the Railway backend at `https://orbitotterchat-production.up.railway.app`.

## Architecture

The frontend already resolves API and WebSocket endpoints from `NEXT_PUBLIC_API_URL`. Tauri packages the static Next export, so the release wrapper will set `NEXT_PUBLIC_API_URL` before invoking the Tauri build. The existing endpoint resolver will convert the HTTPS backend URL into the matching WSS WebSocket base URL.

## Components

- `chatroom/lib/runtimeEndpoints.test.ts` verifies the Railway release URL maps to HTTPS API and WSS WebSocket endpoints.
- `scripts/build-desktop-release.sh` wraps the desktop bundle command and exports the Railway backend URL by default.
- `README.md` and `chatroom/README.md` document the release command and generated bundle paths.

## Error Handling

The wrapper enables strict shell mode and fails immediately if `npm ci` or the Tauri build fails. Users may override the backend by setting `NEXT_PUBLIC_API_URL` before running the wrapper.

## Testing

Run frontend endpoint tests and, where local system dependencies allow it, run the desktop release wrapper.
