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
"$CLI" help | grep -q '^Labs:' || fail "help labs section"
"$CLI" help | grep -q 'create lab' || fail "help create lab"
"$CLI" list | grep -q 'shape    paper-line' || fail "list defaults"
"$CLI" list | grep -q 'org      none' || fail "list default org"
"$CLI" list | grep -q 'symproject labs' || fail "list labs"
if "$CLI" list | grep -q 'csymd'; then fail "list names a lab"; fi
if grep -a -q 'csymd' "$ROOT/target/debug/symproject"; then fail "binary embeds a lab name"; fi

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
  --org lab \
  --no-agents

[[ -f "$T1/README.md" ]] || fail "README"
[[ -f "$T1/papers/README.md" ]] || fail "papers"
[[ -f "$T1/results/README.md" ]] || fail "results"
[[ -f "$T1/data/samples/.gitkeep" ]] || fail "samples"
[[ -f "$T1/src/demo_study/__init__.py" ]] || fail "package init"
[[ -f "$T1/AGENTS.md" ]] || fail "AGENTS"
[[ -f "$T1/docs/design.md" ]] || fail "design"
[[ -f "$T1/docs/funding/README.md" ]] || fail "funding"
[[ -f "$T1/docs/methods/README.md" ]] || fail "methods"
[[ -f "$T1/analysis/README.md" ]] || fail "shared analysis"
[[ ! -e "$T1/docs/aims.md" ]] || fail "must not write docs/aims.md"
[[ -f "$T1/migration-docs/README.md" ]] || fail "default migration README"
[[ -f "$T1/migration-docs/.gitkeep" ]] || fail "default migration gitkeep"
grep -q 'BEGIN symproject migration-docs' "$T1/.gitignore" || fail "migration gitignore"
[[ -d "$T1/.git" ]] || fail "git init"
git -C "$T1" branch --show-current | grep -qx main || fail "default branch main"
[[ ! -e "$T1/pyproject.toml" ]] || fail "runtime none must not write pyproject.toml"
grep -q "Demo study" "$T1/README.md" || fail "title substitution"
grep -q "lab.example" "$T1/README.md" || fail "default lab holding"
[[ -f "$T1/.symproject.yaml" ]] || fail "sidecar"
if grep -q "csymd" "$T1/README.md"; then fail "default README names a lab"; fi
grep -q 'papers/<slug>/' "$T1/AGENTS.md" || fail "AGENTS paper-line map"
grep -q 'take precedence' "$T1/AGENTS.md" || fail "AGENTS precedence"
grep -q 'Do not add `docs/aims.md`' "$T1/AGENTS.md" || fail "AGENTS keeps aims out of docs"

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
grep -q '^## Plan$' "$T1/papers/multirate-fusion/notes/claims.md" || fail "claims plan"
CLAIMS="$T1/papers/multirate-fusion/notes/claims.md"
CLAIM_SUM="$(cksum "$CLAIMS")"

LAB="$TMP/lab.yml"
cat >"$LAB" <<'EOF'
version: 1
github_org: other-org
lab_name: Other Lab
lab_url: https://other.example
issue_line: "Tickets live in the lab tracker."
EOF
"$CLI" set-lab "$LAB" "$T1" --dry-run
grep -q "lab.example" "$T1/README.md" || fail "dry-run must leave the default lab"
"$CLI" set-lab "$LAB" "$T1"
grep -F -q "Lab: [Other Lab](https://other.example)." "$T1/README.md" || fail "set-lab lab line"
grep -q "github.com/other-org/demo-study" "$T1/README.md" || fail "set-lab clone URL"
grep -q "Tickets live in the lab tracker." "$T1/README.md" || fail "set-lab issue in README"
grep -q "Tickets live in the lab tracker." "$T1/CONTRIBUTING.md" || fail "set-lab issue in CONTRIBUTING"
grep -q "keep-me" "$T1/README.md" || fail "set-lab must keep surrounding prose"
if grep -q "lab.example" "$T1/README.md"; then fail "default holding remains"; fi
[[ "$(cksum "$CLAIMS")" == "$CLAIM_SUM" ]] || fail "set-lab touched claims"
"$CLI" set-lab "$LAB" "$T1"

cat >"$TMP/lab-2.yml" <<'EOF'
version: 1
github_org: third-org
lab_name: Third Lab
lab_url: https://third.example
issue_line: "Use the lab board."
EOF
"$CLI" set-lab "$TMP/lab-2.yml" "$T1"
grep -q "github.com/third-org/demo-study" "$T1/README.md" || fail "second set-lab"
grep -F -q "Lab: [Third Lab](https://third.example)." "$T1/README.md" || fail "second lab line"
if grep -q "other-org" "$T1/README.md"; then fail "first lab remains"; fi

