# RFC-5 Follow-ups: Items to Nail Down Before Phase 3

> Companion to [rfc-5-tooling.md](rfc-5-tooling.md). Captures clarifications that should be resolved between phase 2 (rule-engine scaffold) and phase 3 (porting actual checks). None of these are objections to the framing; each is a detail whose absence will be re-litigated in PR review or contributor onboarding if it isn't codified now.

## 1. Cross-repo Cargo dep mechanics

**Issue.** RFC §"Workspace layout" says: "git dep pinned to a tag for releases, with a `[patch.crates-io]` override available for local sibling-checkout development." That stacks three options on top of each other without saying which one is canonical.

**Recommendation.** Pick **committed `[patch.crates-io]` with a path override that activates when the sibling checkout exists**, mirroring how `SPECIFY_CLI_DIR=../specify-cli` works for the Deno scripts today.

```toml
# tooling/Cargo.toml
[workspace.dependencies]
specify-domain = { git = "https://github.com/augentic/specify-cli.git", tag = "v<X.Y.Z>" }
specify-error  = { git = "https://github.com/augentic/specify-cli.git", tag = "v<X.Y.Z>" }

[patch."https://github.com/augentic/specify-cli.git"]
specify-domain = { path = "../../specify-cli/crates/domain" }
specify-error  = { path = "../../specify-cli/crates/error" }
```

The `[patch]` block is committed and resolves only when `../../specify-cli` exists; CI checks out both repos as it does today and gets the path override "for free." Releases pin the tag; `scripts/bump-specify-cli` (one-line `sed` over `tag = "..."`) handles the bump as a normal PR.

**Rejected alternatives.** Pure git rev/tag with no patch (forces every local contributor to run a private `cargo` patch); publishing `specify-domain` to crates.io (premature — adds release ceremony, exposes internal API, and the workspace genuinely is dev-time tooling that consumes unstable surfaces).

**Where to write it.** A new §"Cross-repo dependency" subsection under §"Workspace layout".

## 2. `--changed` correctness

**Issue.** Several rules are inherently global — marketplace ↔ plugins consistency, codex namespace ownership (the RFC-28 contract), duplicate-id checks, symlink integrity. RFC §"`tooling check`" says predicates "may fall back to a full-repo predicate when a rule is inherently global" but doesn't say *who* decides per rule, leaving the per-predicate dependency context implicit.

**Recommendation.** Add a `changed_strategy` method to the `Check` trait with a closed enum and a safe default of `FullRepo`:

```rust
pub enum ChangedStrategy {
    /// Rule is inherently global; --changed falls back to a full scan.
    FullRepo,
    /// Rule is path-local; runs only over `changed ∪ extra_paths`.
    Restricted { extra_paths: fn(&Context) -> Vec<PathBuf> },
}

pub trait Check {
    fn id(&self) -> &'static str;
    fn run(&self, ctx: &Context) -> Vec<Finding>;
    fn changed_strategy(&self) -> ChangedStrategy { ChangedStrategy::FullRepo }
}
```

Predicates opt **in** to the fast path; the safe default catches global rules even when the author forgets. The pre-commit hook documentation declares `--changed` as **best-effort speed, not authoritative**, with CI as the only source of truth.

**Where to write it.** A new §"`--changed` semantics" subsection under §"`rules` library".

## 3. Message-parity invariant scope

**Issue.** RFC §"Workspace layout" says "Failure messages MUST match the current `check.ts` wording during the overlap period so PR diffs stay readable." This is the right regression-test guarantee while both surfaces run side-by-side, but read literally it locks the Rust port into Deno-era wording forever.

**Recommendation.** Make the invariant explicitly time-bounded: parity is required **until a port deletes its Deno counterpart**, after which Rust wording is free to evolve provided the corresponding fixtures are updated in the same PR.

Reword as: "During side-by-side overlap, Rust messages MUST match `check.ts` wording verbatim — message diffs are treated as port regressions. Once the Deno counterpart is deleted (per migration step 3), wording may evolve under normal fixture-update discipline."

