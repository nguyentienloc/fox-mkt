#!/bin/bash
# Usage: scripts/set-min-supported-version.sh <x.y.z|none> [tag]
# Writes (or removes) the `min_supported_version` line in a GitHub release body.
# Apps older than this version are forced to update to the latest release.
set -euo pipefail

MIN_VERSION="${1:-}"
TAG="${2:-}"
MARKER_REGEX='^[[:space:]`*-]*min_supported_version[[:space:]`*]*[:=]'

if ! command -v gh >/dev/null 2>&1; then
  echo "Error: GitHub CLI (gh) is not installed. Install it with: brew install gh" >&2
  exit 1
fi

if [[ -z "$MIN_VERSION" ]]; then
  echo "Usage: $0 <x.y.z|none> [tag]" >&2
  exit 1
fi

if [[ "$MIN_VERSION" != "none" && ! "$MIN_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Error: MIN must look like 0.0.18 (or 'none' to remove the marker)" >&2
  exit 1
fi

if [[ -z "$TAG" ]]; then
  TAG=$(gh release view --json tagName -q .tagName)
fi

RELEASE_VERSION="${TAG#v}"
if [[ "$MIN_VERSION" != "none" ]]; then
  HIGHEST=$(printf '%s\n%s\n' "$MIN_VERSION" "$RELEASE_VERSION" | sort -V | tail -n 1)
  if [[ "$HIGHEST" != "$RELEASE_VERSION" ]]; then
    echo "Error: MIN ($MIN_VERSION) cannot be higher than the release version ($RELEASE_VERSION)" >&2
    exit 1
  fi
fi

NOTES_FILE=$(mktemp)
trap 'rm -f "$NOTES_FILE"' EXIT

gh release view "$TAG" --json body -q .body | tr -d '\r' | grep -viE "$MARKER_REGEX" > "$NOTES_FILE" || true

if [[ "$MIN_VERSION" != "none" ]]; then
  printf '\nmin_supported_version: %s\n' "$MIN_VERSION" >> "$NOTES_FILE"
fi

gh release edit "$TAG" --notes-file "$NOTES_FILE"

if [[ "$MIN_VERSION" == "none" ]]; then
  echo "Removed min_supported_version from release $TAG"
else
  echo "Release $TAG: apps older than $MIN_VERSION will be forced to update"
fi