"$CLI" create lab "$TMP/lab-created.yml"
grep -q 'github_org: lab-org' "$TMP/lab-created.yml" || fail "create lab template"
"$CLI" create lab "$TMP/lab-created.yml"
grep -q 'github_org: lab-org' "$TMP/lab-created.yml" || fail "create lab must not replace"
"$CLI" new neutral "$TMP/neutral" \
  --title "Neutral" \
  --runtime none \
  --no-agents
if grep -q 'lab.example' "$TMP/neutral/README.md"; then fail "default stamps a lab"; fi
grep -q 'git clone <repository-url>' "$TMP/neutral/README.md" || fail "neutral clone line"

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

T_NONE="$TMP/neutral"
"$CLI" new neutral "$T_NONE" \
  --title "Neutral line" \
  --org none \
  --runtime none \
  --no-agents
if grep -q "lab.example" "$T_NONE/README.md"; then fail "org none must leave the lab line blank"; fi
if "$CLI" set-lab "$LAB" "$T_NONE" >/dev/null 2>&1; then fail "set-lab must refuse org none"; fi

T_STAMP="$TMP/stamped"
"$CLI" new stamped "$T_STAMP" \
  --title "Stamped" \
  --runtime none \
  --no-agents \
  --lab "$LAB"
grep -q "other.example" "$T_STAMP/README.md" || fail "new --lab"
"$CLI" new stamped "$T_STAMP" \
  --title "Stamped" \
  --runtime none \
  --no-agents \
  --lab "$TMP/lab-2.yml"
grep -q "other.example" "$T_STAMP/README.md" || fail "new rerun must keep the first lab"
if grep -q "third-org" "$T_STAMP/README.md"; then fail "new rerun restamped the lab"; fi

T2="$TMP/migrated"
"$CLI" new migrated "$T2" \
  --title "Migrated line" \
  --runtime none \
  --no-agents
[[ -f "$T2/migration-docs/README.md" ]] || fail "migration readme"
[[ -f "$T2/migration-docs/.gitkeep" ]] || fail "migration gitkeep"
grep -q 'Back it up somewhere else' "$T2/migration-docs/README.md" || fail "migration disclaimer"
printf '\nlegacy.pdf\nscans/visit.pdf\n' >>"$T2/migration-docs/.gitkeep"
"$CLI" new migrated "$T2" \
  --title "Migrated line" \
  --runtime none \
  --no-agents
grep -q '!migration-docs/legacy.pdf' "$T2/.gitignore" || fail "gitkeep path"
grep -q '!migration-docs/scans/visit.pdf' "$T2/.gitignore" || fail "nested gitkeep path"
[[ "$(grep -c 'BEGIN symproject migration-docs' "$T2/.gitignore")" -eq 1 ]] || fail "duplicate migration block"
printf '\nextra.pdf\n' >>"$T2/migration-docs/.gitkeep"
"$CLI" new migrated "$T2" \
  --title "Migrated line" \
  --runtime none \
  --no-agents \
  --no-migration-docs
if grep -q '!migration-docs/extra.pdf' "$T2/.gitignore"; then
  fail "--no-migration-docs must not refresh gitkeep exceptions"
fi
[[ -f "$T2/migration-docs/README.md" ]] || fail "--no-migration-docs must not delete migration-docs"

T_OFF="$TMP/no-migration"
"$CLI" new plain "$T_OFF" \
  --title "Plain line" \
  --runtime none \
  --no-agents \
  --no-migration-docs
[[ ! -e "$T_OFF/migration-docs" ]] || fail "--no-migration-docs must not write migration-docs"
grep -q 'BEGIN symproject migration-docs' "$T_OFF/.gitignore" || fail "gitignore migration block without the directory"

LABS="$TMP/labs"
SYMPROJECT_LABS="$LABS" "$CLI" labs | grep -q 'symproject create lab' || fail "labs new shortcut"
SYMPROJECT_LABS="$LABS" "$CLI" labs | grep -q 'symproject edit lab <name>' || fail "labs edit shortcut"
SYMPROJECT_LABS="$LABS" "$CLI" labs | grep -q 'symproject delete lab <name>' || fail "labs delete shortcut"
SYMPROJECT_LABS="$LABS" "$CLI" labs | grep -q '(none)' || fail "labs empty"
if SYMPROJECT_LABS="$LABS" "$CLI" create lab --lab-name "Only Name" >/dev/null 2>&1; then
  fail "create lab must require the identity flags together"
