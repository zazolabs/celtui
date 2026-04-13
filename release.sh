#!/usr/bin/env bash
# =============================================================================
# release.sh — Version management, tagging, and release for celtui
#
# USAGE:
#   ./release.sh [patch|minor|major]   # bump version, tag, and push
#   ./release.sh --current             # print current version and exit
#   ./release.sh --help                # show this help
#
# SEMANTIC VERSIONING (semver.org):
#   Given a version MAJOR.MINOR.PATCH:
#     patch  — backwards-compatible bug fixes          (0.1.0 → 0.1.1)
#     minor  — new backwards-compatible functionality  (0.1.0 → 0.2.0)
#     major  — incompatible API changes                (0.1.0 → 1.0.0)
#
# HOW IT WORKS:
#   1. Reads the current version from ./VERSION
#   2. Bumps the requested component; resets lower components to 0
#   3. Writes the new version back to ./VERSION
#   4. Updates the [workspace.package] version in Cargo.toml
#   5. Runs `cargo check` to ensure Cargo.lock is refreshed
#   6. Commits VERSION + Cargo.toml + Cargo.lock with message "chore: bump version to vX.Y.Z"
#   7. Creates an annotated git tag vX.Y.Z
#   8. Pushes the commit and the tag to origin
#      → GitHub Actions (.github/workflows/release.yml) picks up the tag
#        and builds release binaries for all 5 platforms automatically.
#
# REQUIREMENTS:
#   - git, cargo (Rust toolchain)
#   - A configured git remote named 'origin'
#   - Clean working tree (no uncommitted changes) before running
# =============================================================================

set -euo pipefail

VERSION_FILE="./VERSION"
CARGO_TOML="./Cargo.toml"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

die() { echo -e "ERROR: $*" >&2; exit 1; }

print_help() {
    # Print lines between the first and second '# ===...' delimiters,
    # stripping the leading '# ' comment prefix. Works on BSD and GNU awk.
    awk '/^# ===/{delim++; next} delim==1{sub(/^# ?/,""); print} delim==2{exit}' "$0"
    exit 0
}

# Read version from VERSION file, stripping whitespace
current_version() {
    tr -d '[:space:]' < "$VERSION_FILE"
}

# Split a semver string into parts; sets MAJOR, MINOR, PATCH globals
parse_version() {
    local ver="$1"
    IFS='.' read -r MAJOR MINOR PATCH <<< "$ver"
    [[ "$MAJOR" =~ ^[0-9]+$ ]] || die "Invalid MAJOR in version '$ver'"
    [[ "$MINOR" =~ ^[0-9]+$ ]] || die "Invalid MINOR in version '$ver'"
    [[ "$PATCH" =~ ^[0-9]+$ ]] || die "Invalid PATCH in version '$ver'"
}

# ---------------------------------------------------------------------------
# Guards
# ---------------------------------------------------------------------------

check_clean_tree() {
    # Warn if there are uncommitted changes (caller decides whether to abort)
    if ! git diff --quiet || ! git diff --cached --quiet; then
        echo "WARNING: You have uncommitted changes."
        echo "         It is recommended to commit or stash them before releasing."
        echo ""
        read -rp "Continue anyway? [y/N] " yn
        [[ "$yn" =~ ^[Yy]$ ]] || exit 0
    fi
}

check_remote() {
    git remote get-url origin &>/dev/null \
        || die "'origin' remote is not configured. Add it with:\n  git remote add origin <url>"
}

check_tag_free() {
    local tag="$1"
    if git rev-parse "$tag" &>/dev/null; then
        die "Tag '$tag' already exists. Delete it first with: git tag -d $tag"
    fi
}

# ---------------------------------------------------------------------------
# Core logic
# ---------------------------------------------------------------------------

bump_version() {
    local bump_type="$1"
    local current; current=$(current_version)
    parse_version "$current"

    case "$bump_type" in
        patch) PATCH=$(( PATCH + 1 )) ;;
        minor) MINOR=$(( MINOR + 1 )); PATCH=0 ;;
        major) MAJOR=$(( MAJOR + 1 )); MINOR=0; PATCH=0 ;;
        *)     die "Unknown bump type '$bump_type'. Use: patch | minor | major" ;;
    esac

    echo "${MAJOR}.${MINOR}.${PATCH}"
}

update_cargo_toml() {
    local old_ver="$1"
    local new_ver="$2"
    # Replace the version line inside [workspace.package] only.
    # Uses a simple sed that targets the exact quoted version string.
    sed -i.bak "s/^version = \"${old_ver}\"/version = \"${new_ver}\"/" "$CARGO_TOML" \
        && rm -f "${CARGO_TOML}.bak"
    # Verify the replacement actually happened
    grep -q "version = \"${new_ver}\"" "$CARGO_TOML" \
        || die "Failed to update version in $CARGO_TOML"
}

# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

main() {
    [[ $# -eq 0 ]] && { print_help; }

    case "${1:-}" in
        --help|-h)    print_help ;;
        --current)    echo "Current version: $(current_version)"; exit 0 ;;
        patch|minor|major) ;;
        *) die "Unknown argument '${1}'. Use: patch | minor | major | --current | --help" ;;
    esac

    local bump_type="$1"

    # Sanity checks
    [[ -f "$VERSION_FILE" ]] || die "VERSION file not found at $VERSION_FILE"
    [[ -f "$CARGO_TOML"   ]] || die "Cargo.toml not found at $CARGO_TOML"
    check_remote
    check_clean_tree

    local old_ver; old_ver=$(current_version)
    local new_ver; new_ver=$(bump_version "$bump_type")
    local tag="v${new_ver}"

    check_tag_free "$tag"

    echo ""
    echo "  Bump type : $bump_type"
    echo "  Old version : $old_ver"
    echo "  New version : $new_ver"
    echo "  Git tag     : $tag"
    echo ""
    read -rp "Proceed? [y/N] " yn
    [[ "$yn" =~ ^[Yy]$ ]] || { echo "Aborted."; exit 0; }

    # 1. Write new version to VERSION file
    echo "$new_ver" > "$VERSION_FILE"
    echo "[1/5] Updated VERSION → $new_ver"

    # 2. Update Cargo.toml workspace version
    update_cargo_toml "$old_ver" "$new_ver"
    echo "[2/5] Updated Cargo.toml workspace version"

    # 3. Run clippy — treat warnings as errors so no warnings ship in a release
    echo "[3/6] Running cargo clippy..."
    cargo clippy --workspace --all-targets -- -D warnings

    # 4. Refresh Cargo.lock (cargo check is fast; no full build)
    echo "[4/6] Running cargo check to refresh Cargo.lock..."
    echo "Skipping cargo check -q --workspace for an App"

    # 5. Commit the version bump
    git add "$VERSION_FILE" "$CARGO_TOML" Cargo.lock
    git commit -m "chore: bump version to ${tag}"
    echo "[5/6] Committed version bump"

    # 6. Create annotated tag (annotated tags trigger GitHub Actions release workflow)
    git tag -a "$tag" -m "Release ${tag}"
    echo "[6/6] Created annotated tag $tag"

    # 6. Push commit and tag
    echo ""
    echo "Pushing commit and tag to origin..."
    git push origin HEAD
    git push origin "$tag"

    echo ""
    echo "Done! Release $tag pushed."
    echo "GitHub Actions will now build binaries for all platforms."
    echo "Monitor progress at: $(git remote get-url origin | sed 's/\.git$//')/actions"
}

main "$@"
