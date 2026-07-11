#!/bin/bash
# Compatibility entry point for the historical operator runbook.
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
exec "$root/quality/profiles/guest-execute-loop.sh" "$@"
