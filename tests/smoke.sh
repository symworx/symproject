#!/usr/bin/env bash
# Copyright (c) 2026, Nathaniel T. Berry
# Licensed under the Apache License, Version 2.0.

# Integration gate for ./cli/symproject
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CLI="$ROOT/cli/symproject"
fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }

[[ -x "$CLI" ]] || fail "cli/symproject is not executable"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CLI" help >/dev/null
"$CLI" -h >/dev/null
"$CLI" list | grep -q 'shape    paper-line' || fail "list defaults"
"$CLI" list | grep -q 'csymd' || fail "list csymd"

if "$CLI" new smoke-bad "$TMP/bad" >/dev/null 2>&1; then
  fail "new without --title must fail"
fi
if "$CLI" new smoke-bad "$TMP/bad" --title Y --shape nope >/dev/null 2>&1; then
  fail "unknown --shape must fail"
fi

T1="$TMP/demo-study"
"$CLI" new demo-study "$T1" \
  --title "Demo study" \
  --runtime none \
  --no-agents

[[ -f "$T1/README.md" ]] || fail "README"
[[ -f "$T1/papers/README.md" ]] || fail "papers"
[[ -f "$T1/results/README.md" ]] || fail "results"
[[ -f "$T1/data/samples/.gitkeep" ]] || fail "samples"
[[ -f "$T1/src/demo_study/__init__.py" ]] || fail "package init"
[[ -f "$T1/AGENTS.md" ]] || fail "AGENTS"
[[ -d "$T1/.git" ]] || fail "git init"
git -C "$T1" branch --show-current | grep -qx main || fail "default branch main"
[[ ! -e "$T1/analysis" ]] || fail "must not write root analysis/"
[[ ! -e "$T1/docs" ]] || fail "must not write docs/"
[[ ! -e "$T1/pyproject.toml" ]] || fail "runtime none must not write pyproject.toml"
grep -q "Demo study" "$T1/README.md" || fail "title substitution"
if grep -q "csymd.com" "$T1/README.md"; then fail "generic README must not mention csymd.com"; fi
grep -q 'papers/<slug>/' "$T1/AGENTS.md" || fail "AGENTS paper-line map"
grep -q 'take precedence' "$T1/AGENTS.md" || fail "AGENTS precedence"

echo keep-me >>"$T1/README.md"
"$CLI" new demo-study "$T1" \
  --title "Demo study" \
  --runtime none \
  --no-agents
grep -q keep-me "$T1/README.md" || fail "must not clobber README"

ROW="$("$CLI" paper multirate-fusion "$T1" \
  --title "Multirate fusion predicts GH from HRV")"
printf '%s\n' "$ROW" | grep -q 'papers/multirate-fusion/' || fail "paper row"
[[ -f "$T1/papers/multirate-fusion/notes/claims.md" ]] || fail "claims"
[[ -f "$T1/papers/multirate-fusion/analysis/README.md" ]] || fail "analysis readme"
[[ -f "$T1/papers/multirate-fusion/manuscript.md" ]] || fail "manuscript"
[[ -f "$T1/results/multirate-fusion/.gitkeep" ]] || fail "results gitkeep"
grep -q "Multirate fusion predicts GH from HRV" "$T1/papers/multirate-fusion/manuscript.md" || fail "paper title"
git -C "$T1" check-ignore -q -- "data/private/x.csv" || fail "data/private ignored"
git -C "$T1" check-ignore -q -- "results/multirate-fusion/fig.png" || fail "results artifact ignored"
if git -C "$T1" check-ignore -q -- "results/README.md"; then fail "results/README.md must stay tracked"; fi
if git -C "$T1" check-ignore -q -- "results/multirate-fusion/.gitkeep"; then fail "results gitkeep must stay tracked"; fi

T_PKG="$TMP/chp-coupling"
"$CLI" new chp-coupling "$T_PKG" \
  --title "CHP coupling" \
  --package coupling \
  --runtime none \
  --no-agents
[[ -f "$T_PKG/src/coupling/__init__.py" ]] || fail "--package coupling"
[[ ! -e "$T_PKG/src/chp_coupling" ]] || fail "default package must not be written when --package is set"

T_UV="$TMP/stats-uv"
"$CLI" new stats-uv "$T_UV" \
  --title "Stats line" \
  --runtime uv \
  --no-agents
[[ -f "$T_UV/pyproject.toml" ]] || fail "uv pyproject"
grep -q 'name = "stats-uv"' "$T_UV/pyproject.toml" || fail "pyproject name"
grep -q "BEGIN symproject runtime uv" "$T_UV/.gitignore" || fail "uv gitignore fragment"
grep -q "uv sync" "$T_UV/README.md" || fail "uv run note"
if command -v uv >/dev/null 2>&1; then
  [[ -f "$T_UV/uv.lock" ]] || fail "uv.lock"
fi

T3="$TMP/lab-line"
"$CLI" new lab-line "$T3" \
  --title "Lab line" \
  --preset csymd \
  --runtime none \
  --no-agents
grep -q "csymd.com" "$T3/README.md" || fail "csymd lab link"
grep -q "github.com/csymd/lab-line" "$T3/README.md" || fail "csymd clone URL"
grep -q "issue-driven" "$T3/README.md" || fail "csymd issue wording"
[[ ! -e "$T3/pyproject.toml" ]] || fail "runtime override none must skip pyproject"

echo OK
