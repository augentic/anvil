#!/usr/bin/env python3
"""Single-repo `/spec:execute` replay helper — drives refine → build → merge under `specify plan lock --`.

Operator replay only; not wired into CI. Scenario entry points live beside this module.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any

_DRIVERS = Path(__file__).resolve().parent
FRAMEWORK = Path(os.environ.get("SPECIFY_FRAMEWORK", _DRIVERS.parents[1]))
MAIN = Path(__file__).resolve()
SPECIFY = os.environ.get("SPECIFY_BIN", os.environ.get("SPECIFY", "specify"))


def run(
    args: list[str],
    cwd: Path,
    *,
    check: bool = True,
    env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    merged = os.environ.copy()
    if env:
        merged.update(env)
    proc = subprocess.run(
        [SPECIFY, *args],
        cwd=cwd,
        text=True,
        capture_output=True,
        env=merged,
    )
    if check and proc.returncode != 0:
        raise RuntimeError(
            f"command failed ({proc.returncode}): specify {' '.join(args)}\n"
            f"stdout:\n{proc.stdout}\nstderr:\n{proc.stderr}"
        )
    return proc


def run_shell(cmd: str, cwd: Path, *, check: bool = True) -> subprocess.CompletedProcess[str]:
    proc = subprocess.run(cmd, cwd=cwd, text=True, capture_output=True, shell=True)
    if check and proc.returncode != 0:
        raise RuntimeError(f"shell failed ({proc.returncode}): {cmd}\n{proc.stderr}")
    return proc


def plan_status(cwd: Path) -> dict[str, Any]:
    proc = run(["plan", "status", "--format", "json"], cwd)
    return json.loads(proc.stdout)


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2) + "\n")


def setup_project(sandbox: Path) -> None:
    if sandbox.exists():
        import shutil

        shutil.rmtree(sandbox)
    sandbox.mkdir(parents=True)
    run(["init", str(FRAMEWORK / "adapters/targets/omnia")], sandbox)
    src = sandbox / "adapters/sources"
    src.mkdir(parents=True, exist_ok=True)
    intent_link = src / "intent"
    if intent_link.exists() or intent_link.is_symlink():
        intent_link.unlink()
    intent_link.symlink_to(FRAMEWORK / "adapters/sources/intent")
    run_shell("git init -b main -q", sandbox, check=False)


def survey_intent(cwd: Path, plan_name: str, leads: list[tuple[str, str]], *, intent_value: str) -> None:
    run(
        [
            "plan",
            "create",
            plan_name,
            "--source",
            f"intent=intent:value:{intent_value}",
        ],
        cwd,
    )
    run(["source", "survey", "intent", "--phase", "prepare", "--format", "json"], cwd)
    leads_md = "\n\n".join(
        f"### {lead}\n\n- lead: {lead}\n- synopsis: {synopsis}" for lead, synopsis in leads
    )
    scratch = cwd / ".specify/scratch/intent/survey/leads.md"
    scratch.parent.mkdir(parents=True, exist_ok=True)
    scratch.write_text(leads_md + "\n")
    run(["source", "survey", "intent", "--phase", "finalize"], cwd)


def propose_slices(cwd: Path, slices: list[dict[str, Any]]) -> None:
    run(["plan", "propose", "--dry-run", "--format", "json"], cwd)
    response = {"version": 1, "kind": "response", "slices": slices}
    path = cwd / ".specify/scratch/plan/propose-response.json"
    write_json(path, response)
    run(["plan", "propose", "--from", str(path)], cwd)


def stamp_gate1(cwd: Path, plan_name: str) -> None:
    run(["plan", "transition", plan_name, "approved", "--actor", "agent"], cwd)


def refine_slice(
    cwd: Path,
    slice_name: str,
    lead: str,
    statement: str,
    *,
    domain: str | None = None,
) -> None:
    domain = domain or slice_name
    run(["slice", "create", slice_name], cwd)
    run(
        [
            "source",
            "extract",
            "intent",
            lead,
            "--slice",
            slice_name,
            "--phase",
            "prepare",
        ],
        cwd,
    )
    evidence = {
        "authority": "intent",
        "lead": lead,
        "claims": [{"id": lead, "kind": "intent", "statement": statement}],
    }
    evidence_path = cwd / f".specify/scratch/intent/{slice_name}/evidence.yaml"
    write_json(evidence_path, evidence)
    run(
        [
            "source",
            "extract",
            "intent",
            lead,
            "--slice",
            slice_name,
            "--phase",
            "finalize",
        ],
        cwd,
    )
    run(["slice", "synthesize", slice_name, "--dry-run", "--format", "json"], cwd)
    response = {
        "version": 1,
        "kind": "response",
        "slice": slice_name,
        "model": {
            "requirements": [
                {
                    "title": statement,
                    "domain": domain,
                    "claims": [{"source": "intent", "id": lead, "kind": "intent"}],
                    "agreement": "agreed",
                    "statement": statement,
                    "scenarios": [
                        f"WHEN the service handles {slice_name} THEN behaviour matches the intent"
                    ],
                }
            ],
            "tasks": [
                {
                    "id": "TASK-001",
                    "text": f"Implement {slice_name} library surface.",
                    "satisfies": ["REQ-001"],
                },
                {
                    "id": "TASK-002",
                    "text": "Run cargo test to verify behaviour.",
                    "satisfies": ["REQ-001"],
                },
            ],
        },
        "artifacts": {
            "proposal": (
                f"# {slice_name}\n\n## Domains\n\n- {domain} — eval slice crate.\n"
            ),
            "design": f"# Design\n\nMinimal serde library for {slice_name}.\n",
            "tasks": (
                "# Tasks\n\n## 1. Implement\n\n"
                "- [ ] 1.1 Implement library surface.\n"
                "- [ ] 1.2 Run cargo test.\n"
            ),
            "specs": [
                {
                    "domain": domain,
                    "content": (
                        f"## Overview\n\n{statement}\n\n"
                        f"#### Scenario: {slice_name} behaviour\n\n"
                        f"WHEN the service handles {slice_name} "
                        "THEN behaviour matches the intent\n"
                    ),
                }
            ],
        },
    }
    synth_path = cwd / f".specify/scratch/{slice_name}/synthesize-response.json"
    write_json(synth_path, response)
    run(["slice", "synthesize", slice_name, "--from", str(synth_path)], cwd)
    run(["slice", "validate", slice_name], cwd, check=False)
    run(["slice", "transition", slice_name, "refined"], cwd)


def ensure_workspace_crate(cwd: Path, crate_name: str) -> None:
    cargo = cwd / "Cargo.toml"
    if not cargo.exists():
        cargo.write_text(
            '[workspace]\nmembers = []\nresolver = "2"\n',
        )
    text = cargo.read_text()
    member = f'crates/{crate_name}'
    if member not in text:
        if 'members = []' in text:
            text = text.replace('members = []', f'members = ["{member}"]')
        elif 'members = [' in text:
            text = text.replace('members = [', f'members = ["{member}", ')
        cargo.write_text(text)


def write_minimal_crate(
    cwd: Path,
    crate_name: str,
    *,
    secure_flag: bool | None = None,
    extra_test: str | None = None,
) -> Path:
    ensure_workspace_crate(cwd, crate_name)
    crate_dir = cwd / "crates" / crate_name
    crate_dir.mkdir(parents=True, exist_ok=True)
    (crate_dir / "Cargo.toml").write_text(
        f"""[package]
