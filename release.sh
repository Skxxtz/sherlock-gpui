#!/usr/bin/env bash
# Usage: ./release.sh <version>      e.g. ./release.sh 0.2.5  (or v0.2.5)
#
# Bumps versions, rolls CHANGELOG [Unreleased] into a new section, builds,
# tests, regenerates docs, commits, tags and (after confirmation) pushes.
# Pushing the tag triggers .github/workflows/release.yml (GitHub + AUR).
set -euo pipefail

RED='\033[0;31m'; GREEN='\033[0;32m'; BLUE='\033[1;34m'; NC='\033[0m'
step() { echo -e "${BLUE}==>${NC} $*"; }
die()  { echo -e "${RED}error:${NC} $*" >&2; exit 1; }

cd "$(git rev-parse --show-toplevel)"

# Arguments
[ $# -eq 1 ] || die "usage: $0 <version>  (e.g. 0.2.5)"
VERSION="${1#v}"
TAG="v$VERSION"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] || die "'$VERSION' is not semver"

OLD_VERSION=$(awk -F'"' '/^\[package\]/{p=1} p && /^version *=/{print $2; exit}' Cargo.toml)
[ -n "$OLD_VERSION" ] || die "could not read version from Cargo.toml"

# Pre-flight checks
step "Pre-flight checks"
[ "$(git branch --show-current)" = "main" ] || die "not on 'main'"
git diff --quiet && git diff --cached --quiet || die "uncommitted changes, commit or stash first"
git fetch --quiet origin main --tags
[ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] || die "local main differs from origin/main (pull/push first)"
git rev-parse -q --verify "refs/tags/$TAG" >/dev/null && die "tag $TAG already exists"
if grep -qF "## [$VERSION]" CHANGELOG.md; then
  HAS_HEADER=1
else
  HAS_HEADER=0
  grep -q '^## \[Unreleased\]' CHANGELOG.md || die "CHANGELOG.md has no '## [Unreleased]' section"
  UNRELEASED=$(awk '/^## \[Unreleased\]/{f=1; next} f && /^## \[/{exit} f' CHANGELOG.md | grep -cv '^[[:space:]]*$' || true)
  [ "$UNRELEASED" -gt 0 ] || die "[Unreleased] section in CHANGELOG.md is empty"
fi

echo "  $OLD_VERSION -> $VERSION"

# Version bumps
step "Bumping Cargo.toml"
awk -v v="$VERSION" '
  /^\[package\]/ {p=1}
  p && !done && /^version *=/ {print "version = \"" v "\""; done=1; next}
  {print}' Cargo.toml > Cargo.toml.tmp && mv Cargo.toml.tmp Cargo.toml
# flake.nix reads the version from Cargo.toml, nothing to do there.

if [ "$HAS_HEADER" -eq 1 ]; then
  step "CHANGELOG.md already has a [$VERSION] section, leaving it"
else
  step "Updating CHANGELOG.md"
  DATE=$(date +%d.%m.%y)
  awk -v v="$VERSION" -v d="$DATE" '
    /^## \[Unreleased\]/ {print; print ""; print "## [" v "] - " d; next}
    {print}' CHANGELOG.md > CHANGELOG.md.tmp && mv CHANGELOG.md.tmp CHANGELOG.md
fi

# Build, test, docs
FEATURES="wayland,docs"         # 'docs' is required for --generate-docs
step "Building (release, features: $FEATURES)"
cargo build --release --features "$FEATURES"   # also refreshes Cargo.lock
step "Testing"
cargo test --release --features "$FEATURES"
step "Generating launcher docs"
SHERLOCK_DEV=true ./target/release/sherlock-gpui --generate-docs

# Commit & tag
step "Committing"
git add Cargo.toml Cargo.lock CHANGELOG.md README.md docs/
git commit -m "release: $TAG"
git tag -a "$TAG" -m "Release $TAG"

echo
git --no-pager show --stat HEAD
echo
read -rp "Push 'main' and tag $TAG to origin? [y/N] " answer
if [[ "$answer" =~ ^[Yy]$ ]]; then
  git push --atomic origin main "$TAG"
  echo -e "${GREEN}Released $TAG.${NC} CI will publish the GitHub release and AUR package."
else
  echo "Not pushed. To undo:  git tag -d $TAG && git reset --hard HEAD~1"
fi