**Where to write it.** Edit the existing paragraph in §"Workspace layout".

## 4. Schema-location boundary and graduation rule

**Issue.** RFC §"Schema-first layer" says runtime schemas live in `specify-cli/schemas/` and framework-only schemas live in `tooling/schemas/`. The wall isn't load-bearing — `adapter.schema.json` is consumed by both sides — and there's no documented path for a schema to graduate when its consumer changes.

**Recommendation.** Add a one-paragraph graduation rule:

> A schema lives in `specify-cli/schemas/` if and only if the operator `specify` binary loads it at runtime to validate consumer-project artifacts. A schema lives in `tooling/schemas/` if it only describes framework authoring shapes (`SKILL.md` frontmatter, codex authoring, `marketplace.json`, scenario YAML). When a framework-only schema becomes a runtime concern (e.g. a future runtime feature consumes `marketplace.json`), it moves to `specify-cli/schemas/` in the same PR that introduces the runtime use; `tooling/schemas/` then re-exports it through `specify-domain`. Graduation in the other direction is rare but follows the inverse path.

Document the **current** placement of each schema in a small table next to this rule, so the boundary is visible at a glance.

**Where to write it.** New §"Schema graduation" subsection under §"Schema-first layer".

## 5. `SPECIFY_CLI_DIR` → `SPECIFY_CLI_ROOT` deprecation

**Issue.** RFC §"`tooling docgen`" mentions "the renamed env var: `SPECIFY_CLI_ROOT`, with the old name accepted as fallback during transition." Without a removal target, the fallback drifts forever.

**Recommendation.** Tie removal to RM-16 closure:

- Phase 6 (port `docgen`): both env vars work; `SPECIFY_CLI_DIR` logs a one-line deprecation warning to stderr citing the removal date.
- Phase 8 (CI cleanup, the same PR that drops `denoland/setup-deno`): `SPECIFY_CLI_DIR` is removed from the resolver and from all docs and CI snippets; using it produces a hard error with a "rename to `SPECIFY_CLI_ROOT`" hint.

**Where to write it.** Append two bullets to §"Migration strategy" steps 6 and 8.

## 6. Cross-RFC rename sweep

**Issue.** RFC §"Crate naming" already lists the surfaces that still say `framework-rules` / `framework-check` / `framework-lsp`. I confirmed live hits in `rfcs/roadmap.md` (RM-07 and RM-16) and `rfcs/next/rfc-28-codex-rules.md`. Bundling the rename with the workspace-landing PR makes that diff much harder to review.

**Recommendation.** Land the rename as a **prep PR before the workspace PR**, in this order:

1. Update `rfcs/roadmap.md` (RM-07, RM-16), `rfcs/next/rfc-28-codex-rules.md`, `rfcs/next/rfc-30-init.md`, `rfcs/done/rfc-1-cli.md`, `rfcs/done/rfc-10-skills.md`, `rfcs/done/rfc-13-extensibility.md`, `rfcs/future/rfc-4-dsl.md`, `docs/contributing/checks.md`, and any `.github/actions/tooling/` references in the same PR.
2. The workspace PR (phase 2 in the migration) then introduces the new names without touching unrelated documents.

This keeps the workspace PR focused on the architectural change and the rename PR focused on prose hygiene; both are independently reviewable and each leaves CI green.

**Where to write it.** A new bullet at the top of §"Migration strategy" labelled "Phase 0: rename sweep" (no Rust code, prep only).

## 7. Acceptance-pack continuity

**Issue.** RFC §"Scope" excludes "manual scenario packs under `tests/cross-repo/` and `tests/plan/` — operator-driven by design." Today those packs are bound to `tests/cross_repo.ts`. After phase 7 deletes that file, the manual harness has no entry point unless someone documents what replaced it.

**Recommendation.** Add one sentence to §"`accept` crate":

> Manual scenario packs under `tests/cross-repo/` and `tests/plan/` continue to run via the `gh` recipe documented in [docs/contributing/acceptance.md](../docs/contributing/acceptance.md). Nothing under `tooling/` invokes them automatically, and `cargo test -p accept` does not exercise them.

