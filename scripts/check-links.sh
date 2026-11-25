#!/usr/bin/env bash
#
# check-links.sh - Run the CI markdown link check locally
#
# Uses the same lychee arguments as .github/workflows/markdown-link-check.yml.
# Extra arguments are passed through to lychee (for example: --verbose).
#
# Requirements: lychee (brew install lychee, or cargo install lychee)
#

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

LYCHEE_ARGS=(
    --no-progress
    --exclude '^https?://'
    --include '^https?://(www\.)?github\.com'
    --include '^https?://raw\.githubusercontent\.com'
    --include '^https?://(www\.)?gitlab\.com'
    --max-retries 2
    --retry-wait-time 5
    --timeout 45
    --accept '200..=299,403,429,503,504'
)

if [[ "${1:-}" == "--print-args" ]]; then
    printf '%s\n' "${LYCHEE_ARGS[@]}"
    exit 0
fi

if ! command -v lychee >/dev/null 2>&1; then
    echo "lychee is required: brew install lychee (or cargo install lychee)" >&2
    exit 1
fi

cd "$REPO_ROOT"
exec lychee "${LYCHEE_ARGS[@]}" "$@" './**/*.md'
