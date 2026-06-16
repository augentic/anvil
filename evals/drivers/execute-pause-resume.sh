#!/usr/bin/env bash
# execute-pause-resume: /spec:execute pauses after build prepare on a slice,
# the operator finishes the build in a breakout, and the loop resumes to drained.
# Operator replay aid; never wired into CI.
set -euo pipefail

SCENARIO="execute-pause-resume"
PLAN="dashboard"
PAUSE_SLICE="user-activity-feed"
PARK_SLICE="$PAUSE_SLICE"

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/single-repo.sh"
dispatch "$@"
