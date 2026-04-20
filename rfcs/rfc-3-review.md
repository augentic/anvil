# RFC-3 Review — MVP and Simplification Lens

> Reviewer's analysis of [RFC-3: Initiative Planning](rfc-3-planning.md) with a
> focus on identifying significant gaps, over-complexity, and opportunities to
> simplify toward a minimum viable slice that proves the soundness of the
> approach. Iterative extension toward richer legacy-migration cases is assumed
> as the follow-on trajectory.

## Top-line

RFC-3 as drafted bundles **four semi-independent pieces** into one RFC:

1. **`scope` on plan entries** (part of *Large-Monolith Decomposition*)
2. **Plan-time / define-time split for extraction** (moving `/spec:extract` to define-time; new `/spec:analyze` at plan-time)
3. **Registry-aware multi-repo planning** (Layers 2 & 3: `registry.yaml`, workspace, sync-peers, federation)
4. **`initiative.md` + closed `kind` enum + per-kind dispatch**

For an MVP that proves the soundness of the spec-driven approach against legacy
migration, **only piece #1 is load-bearing, with just enough of #2 to make it
work**. #3 and #4 are valuable but solve *different* problems (multi-repo
coordination; input-type plumbing), and neither is on the critical path to the
thing that currently fails — running the loop against a real monolith. Dropping
them from v1 is the single biggest simplification available.

