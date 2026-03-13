#!/usr/bin/env bash
# Documentation consistency checks for the Augentic Plugins repository.
# Run via: make checks
# Exit code 0 = all checks pass; non-zero = one or more failures.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ERRORS=0
RED='\033[0;31m'
NC='\033[0m'

fail() {
  echo -e "${RED}FAIL${NC}: $1"
  ERRORS=$((ERRORS + 1))
}

# ──────────────────────────────────────────────────────────────
# 1. Markdown link targets exist (relative links only)
# ──────────────────────────────────────────────────────────────
# Use Python for portable markdown link extraction and checking.
# Skip symlinks to avoid false positives (symlinked skills resolve differently).
link_errors=$(python3 - "$REPO_ROOT" <<'PYEOF'
import re, os, sys, glob

repo = sys.argv[1]

def is_under_symlink(filepath, repo_root):
    rel = os.path.relpath(filepath, repo_root)
    parts = rel.split(os.sep)
    current = repo_root
    for part in parts[:-1]:
        current = os.path.join(current, part)
        if os.path.islink(current):
            return True
    return os.path.islink(filepath)

errors = []
for md_file in glob.glob(os.path.join(repo, "**", "*.md"), recursive=True):
    if "/node_modules/" in md_file or "/.git/" in md_file or "/temp/" in md_file:
        continue
    if is_under_symlink(md_file, repo):
        continue
    rel_file = os.path.relpath(md_file, repo)
    parent = os.path.dirname(md_file)
    try:
        content = open(md_file, encoding="utf-8").read()
    except Exception:
        continue
    # Strip fenced code blocks and HTML comments before checking links
    stripped = re.sub(r"```.*?```", "", content, flags=re.DOTALL)
    stripped = re.sub(r"<!--.*?-->", "", stripped, flags=re.DOTALL)
    for m in re.finditer(r"\[[^\]]*\]\(([^)]+)\)", stripped):
        target = m.group(1)
        if target.startswith(("http://", "https://", "mailto:", "#")):
            continue
        path = target.split("#")[0]
        if not path:
            continue
        if path.startswith("src/"):
            continue
        resolved = os.path.join(parent, path)
        if not os.path.exists(resolved):
            errors.append(rel_file + ": " + target)
for e in errors:
    print(e)
PYEOF
)

if [ -n "$link_errors" ]; then
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    echo -e "${RED}FAIL${NC}: Broken link in $line"
  done <<< "$link_errors"
  link_err_count=$(printf "%s\n" "$link_errors" | sed '/^$/d' | wc -l | tr -d ' ')
  ERRORS=$((ERRORS + link_err_count))
fi

# ──────────────────────────────────────────────────────────────
# 2. No "109-point" claims remain
# ──────────────────────────────────────────────────────────────
hits=$(grep -rl '109-point\|109 items\|109 Items' "$REPO_ROOT" --include='*.md' 2>/dev/null | grep -v 'node_modules' | grep -v '.git/' || true)

if [ -n "$hits" ]; then
  for h in $hits; do
    rel="$(python3 -c "import os; print(os.path.relpath('$h', '$REPO_ROOT'))")"
    fail "Stale '109' claim in $rel"
  done
fi

# ──────────────────────────────────────────────────────────────
# 3. Removed workflow surfaces do not reappear
# ──────────────────────────────────────────────────────────────
stale_hits=$(grep -RIl \
  -e '/rt:orchestrator' \
  -e '/omnia:orchestrator' \
  -e 'cursor plugin marketplace' \
  -e 'claude plugin validate' \
  -e 'Provider Capabilities' \
  --include='*.md' --include='*.sh' --include='Makefile' "$REPO_ROOT" 2>/dev/null | \
  grep -v '/.git/' | \
  grep -v '/temp/' | \
  grep -v '/improvements/' | \
  grep -v '^.*/scripts/checks\.sh$' || true)

if [ -n "$stale_hits" ]; then
  for h in $stale_hits; do
    rel="$(python3 -c "import os; print(os.path.relpath('$h', '$REPO_ROOT'))")"
    fail "Stale removed workflow surface in $rel"
  done
fi

echo
if [ "$ERRORS" -gt 0 ]; then
  echo -e "${RED}${ERRORS} check(s) failed.${NC}"
  exit 1
fi

echo "All checks passed."
