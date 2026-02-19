#!/usr/bin/env bash
set -euo pipefail

OWNER="mikumin27"
PACKAGES=("oscbot" "oscbot-build")

if ! command -v gh >/dev/null 2>&1; then
  echo "gh CLI is required but not found in PATH." >&2
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required but not found in PATH." >&2
  exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
  echo "gh is not authenticated. Run: gh auth login" >&2
  exit 1
fi

CUTOFF_EPOCH="$(date -u -d '24 hours ago' +%s)"

DELETED=0
FAILED=0
SCANNED=0
HAD_ERRORS=0

for package in "${PACKAGES[@]}"; do
  echo "Checking ghcr.io/${OWNER}/${package}"

  api_output=""
  if ! api_output="$(gh api --paginate "/users/${OWNER}/packages/container/${package}/versions?per_page=100" 2>&1)"; then
    if grep -q "read:packages scope" <<<"$api_output"; then
      echo "Missing required gh scope for ${package}: read:packages (and delete:packages for deletion)." >&2
      echo "Run: gh auth refresh -h github.com -s read:packages,delete:packages" >&2
      HAD_ERRORS=1
      continue
    fi

    if grep -q "Package not found" <<<"$api_output"; then
      echo "Package ghcr.io/${OWNER}/${package} not found, skipping."
      continue
    fi

    echo "Failed listing versions for ${package}: $api_output" >&2
    FAILED=$((FAILED + 1))
    HAD_ERRORS=1
    continue
  fi

  version_lines="$(jq -r --argjson cutoff "$CUTOFF_EPOCH" '.[] | select((.created_at | fromdateiso8601) < $cutoff) | "\(.id)|\(.created_at)"' <<<"$api_output" 2>/dev/null || true)"

  if [[ -z "$version_lines" ]]; then
    echo "No versions older than 24h found for ${package}"
    continue
  fi

  while IFS='|' read -r version_id created_at; do
    [[ -z "$version_id" ]] && continue
    SCANNED=$((SCANNED + 1))

    echo "Deleting ${package} version id=${version_id} created_at=${created_at}"
    if gh api -X DELETE "/users/${OWNER}/packages/container/${package}/versions/${version_id}" >/dev/null 2>&1; then
      DELETED=$((DELETED + 1))
    else
      echo "Failed to delete ${package} version id=${version_id}" >&2
      FAILED=$((FAILED + 1))
    fi
  done <<< "$version_lines"
done

echo "Done. Matched: ${SCANNED}, Deleted: ${DELETED}, Failed: ${FAILED}"

if [[ "$HAD_ERRORS" -ne 0 ]]; then
  exit 1
fi
