#!/usr/bin/env bash
#
# setup-hooks.sh - Install git hooks for this repository
#
# Two installation methods are supported:
#   1. core.hooksPath (recommended) - points git directly at .githooks/
#   2. copy - copies hooks to .git/hooks/ (keeps other hooks in .git/hooks/)
#
# Usage:
#   ./scripts/setup-hooks.sh           # Uses core.hooksPath (default)
#   ./scripts/setup-hooks.sh --copy    # Copies to .git/hooks/
#   ./scripts/setup-hooks.sh --status  # Show current hook configuration
#   ./scripts/setup-hooks.sh --remove  # Remove hook configuration
#

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# Get repository root
REPO_ROOT="$(git rev-parse --show-toplevel)"
HOOKS_SOURCE="$REPO_ROOT/.githooks"
# The common dir, not --git-path hooks: that one resolves to core.hooksPath
# once it is set. Absolute so the script works from any subdirectory.
HOOKS_TARGET="$(git rev-parse --path-format=absolute --git-common-dir)/hooks"

print_hook_summary() {
    echo "  pre-commit: cargo fmt --check, cargo clippy"
    echo "  commit-msg: reject unratified co-authors and session trailers"
    echo "  pre-push:   cargo clippy, script tests, cargo test (full workspace)"
}

# Only names that git runs as hooks (githooks(5)) count. Other executables in
# .git/hooks/, such as a ctags helper that post-* hooks call, are not hooks.
is_git_hook_name() {
    case "$1" in
    applypatch-msg | pre-applypatch | post-applypatch | pre-commit | \
        pre-merge-commit | prepare-commit-msg | commit-msg | post-commit | \
        pre-rebase | post-checkout | post-merge | pre-push | pre-receive | \
        update | proc-receive | post-receive | post-update | reference-transaction | \
        push-to-checkout | pre-auto-gc | post-rewrite | sendemail-validate | \
        fsmonitor-watchman | p4-changelist | p4-prepare-changelist | \
        p4-post-changelist | p4-pre-submit | post-index-change)
        return 0
        ;;
    *) return 1 ;;
    esac
}

# Hooks in .git/hooks/ that core.hooksPath would stop running.
shadowed_local_hooks() {
    local hook name
    for hook in "$HOOKS_TARGET"/*; do
        [[ -f "$hook" && -x "$hook" ]] || continue
        name=$(basename "$hook")
        is_git_hook_name "$name" || continue
        [[ -f "$HOOKS_SOURCE/$name" ]] && continue
        echo "$name"
    done
}

show_status() {
    echo -e "${CYAN}Git Hooks Status${NC}"
    echo "================"

    local hooks_path
    hooks_path=$(git config --local core.hooksPath 2>/dev/null || echo "")

    if [[ -n "$hooks_path" ]]; then
        echo -e "core.hooksPath: ${GREEN}$hooks_path${NC}"
        echo "Method: core.hooksPath (git-native)"
    elif [[ -f "$HOOKS_TARGET/pre-push" ]]; then
        echo -e "Hooks installed: ${GREEN}$HOOKS_TARGET${NC}"
        echo "Method: copied to .git/hooks/"
    else
        echo -e "${YELLOW}No hooks configured${NC}"
        echo ""
        echo "Run: $0 to install hooks"
    fi

    echo ""
    echo "Available hooks in $HOOKS_SOURCE:"
    for hook in "$HOOKS_SOURCE"/*; do
        if [[ -f "$hook" ]]; then
            local name
            name=$(basename "$hook")
            if [[ -x "$hook" ]]; then
                echo -e "  - $name ${GREEN}(executable)${NC}"
            else
                echo -e "  - $name ${YELLOW}(not executable)${NC}"
            fi
        fi
    done

    echo ""
    echo "Hooks provide:"
    print_hook_summary
}

install_via_hookspath() {
    echo -e "${CYAN}Installing hooks via core.hooksPath...${NC}"

    # Set relative path from repo root
    git config --local core.hooksPath .githooks

    # Ensure hooks are executable
    chmod +x "$HOOKS_SOURCE"/*

    echo -e "${GREEN}Done!${NC} Git will now use hooks from .githooks/"
    echo ""
    echo "Hooks installed:"
    print_hook_summary
    echo ""
    echo "To verify: git config --local core.hooksPath"

    local shadowed
    shadowed="$(shadowed_local_hooks)"
    if [[ -n "$shadowed" ]]; then
        echo ""
        echo -e "${YELLOW}Warning:${NC} git no longer runs these hooks from .git/hooks/:"
        printf '%s\n' "$shadowed" | sed 's/^/  - /'
        echo "To keep them, run: $0 --copy"
    fi
}

install_via_copy() {
    echo -e "${CYAN}Installing hooks via copy to .git/hooks/...${NC}"

    # Remove core.hooksPath if set
    git config --local --unset core.hooksPath 2>/dev/null || true
    mkdir -p "$HOOKS_TARGET"

    # Copy each hook
    for hook in "$HOOKS_SOURCE"/*; do
        if [[ -f "$hook" ]]; then
            local name
            name=$(basename "$hook")
            # Only copy actual hook files (pre-commit, pre-push, etc.)
            if [[ "$name" != *.sh && "$name" != *.md && "$name" != *.sample ]]; then
                cp "$hook" "$HOOKS_TARGET/$name"
                chmod +x "$HOOKS_TARGET/$name"
                echo "  Copied: $name"
            fi
        fi
    done

    echo -e "${GREEN}Done!${NC} Hooks copied to .git/hooks/"
}

remove_hooks() {
    echo -e "${CYAN}Removing hook configuration...${NC}"

    # Remove core.hooksPath
    if git config --local --unset core.hooksPath 2>/dev/null; then
        echo "  Removed core.hooksPath"
    fi

    # Remove copied hooks (only those identical to the .githooks/ version)
    for hook in "$HOOKS_SOURCE"/*; do
        if [[ -f "$hook" ]]; then
            local name
            name=$(basename "$hook")
            if [[ -f "$HOOKS_TARGET/$name" && "$name" != *.sample ]] && cmp -s "$hook" "$HOOKS_TARGET/$name"; then
                rm "$HOOKS_TARGET/$name"
                echo "  Removed: $name"
            fi
        fi
    done

    echo -e "${GREEN}Done!${NC} Hook configuration removed"
}

# Parse arguments
case "${1:-}" in
--status)
    show_status
    ;;
--copy)
    install_via_copy
    ;;
--remove)
    remove_hooks
    ;;
--help | -h)
    echo "Usage: $0 [--copy|--status|--remove|--help]"
    echo ""
    echo "Options:"
    echo "  (default)   Install via core.hooksPath (recommended)"
    echo "  --copy      Copy hooks to .git/hooks/"
    echo "  --status    Show current hook configuration"
    echo "  --remove    Remove hook configuration"
    echo "  --help      Show this help"
    ;;
"")
    install_via_hookspath
    ;;
*)
    echo -e "${RED}Unknown option: $1${NC}"
    echo "Run: $0 --help"
    exit 1
    ;;
esac
