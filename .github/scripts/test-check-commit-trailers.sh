#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TOOL="$ROOT/.github/scripts/check-commit-trailers.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

expect_failure() {
	local label="$1"
	shift
	if "$@" >"$TMP/out" 2>"$TMP/err"; then
		printf 'expected failure: %s\n' "$label" >&2
		exit 1
	fi
}

message() {
	printf 'feat(node): add a feature\n\nBody text.\n'
	[ "$#" -eq 0 ] || printf '\n%s\n' "$@"
}

message >"$TMP/clean.txt"
message 'Co-authored-by: Jane Smith <Jane@Example.com>' >"$TMP/ratified-coauthor.txt"
message 'Co-authored-by: Unknown Person <unknown@example.com>' >"$TMP/unratified-coauthor.txt"
message 'Co-Authored-By: Claude <noreply@anthropic.com>' >"$TMP/claude.txt"
message 'co-authored-by: Claude Fable 5.1 <NOREPLY@ANTHROPIC.COM>' >"$TMP/case.txt"
message 'Claude-Session: https://claude.ai/code/session_example' >"$TMP/session.txt"
message '# Co-Authored-By: Claude <noreply@anthropic.com>' >"$TMP/comment.txt"
printf 'docs: mention that Co-authored-by: lines name co-authors\n' >"$TMP/subject.txt"

# --- Fixture history: the base branch holds the ratified authors ------------
# test@example.com and jane@example.com authored merged commits. Claude only
# appears as a co-author trailer, which never ratifies an identity.
REPO="$TMP/repo"
git init -q "$REPO"
git -C "$REPO" config user.name 'Trailer Test'
git -C "$REPO" config user.email 'test@example.com'
commit() { git -C "$REPO" commit -q --no-verify --allow-empty -F "$1"; }
in_repo() { (cd "$REPO" && "$@"); }
commit "$TMP/clean.txt"
GIT_AUTHOR_NAME='Jane Smith' GIT_AUTHOR_EMAIL='jane@example.com' commit "$TMP/clean.txt"
commit "$TMP/claude.txt"
BASE="$(git -C "$REPO" rev-parse HEAD)"

# --- Single message (commit-msg hook): ratified from the local history -------
in_repo "$TOOL" message "$TMP/clean.txt"
in_repo "$TOOL" message "$TMP/ratified-coauthor.txt"
expect_failure 'unratified co-author' in_repo "$TOOL" message "$TMP/unratified-coauthor.txt"
grep -q 'Unknown Person' "$TMP/err"
expect_failure 'AI co-author that only appears in trailers' in_repo "$TOOL" message "$TMP/claude.txt"
expect_failure 'trailer in another letter case' in_repo "$TOOL" message "$TMP/case.txt"
expect_failure 'session link trailer' in_repo "$TOOL" message "$TMP/session.txt"
in_repo "$TOOL" message "$TMP/comment.txt"
in_repo "$TOOL" message "$TMP/subject.txt"
# The local hook checks co-authors only: a first-time contributor has no
# merged commit yet, and CI decides author ratification.
env GIT_AUTHOR_NAME='Stranger' GIT_AUTHOR_EMAIL='stranger@example.com' \
	bash -c 'cd "$1" && "$2" message "$3"' _ "$REPO" "$TOOL" "$TMP/clean.txt"
expect_failure 'first-time contributor with an unratified co-author' \
	env GIT_AUTHOR_NAME='Stranger' GIT_AUTHOR_EMAIL='stranger@example.com' \
	bash -c 'cd "$1" && "$2" message "$3"' _ "$REPO" "$TOOL" "$TMP/unratified-coauthor.txt"

# The checker must run under bash 3.2, the macOS system bash, because the
# commit-msg hook runs on contributor machines.
if [ -x /bin/bash ] && [ "$(/bin/bash -c 'echo ${BASH_VERSINFO[0]}')" -lt 4 ]; then
	(cd "$REPO" && /bin/bash "$TOOL" message "$TMP/ratified-coauthor.txt")
	expect_failure 'bash 3 with an unratified co-author' \
		bash -c 'cd "$1" && /bin/bash "$2" message "$3"' _ "$REPO" "$TOOL" "$TMP/unratified-coauthor.txt"
