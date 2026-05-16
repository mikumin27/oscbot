#!/usr/bin/env bash
set -e

PUID="${PUID:-1000}"
PGID="${PGID:-1000}"

chown -R "$PUID:$PGID" /app

exec gosu "$PUID:$PGID" /app/oscbot/oscbot "$@"