fi
SYMPROJECT_LABS="$LABS" "$CLI" create lab \
  --institution "State University" \
  --lab-name "Cog Neuro" \
  --github-org cog-neuro \
  --lab-url "https://example.edu/cog" \
  --investigator "Ada Lovelace" \
  --investigator "Grace Hopper"
[[ -f "$LABS/cog-neuro.yml" ]] || fail "lab file"
grep -q 'Ada Lovelace' "$LABS/cog-neuro.yml" || fail "investigator stored"
if SYMPROJECT_LABS="$LABS" "$CLI" create lab \
  --institution "State University" \
  --lab-name "Cog Neuro" \
  --github-org cog-neuro \
  --lab-url "https://example.edu/cog" >/dev/null 2>&1; then
  fail "create lab must not replace"
fi

T_LAB="$TMP/from-lab"
SYMPROJECT_LABS="$LABS" "$CLI" new from-lab "$T_LAB" \
  --title "From lab" \
  --runtime none \
  --no-agents \
  --lab cog-neuro
grep -F -q "Lab: [Cog Neuro](https://example.edu/cog), State University." "$T_LAB/README.md" || fail "lab line with institution"
if grep -q 'Ada Lovelace' "$T_LAB/README.md"; then fail "investigator stamped into README"; fi

if SYMPROJECT_LABS="$LABS" "$CLI" edit lab cog-neuro >/dev/null 2>&1; then
  fail "edit without a field must fail"
fi
SYMPROJECT_LABS="$LABS" "$CLI" edit lab cog-neuro --institution "Other University"
grep -q 'Other University' "$LABS/cog-neuro.yml" || fail "edit institution"
grep -F -q "State University." "$T_LAB/README.md" || fail "edit must not rewrite the project"

if SYMPROJECT_LABS="$LABS" "$CLI" delete lab cog-neuro >/dev/null 2>&1; then
  fail "delete without --yes must fail"
fi
[[ -f "$LABS/cog-neuro.yml" ]] || fail "delete without --yes removed the file"
SYMPROJECT_LABS="$LABS" "$CLI" delete lab cog-neuro --yes
[[ ! -e "$LABS/cog-neuro.yml" ]] || fail "delete --yes"
grep -F -q "State University." "$T_LAB/README.md" || fail "delete must not rewrite the project"

command -v python3 >/dev/null 2>&1 || fail "python3 is required for the arrow-menu check"
python3 - "$CLI" "$TMP/arrow-labs" <<'PY'
import os, select, sys, time

cli, labs = sys.argv[1], sys.argv[2]
os.environ["SYMPROJECT_LABS"] = labs
os.environ.pop("SYMPROJECT_ROOT", None)
os.environ.pop("SYMPROJECT_DATA", None)

master, slave = os.openpty()
pid = os.fork()
if pid == 0:
    try:
        os.setsid()
        os.dup2(slave, 0)
        os.dup2(slave, 1)
        os.dup2(slave, 2)
        if slave > 2:
            os.close(slave)
        os.close(master)
        os.execv(cli, [cli, "labs"])
    finally:
        os._exit(127)

os.close(slave)

def read_until(needle, timeout):
    buf = b""
    deadline = time.time() + timeout
    while time.time() < deadline:
        ready, _, _ = select.select([master], [], [], 0.2)
        if master not in ready:
            continue
        try:
            chunk = os.read(master, 4096)
        except OSError:
            break
        if not chunk:
            break
        buf += chunk
        if needle in buf:
            return buf
    sys.stderr.buffer.write(buf)
    raise SystemExit(f"missing {needle!r}")

try:
    read_until(b"> new", 5)
    os.write(master, b"\x1b[B")
    read_until(b"> edit", 5)
    os.write(master, b"q")
    deadline = time.time() + 5
    status = None
    while time.time() < deadline:
        done, status = os.waitpid(pid, os.WNOHANG)
        if done == pid:
            break
        ready, _, _ = select.select([master], [], [], 0.1)
        if master in ready:
            try:
                os.read(master, 4096)
            except OSError:
                pass
    else:
        os.kill(pid, 15)
        raise SystemExit("labs did not exit after q")
    if not os.WIFEXITED(status) or os.WEXITSTATUS(status) != 0:
        raise SystemExit(f"labs exited with status {status}")
finally:
    os.close(master)
PY

echo OK
