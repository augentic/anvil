#!/usr/bin/env bash
# execute-pause-resume: the operator interrupts `specify plan execute`
# mid-slice (Ctrl-C), inspects or finishes the build as a breakout, and
# `bash $0 resume` re-enters from the in-progress entry to drained.
# Operator replay aid; never wired into CI.
set -euo pipefail

SCENARIO="execute-pause-resume"
PLAN="dashboard"

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/single-repo.sh"
dispatch "$@"
