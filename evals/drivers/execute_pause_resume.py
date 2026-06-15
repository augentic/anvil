#!/usr/bin/env python3
"""Drive the execute-pause-resume eval scenario."""

from __future__ import annotations

import json
import sys
from pathlib import Path

DRIVERS = Path(__file__).resolve().parent
FRAMEWORK = DRIVERS.parent.parent
if str(DRIVERS) not in sys.path:
    sys.path.insert(0, str(DRIVERS))
from execute_loop import (  # noqa: E402
    main_lock_loop,
    propose_slices,
    run,
    setup_project,
    stamp_gate1,
    survey_intent,
    with_plan_lock,
)

SANDBOX = FRAMEWORK / "evals/.sandbox/execute-pause-resume"
PLAN = "dashboard"
LEADS = [
    ("metrics-summary", "Expose a metrics summary endpoint for dashboard KPIs."),
    ("user-activity-feed", "Stream recent user activity for the dashboard feed."),
]
LEAD_MAP = {lead: lead for lead, _ in LEADS}
SLICES = [
    {
        "name": name,
        "sources": [{"source": "intent", "lead": name}],
        "rationale": synopsis,
    }
    for name, synopsis in LEADS
]


def main() -> int:
    setup_project(SANDBOX)
    survey_intent(
        SANDBOX,
        PLAN,
        LEADS,
        intent_value=(
            "Dashboard backend: metrics summary KPIs and a recent user activity feed."
        ),
    )
    propose_slices(SANDBOX, SLICES)
    stamp_gate1(SANDBOX, PLAN)

    paused = main_lock_loop(
        SANDBOX,
        lead_map=LEAD_MAP,
        build_stop_after_prepare="user-activity-feed",
    )
    if paused.get("action") != "build" or paused.get("slice") != "user-activity-feed":
        print(json.dumps({"error": "expected paused at build user-activity-feed", "status": paused}, indent=2))
        return 1
    in_progress = int(
        __import__("subprocess")
        .run(
            ["grep", "-c", "status: in-progress", "plan.yaml"],
            cwd=SANDBOX,
            text=True,
            capture_output=True,
        )
        .stdout.strip()
        or "0"
    )
    print("PAUSE", json.dumps({"status": paused, "in_progress": in_progress}, indent=2))

    breakout = with_plan_lock(
        SANDBOX,
        [
            sys.executable,
            "-c",
            (
                "import sys; sys.path.insert(0,%r); "
                "from execute_loop import build_slice; "
                "from pathlib import Path; "
                "build_slice(Path(%r), 'user-activity-feed')"
            )
            % (str(DRIVERS), str(SANDBOX)),
        ],
    )
    if breakout.returncode != 0:
        print(breakout.stderr)
        return 1

    validate = run(["plan", "validate", "--format", "json"], SANDBOX, check=False)
    print("BREAKOUT validate exit", validate.returncode)

    final = main_lock_loop(SANDBOX, lead_map=LEAD_MAP)
    if final.get("action") != "drained":
        print(json.dumps({"error": "expected drained", "status": final}, indent=2))
        return 1

    done_count = int(
        __import__("subprocess")
        .run(
            ["grep", "-c", "status: done", "plan.yaml"],
            cwd=SANDBOX,
            text=True,
            capture_output=True,
        )
        .stdout.strip()
        or "0"
    )
    print("FINAL", json.dumps({"status": final, "done": done_count}, indent=2))
    return 0 if done_count == 2 else 1


if __name__ == "__main__":
    raise SystemExit(main())
