#!/usr/bin/env bash
# execute-fail-resume: `specify plan execute` parks on a build failure,
# the operator fixes the slice, and `bash $0 resume` drives the loop to
# drained. Operator replay aid; never wired into CI.
set -euo pipefail

SCENARIO="execute-fail-resume"
PLAN="rate-limit"

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/single-repo.sh"
dispatch "$@"
