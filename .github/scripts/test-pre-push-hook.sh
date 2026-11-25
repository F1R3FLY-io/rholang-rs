#!/usr/bin/env bash
# Behavioral tests for .githooks/pre-push. Fake git, cargo, and timeout
# binaries record what the hook runs, so no Rust build happens.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HOOK="$ROOT/.githooks/pre-push"
HARNESS="$(mktemp -d)"
trap 'rm -rf "$HARNESS"' EXIT
mkdir -p "$HARNESS/bin" "$HARNESS/tmp"
REAL_GIT="$(command -v git)"

fail() {
  printf 'pre-push hook test failed: %s\n' "$1" >&2
  exit 1
}

cat >"$HARNESS/bin/git" <<EOF
#!/usr/bin/env bash
set -euo pipefail
case "\${1:-} \${2:-}" in
  "rev-parse --show-toplevel") exec "$REAL_GIT" "\$@" ;;
  "ls-remote origin") printf '%s\t%s\n' "\$FAKE_REMOTE_OID" "\${3:-refs/heads/test}" ;;
  "fetch origin") exit 0 ;;
  *) exec "$REAL_GIT" "\$@" ;;
esac
EOF
cat >"$HARNESS/bin/timeout" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "$FAKE_TIMEOUT_LOG"
shift
exec "$@"
EOF
cat >"$HARNESS/bin/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s %s\n' "$$" "$*" >> "$FAKE_CARGO_LOG"
if [[ "${MISSING_NEXTEST:-0}" == 1 && "$*" == "nextest --version" ]]; then
    exit 101
fi
if [[ "${FAIL_CLIPPY:-0}" == 1 && "$1" == clippy ]]; then
    echo "error: fake clippy failure"
    exit 101
fi
if [[ "${SLEEP_TEST:-0}" == 1 && "$1" == test ]]; then
    : > "$FAKE_TEST_STARTED"
    sleep 300
fi
EOF
chmod +x "$HARNESS/bin/git" "$HARNESS/bin/timeout" "$HARNESS/bin/cargo"
grep -Fq "trap 'cleanup 143' TERM" "$HOOK" || fail 'hook must clean up on TERM'
if grep -q -E 'declare -A|,,\}|\^\^\}|\^\}|wait -n' "$HOOK"; then
  fail 'the hook must avoid bash 4 features (it runs on the macOS system bash)'
fi

HEAD_SHA="$(git -C "$ROOT" rev-parse HEAD)"
REMOTE_SHA="0000000000000000000000000000000000000001"
PUSH_INPUT="refs/heads/test $HEAD_SHA refs/heads/test $REMOTE_SHA"
# Clear every knob the hook reads, so a caller that pushes with e.g.
# SKIP_TESTS=1 does not leak that setting into the hook under test.
COMMON_ENV=(
  env
  PATH="$HARNESS/bin:$PATH"
  TMPDIR="$HARNESS/tmp"
  FAKE_REMOTE_OID="$REMOTE_SHA"
  FAKE_TIMEOUT_LOG="$HARNESS/timeouts"
  FAKE_CARGO_LOG="$HARNESS/cargo"
  CI=
  GITLAB_CI=
  GITHUB_ACTIONS=
  SKIP_TESTS=
  SKIP_CLIPPY=
  SKIP_SCRIPT_TESTS=1
  TEST_CRATES=
  TEST_RUNNER=
  TEST_TIMEOUT=
  VERBOSE=
)

reset_logs() {
  : >"$HARNESS/timeouts"
  : >"$HARNESS/cargo"
}

assert_no_temp_left() {
  if find "$HARNESS/tmp" -mindepth 1 -print -quit | grep -q .; then
    fail "$1 left temporary results"
  fi
}

# --- Default run: clippy then the whole workspace under cargo test ----------
reset_logs
printf '%s\n' "$PUSH_INPUT" |
  "${COMMON_ENV[@]}" bash "$HOOK" origin fake >"$HARNESS/normal.out" 2>&1 ||
  { cat "$HARNESS/normal.out" >&2; fail 'default run should pass'; }
grep -q '^1800 cargo clippy --workspace --all-features --all-targets -- -D warnings$' "$HARNESS/timeouts" ||
  fail 'clippy must run with the CI flags and the default timeout'
grep -q '^1800 cargo test --workspace$' "$HARNESS/timeouts" || fail 'default run must test the workspace'
grep -Eq '\[test\].*PASS.*\([0-9]+s\)' "$HARNESS/normal.out" || fail 'test step must report PASS'
grep -q 'All checks passed! Proceeding with push.' "$HARNESS/normal.out" || fail 'missing success summary'
assert_no_temp_left 'normal hook exit'

# --- Selected crates and a custom timeout ------------------------------------
reset_logs
printf '%s\n' "$PUSH_INPUT" |
  "${COMMON_ENV[@]}" TEST_CRATES='rholang-vm rholang-parser' TEST_TIMEOUT=60 SKIP_CLIPPY=1 \
    bash "$HOOK" origin fake >"$HARNESS/crates.out" 2>&1