name = "{crate_name}"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = {{ version = "1", features = ["derive"] }}
"""
    )
    if secure_flag is None:
        lib = f"""pub fn marker() -> &'static str {{
    "{crate_name}"
}}
"""
    else:
        lib = f"""#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CookieConfig {{
    pub secure: bool,
}}

impl Default for CookieConfig {{
    fn default() -> Self {{
        Self {{ secure: {str(secure_flag).lower()} }}
    }}
}}

pub fn session_cookie_config() -> CookieConfig {{
    CookieConfig::default()
}}
"""
    (crate_dir / "src").mkdir(exist_ok=True)
    (crate_dir / "src/lib.rs").write_text(lib)
    tests_dir = crate_dir / "tests"
    tests_dir.mkdir(exist_ok=True)
    if secure_flag is None:
        test_body = f"""use {crate_name}::marker;

#[test]
fn marker_is_stable() {{
    assert_eq!(marker(), "{crate_name}");
}}
"""
    else:
        test_body = f"""use {crate_name}::session_cookie_config;

#[test]
fn session_cookie_secure_flag_set() {{
    assert!(session_cookie_config().secure);
}}
"""
    if extra_test:
        test_body += extra_test
    (tests_dir / "integration.rs").write_text(test_body)
    return crate_dir


def build_slice(
    cwd: Path,
    slice_name: str,
    *,
    secure_flag: bool | None = None,
    expect_failure: bool = False,
    stop_after_prepare: bool = False,
) -> int:
    crate_name = slice_name.replace("-", "_")
    run(["slice", "build", slice_name, "--phase", "prepare", "--format", "json"], cwd)
    if stop_after_prepare:
        return 0
    crate_dir = write_minimal_crate(cwd, crate_name, secure_flag=secure_flag)
    run(["slice", "task", "mark", slice_name, "1.1"], cwd, check=False)
    run_shell("cargo fmt", crate_dir)
    test = run_shell("cargo test 2>&1", crate_dir, check=False)
    log_path = cwd / f".specify/slices/{slice_name}/.build-log"
    log_path.parent.mkdir(parents=True, exist_ok=True)
    log_path.write_text(test.stdout + test.stderr)
    passed = test.returncode == 0
    report = {
        "version": 1,
        "slice": slice_name,
        "target": "omnia@v1",
        "status": "success" if passed else "failure",
        "findings": [],
    }
    report_path = cwd / f".specify/slices/{slice_name}/build/report.yaml"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(
        "\n".join(
            [
                "version: 1",
                f"slice: {slice_name}",
                "target: omnia@v1",
                f"status: {report['status']}",
                "findings: []",
                "",
            ]
        )
    )
    if passed:
        run(["slice", "task", "mark", slice_name, "1.2"], cwd, check=False)
    proc = run(
        ["slice", "build", slice_name, "--phase", "finalize", "--format", "json"],
        cwd,
        check=False,
    )
    if expect_failure:
        if proc.returncode == 0:
            raise RuntimeError("expected build finalize to fail")
        return proc.returncode
    if proc.returncode != 0:
        raise RuntimeError(f"build finalize failed:\n{proc.stderr}")
    return 0


def merge_slice(cwd: Path, slice_name: str) -> None:
    crate_name = slice_name.replace("-", "_")
    crate_dir = cwd / "crates" / crate_name
    if crate_dir.exists():
        run_shell("cargo fmt", crate_dir)
        run_shell("cargo test", crate_dir)
    run(["slice", "merge", "run", slice_name], cwd)


def execute_once(cwd: Path, *, lead_map: dict[str, str] | None = None) -> dict[str, Any]:
    status = plan_status(cwd)
    action = status.get("action")
    if action == "drained":
        return status
    if action == "stop":
        return status
    if action not in {"refine", "build", "merge"}:
        raise RuntimeError(f"unexpected action: {action}\n{status}")
    slice_name = status["slice"]
    run(["plan", "next", "--format", "json"], cwd)
    if action == "refine":
        if not lead_map:
            raise RuntimeError("lead_map required for refine")
        lead, statement = _slice_lead(cwd, slice_name, lead_map)
        refine_slice(cwd, slice_name, lead, statement)
    elif action == "build":
        secure = secure_for_slice(slice_name)
        expect_fail = slice_name == "session-cookie-harden" and secure is False
        code = build_slice(
            cwd,
            slice_name,
            secure_flag=secure,
            expect_failure=expect_fail,
        )
        if code != 0:
            return plan_status(cwd)
    elif action == "merge":
        merge_slice(cwd, slice_name)
    return plan_status(cwd)


def execute_loop(
    cwd: Path,
    *,
    max_steps: int = 40,
    stop_on: str | None = None,
    build_stop_after_prepare: str | None = None,
    lead_map: dict[str, str] | None = None,
) -> dict[str, Any]:
    for _ in range(max_steps):
        status = plan_status(cwd)
        if status.get("action") == "drained":
            return status
        if status.get("action") == "stop":
            if stop_on and status.get("stop", {}).get("reason") == stop_on:
                return status
            return status
        if (
            build_stop_after_prepare
            and status.get("action") == "build"
            and status.get("slice") == build_stop_after_prepare
        ):
            run(["plan", "next", "--format", "json"], cwd)
            build_slice(cwd, build_stop_after_prepare, stop_after_prepare=True)
            return plan_status(cwd)
        status = execute_once(cwd, lead_map=lead_map)
        if status.get("action") == "drained":
            return status
        if status.get("action") == "stop":
            if stop_on and status.get("stop", {}).get("reason") == stop_on:
                return status
            return status
    raise RuntimeError("execute loop exceeded max steps")


def with_plan_lock(cwd: Path, args: list[str]) -> subprocess.CompletedProcess[str]:
    return run(["plan", "lock", "--", *args], cwd, check=False)


def _slice_lead(cwd: Path, slice_name: str, lead_map: dict[str, str]) -> tuple[str, str]:
    lead = lead_map[slice_name]
    return lead, _lead_synopsis(cwd, lead)


def _lead_synopsis(cwd: Path, lead: str) -> str:
    text = (cwd / "discovery.md").read_text()
    for block in text.split("### "):
        if block.startswith(lead):
            for line in block.splitlines():
                if line.strip().startswith("- synopsis:"):
                    return line.split(":", 1)[1].strip()
    return lead.replace("-", " ")


def secure_for_slice(slice_name: str) -> bool | None:
    if slice_name == "session-cookie-harden":
        return False
    return None


def main_lock_loop(cwd: Path, **kwargs) -> dict[str, Any]:
    """Run execute_loop inside specify plan lock via a child Python process."""
    cmd = [
        sys.executable,
        str(MAIN),
        "loop",
        str(cwd),
        json.dumps(kwargs),
    ]
    proc = with_plan_lock(cwd, cmd)
    if proc.returncode != 0:
        # loop may intentionally exit on stop with code 0 from child; propagate body
        if proc.stdout.strip():
            try:
                return json.loads(proc.stdout)
            except json.JSONDecodeError:
                pass
        raise RuntimeError(f"lock loop failed:\n{proc.stderr}\n{proc.stdout}")
    return json.loads(proc.stdout)


if __name__ == "__main__":
    if len(sys.argv) < 3 or sys.argv[1] != "loop":
        print(f"usage: {MAIN.name} loop <cwd> [<json-kwargs>]", file=sys.stderr)
        sys.exit(2)
    cwd = Path(sys.argv[2])
    kwargs = json.loads(sys.argv[3]) if len(sys.argv) > 3 else {}
    result = execute_loop(cwd, **kwargs)
    print(json.dumps(result))
