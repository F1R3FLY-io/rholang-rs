#!/usr/bin/env bash
set -euo pipefail

# check_golden_snapshot_diff.sh
# Golden-snapshot gate for the Reified RSpaces parser change.
#
# Compares every golden snapshot of rholang-parser with its version at the
# baseline revision (02cef80 unless given). The only allowed differences are:
#   - added AST lines `params: None,`, `query_spec: None,` and `space: None,`
#     (the new optional fields of sends, linear binds and `new` binders);
#   - the insta metadata line `assertion_line: N` in the snapshot header.
# A snapshot that does not exist at the baseline must be one of the new
# Reified RSpaces snapshots listed in NEW_SNAPSHOTS, with its corpus file.
#
# Usage:
#   ./scripts/check_golden_snapshot_diff.sh [BASELINE]
#   ./scripts/check_golden_snapshot_diff.sh --self-test [BASELINE]
#
# --self-test also runs negative controls: a forbidden body change, a removed
# line, a header change and an unlisted new snapshot must each be rejected.
# Scratch files go under target/tmp (never /tmp).

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
SNAPSHOT_REL=rholang-parser/tests/corpus/golden_snapshots
CORPUS_DIR="$ROOT_DIR/rholang-parser/tests/corpus"
NEW_SNAPSHOTS=(
  golden__reified_rspaces.snap
  golden__reified_query_forms.snap
  golden__reified_bare_floats.snap
  golden__reified_parse_errors.snap
  golden__use_free_keywords_as_identifiers.snap
)

SELF_TEST=false
if [[ "${1:-}" == "--self-test" ]]; then
  SELF_TEST=true
  shift
fi
BASELINE=${1:-02cef80}

SCRATCH_ROOT="$ROOT_DIR/target/tmp"
mkdir -p "$SCRATCH_ROOT"
SCRATCH="$(mktemp -d "$SCRATCH_ROOT/golden-diff.XXXXXX")"
trap 'rm -rf "$SCRATCH"' EXIT

# The insta header: the lines between the first and the second `---`.
header() { awk 'NR == 1 && $0 == "---" { inside = 1; next } inside && $0 == "---" { exit } inside { print }' "$1"; }
# The snapshot body: everything after the second `---`.
body() { awk 'separators >= 2 { print; next } $0 == "---" { separators++ }' "$1"; }

# check_dir DIR: checks every snapshot in DIR against the baseline.
# Prints the violations and a summary; returns 1 when a violation exists.
check_dir() {
  local dir=$1 violations=0 compared=0 unchanged=0 metadata=0
  local params=0 query_spec=0 space=0
  local base="$SCRATCH/base.snap" path name changes forbidden

  while IFS= read -r path; do
    name=${path##*/}
    [[ "$name" == *.snap ]] || continue
    compared=$((compared + 1))
    if [[ ! -f "$dir/$name" ]]; then
      echo "VIOLATION $name: removed (it exists at $BASELINE)"
      violations=$((violations + 1))
      continue
    fi
    git -C "$ROOT_DIR" show "$BASELINE:$path" > "$base"
    if cmp -s "$base" "$dir/$name"; then
      unchanged=$((unchanged + 1))
      continue
    fi
    if ! diff -q <(header "$base" | grep -v '^assertion_line: ') \
                 <(header "$dir/$name" | grep -v '^assertion_line: ') > /dev/null; then
      echo "VIOLATION $name: header changed beyond assertion_line"
      violations=$((violations + 1))
    elif ! diff -q <(header "$base") <(header "$dir/$name") > /dev/null; then
      metadata=$((metadata + 1))
    fi
    changes="$(diff <(body "$base") <(body "$dir/$name") | grep -E '^[<>]' || true)"
    forbidden="$(printf '%s\n' "$changes" | grep -Ev '^> +(params|query_spec|space): None,$' | grep -v '^$' || true)"
    if [[ -n "$forbidden" ]]; then
      echo "VIOLATION $name: body changed beyond the new None fields:"
      printf '%s\n' "$forbidden" | head -20 | sed 's/^/    /'
      violations=$((violations + 1))
    fi
    params=$((params + $(printf '%s\n' "$changes" | grep -cE '^> +params: None,$' || true)))
    query_spec=$((query_spec + $(printf '%s\n' "$changes" | grep -cE '^> +query_spec: None,$' || true)))
    space=$((space + $(printf '%s\n' "$changes" | grep -cE '^> +space: None,$' || true)))
  done < <(git -C "$ROOT_DIR" ls-tree --name-only "$BASELINE" "$SNAPSHOT_REL/")

  local new=0 file stem
  for file in "$dir"/*; do
    name=${file##*/}
    if [[ "$name" != *.snap ]]; then
      echo "VIOLATION $name: not a snapshot (a pending insta review?)"
      violations=$((violations + 1))
      continue
    fi
    git -C "$ROOT_DIR" cat-file -e "$BASELINE:$SNAPSHOT_REL/$name" 2> /dev/null && continue
    new=$((new + 1))
    if [[ " ${NEW_SNAPSHOTS[*]} " != *" $name "* ]]; then
      echo "VIOLATION $name: new snapshot that is not a listed Reified RSpaces snapshot"
      violations=$((violations + 1))
      continue
    fi
    stem=${name#golden__}
    stem=${stem%.snap}
    if [[ ! -f "$CORPUS_DIR/$stem.rho" ]]; then
      echo "VIOLATION $name: corpus file $stem.rho is missing"
      violations=$((violations + 1))
    fi
  done

  echo "baseline $BASELINE: $compared snapshots compared, $unchanged unchanged, $metadata with assertion_line metadata changes"
  echo "added lines: params: None = $params, query_spec: None = $query_spec, space: None = $space"
  echo "new snapshots: $new (allowed: ${#NEW_SNAPSHOTS[@]})"
  if (( violations > 0 )); then
    echo "golden-diff check: FAIL ($violations violations)"
    return 1
  fi
  echo "golden-diff check: PASS"
}

if ! $SELF_TEST; then
  check_dir "$ROOT_DIR/$SNAPSHOT_REL"
  exit $?
fi

# Self-test: the real snapshots pass, and every injected violation fails.
echo "== self-test: worktree snapshots"
check_dir "$ROOT_DIR/$SNAPSHOT_REL"

run_negative_control() {
  local label=$1 mutate=$2 dir="$SCRATCH/controls/$1"
  mkdir -p "$dir"
  cp "$ROOT_DIR/$SNAPSHOT_REL"/*.snap "$dir"/
  eval "$mutate"
  if check_dir "$dir" > "$SCRATCH/$label.log"; then
    echo "self-test: FAIL (negative control '$label' was accepted)"
    exit 1
  fi
  echo "self-test: negative control '$label' rejected: $(grep -m1 VIOLATION "$SCRATCH/$label.log")"
}

run_negative_control body-change \
  'sed -i "0,/col: 5,/s//col: 6,/" "$dir/golden__var_ref.snap"'
run_negative_control removed-line \
  'sed -i "0,/uri: None,/{/uri: None,/d}" "$dir/golden__var_ref.snap"'
run_negative_control header-change \
  'sed -i "s/^expression: result$/expression: changed/" "$dir/golden__var_ref.snap"'
run_negative_control foreign-field \
  'sed -i "0,/space: None,/s//space_type: None,/" "$dir/golden__var_ref.snap"'
run_negative_control unlisted-snapshot \
  'cp "$dir/golden__var_ref.snap" "$dir/golden__unlisted.snap"'
echo "self-test: PASS"
