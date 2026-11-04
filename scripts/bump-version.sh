#!/usr/bin/env bash
# Copyright (c) 2026, Nathaniel T. Berry
# Licensed under the Apache License, Version 2.0.
#
# Bump the symproject crate version.
#
# Source of truth: root Cargo.toml  [package] version
# This is a single crate (not a workspace). No Python / R pins.
#
# Updates (only these sites):
#   1. [package] version in Cargo.toml
#   2. Cargo.lock entry for name = "symproject"
#   3. Optional CHANGELOG.md stub (--changelog)
#
# Usage:
#   ./scripts/bump-version.sh              # print current version + check
#   ./scripts/bump-version.sh patch        # 0.1.0 → 0.1.1
#   ./scripts/bump-version.sh minor        # 0.1.0 → 0.2.0
#   ./scripts/bump-version.sh major        # 0.1.0 → 1.0.0
#   ./scripts/bump-version.sh set 0.2.0
#   ./scripts/bump-version.sh set 0.2.0-rc.1
#   ./scripts/bump-version.sh set 0.1.0 --yes   # downgrade (not if already on crates.io)
#   ./scripts/bump-version.sh patch --dry-run
#   ./scripts/bump-version.sh minor --changelog
#
# Safe workflow:
#   1. On worx (or an optional release/vX.Y.Z freeze branch):
#        ./scripts/bump-version.sh patch --dry-run
#        ./scripts/bump-version.sh patch --changelog
#   2. Fill in CHANGELOG.md
#   3. git diff, commit, open PR → worx
#   4. After merge: git tag -a vX.Y.Z && git push origin vX.Y.Z
#      (Release workflow checks the tag; it does not publish)
#
# See DEVELOPMENT.md § Releasing.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

DRY_RUN=0
DO_CHANGELOG=0
ASSUME_YES=0
ACTION=""
EXPLICIT=""

usage() {
  sed -n '4,32p' "$0" | sed 's/^# \{0,1\}//'
  exit "${1:-0}"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage 0 ;;
    --dry-run) DRY_RUN=1; shift ;;
    --changelog) DO_CHANGELOG=1; shift ;;
    -y|--yes) ASSUME_YES=1; shift ;;
    show|current|check)
      ACTION="show"; shift ;;
    sync)
      ACTION="sync"; shift ;;
    patch|minor|major)
      ACTION="$1"; shift ;;
    set)
      ACTION="set"
      shift
      EXPLICIT="${1:-}"
      if [[ -z "$EXPLICIT" ]]; then
        echo "error: set requires a version, e.g. set 0.2.0" >&2
        exit 1
      fi
      shift
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage 1
      ;;
  esac
done

ACTION="${ACTION:-show}"

read_package_version() {
  awk '
    $0 ~ /^\[package\]/ { in_pkg=1; next }
    in_pkg && $0 ~ /^\[/ { in_pkg=0 }
    in_pkg && $0 ~ /^version/ {
      if (match($0, /"[^"]+"/)) {
        print substr($0, RSTART+1, RLENGTH-2)
        exit
      }
    }
  ' Cargo.toml
}

read_lock_version() {
  awk '
    $0 ~ /^name = "symproject"$/ { hit=1; next }
    hit && $0 ~ /^version = "/ {
      if (match($0, /"[^"]+"/)) {
        print substr($0, RSTART+1, RLENGTH-2)
        exit
      }
    }
    hit && $0 ~ /^\[\[/ { exit }
  ' Cargo.lock
}

is_valid_version() {
  [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]]
}

base_version() {
  local v="$1"
  v="${v%%-*}"
  v="${v%%+*}"
  echo "$v"
}

bump_semver() {
  local kind="$1" current="$2"
  local base major minor patch
  base="$(base_version "$current")"
  IFS=. read -r major minor patch <<<"$base"
  case "$kind" in
    major) echo "$((major + 1)).0.0" ;;
    minor) echo "${major}.$((minor + 1)).0" ;;
    patch) echo "${major}.${minor}.$((patch + 1))" ;;
    *) echo "error: bad bump kind: $kind" >&2; exit 1 ;;
  esac
}

cmp_base_versions() {
  local a b
  a="$(base_version "$1")"
  b="$(base_version "$2")"
  local am an ap bm bn bp
  IFS=. read -r am an ap <<<"$a"
  IFS=. read -r bm bn bp <<<"$b"
  if (( am < bm )); then echo -1; return; fi
  if (( am > bm )); then echo 1; return; fi
  if (( an < bn )); then echo -1; return; fi
  if (( an > bn )); then echo 1; return; fi
  if (( ap < bp )); then echo -1; return; fi
  if (( ap > bp )); then echo 1; return; fi
  echo 0
}

confirm_or_abort() {
  local prompt="$1"
  if [[ "$ASSUME_YES" -eq 1 || "$DRY_RUN" -eq 1 ]]; then
    return 0
  fi
  if [[ ! -t 0 ]]; then
    echo "error: non-interactive shell; re-run with --yes to confirm" >&2
    exit 1
  fi
  local reply
  read -r -p "${prompt} [y/N] " reply
  case "$reply" in
    y|Y|yes|YES) return 0 ;;
    *) echo "Aborted."; exit 1 ;;
  esac
}

