#!/bin/sh
set -eu

cd /app

if [ -z "${DATABASE_URL:-}" ]; then
  echo "[entrypoint] DATABASE_URL is missing"
else
  echo "[entrypoint] DATABASE_URL is set"
fi

echo "[entrypoint] Running migrations..."
i=0
until diesel migration run; do
  i=$((i+1))
  if [ "$i" -gt 60 ]; then
    echo "[entrypoint] DB not ready after 60s"
    exit 1
  fi
  echo "[entrypoint] DB not ready yet... retry ($i)"
  sleep 1
done

echo "[entrypoint] Starting backend..."
exec /usr/local/bin/rtc_backend