grep -q '^60 cargo test -p rholang-vm$' "$HARNESS/timeouts" || fail 'TEST_CRATES must test rholang-vm'
grep -q '^60 cargo test -p rholang-parser$' "$HARNESS/timeouts" || fail 'TEST_CRATES must test rholang-parser'
if grep -q 'clippy' "$HARNESS/cargo"; then fail 'SKIP_CLIPPY=1 still ran clippy'; fi
grep -q 'SKIP_CLIPPY=1' "$HARNESS/crates.out" || fail 'skipped clippy must be reported'

# --- nextest runner ----------------------------------------------------------
reset_logs
printf '%s\n' "$PUSH_INPUT" |
  "${COMMON_ENV[@]}" TEST_RUNNER=nextest SKIP_CLIPPY=1 bash "$HOOK" origin fake >"$HARNESS/nextest.out" 2>&1
grep -q '^1800 cargo nextest run --no-tests=pass --workspace$' "$HARNESS/timeouts" ||
  fail 'TEST_RUNNER=nextest must use cargo nextest'

set +e
printf '%s\n' "$PUSH_INPUT" |
  "${COMMON_ENV[@]}" TEST_RUNNER=nextest MISSING_NEXTEST=1 bash "$HOOK" origin fake >"$HARNESS/missing.out" 2>&1
MISSING_RC=$?
printf '%s\n' "$PUSH_INPUT" |
  "${COMMON_ENV[@]}" TEST_RUNNER=bogus bash "$HOOK" origin fake >"$HARNESS/bogus.out" 2>&1
BOGUS_RC=$?
set -e
[[ "$MISSING_RC" -eq 1 ]] || fail 'missing cargo-nextest must abort the push'
grep -q 'cargo install cargo-nextest --locked' "$HARNESS/missing.out" || fail 'missing nextest must print install hint'
[[ "$BOGUS_RC" -eq 1 ]] || fail 'invalid TEST_RUNNER must abort the push'

# --- A failing step blocks the push ------------------------------------------
reset_logs
set +e
printf '%s\n' "$PUSH_INPUT" |
  "${COMMON_ENV[@]}" FAIL_CLIPPY=1 bash "$HOOK" origin fake >"$HARNESS/clippy-fail.out" 2>&1
FAIL_RC=$?
set -e
[[ "$FAIL_RC" -eq 1 ]] || fail 'a clippy failure must abort the push'
grep -q 'fake clippy failure' "$HARNESS/clippy-fail.out" || fail 'failure details must be shown'
grep -q 'Push aborted' "$HARNESS/clippy-fail.out" || fail 'missing failure summary'
grep -q ' test --workspace$' "$HARNESS/cargo" || fail 'tests must still run after a clippy failure'
assert_no_temp_left 'failed hook exit'

# --- Nothing to push: no checks run ------------------------------------------
reset_logs
printf 'refs/heads/test %s refs/heads/test %s\n' "$HEAD_SHA" "$HEAD_SHA" |
  "${COMMON_ENV[@]}" bash "$HOOK" origin fake >"$HARNESS/noop.out" 2>&1
grep -q 'No new commits to push' "$HARNESS/noop.out" || fail 'unchanged refs must skip checks'
[[ ! -s "$HARNESS/cargo" ]] || fail 'unchanged refs must not run cargo'

# --- Racing push already landed: abort without running checks ---------------
reset_logs
set +e
printf '%s\n' "$PUSH_INPUT" |
  "${COMMON_ENV[@]}" FAKE_REMOTE_OID="$HEAD_SHA" bash "$HOOK" origin fake >"$HARNESS/race.out" 2>&1
RACE_RC=$?
set -e
[[ "$RACE_RC" -eq 1 ]] || fail 'a ref already on the remote must stop the push'
grep -q 'Already up to date with remote' "$HARNESS/race.out" || fail 'race guard message missing'
[[ ! -s "$HARNESS/cargo" ]] || fail 'race guard must not run cargo'

# --- CI environments skip the hook --------------------------------------------
reset_logs
printf '%s\n' "$PUSH_INPUT" |
  "${COMMON_ENV[@]}" GITHUB_ACTIONS=true bash "$HOOK" origin fake >"$HARNESS/ci.out" 2>&1
grep -q 'Skipping pre-push hooks in CI environment' "$HARNESS/ci.out" || fail 'CI must skip the hook'
[[ ! -s "$HARNESS/cargo" ]] || fail 'CI skip must not run cargo'

# --- Cancellation stops the running cargo process tree ------------------------
reset_logs
printf '%s\n' "$PUSH_INPUT" |
  "${COMMON_ENV[@]}" SKIP_CLIPPY=1 SLEEP_TEST=1 FAKE_TEST_STARTED="$HARNESS/started" \
    bash "$HOOK" origin fake >"$HARNESS/cancel.out" 2>&1 &
HOOK_PID=$!
for _ in $(seq 1 100); do
  [[ -f "$HARNESS/started" ]] && break
  sleep 0.05
done
[[ -f "$HARNESS/started" ]] || fail 'fake test never started'
kill -TERM "$HOOK_PID"
set +e
wait "$HOOK_PID"
RC=$?
set -e
[[ "$RC" -eq 143 ]] || fail "cancelled hook must exit 143, got $RC"
sleep 0.2
while read -r pid _; do
  if kill -0 "$pid" 2>/dev/null; then
    fail "hook cancellation left cargo process $pid"
  fi
done <"$HARNESS/cargo"
assert_no_temp_left 'hook cancellation'

printf 'pre-push hook tests passed\n'