If `docs/contributing/acceptance.md` doesn't currently make this explicit, update it in the same PR that deletes `tests/cross_repo.ts` (migration step 7).

**Where to write it.** Append to §"`accept` crate"; cross-link from migration step 7.

## 8. CI cold-cache build time

**Issue.** Today's Deno CI is sub-30s. A cold Cargo build of `tooling` + transitive `specify-domain` deps could be 1–3 minutes per job. Tolerable, but adoption pain is highest in the first few weeks after switchover.

**Recommendation.** Bake `Swatinem/rust-cache@v2` into the CI snippet from phase 2, not after the first slow PR:

```yaml
- uses: dtolnay/rust-toolchain@stable
  with:
    toolchain-file: tooling/rust-toolchain.toml
- uses: Swatinem/rust-cache@v2
  with:
    workspaces: tooling
    shared-key: tooling
- run: cargo build --manifest-path tooling/Cargo.toml -p tooling --release
- run: tooling/target/release/tooling check --repo .
```

**Rejected alternative.** Sccache via shared remote storage — premature for one workspace this size, adds infra dependency.

**Where to write it.** Update the YAML snippet in §"Invisible entry points and auto-build" → "CI".

## 9. RFC-28 rule-id discipline from day one

**Issue.** RFC §"`tooling check`" says JSON output "lands after the first rule ids and locations are stable enough to pin with fixtures." RFC-28 depends on RFC-5 for the `rules::codex` namespace contract. The two could race if someone interprets "stable enough to fixture" as "ids may churn until then."

**Recommendation.** Add a one-line invariant to §"`rules` library":

> Rule ids minted by `rules::codex` follow RFC-28's namespace ownership and id-stability rules from the first ported predicate, even though the wider `tooling check --format json` envelope is fixtured later. Other modules' ids may churn until phase 4 stabilises them.

This makes the cross-RFC interlock explicit without delaying RFC-5's own JSON-envelope work.

**Where to write it.** New paragraph at the end of §"`rules` library".

## 10. Soft LOC budget

**Issue.** Today's Deno surface is ~4,027 LOC across `scripts/check.ts`, `scripts/checks/*.ts`, `scripts/gen-envelope-doc.ts`, `tests/cross_repo.ts`, and `tests/lib/*.ts`. A naive Rust port will balloon (filesystem walks, YAML parsing); a library port that delegates to `specify-domain` should shrink. Without a target, drift is invisible until late.

**Recommendation.** Set a soft budget in §"Scope":

> Soft target: `rules` + `tooling` + `accept` combined ≤ ~5,000 LOC including tests, with at least 500 LOC of net deletion in `specify` (Deno) by phase 8 closure. Materially exceeding the budget is a signal to revisit predicate factoring before more modules port.

This is a guard rail for reviewers, not a hard gate.

**Where to write it.** New bullet at the end of §"Scope".

## Summary

| # | Item | Severity | Where it lands |
|---|---|---|---|
| 1 | Cross-repo Cargo dep mechanics | Blocking before phase 2 | §"Workspace layout" |
| 2 | `--changed` correctness | Blocking before phase 4 | §"`rules` library" |
| 3 | Message-parity invariant scope | Clarification | §"Workspace layout" |
| 4 | Schema-location graduation rule | Clarification | §"Schema-first layer" |
| 5 | `SPECIFY_CLI_DIR` deprecation | Clarification | §"Migration strategy" |
| 6 | Cross-RFC rename sweep | Process | New phase 0 in §"Migration strategy" |
| 7 | Acceptance-pack continuity | Clarification | §"`accept` crate" |
| 8 | CI cold-cache build time | Operational | §"Invisible entry points and auto-build" |
| 9 | RFC-28 rule-id discipline from day one | Cross-RFC interlock | §"`rules` library" |
| 10 | Soft LOC budget | Guard rail | §"Scope" |

Items 1, 2, and 6 are the only ones that should land **before** the workspace PR; the rest can be folded into the RFC during normal phase progression.
