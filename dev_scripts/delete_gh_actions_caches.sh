#!/usr/bin/env bash
set -euo pipefail

REPO="mikumin27/oscbot"

if ! command -v gh >/dev/null 2>&1; then
  echo "gh CLI is required but not found in PATH." >&2
  exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
  echo "gh is not authenticated. Run: gh auth login" >&2
  exit 1
fi

CACHE_IDS=$(gh cache list --repo "$REPO" --limit 1000 --json id --jq ".[].id")

if [[ -z "$CACHE_IDS" ]]; then
  echo "No caches found for $REPO"
  exit 0
fi

COUNT=0
while IFS= read -r cache_id; do
  [[ -z "$cache_id" ]] && continue
  COUNT=$((COUNT + 1))

  echo "Deleting cache id=$cache_id"
  gh cache delete "$cache_id" --repo "$REPO"
done <<< "$CACHE_IDS"

echo "Done. Deleted caches: $COUNT"
