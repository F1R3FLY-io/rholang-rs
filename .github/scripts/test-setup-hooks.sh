#!/usr/bin/env bash
# Behavioral tests for scripts/setup-hooks.sh, run against a scratch clone so
# the developer's own hook configuration is never touched.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail() {
  printf 'setup-hooks test failed: %s\n' "$1" >&2
  exit 1
}

REPO="$TMP/repo"
git init -q "$REPO"
mkdir -p "$REPO/scripts"
cp -R "$ROOT/.githooks" "$REPO/.githooks"
cp "$ROOT/scripts/setup-hooks.sh" "$REPO/scripts/setup-hooks.sh"
chmod -x "$REPO/.githooks/pre-commit"
SETUP="$REPO/scripts/setup-hooks.sh"
HOOKS_DIR="$REPO/.git/hooks"
mkdir -p "$HOOKS_DIR"

# A local hook that core.hooksPath would shadow, and an executable helper
# script that is not a hook (ctags setups call one from their post-* hooks).
printf '#!/bin/sh\nexit 0\n' >"$HOOKS_DIR/post-checkout"
printf '#!/bin/sh\nexit 0\n' >"$HOOKS_DIR/ctags"
chmod +x "$HOOKS_DIR/post-checkout" "$HOOKS_DIR/ctags"

in_repo() { (cd "$REPO" && "$@"); }

# --- Default: core.hooksPath -------------------------------------------------
in_repo bash "$SETUP" >"$TMP/install.out" 2>&1
[[ "$(git -C "$REPO" config --local core.hooksPath)" == .githooks ]] || fail 'core.hooksPath must point at .githooks'
[[ -x "$REPO/.githooks/pre-commit" ]] || fail 'install must make the hooks executable'
grep -q -- '- post-checkout$' "$TMP/install.out" || fail 'install must warn about shadowed .git/hooks entries'
if grep -q -- '- ctags$' "$TMP/install.out"; then fail 'install must not list helper scripts that are not git hooks'; fi
in_repo bash "$SETUP" --status >"$TMP/status.out" 2>&1
grep -q 'core.hooksPath: .*\.githooks' "$TMP/status.out" || fail 'status must report core.hooksPath'

# --- --copy: hooks in .git/hooks, core.hooksPath removed ----------------------
in_repo bash "$SETUP" --copy >/dev/null 2>&1
if git -C "$REPO" config --local core.hooksPath >/dev/null; then fail '--copy must unset core.hooksPath'; fi
for hook in pre-commit pre-push commit-msg; do
  [[ -x "$HOOKS_DIR/$hook" ]] || fail "--copy must install $hook"
  cmp -s "$REPO/.githooks/$hook" "$HOOKS_DIR/$hook" || fail "--copy must copy $hook unchanged"
done
[[ -x "$HOOKS_DIR/post-checkout" ]] || fail '--copy must keep other .git/hooks entries'

# --- --remove: only the copied hooks go -------------------------------------
printf '# local edit\n' >>"$HOOKS_DIR/pre-push"
in_repo bash "$SETUP" --remove >/dev/null 2>&1
[[ ! -e "$HOOKS_DIR/pre-commit" ]] || fail '--remove must delete copied hooks'
[[ -e "$HOOKS_DIR/pre-push" ]] || fail '--remove must keep a hook that differs from .githooks'
[[ -x "$HOOKS_DIR/post-checkout" ]] || fail '--remove must keep other .git/hooks entries'

# --- Unknown option -----------------------------------------------------------
if in_repo bash "$SETUP" --bogus >/dev/null 2>&1; then fail 'an unknown option must fail'; fi

printf 'setup-hooks tests passed\n'