report_versions() {
  local expected="$1"
  local ok=1
  local lock

  echo "Package version: ${expected}"
  echo
  echo "  site                          version       status"
  echo "  ----------------------------  ------------  ------------"
  printf "  %-28s  %-12s  %-12s\n" "Cargo.toml [package]" "$expected" "ok"

  if [[ -f Cargo.lock ]]; then
    lock="$(read_lock_version)"
    if [[ "$lock" == "$expected" ]]; then
      printf "  %-28s  %-12s  %-12s\n" "Cargo.lock (symproject)" "$lock" "ok"
    else
      printf "  %-28s  %-12s  %-12s\n" "Cargo.lock (symproject)" "${lock:--}" "want ${expected}"
      ok=0
    fi
  else
    printf "  %-28s  %-12s  %-12s\n" "Cargo.lock (symproject)" "-" "missing"
    ok=0
  fi

  echo
  if [[ -f CHANGELOG.md ]]; then
    if grep -Eq "^## \[${expected}\]" CHANGELOG.md; then
      echo "CHANGELOG.md: has ## [${expected}] section"
    else
      echo "CHANGELOG.md: no ## [${expected}] section yet (use --changelog for a stub)"
    fi
  else
    echo "CHANGELOG.md: missing"
  fi

  return $((1 - ok))
}

print_change_plan() {
  local old="$1" new="$2"
  local lock
  echo "Plan → ${new}"
  echo
  echo "  site                          old           new"
  echo "  ----------------------------  ------------  ------------"
  if [[ "$old" != "$new" ]]; then
    printf "  %-28s  %-12s  %-12s\n" "Cargo.toml [package]" "$old" "$new"
  else
    printf "  %-28s  %-12s  %-12s\n" "Cargo.toml [package]" "$old" "(unchanged)"
  fi
  if [[ -f Cargo.lock ]]; then
    lock="$(read_lock_version)"
    printf "  %-28s  %-12s  %-12s\n" "Cargo.lock (symproject)" "${lock:--}" "$new"
  fi
  echo
}

bump_cargo_toml() {
  local new="$1"
  local tmp
  tmp="$(mktemp)"
  awk -v new="$new" '
    BEGIN { sec = "" }
    /^\[package\]/ { sec = "pkg"; print; next }
    /^\[/ { sec = ""; print; next }
    sec == "pkg" && /^version[[:space:]]*=/ {
      if (match($0, /"[^"]+"/)) {
        print substr($0, 1, RSTART) new substr($0, RSTART + RLENGTH - 1)
        next
      }
    }
    { print }
  ' Cargo.toml >"$tmp"
  mv "$tmp" Cargo.toml
}

bump_cargo_lock() {
  local new="$1"
  [[ -f Cargo.lock ]] || return 0
  local tmp
  tmp="$(mktemp)"
  awk -v new="$new" '
    $0 ~ /^name = "symproject"$/ { hit=1; print; next }
    hit && $0 ~ /^version = "/ {
      if (match($0, /"[^"]+"/)) {
        print substr($0, 1, RSTART) new substr($0, RSTART + RLENGTH - 1)
        hit=0
        next
      }
    }
    hit && $0 ~ /^\[\[/ { hit=0 }
    { print }
  ' Cargo.lock >"$tmp"
  mv "$tmp" Cargo.lock
}

maybe_stub_changelog() {
  local new="$1"
  if [[ ! -f CHANGELOG.md ]]; then
    echo "warning: CHANGELOG.md missing; skip stub" >&2
    return 0
  fi
  if grep -Eq "^## \[${new}\]" CHANGELOG.md; then
    echo "CHANGELOG.md already has ## [${new}]"
    return 0
  fi
  local date
  date="$(date +%Y-%m-%d)"
  local stub
  stub=$(cat <<EOF
## [${new}] - ${date}

### Added

- 

### Changed

- 

### Notes

- 

EOF
)
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "--- dry-run: would prepend CHANGELOG stub for [${new}] ---"
    echo "$stub"
    return 0
  fi
  local tmp
  tmp="$(mktemp)"
  awk -v stub="$stub" -v ver="$new" '
    BEGIN { inserted = 0 }
    /^## \[/ && !inserted {
      print stub
      inserted = 1
    }
    { print }
    END {
      if (!inserted) {
        print ""
        print stub
      }
    }
  ' CHANGELOG.md >"$tmp"
  if grep -Eq '^\[0\.1\.0\]:' "$tmp" && ! grep -Eq "^\[${new}\]:" "$tmp"; then
    awk -v ver="$new" '
      /^## Version Links/ { print; next }
      /^\[0\.1\.0\]:/ && !done {
        print "[" ver "]: https://github.com/symworx/symproject/releases/tag/v" ver
        done=1
      }
      { print }
    ' "$tmp" >"${tmp}.2"
    mv "${tmp}.2" "$tmp"
  fi
  mv "$tmp" CHANGELOG.md
  echo "CHANGELOG.md: added stub section ## [${new}] - ${date}"
  echo "  → edit the bullets, then keep the heading."
}

CURRENT="$(read_package_version)"
if [[ -z "$CURRENT" ]]; then
  echo "error: could not read [package] version from Cargo.toml" >&2
  exit 1
fi

if ! is_valid_version "$CURRENT"; then
  echo "error: current package version looks invalid: '${CURRENT}'" >&2
  exit 1
fi

if [[ "$ACTION" == "show" ]]; then
  echo "Current package version: ${CURRENT}"
  echo
  if report_versions "$CURRENT"; then
    echo
    echo "Cargo.toml and Cargo.lock match."
    exit 0
  else
    echo
    echo "Some sites are out of sync (./scripts/bump-version.sh sync)."
    exit 1
  fi
fi

case "$ACTION" in
  patch|minor|major)
    NEW="$(bump_semver "$ACTION" "$CURRENT")"
    ;;
  sync)
    NEW="$CURRENT"
    ;;
  set)
    NEW="$EXPLICIT"
    if ! is_valid_version "$NEW"; then
      echo "error: invalid version '${NEW}' (want X.Y.Z or X.Y.Z-prerelease)" >&2
      exit 1
    fi
    ;;
  *)
    echo "error: unhandled action: $ACTION" >&2
    exit 1
    ;;
