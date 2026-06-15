#!/usr/bin/env python3
"""Drive the execute-fail-resume eval scenario."""

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
    plan_status,
    propose_slices,
    run,
    setup_project,
    stamp_gate1,
    survey_intent,
    with_plan_lock,
)

SANDBOX = FRAMEWORK / "evals/.sandbox/execute-fail-resume"
PLAN = "rate-limit"
LEADS = [
    ("auth-rate-limit", "Add per-IP auth rate limiting for login attempts."),
    ("password-hash-rotate", "Rotate password hashing parameters for stored credentials."),
    (
        "session-cookie-harden",
        "Harden session cookies with Secure and HttpOnly flags.",
    ),
    ("reset-flow-retire", "Retire the legacy password reset flow."),
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


def patch_session_cookie_secure() -> None:
    lib = SANDBOX / "crates/session_cookie_harden/src/lib.rs"
    text = lib.read_text()
    text = text.replace("secure: false", "secure: true")
    lib.write_text(text)


def probe_build_failure_stop() -> dict:
    failed = run(
        ["journal", "show", "--filter", "slice.build.failed"],
        SANDBOX,
        check=False,
    )
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
    status = plan_status(SANDBOX)
    return {
        "journal_failed": failed.stdout.strip(),
        "in_progress": in_progress,
        "status": status,
    }


def main() -> int:
    setup_project(SANDBOX)
    survey_intent(
        SANDBOX,
        PLAN,
        LEADS,
        intent_value=(
            "Rate-limit and harden authentication: per-IP auth limits, password hash "
            "rotation, session cookie hardening, and retiring the legacy reset flow."
        ),
    )
    propose_slices(SANDBOX, SLICES)
    stamp_gate1(SANDBOX, PLAN)

    parked = main_lock_loop(
        SANDBOX,
        lead_map=LEAD_MAP,
        stop_on="build-failed",
    )
    if parked.get("action") != "stop" or parked.get("stop", {}).get("reason") != "build-failed":
        print(json.dumps({"error": "expected build-failed park", "status": parked}, indent=2))
        return 1
    if parked.get("slice") != "session-cookie-harden":
        print(json.dumps({"error": "expected park on session-cookie-harden", "status": parked}, indent=2))
        return 1
    probes = probe_build_failure_stop()
    print("PARK", json.dumps(probes, indent=2))

    patch_session_cookie_secure()

    breakout = with_plan_lock(
        SANDBOX,
        [
            sys.executable,
            "-c",
            (
                "import sys; sys.path.insert(0,%r); "
                "from execute_loop import build_slice; "
                "from pathlib import Path; "
                "build_slice(Path(%r), 'session-cookie-harden', secure_flag=True)"
            )
            % (str(DRIVERS), str(SANDBOX)),
        ],
    )
    if breakout.returncode != 0:
        print(breakout.stderr)
        return 1

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
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
