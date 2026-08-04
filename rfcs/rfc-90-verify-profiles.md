# RFC-90: Verify Profiles

> Status: Draft — step 5 of the platform-migration series (scale track) ([platform.md](platform.md)). Host-owned verify is [RFC-91](rfc-91-concurrent-execution.md)'s convergence gate and [RFC-92](rfc-92-node-sync.md)'s trial-integration gate. Omnia's `wasi-model` host accepts the `verify` grant but its execution is stubbed today. Depends on completed [RFC-87](rfc-87-working-trees.md) · Owns: closed verification profiles and sandboxing

## Abstract

`verify` is the one native seam in the model tool loop. It compiles, checks, tests, or vets generated code that a `wasm32-wasip2` guest cannot safely execute itself. This RFC owns the closed check profiles, sandbox policy, argv ownership, report mapping, and capability signal for nodes that cannot verify.

## Closed profiles

The model may request a profile by name. It never supplies argv.

Initial profile names:

- `fmt`
- `build`
- `clippy`
- `test`
- `doc`
- `vet`
- `deny`
- `ci`

The host maps each profile to vetted commands for the target project type. For Rust / Omnia projects this mirrors `cargo make ci`, the `wasm32-wasip2` deploy build, and the relevant `cargo` checks. Profiles are versioned host policy, not prompt text.

## Sandbox policy

Every profile runs under native orchestration with:

- ephemeral working directories or equivalent isolation;
- egress denied unless a profile explicitly allows it;
- bounded CPU, memory, wall time, and output size;
- inherited secrets denied by default;
- profile-specific test gating for commands that execute generated code.

`test` and `ci` are higher-risk profiles and may require explicit policy gates before use in automated repair loops.

## Report mapping

The host normalizes command output into the shared report shape:

- compiler, lint, advisory, and test identifiers become `finding.rule-id`;
- severity maps into the existing must-fix vs optional tiers;
- locations are normalized to artifact-relative paths;
- raw command output is retained only as bounded diagnostic context.

The model receives the report, repairs if budget remains, and may request another verification pass through the model loop.

## Capability signal

Not every node can verify. A node without the required toolchain or sandbox support reports verification as unavailable. Build / merge briefs degrade explicitly on that signal instead of guessing or falling back to free-form commands.

## Scope

- Closed profile names and host-owned argv.
- Sandbox and resource policy for verification.
- Report normalization into shared findings.
- Capability signaling for toolchain-less nodes.
- Cache policy for build artifacts, such as `target/`, when safe.

## Fixed implementation cut

- The host selects a profile table from the approved target adapter and platform set in `plan.yaml.projects` / `project.yaml`; model output never selects commands or a toolchain.
- The first complete table is Omnia/Rust (`fmt`, `build`, `clippy`, `test`, `doc`, `vet`, `deny`, `ci`). Other target/platform combinations and undeclared profiles return typed `unavailable`; adding their tables is independent coverage work, not an RFC-90 phase.
- Verification runs in a disposable RFC-87 workspace prepared from the candidate result snapshot, never the authoritative tree.
- Egress, inherited environment, CPU, memory, wall time, and output limits are deny-by-default host policy. `test` and `ci` require the invocation's explicit execution grant; there is no persisted bypass file.
- Cache reuse is workspace-local and keyed by profile, toolchain identity, and snapshot id. Discard deletes the cache with the verification workspace.
- Normalization preserves the closed diagnostic substrate (`source: tool`, `kind: violation`) and attaches bounded raw output only when no structured parser exists.

## Delivery

Implement the closed request type, disposable verification workspace, sandbox/resource policy, execution grant, Omnia/Rust profile table, workspace-local cache keys, normalized reports, and typed unavailable signal as one vertical cut. RFC-90 is complete when that cut passes.

## Out of scope

- Model tool-call dispatch; owned by the implemented `wasi-model` host and its backends.
- Model backend selection; owned by the runtime binary's compile-time binding.
- Preparing work directories; see [RFC-87](rfc-87-working-trees.md).

## Acceptance criteria

1. The model can request only closed profile names; no model-supplied argv reaches the host.
2. Every profile runs inside the configured sandbox and resource limits.
3. Verification output maps to the shared `report` shape with stable severities.
4. Toolchain-less nodes expose a typed unavailable signal.
5. `test` / `ci` execution is gated by explicit host policy.
6. Omnia/Rust executes the declared profile table from the approved project target/platform binding; every unsupported target or profile fails typed.
7. Verification always uses a disposable RFC-87 tree and cannot modify authoritative source, target, or slice state.
8. `cargo make ci` is green in touched repositories with integration coverage for sandbox denial, resource/output bounds, every Omnia/Rust profile, unsupported targets, unavailable toolchains, cache isolation, and report normalization.

## Risks and invariants

- **`verify` executes untrusted code.** It must be treated as a security boundary, not as a convenience helper.
- **Closed profiles only.** Free-form command execution from model output is an RCE surface.
- **No silent degradation.** If verification is unavailable, the operation sees a typed capability signal.
- **Reports are bounded.** Large compiler or test output must be summarized and capped before entering the model loop.