esac

DIRECTION="bump"
if [[ "$NEW" == "$CURRENT" ]]; then
  DIRECTION="sync"
elif [[ "$(cmp_base_versions "$NEW" "$CURRENT")" -lt 0 ]]; then
  DIRECTION="downgrade"
elif [[ "$(cmp_base_versions "$NEW" "$CURRENT")" -eq 0 && "$NEW" != "$CURRENT" ]]; then
  DIRECTION="adjust"
fi

if [[ "$DIRECTION" == "sync" ]]; then
  if report_versions "$CURRENT" >/dev/null 2>&1 && [[ "$DO_CHANGELOG" -eq 0 ]]; then
    echo "Already at ${CURRENT}; Cargo.toml and Cargo.lock match. Nothing to do."
    report_versions "$CURRENT" || true
    exit 0
  fi
  echo "Re-syncing lockfile / changelog to ${CURRENT}."
  echo
else
  case "$DIRECTION" in
    bump)      echo "Bump:      ${CURRENT} → ${NEW}" ;;
    downgrade) echo "Downgrade: ${CURRENT} → ${NEW}" ;;
    adjust)    echo "Adjust:    ${CURRENT} → ${NEW}" ;;
  esac
fi

[[ "$DRY_RUN" -eq 1 ]] && echo "(dry-run: no files will be written)"
echo

print_change_plan "$CURRENT" "$NEW"

if [[ "$DIRECTION" == "downgrade" ]]; then
  cat <<EOF
Note: this only rewrites package metadata in the working tree.
  • Does not delete git tags, CHANGELOG history, or remote releases.
  • Safe if ${CURRENT} was never published / tagged remotely.
  • If ${CURRENT} is already on crates.io, bump forward instead.

EOF
  confirm_or_abort "Proceed with downgrade to ${NEW}?"
fi

if [[ "$DO_CHANGELOG" -eq 1 && "$DIRECTION" != "downgrade" ]]; then
  maybe_stub_changelog "$NEW"
elif [[ "$DO_CHANGELOG" -eq 1 && "$DIRECTION" == "downgrade" ]]; then
  echo "Skipping --changelog on downgrade."
fi

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "Dry-run complete. Re-run without --dry-run to apply."
  exit 0
fi

bump_cargo_toml "$NEW"
bump_cargo_lock "$NEW"

echo "Updated. Consistency check:"
echo
if report_versions "$NEW"; then
  echo
  echo "Cargo.toml and Cargo.lock match ${NEW}."
else
  echo
  echo "warning: some sites still disagree; inspect with git diff" >&2
fi

echo
echo "Next steps:"
echo "  1. Review:  git diff"
if [[ "$DIRECTION" == "downgrade" ]]; then
  echo "  2. If you tagged ${CURRENT} locally:  git tag -d v${CURRENT}"
  echo "  3. Commit the corrected version"
elif [[ "$DIRECTION" == "sync" ]]; then
  echo "  2. Commit if the lockfile drift was unintentional"
else
  if [[ "$DO_CHANGELOG" -eq 0 ]]; then
    echo "  2. Ensure CHANGELOG.md has:  ## [${NEW}]"
    echo "     (or re-run with --changelog for a stub)"
  else
    echo "  2. Fill in CHANGELOG.md bullets under ## [${NEW}]"
  fi
  echo "  3. Commit and open a pull request against worx"
  echo "  4. After merge:  git tag -a v${NEW} -m v${NEW} && git push origin v${NEW}"
  echo "     The release workflow checks the tag. It does not publish."
fi
echo
echo "Accidentally too high?  ./scripts/bump-version.sh set <lower> [--yes]"
