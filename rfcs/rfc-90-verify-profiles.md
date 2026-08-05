# RFC-90: Verify Profiles

> Status: Draft — step 5 of the platform-migration series, scale track ([platform.md](platform.md))
>
> Owns: predefined verification profiles, host-owned command selection, sandbox and resource policy, normalized reports, cache isolation, and the typed unavailable capability.
>
> Builds on completed [RFC-87](rfc-87-working-trees.md). Host-owned verification becomes [RFC-91](rfc-91-concurrent-execution.md)'s bottom-up conflict-domain convergence gate; [RFC-92](rfc-92-node-sync.md) transports the same gate without redefining it. Omnia's `wasi-model` host accepts the `verify` grant, but execution is stubbed today.



## Intent

Replace the build model's shell-based verification with a safe, host-owned way to compile, check, test, and vet generated code.

### Today

The Omnia build model runs prompt-specified Cargo commands inside its lent workspace. This protects the authoritative tree, but the backend agent still controls command execution, output handling, and repair inside an opaque loop.

Because the engine is a `wasm32-wasip2` guest, it cannot run the target project's native toolchain itself. Verification therefore depends on the backend agent's shell access rather than an enforceable, typed host capability.

### Goal

RFC-90 introduces predefined verification profiles with host-owned command selection, sandbox and resource policy, normalized findings, and typed unavailability.

RFC-90 does not primarily add new checks—the Omnia build model already runs several. It moves verification authority from an opaque agent shell to enforceable host policy, making verification isolated, bounded, reproducible, observable, and reusable as RFC-91's convergence gate and across RFC-92's remote nodes.

The build model requests a profile by name. The host selects vetted commands from the bound target and platform, runs them against an isolated candidate tree, and returns bounded findings to the model's repair loop.

This RFC delivers the complete Omnia/Rust path. Other target and platform combinations can add profile tables later without changing the contract.

## Flow and terms

1. The build model requests one predefined **profile**, such as `clippy` or `ci`; it supplies no argv or toolchain.
2. The host selects the profile table from the adapter bound in `plan.yaml.targets` and the platform set in that target's pinned `project.yaml`.
3. The host prepares a disposable RFC-87 **verification workspace** from the candidate result snapshot.
4. Native orchestration applies the profile's sandbox, execution grant, resource limits, and workspace-local cache policy, then runs the vetted commands.
5. The host normalizes tool output into the shared report. The model may repair the candidate and request another pass while its repair budget remains.

An **execution grant** is invocation-scoped permission to run a higher-risk profile. **Unavailable** is the typed result returned when the node lacks the required toolchain or sandbox support, or when the selected target/platform combination has no requested profile. It is not permission to improvise a command. A candidate may be a worker, slice, or single-target conflict-domain result; verification receives only its target binding and snapshot and therefore needs no knowledge of decomposition or lifecycle.

## Worked example

Suppose an Omnia-bound Rust target has a generated payment handler in its candidate result snapshot. The model requests:

```yaml
profile: clippy
```

It does not request `cargo clippy`, choose a Rust toolchain, or add flags. The host resolves the Omnia/Rust `clippy` policy from the target and platform binding, prepares a disposable workspace from that snapshot, and runs the policy's vetted command under its configured limits.

If Clippy reports an avoidable clone in `crates/payments/src/handler.rs`, the normalized finding carries the lint identifier as `finding.rule-id`, a stable must-fix or optional severity, and an artifact-relative location. The model can repair the candidate and request `clippy` again. The authoritative target tree and slice state remain untouched throughout.

If the same request reaches a node without the Rust toolchain, the host returns typed `unavailable`. The build or merge brief reports that limitation explicitly; it does not silently pass verification or fall back to a model-authored shell command.

## Decisions



### D1 — Profile names are predefined host-owned policy

The initial profile names are:

- `fmt`
- `build`
- `clippy`
- `test`
- `doc`
- `vet`
- `deny`
- `ci`

The request type admits only those names. The model never supplies argv, flags, a toolchain, or an executable path.

Profiles are versioned host policy rather than prompt text. This is a security invariant: free-form commands from model output would make `verify` an RCE surface.

### D2 — The bound target and platform select the command table

The host selects commands from the adapter bound in `plan.yaml.targets` and the declared platform set in that target's pinned `project.yaml`. Model output cannot select a different table.

The first complete table is Omnia/Rust. Its profiles mirror `cargo make ci`, the `wasm32-wasip2` deploy build, and the relevant Cargo checks. An undeclared profile or unsupported target/platform combination returns typed `unavailable`. Adding another table is independent coverage work, not another RFC-90 phase.

### D3 — Verification runs only in a disposable RFC-87 workspace

The host prepares verification from the candidate result snapshot, never from the authoritative tree. The workspace is disposable and isolated from authoritative source, target, and slice state.

RFC-87 owns workspace preparation. This RFC owns the requirement that every verification pass uses that facility and never writes verification results back into authoritative state.

### D4 — Native orchestration treats verification as untrusted execution

Every profile runs with deny-by-default host policy:

- egress is denied unless the profile explicitly allows it;
- inherited environment and secrets are denied by default;
- CPU, memory, wall time, and output size are bounded;
- commands that execute generated code receive profile-specific gating.

`test` and `ci` require the current invocation's explicit execution grant, including when used in an automated repair loop. There is no persisted bypass file.

### D5 — Cache reuse cannot escape the verification workspace

One verification workspace spans the complete verify-repair loop rooted at a candidate result snapshot. Build artifacts such as `target/` may be reused across repair passes and between profiles whose host policy declares compatible build configuration. Cache identity includes the workspace, toolchain identity, and effective profile policy; incompatible profiles receive isolated caches.

Discarding the verification workspace deletes its caches. No cache is shared across verification sessions or retained as an authority outside the workspace.

Profile execution replaces the prompt-driven checks it covers; it does not run beside duplicate model-owned commands. Workspace preparation, sandbox startup, and report normalization still add overhead, so completion records their cost and the effect of workspace-local reuse rather than assuming RFC-91's later concurrency will hide it.

### D6 — Tool output becomes bounded shared findings

The host maps compiler, lint, advisory, and test identifiers to `finding.rule-id`; maps severity to the existing must-fix and optional tiers; and normalizes locations to artifact-relative paths.

Every normalized diagnostic uses the closed substrate `source: tool` and `kind: violation`. Structured parsers are preferred. Bounded raw output is attached as diagnostic context only when no structured parser exists, so large compiler or test streams never enter the model loop unbounded.

### D7 — Verification may repeat within the model repair budget

The model receives the normalized report, repairs the candidate if budget remains, and may request another named profile through the existing model loop. Repeated passes remain in the same verification workspace so compatible build artifacts can be reused.

The repeated pass changes neither profile ownership nor sandbox policy. Every request is independently resolved and constrained by D1–D6.

### D8 — Nodes report unavailable capability explicitly

A node without the required toolchain or sandbox support returns typed `unavailable`. Build and merge briefs degrade explicitly on that signal instead of guessing, claiming success, or falling back to free-form commands.

Model tool-call dispatch remains owned by the implemented `wasi-model` host and its backends. Model backend selection remains the runtime binary's compile-time binding. Neither is redesigned here.

## Implementation requirements

- Implement the predefined verification request type and the full Omnia/Rust table for `fmt`, `build`, `clippy`, `test`, `doc`, `vet`, `deny`, and `ci`.
- Select the table only from the bound target adapter and platform set; return typed `unavailable` for unsupported combinations, undeclared profiles, missing toolchains, or missing sandbox support.
- Prepare one disposable RFC-87 workspace from the candidate result snapshot for the complete verify-repair loop, without modifying authoritative source, target, or slice state.
- Enforce deny-by-default egress and inherited environment, bounded CPU, memory, wall time, and output, plus invocation-scoped execution grants for `test` and `ci`.
- Reuse build artifacts only inside that verification workspace across repair passes and host-declared compatible profiles; isolate incompatible profile policies and delete every cache when the workspace is discarded.
- Replace covered prompt-driven checks rather than duplicating them, and record workspace preparation, command, cache reuse, and total wall time against the current loop.
- Normalize output into the shared report with `source: tool`, `kind: violation`, stable severity mapping, artifact-relative locations, and bounded fallback context.
- Deliver the request, workspace, policy, grants, profile table, cache, report mapping, and unavailable signal as one vertical cut. RFC-90 is complete only when that cut passes these acceptance criteria.



## Acceptance criteria

1. The model can request only predefined profile names; no model-supplied argv reaches the host.
2. Every profile runs inside the configured sandbox and resource limits.
3. Verification output maps to the shared `report` shape with stable severities.
4. Toolchain-less nodes expose a typed unavailable signal.
5. `test` and `ci` execution is gated by explicit host policy.
6. Omnia/Rust executes the declared profile table from the bound project target/platform binding; every unsupported target or profile fails typed.
7. Verification always uses a disposable RFC-87 tree and cannot modify authoritative source, target, or slice state.
8. `cargo make ci` is green in touched repositories with integration coverage for sandbox denial, resource and output bounds, every Omnia/Rust profile, unsupported targets, unavailable toolchains, cache isolation, and report normalization.
9. A `wasm-omnia-r9k` comparison records the current and host-owned loops' workspace preparation, command, cache reuse, and total wall time; the host-owned path does not duplicate covered prompt-driven checks, and repeated repair passes demonstrate workspace-local reuse.



## Rejected alternatives

- **Model-supplied commands or toolchains** — turn a validation request into an RCE surface and make policy depend on prompt output.
- **Running against the authoritative target tree** — lets checks mutate product or workflow state and makes failed verification difficult to discard.
- **Silent success or free-form fallback when verification is unavailable** — hides a missing capability from build and merge decisions.
- **A persisted grant or bypass file** — lets a past approval authorize later untrusted execution. Higher-risk grants remain invocation-scoped.
- **Caches shared across verification sessions or outside disposable workspaces** — risk stale or attacker-controlled build artifacts crossing candidate boundaries.
- **Unbounded raw command output in reports** — can exhaust the model context and bypass the stable diagnostic shape.

