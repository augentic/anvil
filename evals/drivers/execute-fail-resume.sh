#!/usr/bin/env bash
# execute-fail-resume: /spec:execute parks on an engineered build failure,
# the operator fixes the slice, and the loop resumes to drained.
# Operator replay aid; never wired into CI.
set -euo pipefail

SCENARIO="execute-fail-resume"
PLAN="rate-limit"
FAIL_SLICE="session-cookie-harden"
PARK_SLICE="$FAIL_SLICE"

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/single-repo.sh"
dispatch "$@"
