# RFC-60: Verify Profiles

> **Status: Deferred.** Omnia's `wasi-model` host accepts the `verify` grant but its execution is stubbed; generated-code verification runs through the adapters' existing native check flows. Revisit when verification moves into the model loop. Depends: [RFC-55](rfc-55-working-tree.md) · Owns: closed verification profiles and sandboxing

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

## Out of scope

- Model tool-call dispatch; owned by the implemented `wasi-model` host and its backends.
- Model backend selection; owned by the runtime binary's compile-time binding.
- Working-tree materialization; see [RFC-55](rfc-55-working-tree.md).

## Acceptance criteria

1. The model can request only closed profile names; no model-supplied argv reaches the host.
2. Every profile runs inside the configured sandbox and resource limits.
3. Verification output maps to the shared `report` shape with stable severities.
4. Toolchain-less nodes expose a typed unavailable signal.
5. `test` / `ci` execution is gated by explicit host policy.

## Risks and invariants

- **`verify` executes untrusted code.** It must be treated as a security boundary, not as a convenience helper.
- **Closed profiles only.** Free-form command execution from model output is an RCE surface.
- **No silent degradation.** If verification is unavailable, the operation sees a typed capability signal.
- **Reports are bounded.** Large compiler or test output must be summarized and capped before entering the model loop.
