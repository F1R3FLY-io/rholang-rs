#!/usr/bin/env bash
# Keeps the local link check (scripts/check-links.sh) and the CI workflow
# (.github/workflows/markdown-link-check.yml) on the same lychee arguments,
# and keeps the workflow actions pinned. Needs no network access.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORKFLOW="$ROOT/.github/workflows/markdown-link-check.yml"
LOCAL="$ROOT/scripts/check-links.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail() {
  printf 'link check test failed: %s\n' "$1" >&2
  exit 1
}

# Arguments from the workflow: the lines of the folded "args: >-" block,
# minus the './**/*.md' input glob, with shell quotes removed.
awk '
  /^ *args: >-/ { in_args = 1; next }
  in_args && /^ *[a-zA-Z_]+:/ { in_args = 0 }
  in_args { sub(/^ +/, ""); if ($0 != "") print }
' "$WORKFLOW" | grep -v -F "'./**/*.md'" | tr -d "'" | tr ' ' '\n' | grep -v '^$' >"$TMP/workflow.args"
bash "$LOCAL" --print-args | tr -d "'" | tr ' ' '\n' | grep -v '^$' >"$TMP/local.args"

[[ -s "$TMP/workflow.args" ]] || fail 'could not read the lychee args from the workflow'
diff -u "$TMP/workflow.args" "$TMP/local.args" >"$TMP/diff" ||
  { cat "$TMP/diff" >&2; fail 'scripts/check-links.sh and the workflow use different lychee arguments'; }

grep -q -F "'./**/*.md'" "$WORKFLOW" || fail 'the workflow must check every markdown file'
grep -q -E '^ *fail: true$' "$WORKFLOW" || fail 'the workflow must fail on broken links'
grep -q -E '^ *persist-credentials: false$' "$WORKFLOW" || fail 'checkout must not persist credentials'

# Third-party actions must be pinned to a full commit SHA.
if grep -E '^ *uses: ' "$WORKFLOW" | grep -v -E '@[0-9a-f]{40}( |$)' | grep -q .; then
  grep -E '^ *uses: ' "$WORKFLOW" | grep -v -E '@[0-9a-f]{40}( |$)' >&2
  fail 'every action in the link check workflow must be pinned to a commit SHA'
fi

[[ -f "$ROOT/.lycheeignore" ]] || fail '.lycheeignore must exist'

printf 'link check tests passed\n'