fi
if grep -q -E 'declare -A|,,\}|\^\^\}' "$TOOL"; then
	printf 'the checker must avoid bash 4 features (associative arrays, case expansion)\n' >&2
	exit 1
fi

# The first commit of a new repository has no history to ratify from, so
# only the Claude-Session rule applies.
EMPTY="$TMP/empty"
git init -q "$EMPTY"
(cd "$EMPTY" && "$TOOL" message "$TMP/unratified-coauthor.txt")
expect_failure 'session trailer in a new repository' bash -c 'cd "$1" && "$2" message "$3"' _ "$EMPTY" "$TOOL" "$TMP/session.txt"

# --- commit-msg hook ----------------------------------------------------------
HOOK="$ROOT/.githooks/commit-msg"
in_repo "$HOOK" "$TMP/clean.txt"
expect_failure 'commit-msg hook with an AI co-author trailer' in_repo "$HOOK" "$TMP/claude.txt"
expect_failure 'commit through the hook with an AI co-author trailer' \
	git -C "$REPO" -c core.hooksPath="$ROOT/.githooks" commit -q --allow-empty -F "$TMP/claude.txt"

# --- Commit range (CI): ratified from the base commit ------------------------
commit "$TMP/ratified-coauthor.txt"
commit "$TMP/clean.txt"
"$TOOL" range "$BASE..HEAD" --repo "$REPO" --ratified-from "$BASE"
commit "$TMP/claude.txt"
BAD="$(git -C "$REPO" rev-parse --short HEAD)"
expect_failure 'range with an AI co-author trailer' "$TOOL" range "$BASE..HEAD" --repo "$REPO" --ratified-from "$BASE"
grep -q "$BAD" "$TMP/err"
STRANGER_BASE="$(git -C "$REPO" rev-parse HEAD)"
GIT_AUTHOR_NAME='Stranger' GIT_AUTHOR_EMAIL='stranger@example.com' commit "$TMP/clean.txt"
expect_failure 'range with an unratified author' \
	"$TOOL" range "$STRANGER_BASE..HEAD" --repo "$REPO" --ratified-from "$STRANGER_BASE"
grep -q 'Stranger' "$TMP/err"
# A pull request cannot ratify itself: an author that first appears inside
# the range is not ratified by the base commit.
"$TOOL" range "$STRANGER_BASE..HEAD" --repo "$REPO" --ratified-from HEAD
expect_failure 'unknown range' "$TOOL" range "no-such-ref..HEAD" --repo "$REPO" --ratified-from "$BASE"
expect_failure 'unknown ratification base' "$TOOL" range "$BASE..HEAD" --repo "$REPO" --ratified-from no-such-ref

# An outside contributor's own commits in their own pull request pass: CI
# names those commits in an exempt-author file. Their co-authors are still
# checked, and a commit by another identity is not exempt.
git -C "$REPO" rev-parse HEAD >"$TMP/exempt.txt"
"$TOOL" range "$STRANGER_BASE..HEAD" --repo "$REPO" --ratified-from "$STRANGER_BASE" --exempt-authors "$TMP/exempt.txt"
OUTSIDE_BASE="$(git -C "$REPO" rev-parse HEAD)"
GIT_AUTHOR_NAME='Stranger' GIT_AUTHOR_EMAIL='stranger@example.com' commit "$TMP/unratified-coauthor.txt"
git -C "$REPO" rev-parse HEAD >"$TMP/exempt.txt"
expect_failure 'exempt author with an unratified co-author' \
	"$TOOL" range "$OUTSIDE_BASE..HEAD" --repo "$REPO" --ratified-from "$OUTSIDE_BASE" --exempt-authors "$TMP/exempt.txt"
grep -q 'Unknown Person' "$TMP/err"
if grep -q 'Stranger' "$TMP/err"; then
	printf 'the exempt author was reported\n' >&2
	exit 1
