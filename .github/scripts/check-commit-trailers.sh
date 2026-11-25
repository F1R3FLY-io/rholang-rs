#!/usr/bin/env bash
set -euo pipefail

# Commit identity check (CLAUDE.md "Commit Messages").
#
# Every commit author and every Co-authored-by trailer must be a ratified
# identity: an email that already authored a commit in the base history.
# Each commit on the base branch arrived through an accepted pull request,
# so its author is ratified by that process. A trailer never ratifies an
# identity. GitHub lists each co-author email that maps to an account in
# the contributors graph, so an unratified identity, human or AI, must not
# enter the history. Claude-Session trailers are always rejected: they are
# not identities, and they publish assistant session links.
#
#   message FILE
#       one message (commit-msg hook): co-authors are checked against the
#       local HEAD history; CI decides author ratification
#   range RANGE --ratified-from REV [--repo DIR] [--exempt-authors FILE]
#       every commit in a range, ratified from REV history (CI)
#
# CI passes the pull request's base commit as REV, so a pull request cannot
# ratify its own identities. CI also passes the commits that GitHub
# attributes to the pull request author: an outside contributor's own
# commits pass, and maintainer review decides their acceptance. Co-authors
# of those commits are still checked.
#
# The script runs under bash 3.2 (the macOS system bash), because the
# commit-msg hook runs on contributor machines.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO=.
RATIFIED_FROM=()
EXEMPT_FILE=""
DENIED_FILE=""
DENIED_SET=0
FORBIDDEN='^claude-session:'
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
RATIFIED="$WORK/ratified"
CHECK_IDENTITIES=0

fail() {
	printf 'commit identity check error: %s\n' "$1" >&2
	exit 2
}

lower() {
	tr '[:upper:]' '[:lower:]'
}

load_ratified() {
	git -C "$REPO" log --format=%ae "$@" -- | lower | sort -u >"$RATIFIED"
	CHECK_IDENTITIES=1
}

is_exempt() {
	[ -n "$EXEMPT_FILE" ] && grep -Fxq "$1" "$EXEMPT_FILE"
}

# A denied line is an exact address or an @domain suffix. A denied identity
# is rejected even when the history ratifies it.
is_denied() {
	local email="$1" pattern
	[ -n "$DENIED_FILE" ] || return 1
	while IFS= read -r pattern || [ -n "$pattern" ]; do
		pattern="$(printf %s "$pattern" | lower)"
		case "$pattern" in
		'' | \#*) continue ;;
		@*) [[ "$email" == *"$pattern" ]] && return 0 ;;
		*) [ "$email" = "$pattern" ] && return 0 ;;
		esac
	done <"$DENIED_FILE"
	return 1
}

# Prints one problem per line for a message, its author, and its
# co-authors. CHECK_AUTHOR=0 skips author ratification but still checks the
# denied list. Git strips comment lines from a message before it records
# the commit.
problems() {
	local text="$1" author="$2" check_author="$3" kind ident email
	text="$(grep -v '^#' <<<"$text" || true)"
	grep -i -E "$FORBIDDEN" <<<"$text" | sed 's/^/forbidden trailer: /' || true
	while IFS=$'\t' read -r kind ident; do
		[ -n "$ident" ] || continue
		email="${ident##*<}"
		email="$(printf %s "${email%%>*}" | lower)"
		if is_denied "$email"; then
			printf 'denied identity: %s\n' "$ident"
		elif [ "$CHECK_IDENTITIES" -eq 1 ] && { [ "$kind" = C ] || [ "$check_author" -eq 1 ]; } &&
			! grep -Fxq -- "$email" "$RATIFIED"; then
			printf 'unratified identity: %s\n' "$ident"
		fi
	done < <(
		printf 'A\t%s\n' "$author"
		grep -i -E '^co-authored-by:' <<<"$text" | sed -E 's/^[^:]*:[[:space:]]*/C	/' || true
	)
}

report() {
	printf '%s:\n' "$1" >&2
	sed 's/^/  /' <<<"$2" >&2
}

check_message() {
	local file="$1" author found
	[ -f "$file" ] || fail "missing message file: $file"
	if git -C "$REPO" rev-parse --verify --quiet HEAD >/dev/null; then
		load_ratified HEAD
	fi
	author="$(git -C "$REPO" var GIT_AUTHOR_IDENT 2>/dev/null | sed -E 's/>.*/>/' || true)"
	found="$(problems "$(cat "$file")" "$author" 0)"
	[ -z "$found" ] && return 0
	report "The commit message" "$found"
	printf 'Remove forbidden trailers. A co-author is ratified when a maintainer merges a pull request that they authored.\n' >&2
	return 1
}

check_range() {
	local range="$1" commits sha author check_author found revision status=0
	[ "${#RATIFIED_FROM[@]}" -gt 0 ] || usage
	for revision in "${RATIFIED_FROM[@]}"; do
		git -C "$REPO" rev-parse --verify --quiet "${revision}^{commit}" >/dev/null ||
			fail "cannot resolve the ratification base $revision"
	done
	load_ratified "${RATIFIED_FROM[@]}"
	commits="$(git -C "$REPO" rev-list "$range")" || fail "cannot list commits in $range"
	for sha in $commits; do
		author="$(git -C "$REPO" log -1 --format='%an <%ae>' "$sha")"
		check_author=1
		if is_exempt "$sha"; then
			check_author=0
		fi
		found="$(problems "$(git -C "$REPO" log -1 --format=%B "$sha")" "$author" "$check_author")"
		[ -z "$found" ] && continue
		report "Commit $(git -C "$REPO" rev-parse --short "$sha")" "$found"
		status=1
	done
	if [ "$status" -ne 0 ]; then
		printf 'A commit author or co-author is ratified when a maintainer merges a pull request that they authored.\n' >&2
	fi
	return "$status"
}

usage() {
	printf '%s\n' "usage: $0 message FILE [--denied FILE]" \
		"       $0 range RANGE --ratified-from REV [--ratified-from REV] [--repo DIR] [--exempt-authors FILE] [--denied FILE]" >&2
	exit 2
}

[ "$#" -ge 2 ] || usage
command="$1"
target="$2"
shift 2
while [ "$#" -gt 0 ]; do
	case "$1" in
	--ratified-from) [ "$#" -ge 2 ] || usage; RATIFIED_FROM+=("$2"); shift 2 ;;
	--repo) [ "$#" -ge 2 ] || usage; REPO="$2"; shift 2 ;;
	--exempt-authors)
		[ "$#" -ge 2 ] || usage
		[ -f "$2" ] || fail "missing exempt-author file: $2"
		EXEMPT_FILE="$2"
		shift 2
		;;
	--denied)
		[ "$#" -ge 2 ] || usage
		[ -f "$2" ] || fail "missing denied-identity file: $2"
		DENIED_FILE="$2"
		DENIED_SET=1
		shift 2
		;;
	*) usage ;;
	esac
done
# The commit-msg hook reads the committed denied list. CI passes the list
# from the base commit explicitly.
if [ "$command" = message ] && [ "$DENIED_SET" -eq 0 ] && [ -f "$SCRIPT_DIR/../denied-identities.txt" ]; then
	DENIED_FILE="$SCRIPT_DIR/../denied-identities.txt"
fi
case "$command" in
message) check_message "$target" ;;
range) check_range "$target" ;;
*) usage ;;
esac
