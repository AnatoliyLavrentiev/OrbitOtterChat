# OrbitOtterChat Frontend

This directory contains the OrbitOtterChat Next.js frontend and Tauri desktop shell.

## Requirements

- Node.js `>=20.9.0`
- npm
- Rust stable toolchain for Tauri builds
- Linux Tauri dependencies for `.deb`, `.rpm`, and `.AppImage` packaging

## Development

```bash
npm ci
npm run dev
```

The web app runs on `http://localhost:3001`.

The frontend expects the backend API on `http://localhost:3000` unless overridden by environment configuration.

## Checks

```bash
npm run lint
npx tsc --noEmit
npm test
npm run build
```

## Desktop Build

```bash
npm run tauri -- build
```

Generated bundles:

```text
src-tauri/target/release/bundle/deb/*.deb
src-tauri/target/release/bundle/rpm/*.rpm
src-tauri/target/release/bundle/appimage/*.AppImage
```

The public desktop product name is `OrbitOtterChat`.
