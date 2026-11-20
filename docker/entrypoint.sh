#!/usr/bin/env bash
set -e

# Started with `user:` (non-root mode): nothing to chown or drop, the
# writable paths are open to any uid (see Dockerfile).
if [ "$(id -u)" != 0 ]; then
  exec /app/oscbot/oscbot "$@"
fi

PUID="${PUID:-1000}"
PGID="${PGID:-1000}"

chown -R "$PUID:$PGID" /app

exec gosu "$PUID:$PGID" /app/oscbot/oscbot "$@"