fi
GIT_AUTHOR_NAME='Claude' GIT_AUTHOR_EMAIL='noreply@anthropic.com' commit "$TMP/clean.txt"
: >"$TMP/exempt-none.txt"
expect_failure 'AI-authored commit that is not the pull request author' \
	"$TOOL" range "HEAD~1..HEAD" --repo "$REPO" --ratified-from "$OUTSIDE_BASE" --exempt-authors "$TMP/exempt-none.txt"
if grep -i -q 'anthropic' "$TOOL"; then
	printf 'the check must not name a vendor; the history and the denied list decide\n' >&2
	exit 1
fi

# --- Denied identities: rejected even when history ratifies them -------------
# The fixture history now holds a commit authored by noreply@anthropic.com,
# so history alone would ratify that address. The denied list overrides it.
DENIED="$TMP/denied.txt"
printf '%s\n' '# denied test identities' '@anthropic.com' 'blocked@example.com' >"$DENIED"
DENY_BASE="$(git -C "$REPO" rev-parse HEAD)"
"$TOOL" range "HEAD~1..HEAD" --repo "$REPO" --ratified-from "$DENY_BASE"
expect_failure 'denied author ratified by history' \
	"$TOOL" range "HEAD~1..HEAD" --repo "$REPO" --ratified-from "$DENY_BASE" --denied "$DENIED"
grep -q 'denied identity: Claude <noreply@anthropic.com>' "$TMP/err"
git -C "$REPO" rev-parse HEAD >"$TMP/exempt-denied.txt"
expect_failure 'denied author in an exempt pull request commit' \
	"$TOOL" range "HEAD~1..HEAD" --repo "$REPO" --ratified-from "$DENY_BASE" --denied "$DENIED" \
	--exempt-authors "$TMP/exempt-denied.txt"
commit "$TMP/case.txt"
expect_failure 'denied co-author in another letter case' \
	"$TOOL" range "HEAD~1..HEAD" --repo "$REPO" --ratified-from "$DENY_BASE" --denied "$DENIED"
message 'Co-authored-by: Blocked <Blocked@Example.com>' >"$TMP/blocked.txt"
commit "$TMP/blocked.txt"
expect_failure 'denied exact address' \
	"$TOOL" range "HEAD~1..HEAD" --repo "$REPO" --ratified-from "$DENY_BASE" --denied "$DENIED"
expect_failure 'missing denied list' \
	"$TOOL" range "HEAD~1..HEAD" --repo "$REPO" --ratified-from "$DENY_BASE" --denied "$TMP/absent.txt"
# The hook checks a denied author too, although it does not check author
# ratification.
expect_failure 'hook with a denied author' \
	env GIT_AUTHOR_NAME='Claude' GIT_AUTHOR_EMAIL='noreply@anthropic.com' \
	bash -c 'cd "$1" && "$2" message "$3" --denied "$4"' _ "$REPO" "$TOOL" "$TMP/clean.txt" "$DENIED"
(cd "$REPO" && "$TOOL" message "$TMP/clean.txt" --denied "$DENIED")

# The committed denied list blocks the AI identity that put an account in the
# contributors graph. The hook and CI read it by default.
COMMITTED_DENIED="$ROOT/.github/denied-identities.txt"
grep -Fxq '@anthropic.com' "$COMMITTED_DENIED" ||
	{ printf 'the committed denied list must block @anthropic.com\n' >&2; exit 1; }
if grep -v -E '^(#.*|@[a-z0-9.-]+|[^@[:space:]]+@[a-z0-9.-]+)?$' "$COMMITTED_DENIED" | grep -q .; then
	printf 'every denied line must be a lowercase @domain or address\n' >&2
	exit 1
fi
expect_failure 'hook with the committed denied list' \
	env GIT_AUTHOR_NAME='Claude' GIT_AUTHOR_EMAIL='noreply@anthropic.com' \
	bash -c 'cd "$1" && "$2" "$3"' _ "$REPO" "$HOOK" "$TMP/clean.txt"

printf 'commit trailer tests passed\n'
