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
