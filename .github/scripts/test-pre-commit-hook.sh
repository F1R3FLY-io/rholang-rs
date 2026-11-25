#!/usr/bin/env bash
# Behavioral tests for .githooks/pre-commit. A fake cargo binary records
# what the hook runs, so no Rust build happens.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HOOK="$ROOT/.githooks/pre-commit"
HARNESS="$(mktemp -d)"
trap 'rm -rf "$HARNESS"' EXIT
mkdir -p "$HARNESS/bin"

fail() {
  printf 'pre-commit hook test failed: %s\n' "$1" >&2
  exit 1
}

cat >"$HARNESS/bin/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s|%s\n' "$*" "${GIT_INDEX_FILE:-unset}" >> "$FAKE_CARGO_LOG"
if [[ "${FAIL_FMT:-0}" == 1 && "$1" == fmt ]]; then
    echo "Diff in fake.rs"
    exit 1
fi
if [[ "${FAIL_CLIPPY:-0}" == 1 && "$1" == clippy ]]; then
    echo "error: fake clippy failure"
    exit 101
fi
EOF
chmod +x "$HARNESS/bin/cargo"

COMMON_ENV=(
  env
  PATH="$HARNESS/bin:$PATH"
  FAKE_CARGO_LOG="$HARNESS/cargo"
  CI=
  GITLAB_CI=
  GITHUB_ACTIONS=
  SKIP_FMT=
  SKIP_CLIPPY=
  VERBOSE=
)

run_hook() {
  (cd "$ROOT" && "${COMMON_ENV[@]}" "$@" bash "$HOOK")
}

# --- Default run: fmt and clippy with the CI flags ---------------------------
: >"$HARNESS/cargo"
run_hook GIT_INDEX_FILE="$ROOT/.git/index.lock" >"$HARNESS/pass.out" 2>&1 ||
  { cat "$HARNESS/pass.out" >&2; fail 'clean run should pass'; }
grep -q '^fmt --all -- --check|' "$HARNESS/cargo" || fail 'fmt must run in check mode'
grep -q '^clippy --workspace --all-features --all-targets -- -D warnings|' "$HARNESS/cargo" ||
  fail 'clippy must run with the CI flags'
if grep -v '|unset$' "$HARNESS/cargo" | grep -q .; then
  fail 'cargo must not inherit GIT_INDEX_FILE from the hook'
fi
grep -q 'All pre-commit checks passed!' "$HARNESS/pass.out" || fail 'missing success summary'

# --- Failures block the commit -----------------------------------------------
for knob in FAIL_FMT FAIL_CLIPPY; do
  : >"$HARNESS/cargo"
  set +e
  run_hook "$knob=1" >"$HARNESS/fail.out" 2>&1
  rc=$?
  set -e
  [[ "$rc" -eq 1 ]] || fail "$knob must abort the commit"
  grep -q 'Commit aborted' "$HARNESS/fail.out" || fail "$knob must print the failure summary"
done
grep -q 'fake clippy failure' "$HARNESS/fail.out" || fail 'clippy errors must be shown'

# --- Skip knobs ----------------------------------------------------------------
: >"$HARNESS/cargo"
run_hook SKIP_FMT=1 SKIP_CLIPPY=1 >"$HARNESS/skip.out" 2>&1
[[ ! -s "$HARNESS/cargo" ]] || fail 'SKIP_FMT=1 SKIP_CLIPPY=1 must not run cargo'
grep -q 'SKIP_FMT=1' "$HARNESS/skip.out" || fail 'skipped fmt must be reported'

# --- CI environments skip the hook --------------------------------------------
: >"$HARNESS/cargo"
run_hook CI=true >"$HARNESS/ci.out" 2>&1
grep -q 'Skipping pre-commit hooks in CI environment' "$HARNESS/ci.out" || fail 'CI must skip the hook'
[[ ! -s "$HARNESS/cargo" ]] || fail 'CI skip must not run cargo'

printf 'pre-commit hook tests passed\n'