The RFC already hints at this in the §*Staged rollout* section ("Stage A — the
`scope` field … unblocks real monolith work without touching discovery or
propose"). That staging is the MVP; the rest of the RFC is the follow-ons. The
recommendation is to **promote Stage A to be the whole of RFC-3 v1** and push
Stages B / C and Layers 2 / 3 into their own RFCs.

---

## Significant gaps

These are the things to pin down before any implementation, regardless of scope
choice.

**G1. How `scope` composes with `/spec:define` is undefined.** The RFC says
extraction runs "per-change against the scope each change owns", but
`plugins/spec/skills/define/` has no description of how `/spec:define` consumes
`scope`. Today the Omnia discovery brief calls `/spec:extract` at *plan time*
for `--source` inputs (see `schemas/omnia/briefs/plan/discovery.md` step 1).
Moving that call to define-time is a behaviour change for existing single-repo
users, and the RFC doesn't trace the path — *what does `/spec:define` do
differently, what happens to plans authored pre-RFC-3, and who writes the
`design.md`/`specs/` that the define phase previously consumed?* This is the
interface most likely to get hand-waved and then bite later.

**G2. Propose brief contract under the plan-time/define-time split.** Today
propose consumes a **capability** inventory (user-registration,
email-verification, …). Under §*Large-Monolith Decomposition* the plan-time
output becomes a **module** inventory (entry points, dependency edges). The
RFC says both feed `discovery.md` "in the same shape" but those are not the
same shape — one is business-capability grained, one is code-structure grained.
Propose either has to handle both or needs rewriting. This is deferred to a
"follow-up skill RFC" while simultaneously being declared "RFC-3 fixes only the
skill boundary and the output contract". The contract is exactly what needs
fixing.

**G3. No concrete acceptance scenario.** The RFC has no worked example of
"successful MVP run against a real monolith". A single fixture — "~20-module
Express/Node monolith → 3 slices with scopes → `/spec:execute --loop` to green"
— would anchor scope debates and give a regression surface. Current fixtures
(`plugins/spec/skills/plan/fixtures/discovery/legacy/`) are toy-sized.

**G4. Backwards compatibility for today's `/spec:plan` users.** The RFC claims
*"A bare `/spec:plan` with CLI-only inputs remains valid"* and *"Changes
without `scope` are unaffected"*, but several changes — discovery dispatching
via closed `kind` enum, `--source key=path:kind` syntax extension, extract
moving to define-time — do change observable behaviour. A short "migration
impact" subsection (or an explicit "existing plans continue to work as before,
with these exceptions: …") would help.

## Over-complexity (ranked by MVP impact)

**O1. Multi-repo (Layers 2 & 3) is premature for proving soundness.** Layer 3
is explicitly *(Detail TBD)* and reads "ported from the federation draft";
Layer 2 introduces a registry file format, cloning, symlinking,
`workspace.md`, four new CLI subcommands, peer inventory semantics, cross-repo
references in `sources`/`affects`, a `--dry-run` rule for sync-peers, and an
`--extend` refresh policy *(TBD)*. None of this is required to validate the
Stage-A monolith slicing story, and multi-repo coordination is a fundamentally
harder problem than within-repo decomposition (contract reconciliation, peer
autonomy, aggregate status). **Defer Layers 2 & 3 to a follow-up RFC.** The
§*Alternatives Considered — Registry repo* paragraph already notes this path
is cheap to add later because the file is initiative-free.

**O2. `initiative.md` adds a file whose body does nothing in v1.** The RFC
deliberately says "the body prose is **not** an input to `/spec:analyze` in
v1". That's an entire file, a scaffold command (`specify initiative brief
init`), an archive hook, and a sub-command tree — for structured data (`name`,
`inputs[]`) that already arrives via CLI flags today. Two possibilities, both
simpler:

- **(a) Drop `initiative.md` for v1.** Keep `--from`/`--against`/`--source` as
  the authoritative input surface. If the operator wants a durable framing,
  they put a markdown file somewhere in the repo and pass it via `--from`. The
  file is only worth its cost once the body is actually consumed.
- **(b) Keep it only as a transcript-style log of what was passed on the CLI**,
  auto-written by `/spec:plan`. Zero new syntax.

**O3. `/spec:analyze` as a two-branch skill (code + documentation) is more
skill than MVP needs.** The existing Omnia `discovery.md` brief already
handles both (`--from` for prose, `/spec:extract` for code). The new-skill case
rests on "extract is too heavy for whole monoliths" (true) and "dispatch prose
in a brief is worse than a dedicated skill" (unclear). For MVP, the
legacy-code case can be covered by **humans hand-authoring scopes** (Stage A
as the RFC describes it: *"Humans author scopes by hand. Unblocks real
monolith work without touching discovery or propose"*). No new skill required.
Documentation stays in the existing `--from` path. `/spec:analyze` can be
introduced in a later RFC once real pain from hand-authored scopes shows up.

**O4. Closed `kind` enum + `--source key=path:kind` + "open vocabulary
rejected" subsection.** The enum has exactly two values, both of which can
already be inferred: `--from`/`--against`/`--source local` is documentation
iff it's a text file and legacy iff it's a directory or git URL. The entire
*Alternatives Considered — Open `kind` vocabulary* paragraph, the syntax
extension, and the hard-error-at-analyse-phase behaviour exist to defend a
two-value enum. For MVP, the existing flag shape is sufficient.

**O5. Manifest escape hatch and `slices/` directory when both "ship empty in
v1".** *"Manifest shape (files, optional line-range subsets) is TBD and ships
empty in v1."* Then don't put it in the RFC yet. Strict YAGNI. When a real
tangled case breaks path-based globbing, add the hatch in a point-release.

**O6. Auto-slicing heuristics (entry-point clustering + tie-breakers + LOC
budget).** §*Propose-brief slicing heuristics* designs a real algorithm for
schemas to own, with edit-path tuning via new `--scope-include` /
`--scope-exclude` flags on `specify initiative create`. That's Stage C. The
RFC's own staging correctly places this last. For MVP, propose emits
scope-less slices; humans add scope via `amend`. Ship Stage C when
hand-authored scopes are clearly the bottleneck.

**O7. Validation checks introduce orphan warnings for a workflow that doesn't
yet exist.** The *Overlap warning* is valuable. *Path existence* is clearly an
error. *Orphan warning* ("a file under `sources[<key>]` claimed by zero
changes") is only useful if an operator is using scope to *cover* a codebase
exhaustively — which is a Stage-B/C workflow where propose auto-slices. For
hand-authored Stage-A scope, orphans are the normal case (operator slices the
parts they're actively migrating, leaves the rest). Defer orphan warning.

**O8. Three `(TBD)` CLI command groups in the Diagram-labels table.** `specify
initiative registry {show, validate}`, `specify initiative brief {init, show}`,
`specify initiative workspace {sync, status}` — all marked `(TBD)`. TBDs in an
RFC that is itself a draft are a code smell. If a command is needed, it's
needed; if it's not needed for v1, remove it. Removing them naturally falls
out of O1 and O2.

**O9. Shared-infrastructure-change pattern as a documented scheme.** The
§*Cross-slice shared files* section designs a convention for how cross-cutting
code should be carved out. This is a **propose-brief authoring convention**
that is already covered by propose.md step 4 (*"Cross-cutting refactors are
their own entries"*). Lifting it into the RFC as a scheme adds a decision
point ("should I create a shared-infra slice, or accept the overlap warning?")
that doesn't need to be an architectural commitment. For MVP: say overlap is
allowed with a warning, call the shared-infra pattern a schema-owned
convention, move on.

## Opportunities to improve (by simplifying)

**S1. Rename and re-scope to the one thing.** Retitle RFC-3 as **"RFC-3:
Scoped extraction for large monoliths"**. The abstract then becomes a
paragraph, not a page:

> RFC-2's define-time loop works for small legacy and greenfield inputs, but a
> whole-monolith `/spec:extract` at plan time overflows context and forecloses
> slice boundaries. RFC-3 adds an optional `scope` field on plan entries
> (include/exclude globs) and moves `/spec:extract` to define-time, where it
> runs against each change's declared scope. Humans author scopes by hand;
> auto-slicing heuristics are deferred. Multi-repo planning and federation are
> deferred to a separate RFC.

Everything else in the current document either survives as-is under
*Large-Monolith Decomposition* or moves to a future RFC.

**S2. Specify the minimum define-time contract explicitly.** To close G1,
write a one-paragraph "What `/spec:define` does with `scope`": *"When a change
has `scope.<source>.{include,exclude}`, the define phase invokes
`/spec:extract` with that path filter applied; when scope is absent, it
invokes `/spec:extract` against the full `sources[<key>]` path (current
behaviour). No other define-time behaviour changes."* One paragraph kills a
class of ambiguity.

**S3. One concrete fixture that pins the shape end-to-end.** A
`fixtures/scoped/monolith/` tree with 10–20 files under clear module
boundaries, an expected `plan.yaml` with three `scope`-bearing entries, and an
expected `specs/` output after `/spec:define` on one slice. Small enough to
commit; large enough to be non-trivial.

**S4. Two-step validator, not three.** For v1 of `specify initiative
validate`'s scope rules: path-existence (error) + overlap (warning). Drop
orphan. Re-add with Stage C.

**S5. No new CLI commands.** Scope authoring via `specify initiative create
--scope-include <glob>... --scope-exclude <glob>...` is the one CLI addition.
No `registry`, `brief`, `workspace` trees in v1.

**S6. Keep the discovery brief intact.** `schemas/omnia/briefs/plan/discovery.md`
today calls `/spec:extract` at plan time via step 1. The MVP-respecting change
is: **stop calling extract at plan time.** For legacy `--source` inputs,
discovery emits a coarse "source roots to scope" summary (file-tree outline +
top-level READMEs + `package.json`/equivalent) into `discovery.md`; propose
emits scope-less slices; human writes scope. That's a brief-level tweak, not
a new skill.

---

## What's worth preserving

A few things in the current draft are doing real work and should survive
whatever paring is done:

- **The `scope.<source>.{include,exclude}` shape itself.** Include/exclude
  globs resolved under the top-level `sources[<key>]` path is the right
  primitive. Cleanly backwards-compatible because absent scope = today's
  behaviour. Extends the Plan schema with one optional field and no semantic
  changes to `sources` / `depends-on` / `affects`.
- **The split between platform catalogue and cycle-scoped state**, if
  multi-repo survives to a future RFC. The §*Alternatives Considered —
  Combined registry + initiative file* argument is sound and worth keeping as
  the framing when the multi-repo RFC happens.
- **The *Staged rollout* subsection.** It already identifies the MVP
  correctly. Promote it from "subsection of Large-Monolith Decomposition" to
  "the structure of RFC-3".
- **Most of the Alternatives Considered section survives** as a record of
  rejected directions, though several entries (registry repo, plan-per-repo +
  feature manifest, `/spec:scope`, `/spec:decompose`, `--mode=survey`) become
  alternatives for a *future* multi-repo or analyze RFC rather than for this
  one.

---

## Suggested MVP RFC-3 (shape, not prose)

1. **Motivation.** Whole-monolith `/spec:extract` at plan time overflows;
   humans have no chance to draw slice boundaries. One paragraph.
2. **The `scope` field.** Schema extension; semantics for include/exclude;
   backwards compat.
3. **Plan-time / define-time split.** One paragraph: discovery stops calling
   extract; define calls extract scoped to the change's scope.
4. **Authoring flow.** Humans author scopes by hand via `specify initiative
   create --scope-include ... --scope-exclude ...` or `specify initiative
   amend`. Propose continues to emit scope-less slices.
5. **Validation.** Path-existence (error) + overlap (warning).
6. **Fixture.** One realistic monolith fixture.
7. **Non-goals.** Multi-repo, `registry.yaml`, `initiative.md`, `/spec:analyze`
   skill, auto-slicing heuristics, manifest escape hatch, symbol-level scope,
   orphan warning, shared-change convention. All explicitly deferred to
   follow-up RFCs, each called out by name.

That version fits in ~150–200 lines of RFC, is end-to-end implementable in one
sprint, and gives a real test ground for "does spec-driven development hold up
against a serious legacy codebase?" — which is the soundness question. The
richer features can land as follow-up RFCs (multi-repo; auto-slicing /
`/spec:analyze`) once experience shows which ones actually earn their
complexity.
