#!/usr/bin/env python3
"""Headless workspace `/spec:execute` replay helper for eval scenarios."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import textwrap
from pathlib import Path

FRAMEWORK = Path(
    os.environ.get(
        "SPECIFY_FRAMEWORK",
        Path(__file__).resolve().parents[2],
    )
)
SPECIFY = os.environ.get("SPECIFY", "specify")
DOC_KEY = os.environ.get("DOC_KEY", "brief")


def run(
    args: list[str],
    *,
    cwd: Path,
    env: dict[str, str] | None = None,
    check: bool = True,
) -> subprocess.CompletedProcess[str]:
    merged = os.environ.copy()
    if env:
        merged.update(env)
    print(f"+ ({cwd}) {' '.join([SPECIFY, *args])}", flush=True)
    proc = subprocess.run(
        [SPECIFY, *args],
        cwd=cwd,
        env=merged,
        text=True,
        capture_output=True,
    )
    if proc.stdout:
        print(proc.stdout, end="", flush=True)
    if proc.stderr:
        print(proc.stderr, end="", file=sys.stderr, flush=True)
    if check and proc.returncode != 0:
        raise RuntimeError(f"command failed ({proc.returncode}): {' '.join(args)}")
    return proc


def run_lock(args: list[str], *, cwd: Path, env: dict[str, str] | None = None) -> None:
    run(["plan", "lock", "--", SPECIFY, *args], cwd=cwd, env=env)


def plan_status(cwd: Path) -> dict:
    proc = run(["plan", "status", "--format", "json"], cwd=cwd, check=True)
    return json.loads(proc.stdout)


def oauth_brief() -> str:
    return textwrap.dedent(
        """\
        # OAuth Login

        The platform needs OAuth login so mobile customers can sign in with an
        external identity provider.

        ## Participants

        - backend: owns token exchange and session creation
        - mobile: owns the sign-in screen and callback handling
        - identity-provider: external OAuth provider

        ## Contract

        Define a shared OAuth login contract before implementation begins.

        HTTP endpoints:

        1. POST /oauth/exchange
           - Request OAuthExchangeRequest:
             - provider: string, required, enum: apple, google
             - authorization_code: string, required
             - redirect_uri: string, required
             - code_verifier: string, required
           - 200 response OAuthSession:
             - access_token: string
             - refresh_token: string
             - expires_at: date-time
             - user_id: string
           - 400 ErrorResponse for invalid input
           - 401 ErrorResponse when the provider rejects the code

        2. POST /oauth/refresh
           - Request OAuthRefreshRequest:
             - refresh_token: string, required
           - 200 response OAuthSession
           - 401 ErrorResponse when the refresh token is invalid or expired

        ## Backend implementation

        The backend should validate requests, call the identity provider, create or
        update the local user session, and return the shared response contract.

        ## Mobile implementation

        The mobile client should present provider choices, launch the OAuth flow, handle
        the callback, and call the backend exchange endpoint using the shared contract.
        """
    )


def survey_leads(mode: str) -> str:
    if mode == "fail-resume":
        return textwrap.dedent(
            """\
            ### backend-implementation

            - lead: backend-implementation
            - synopsis: Backend validates OAuth requests, exchanges codes, and creates sessions.

            ### mobile-implementation

            - lead: mobile-implementation
            - synopsis: Mobile presents provider choices, runs OAuth flow, and calls backend exchange.
            """
        )
    return textwrap.dedent(
        """\
        ### oauth-contract

        - lead: oauth-contract
        - synopsis: Shared OAuth login HTTP contract for exchange and refresh endpoints.

        ### backend-implementation

        - lead: backend-implementation
        - synopsis: Backend validates OAuth requests, exchanges codes, and creates sessions.

        ### mobile-implementation

        - lead: mobile-implementation
        - synopsis: Mobile presents provider choices, runs OAuth flow, and calls backend exchange.
        """
    )


def propose_response(mode: str) -> dict:
    if mode == "fail-resume":
        return {
            "version": 1,
            "kind": "response",
            "slices": [
                {
                    "name": "auth-rotate",
                    "project": "backend",
                    "sources": [{"source": DOC_KEY, "lead": "backend-implementation"}],
                    "rationale": "Backend auth secret rotation slice parks on build failure.",
                },
                {
                    "name": "oauth-mobile",
                    "project": "mobile",
                    "sources": [{"source": DOC_KEY, "lead": "mobile-implementation"}],
                    "rationale": "Mobile OAuth sign-in UI and callback handling.",
                },
            ],
        }
    return {
        "version": 1,
        "kind": "response",
        "slices": [
            {
                "name": "oauth-contract",
                "project": "contracts",
                "sources": [{"source": DOC_KEY, "lead": "oauth-contract"}],
                "rationale": "Contract-first shared OAuth API surface.",
            },
            {
                "name": "oauth-backend",
                "project": "backend",
                "sources": [{"source": DOC_KEY, "lead": "backend-implementation"}],
                "depends-on": ["oauth-contract"],
                "rationale": "Backend token exchange against the shared contract.",
            },
            {
                "name": "oauth-mobile",
                "project": "mobile",
                "sources": [{"source": DOC_KEY, "lead": "mobile-implementation"}],
                "depends-on": ["oauth-contract"],
                "rationale": "Mobile OAuth UI consuming the shared contract.",
            },
        ],
    }


def evidence_yaml(lead: str, claim_id: str, statement: str) -> str:
    return textwrap.dedent(
        f"""\
        authority: documentation
        lead: {lead}
        claims:
          - kind: requirement
            id: {claim_id}
            path: docs/oauth-login.md#L1
            statement: "{statement}"
        """
    )


def synth_response(slice_name: str, lead: str, claim_id: str, statement: str, domain: str) -> dict:
    title = slice_name.replace("-", " ").title()
    return {
        "version": 1,
        "kind": "response",
        "slice": slice_name,
        "model": {
            "requirements": [
                {
                    "title": title,
                    "domain": domain,
                    "claims": [{"source": DOC_KEY, "id": claim_id, "kind": "requirement"}],
                    "statement": f"{statement}\n\n#### Scenario: Happy path\n\n- GIVEN the system\n- WHEN invoked\n- THEN it succeeds",
                }
            ],
            "tasks": [
                {
                    "id": "TASK-001",
                    "text": f"Implement {title}.",
                    "satisfies": ["REQ-001"],
                }
            ],
        },
        "artifacts": {
            "proposal": f"# {title}\n\n## Why\n\n{statement}\n\n## Domains\n\n- {domain} — {statement}\n",
            "design": f"# Design\n{statement}\n",
            "tasks": f"# Tasks\n\n## 1. Implementation\n\n- [ ] 1.1 TASK-001 Implement {title}.\n",
            "specs": [{"domain": domain, "content": f"## {title}\n{statement}\n"}],
        },
    }


def build_report(slice_name: str, target: str, *, status: str = "success", outputs: list[dict] | None = None) -> str:
    lines = [
        "version: 1",
        f"slice: {slice_name}",
        f"target: {target}",
        f"status: {status}",
        "findings: []",
    ]
    if outputs:
        lines.append("outputs:")
        for item in outputs:
            lines.append(f"  - platform: {item['platform']}")
            lines.append(f"    path: {item['path']}")
    return "\n".join(lines) + "\n"


def scaffold_mobile_platforms(mobile_dir: Path) -> None:
    (mobile_dir / "shared/src").mkdir(parents=True, exist_ok=True)
    (mobile_dir / "shared/src/app.rs").write_text("pub struct App;\n")
    (mobile_dir / "iOS").mkdir(parents=True, exist_ok=True)
    (mobile_dir / "iOS/App.swift").write_text("import SwiftUI\nstruct App: SwiftUI.App { var body: some Scene { WindowGroup { Text(\"App\") } } }\n")
    android = mobile_dir / "Android/app/src/main/kotlin/com/example/app"
    android.mkdir(parents=True, exist_ok=True)
    (android / "App.kt").write_text("package com.example.app\nclass App\n")


def git(args: list[str], *, cwd: Path | None = None, check: bool = True) -> None:
    cmd = ["git", *args]
    if cwd is not None:
        cmd = ["git", "-C", str(cwd), *args]
    proc = subprocess.run(cmd, text=True, capture_output=True)
    if proc.stdout:
        print(proc.stdout, end="")
    if proc.stderr:
        print(proc.stderr, end="", file=sys.stderr)
    if check and proc.returncode != 0:
        raise RuntimeError(f"git failed ({proc.returncode}): {' '.join(args)}")


def setup_git_remotes(sandbox: Path, projects: list[str]) -> None:
    for proj in projects:
        root = sandbox / proj
        git(["init", "-b", "main"], cwd=root, check=False)
        subprocess.run(["git", "-C", str(root), "add", "-A"], check=False)
        subprocess.run(
            ["git", "-C", str(root), "diff", "--cached", "--quiet"],
            check=False,
        )
        subprocess.run(
            ["git", "-C", str(root), "commit", "-q", "--no-gpg-sign", "-m", f"init {proj}"],
            check=False,
        )
        bare = sandbox / f"{proj}-origin.git"
        subprocess.run(["git", "init", "--bare", "-q", str(bare)], check=True)
        subprocess.run(
            ["git", "-C", str(root), "remote", "add", "origin", f"file://{sandbox}/{proj}-origin.git"],
            check=True,
        )
        subprocess.run(["git", "-C", str(root), "push", "-q", "-u", "origin", "main"], check=True)


def setup_workspace(sandbox: Path, projects: list[str], *, scaffold_mobile: bool) -> Path:
    if sandbox.exists():
        import shutil

        shutil.rmtree(sandbox)
    for name in ["platform", *projects]:
        (sandbox / name).mkdir(parents=True)

    platform = sandbox / "platform"
    run(["init", "--workspace"], cwd=platform)
    (platform / "adapters/sources").mkdir(parents=True, exist_ok=True)
    doc_src = platform / "adapters/sources/documentation"
    if not doc_src.exists():
        doc_src.symlink_to(FRAMEWORK / "adapters/sources/documentation")

    adapters = {
        "backend": FRAMEWORK / "adapters/targets/omnia",
        "mobile": FRAMEWORK / "adapters/targets/vectis",
        "contracts": FRAMEWORK / "adapters/targets/contracts",
    }
    for proj in projects:
        init_args = ["init", str(adapters[proj])]
        if proj == "mobile":
            init_args += ["--platforms", "core,ios,android"]
        run(init_args, cwd=sandbox / proj)
        if proj == "mobile" and scaffold_mobile:
            scaffold_mobile_platforms(sandbox / proj)

    setup_git_remotes(sandbox, projects)

    descriptions = {
        "backend": "Omnia backend service for OAuth token exchange, sessions, and provider integration.",
        "mobile": "Vectis mobile client for OAuth sign-in UI, callback handling, and API consumption.",
        "contracts": "Shared OAuth login API contracts for cross-repo consumption.",
    }
    for proj in projects:
        run(
            [
                "registry",
                "add",
                proj,
                "--url",
                f"../{proj}",
                "--adapter",
                {"backend": "omnia", "mobile": "vectis", "contracts": "contracts"}[proj],
                "--description",
                descriptions[proj],
            ],
            cwd=platform,
        )
    run(["registry", "validate"], cwd=platform)

    docs = platform / "docs"
    docs.mkdir(exist_ok=True)
    (docs / "oauth-login.md").write_text(oauth_brief())
    return platform


def write_survey_leads(platform: Path, mode: str) -> None:
    proc = run(["source", "survey", DOC_KEY, "--phase", "prepare", "--format", "json"], cwd=platform)
    handoff = json.loads(proc.stdout)
    scratch = Path(handoff["scratch-dir"])
    (scratch / "leads.md").write_text(survey_leads(mode))
    run(["source", "survey", DOC_KEY, "--phase", "finalize"], cwd=platform)


def write_change_md(platform: Path, plan_name: str) -> None:
    (platform / "change.md").write_text(
        textwrap.dedent(
            f"""\
            # {plan_name}

            OAuth login across backend, mobile, and contracts projects.

            ## Cross-cutting leads

            None.
            """
        )
    )


def create_plan(platform: Path, mode: str) -> str:
    plan_name = "oauth-login"
    run(
        [
            "plan",
            "create",
            plan_name,
            "--source",
            f"{DOC_KEY}=documentation:docs/oauth-login.md",
        ],
        cwd=platform,
    )
    run(["workspace", "sync"], cwd=platform)
    write_survey_leads(platform, mode)
    write_change_md(platform, plan_name)
    response_path = platform / ".specify/scratch/plan/propose-response.json"
    response_path.parent.mkdir(parents=True, exist_ok=True)
    response_path.write_text(json.dumps(propose_response(mode), indent=2))
    args = ["plan", "propose", "--from", str(response_path)]
    if mode != "fail-resume":
        args.append("--reconcile-platforms")
    run(args, cwd=platform)
    run(["plan", "validate"], cwd=platform)
    return plan_name


def approve_plan(platform: Path, plan_name: str) -> None:
    run(["plan", "transition", plan_name, "approved", "--actor", "agent"], cwd=platform)


def resolve_target(project: str) -> str:
    return {"backend": "omnia", "mobile": "vectis", "contracts": "contracts"}[project]


def route_to_slot(
    platform: Path,
    project: str,
    plan_name: str,
    *,
    slice_name: str | None = None,
) -> tuple[Path, dict[str, str]]:
    run(["workspace", "sync", project], cwd=platform)
    proc = run(
        ["workspace", "prepare", project, "--change", plan_name],
        cwd=platform,
        check=False,
    )
    slot = platform / "workspace" / project
    if proc.returncode != 0 and slice_name and "dirty-unrelated-tracked" in (proc.stdout + proc.stderr):
        commit_residue(slot, slice_name)
        run(["workspace", "prepare", project, "--change", plan_name], cwd=platform)
    elif proc.returncode != 0:
        raise RuntimeError(f"workspace prepare failed for {project}")
    env = {"SPECIFY_PLAN_DIR": str(platform)}
    return slot, env


def commit_residue(slot: Path, slice_name: str) -> None:
    subprocess.run(["git", "-C", str(slot), "add", "-A"], check=False)
    subprocess.run(
        ["git", "-C", str(slot), "commit", "-q", "--no-gpg-sign", "-m", f"specify: residue {slice_name}"],
        check=False,
    )


def seed_contracts_input(slot: Path, slice_name: str) -> None:
    contracts_dir = slot / f".specify/slices/{slice_name}/contracts"
    contracts_dir.mkdir(parents=True, exist_ok=True)
    (contracts_dir / "openapi.yaml").write_text(
        "openapi: 3.1.0\ninfo:\n  title: OAuth Login\n  version: 1.0.0\npaths: {}\n"
    )


def seed_auth_rotate_failure(slot: Path, slice_name: str) -> None:
    crate = slot / "crates/auth_rotate"
    crate.mkdir(parents=True, exist_ok=True)
    (crate / "Cargo.toml").write_text(
        textwrap.dedent(
            """\
            [package]
            name = "auth_rotate"
            version = "0.1.0"
            edition = "2021"

            [dependencies]

            [dev-dependencies]
            """
        )
    )
    (crate / "src").mkdir(exist_ok=True)
    (crate / "src/lib.rs").write_text(
        textwrap.dedent(
            """\
            pub fn session_cookie_secure() -> bool {
                false
            }

            #[cfg(test)]
            mod tests {
                use super::*;

                #[test]
                fn session_cookie_secure_flag_set() {
                    assert!(session_cookie_secure());
                }
            }
            """
        )
    )


def fix_auth_rotate(slot: Path) -> None:
    lib = slot / "crates/auth_rotate/src/lib.rs"
    lib.write_text(
        textwrap.dedent(
            """\
            pub fn session_cookie_secure() -> bool {
                true
            }

            #[cfg(test)]
            mod tests {
                use super::*;

                #[test]
                fn session_cookie_secure_flag_set() {
                    assert!(session_cookie_secure());
                }
            }
            """
        )
    )


def synth_response_bootstrap(slice_name: str, domain: str, statement: str) -> dict:
    title = slice_name.replace("-", " ").title()
    return {
        "version": 1,
        "kind": "response",
        "slice": slice_name,
        "model": {
            "requirements": [
                {
                    "title": title,
                    "domain": domain,
                    "claims": [],
                    "statement": f"{statement}\n\n#### Scenario: Bootstrap\n\n- GIVEN a greenfield mobile project\n- WHEN bootstrap runs\n- THEN core, iOS, and Android shells exist",
                }
            ],
            "tasks": [
                {
                    "id": "TASK-001",
                    "text": f"Scaffold {title} shells.",
                    "satisfies": ["REQ-001"],
                }
            ],
        },
        "artifacts": {
            "proposal": f"# {title}\n\n## Why\n\n{statement}\n\n## Domains\n\n- {domain} — bootstrap shell trees for core, iOS, and Android.\n\n## Platforms\n\n- core\n- ios\n- android\n",
            "design": f"# Design\n{statement}\n",
            "tasks": f"# Tasks\n\n## 1. Implementation\n\n- [ ] 1.1 TASK-001 Scaffold {title} shells.\n",
            "specs": [{"domain": domain, "content": f"## {title}\n{statement}\n"}],
        },
    }


def slice_sources(platform: Path, slice_name: str) -> list[tuple[str, str]]:
    lines = (platform / "plan.yaml").read_text().splitlines()
    in_slice = False
    in_sources = False
    out: list[tuple[str, str]] = []
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("- name:"):
            current = stripped.split(":", 1)[1].strip()
            in_slice = current == slice_name
            in_sources = False
            continue
        if not in_slice:
            continue
        if stripped == "sources: []":
            return []
        if stripped == "sources:":
            in_sources = True
            continue
        if in_sources and stripped.startswith("- source:"):
            source = stripped.split(":", 1)[1].strip()
            out.append((source, slice_name))
            continue
        if in_sources and stripped.startswith("lead:"):
            lead = stripped.split(":", 1)[1].strip()
            if out:
                src = out[-1][0]
                out[-1] = (src, lead)
            continue
        if stripped.startswith("- name:") or (stripped and not line.startswith(" ") and stripped.endswith(":")):
            if out or in_sources:
                break
    return out


def drive_slice_iteration(platform: Path, slice_name: str, *, fail: bool = False) -> None:
    os.environ["SPECIFY_PLAN_LOCK_HELD"] = "1"
    project, lead, claim_id, statement, domain = SLICE_META[slice_name]
    run(["plan", "next"], cwd=platform)
    refine_slice(platform, slice_name, project, lead, claim_id, statement, domain, under_lock=True)
    slot = platform / "workspace" / project
    commit_residue(slot, slice_name)
    build_slice(platform, slice_name, project, fail=fail, under_lock=True)
    if fail:
        return
    commit_residue(slot, slice_name)
    merge_slice(platform, slice_name, project, under_lock=True)


def refine_slice(
    platform: Path,
    slice_name: str,
    project: str,
    lead: str,
    claim_id: str,
    statement: str,
    domain: str,
    *,
    under_lock: bool = False,
) -> None:
    slot, env = route_to_slot(platform, project, "oauth-login", slice_name=slice_name)
    target = resolve_target(project)
    create = ["slice", "create", slice_name, "--target", target]
    if under_lock:
        run(create, cwd=slot, env=env)
    else:
        run_lock(create, cwd=slot, env=env)
    bindings = slice_sources(platform, slice_name)
    if bindings:
        for source_key, binding_lead in bindings:
            proc = run(
                ["source", "extract", source_key, binding_lead, "--slice", slice_name, "--phase", "prepare", "--format", "json"],
                cwd=slot,
                env=env,
            )
            handoff = json.loads(proc.stdout)
            scratch = Path(handoff["scratch-dir"])
            (scratch / "evidence.yaml").write_text(
                evidence_yaml(binding_lead, claim_id, statement)
            )
            run(
                ["source", "extract", source_key, binding_lead, "--slice", slice_name, "--phase", "finalize"],
                cwd=slot,
                env=env,
            )
        synth_path = slot / "synth.json"
        synth_path.write_text(
            json.dumps(synth_response(slice_name, lead, claim_id, statement, domain), indent=2)
        )
    else:
        synth_path = slot / "synth.json"
        synth_path.write_text(json.dumps(synth_response_bootstrap(slice_name, domain, statement), indent=2))
    run(["slice", "synthesize", slice_name, "--from", str(synth_path), "--format", "json"], cwd=slot, env=env)
    run(["slice", "validate", slice_name], cwd=slot, env=env)
    if target == "contracts":
        seed_contracts_input(slot, slice_name)
    if slice_name == "auth-rotate":
        seed_auth_rotate_failure(slot, slice_name)
    run(["slice", "transition", slice_name, "refined"], cwd=slot, env=env)


def build_slice(
    platform: Path,
    slice_name: str,
    project: str,
    *,
    fail: bool = False,
    stop_after_prepare: bool = False,
    dirty_marker: str | None = None,
    under_lock: bool = False,
) -> None:
    slot, env = route_to_slot(platform, project, "oauth-login", slice_name=slice_name)
    if dirty_marker:
        (slot / dirty_marker).write_text("dirty\n")
    run(["slice", "build", slice_name, "--phase", "prepare", "--format", "json"], cwd=slot, env=env)
    if stop_after_prepare:
        return
    target = resolve_target(project)
    if fail:
        report = build_report(slice_name, f"{target}@v1", status="failure")
    elif target == "vectis":
        (slot / "shared/src").mkdir(parents=True, exist_ok=True)
        (slot / "shared/src/app.rs").write_text("pub struct App;\n")
        (slot / "iOS").mkdir(parents=True, exist_ok=True)
        (slot / "iOS/App.swift").write_text("import SwiftUI\n")
        android = slot / "Android/app/src/main/kotlin/com/example/app"
        android.mkdir(parents=True, exist_ok=True)
        (android / "App.kt").write_text("package com.example.app\nclass App\n")
        report = build_report(
            slice_name,
            f"{target}@v1",
            outputs=[
                {"platform": "core", "path": "shared/src"},
                {"platform": "ios", "path": "iOS"},
                {"platform": "android", "path": "Android"},
            ],
        )
    else:
        report = build_report(slice_name, f"{target}@v1")
    report_path = slot / f".specify/slices/{slice_name}/build/report.yaml"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(report)
    proc = run(
        ["slice", "build", slice_name, "--phase", "finalize", "--format", "json"],
        cwd=slot,
        env=env,
        check=not fail,
    )
    if fail:
        assert proc.returncode != 0


def merge_slice(platform: Path, slice_name: str, project: str, *, under_lock: bool = False) -> None:
    slot = platform / "workspace" / project
    env = {"SPECIFY_PLAN_DIR": str(platform)}
    if not under_lock:
        slot, env = route_to_slot(platform, project, "oauth-login", slice_name=slice_name)
    else:
        commit_residue(slot, slice_name)
    # Stage all slice-boundary changes so branch preparation does not see dirty-unrelated-tracked.
    subprocess.run(["git", "-C", str(slot), "add", "-A"], check=False)
    subprocess.run(
        ["git", "-C", str(slot), "commit", "-q", "--no-gpg-sign", "-m", f"specify: pre-merge {slice_name}"],
        check=False,
    )
    if under_lock:
        run(["slice", "merge", "run", slice_name], cwd=slot, env=env)
    else:
        run_lock(["slice", "merge", "run", slice_name], cwd=slot, env=env)
    commit_residue(slot, slice_name)


SLICE_META = {
    "oauth-contract": ("contracts", "oauth-contract", "oauth-contract-req", "Shared OAuth contract.", "oauth"),
    "oauth-backend": ("backend", "backend-implementation", "backend-impl-req", "Backend OAuth exchange.", "oauth"),
    "oauth-mobile": ("mobile", "mobile-implementation", "mobile-impl-req", "Mobile OAuth sign-in.", "oauth"),
    "auth-rotate": ("backend", "backend-implementation", "auth-rotate-req", "Rotate auth secrets.", "auth"),
    "app-foundation": ("mobile", "mobile-implementation", "mobile-bootstrap-req", "Bootstrap mobile shells.", "app"),
}


def drive_slice(platform: Path, slice_name: str, *, build_fail: bool = False) -> None:
    project, lead, claim_id, statement, domain = SLICE_META[slice_name]
    run_lock(["plan", "next"], cwd=platform)
    refine_slice(platform, slice_name, project, lead, claim_id, statement, domain)
    build_slice(platform, slice_name, project, fail=build_fail)
    if build_fail:
        return
    merge_slice(platform, slice_name, project)


def execute_loop(platform: Path, *, build_fail_slice: str | None = None) -> None:
    driver = Path(__file__).resolve()
    while True:
        status = plan_status(platform)
        action = status.get("action")
        if action == "drained":
            print("drained")
            return
        if action == "stop":
            print(json.dumps(status, indent=2))
            return
        slice_name = status.get("slice") or status.get("entry")
        if not slice_name:
            raise RuntimeError(f"unexpected status: {status}")
        fail_flag = "fail" if slice_name == build_fail_slice else "ok"
        run(
            [
                "plan",
                "lock",
                "--",
                sys.executable,
                str(driver),
                "_iterate",
                str(platform),
                slice_name,
                fail_flag,
            ],
            cwd=platform,
        )
        if slice_name == build_fail_slice:
            return


def drive_build_only(platform: Path, slice_name: str) -> None:
    os.environ["SPECIFY_PLAN_LOCK_HELD"] = "1"
    project = SLICE_META[slice_name][0]
    build_slice(platform, slice_name, project, fail=False, under_lock=True)


def breakout_build(platform: Path, slice_name: str) -> None:
    project = SLICE_META[slice_name][0]
    slot = platform / "workspace" / project
    fix_auth_rotate(slot)
    subprocess.run(["git", "-C", str(slot), "add", "-A"], check=False)
    subprocess.run(
        ["git", "-C", str(slot), "commit", "-q", "--no-gpg-sign", "-m", "specify: triage auth-rotate build failure"],
        check=False,
    )
    run(
        [
            "plan",
            "lock",
            "--",
            sys.executable,
            str(Path(__file__).resolve()),
            "_build-only",
            str(platform),
            slice_name,
        ],
        cwd=platform,
    )


def resume_after_park(platform: Path) -> None:
    driver = Path(__file__).resolve()
    while True:
        status = plan_status(platform)
        action = status.get("action")
        if action == "drained":
            print("drained")
            return
        if action == "stop":
            raise RuntimeError(f"still stopped: {status}")
        slice_name = status.get("slice") or status.get("entry")
        project = SLICE_META[slice_name][0]
        slot = platform / "workspace" / project
        slice_meta_path = slot / f".specify/slices/{slice_name}/metadata.yaml"
        if slice_meta_path.exists():
            slice_meta = slice_meta_path.read_text()
            if "status: built" in slice_meta:
                run(
                    [
                        "plan",
                        "lock",
                        "--",
                        sys.executable,
                        str(driver),
                        "_merge-only",
                        str(platform),
                        slice_name,
                    ],
                    cwd=platform,
                )
                continue
            if "status: refined" in slice_meta and (slot / f".specify/slices/{slice_name}/build/request.yaml").exists():
                run(
                    [
                        "plan",
                        "lock",
                        "--",
                        sys.executable,
                        str(driver),
                        "_build-merge",
                        str(platform),
                        slice_name,
                    ],
                    cwd=platform,
                )
                continue
        run(
            [
                "plan",
                "lock",
                "--",
                sys.executable,
                str(driver),
                "_iterate",
                str(platform),
                slice_name,
                "ok",
            ],
            cwd=platform,
        )


def drive_build_merge(platform: Path, slice_name: str) -> None:
    os.environ["SPECIFY_PLAN_LOCK_HELD"] = "1"
    project = SLICE_META[slice_name][0]
    build_slice(platform, slice_name, project, fail=False, under_lock=True)
    slot, _ = route_to_slot(platform, project, "oauth-login", slice_name=slice_name)
    commit_residue(slot, slice_name)
    merge_slice(platform, slice_name, project, under_lock=True)


def drive_merge_only(platform: Path, slice_name: str) -> None:
    os.environ["SPECIFY_PLAN_LOCK_HELD"] = "1"
    project = SLICE_META[slice_name][0]
    merge_slice(platform, slice_name, project, under_lock=True)


def interrupt_mid_build(platform: Path, slice_name: str) -> None:
    run(
        [
            "plan",
            "lock",
            "--",
            sys.executable,
            str(Path(__file__).resolve()),
            "_interrupt",
            str(platform),
            slice_name,
        ],
        cwd=platform,
    )


def drive_interrupt(platform: Path, slice_name: str) -> None:
    os.environ["SPECIFY_PLAN_LOCK_HELD"] = "1"
    project, lead, claim_id, statement, domain = SLICE_META[slice_name]
    run(["plan", "next"], cwd=platform)
    refine_slice(platform, slice_name, project, lead, claim_id, statement, domain, under_lock=True)
    slot, env = route_to_slot(platform, project, "oauth-login", slice_name=slice_name)
    commit_residue(slot, slice_name)
    build_slice(
        platform,
        slice_name,
        project,
        stop_after_prepare=True,
        dirty_marker="eval-dirty-uncommitted.txt",
        under_lock=True,
    )


def main() -> None:
    if len(sys.argv) >= 2 and sys.argv[1] == "_build-only":
        platform = Path(sys.argv[2])
        slice_name = sys.argv[3]
        drive_build_only(platform, slice_name)
        return

    if len(sys.argv) >= 2 and sys.argv[1] == "_build-merge":
        platform = Path(sys.argv[2])
        slice_name = sys.argv[3]
        drive_build_merge(platform, slice_name)
        return

    if len(sys.argv) >= 2 and sys.argv[1] == "_merge-only":
        platform = Path(sys.argv[2])
        slice_name = sys.argv[3]
        drive_merge_only(platform, slice_name)
        return

    if len(sys.argv) >= 2 and sys.argv[1] == "_interrupt":
        platform = Path(sys.argv[2])
        slice_name = sys.argv[3]
        drive_interrupt(platform, slice_name)
        return

    if len(sys.argv) >= 2 and sys.argv[1] == "_iterate":
        platform = Path(sys.argv[2])
        slice_name = sys.argv[3]
        fail = sys.argv[4] == "fail"
        drive_slice_iteration(platform, slice_name, fail=fail)
        return

    if len(sys.argv) < 2:
        print("usage: workspace.py <scenario>", file=sys.stderr)
        sys.exit(2)
    scenario = sys.argv[1]
    sandbox = FRAMEWORK / "evals/.sandbox" / scenario
    if scenario == "workspace-two-projects":
        platform = setup_workspace(sandbox, ["backend", "mobile", "contracts"], scaffold_mobile=False)
        plan = create_plan(platform, "full")
        approve_plan(platform, plan)
        execute_loop(platform)
    elif scenario == "workspace-fail-resume":
        platform = setup_workspace(sandbox, ["backend", "mobile"], scaffold_mobile=True)
        plan = create_plan(platform, "fail-resume")
        approve_plan(platform, plan)
        execute_loop(platform, build_fail_slice="auth-rotate")
        breakout_build(platform, "auth-rotate")
        resume_after_park(platform)
    elif scenario == "workspace-stale-recovery":
        platform = setup_workspace(sandbox, ["backend", "mobile", "contracts"], scaffold_mobile=False)
        plan = create_plan(platform, "full")
        approve_plan(platform, plan)
        # Complete contract + app-foundation first.
        for _ in range(2):
            status = plan_status(platform)
            done_slice = status.get("slice") or status.get("entry")
            run(
                [
                    "plan",
                    "lock",
                    "--",
                    sys.executable,
                    str(Path(__file__).resolve()),
                    "_iterate",
                    str(platform),
                    done_slice,
                    "ok",
                ],
                cwd=platform,
            )
        interrupt_mid_build(platform, "oauth-backend")
        run(["workspace", "sync"], cwd=platform)
        slot = platform / "workspace/backend"
        subprocess.run(["git", "-C", str(slot), "rm", "-f", "eval-dirty-uncommitted.txt"], check=False)
        subprocess.run(["git", "-C", str(slot), "add", "-A"], check=False)
        subprocess.run(
            ["git", "-C", str(slot), "commit", "-q", "--no-gpg-sign", "-m", "specify: triage stale oauth-backend"],
            check=False,
        )
        resume_after_park(platform)
    else:
        raise SystemExit(f"unknown scenario: {scenario}")


if __name__ == "__main__":
    main()
