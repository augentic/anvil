# Expected Plan Roles

> Role-based rules the `rm01-cross-repo` suite asserts on the `plan.yaml`
> produced by `/change:plan oauth-login` (against the fixture brief at
> [`../inputs/docs/oauth-login.md`](../inputs/docs/oauth-login.md)).
> Owner: [`scenario.md`](../scenario.md) (Assertions section).

This file is the **machine-liftable source of truth** for the C09 plan
assertions. The C09 implementer should be able to read each `Rule:` block
below and translate it directly into an assertion module without
re-interpreting prose. The assertion ids and verdict semantics match the
shared vocabulary in
[`../../../assertions/README.md`](../../../assertions/README.md).

## Why Role-Based, Not Name-Based

A live agent invoking `/change:plan` from the OAuth login brief can
legitimately produce slice names like `oauth-login-contract`,
`oauth-shared-http-api`, `oauth-tokens-shared-contract`,
`add-oauth-tokens`, `add-oauth-token-endpoints`, `add-oauth-screens`, or
`add-oauth-login-ui`. Every one of those is correct **as long as** the
roles, dependencies, and routing below hold. Name-based assertions would
make this suite reject correct plans.

The Layer 0 `specify-cli/tests/cross_repo.rs` test seeds exact names
because it tests the CLI substrate, not the planner's wording. This
suite is the inverse: it tests the planner's wording-independent
*structure*.

## Reference Plan (Illustrative)

A passing run might produce a plan like the one below. The exact slice
**names** can vary; the **roles** must match the rules in the next
section. This block is reference only — the suite never asserts on the
exact names here.

```yaml
# illustrative; names may legitimately vary
change: oauth-login
entries:
  - name: oauth-login-contract
    schema: contracts@v1
    depends-on: []
    description: Author the shared OAuth login HTTP contract.
  - name: add-oauth-tokens
    project: shop-backend
    depends-on: [oauth-login-contract]
    description: Implement OAuth provider token persistence and refresh endpoints.
  - name: add-oauth-screens
    project: shop-mobile
    depends-on: [oauth-login-contract]
    description: Implement login UI and OAuth redirect handling.
```

## Role Definitions

The suite recognises three roles in any RM-01 plan, identified
**structurally** rather than by name:

- **Contract role.** A plan entry where:
  - the entry has a `schema:` field whose value matches `^contracts@v\d+$`
    (in this fixture: `contracts@v1`), and
  - the entry **does not** have a `project:` field, and
  - the entry's `depends-on:` is empty.
- **Backend implementation role.** A plan entry where:
  - the entry has `project: shop-backend`, and
  - the entry's `depends-on:` includes the contract entry (matched by id).
- **Mobile implementation role.** A plan entry where:
  - the entry has `project: shop-mobile`, and
  - the entry's `depends-on:` includes the contract entry (matched by id).

These are the only roles RM-01 expects. A pass requires **exactly** one
entry of each role. A pass does not forbid additional entries that match
none of the three roles, but the C09 implementer should treat any
extra-entry case as `live-agent-nondeterminism` and surface it as a
warning rather than a hard failure.

## Rules (Assertion-Liftable)

Every `Rule:` block below maps 1:1 to one assertion id from
[`../scenario.md` §Assertions](../scenario.md#assertions). The C09
implementer is expected to ingest each rule as an assertion module that
returns `pass` / `fail` / `skipped` plus, on failure, the evidence
pointer named in `Evidence on fail:`.

---

Rule: `plan-yaml-exists`

- Subject: the file `plan.yaml` at the hub root (`shop-platform/plan.yaml`).
- Pass when: the file exists and is readable.
- Evidence on fail: the absent path; check `final-tree.txt` to confirm.
- Fault domain on fail: `skill-orchestration` (planner did not write a
  plan) or `cli-substrate` (`specify change plan create` failed silently).

---

Rule: `plan-validate-clean`

- Subject: the result of `specify change plan validate` run from
  `shop-platform/`.
- Pass when: exit code `0`. Stderr is empty or carries only informational
  output.
- Evidence on fail: `stderr.log` and the captured exit code.
- Fault domain on fail: `cli-substrate` (validator caught a structural
  issue) or `skill-orchestration` (planner produced an invalid plan).

---

Rule: `plan-has-one-contract-slice`

- Subject: the entries of `plan.yaml`.
- Pass when: exactly one entry matches the **contract role** definition
  above. Both clauses (schema match, no `project:`, empty `depends-on:`)
  must hold.
- Evidence on fail: the count and names of contract-role-matching entries
  read from `plan.yaml`.
- Fault domain on fail: `skill-orchestration` (planner skipped the
  contract slice or authored more than one) or `capability-brief` (the
  brief failed to imply a shared contract).

---

Rule: `plan-has-one-backend-slice`

- Subject: the entries of `plan.yaml`.
- Pass when: exactly one entry has `project: shop-backend`.
- Evidence on fail: the count and names of `project: shop-backend` entries.
- Fault domain on fail: `skill-orchestration` (assignment step routed the
  wrong number of slices to the backend project).

---

Rule: `plan-has-one-mobile-slice`

- Subject: the entries of `plan.yaml`.
- Pass when: exactly one entry has `project: shop-mobile`.
- Evidence on fail: the count and names of `project: shop-mobile` entries.
- Fault domain on fail: `skill-orchestration` (assignment step routed the
  wrong number of slices to the mobile project).

---

Rule: `backend-slice-routed-to-shop-backend`

- Subject: the unique entry identified by `plan-has-one-backend-slice`.
- Pass when: that entry's `project` field equals exactly `shop-backend`.
  This is a defensive restatement of `plan-has-one-backend-slice` so a
  future runner can attribute a routing failure (entry exists, points
  somewhere else) separately from a counting failure (no entry, or two
  entries).
- Evidence on fail: the entry's id and its `project` value as read from
  `plan.yaml`.
- Fault domain on fail: `skill-orchestration` (assignment step misread
  the registry description) or `cli-substrate` (`specify change plan
  amend --project` rewrote the wrong field).

---

Rule: `mobile-slice-routed-to-shop-mobile`

- Subject: the unique entry identified by `plan-has-one-mobile-slice`.
- Pass when: that entry's `project` field equals exactly `shop-mobile`.
- Evidence on fail: the entry's id and its `project` value as read from
  `plan.yaml`.
- Fault domain on fail: `skill-orchestration` or `cli-substrate`, as
  above.

---

Rule: `implementation-slices-depend-on-contract`

- Subject: the two implementation entries identified by
  `plan-has-one-backend-slice` and `plan-has-one-mobile-slice`.
- Pass when: each implementation entry's `depends-on:` list contains the
  unique contract-role entry's id (string equality on the id, not on a
  human-readable name).
- Evidence on fail: each implementation entry's id and its `depends-on:`
  list.
- Fault domain on fail: `skill-orchestration` (planner failed to express
  contract-first ordering) or `capability-brief` (a brief failed to
  surface the dependency to the planner). This is the **load-bearing
  RM-01 invariant** — see also the matching negative expectation
  `implementation-slices-author-contract-shapes-inline` in
  [`../scenario.md`](../scenario.md#negative-expectations).

---

Rule: `contract-slice-projectless`

- Subject: the unique contract-role entry identified by
  `plan-has-one-contract-slice`.
- Pass when: the entry has no `project:` field, or the field is present
  with a null/empty value (the CLI's `specify change plan show` reports
  this as `project: null`).
- Evidence on fail: the entry's id and the offending `project` value.
- Fault domain on fail: `skill-orchestration` (assignment step routed
  the contract entry to a project) or `cli-substrate`
  (`specify change plan amend --project` accepted a write it should
  have refused).

---

## Notes For C09 Implementation

- All rules read from `plan.yaml` after the planner returns. None of
  them require executing a slice. Reading the file once (or relying on
  one `specify --format json change plan status` invocation) is enough
  for every rule above.
- The `depends-on:` list is matched by entry id. The CLI assigns ids
  when entries are added; assertion code should not parse human prose to
  resolve them.
- When a rule is `skipped` (because an upstream rule failed and the
  evidence it would inspect cannot be trusted), the C09 implementer
  should record `skipped` rather than `fail` so the failure attribution
  in `summary.md` stays accurate. Example: if `plan-yaml-exists` fails,
  every other rule above is `skipped`, not `fail`.

## Execute Rules (C10) — Asserted

The four rules below are owned by C10. They run only when the suite is
driven through a backend that produces an `executeState` on the run
context (today: `scripted-execute`); under the plan-only `scripted-plan`
backend they cleanly demote to `skip`. Cascade-skip semantics also apply
on upstream `setup-*` or `plan-*` failure — the loop driver consumed a
malformed plan, so per-clone evidence is untrustworthy.

---

Rule: `branch-prepared`

- Subject: each routed clone at
  `<hub>/.specify/workspace/<project>/`.
- Pass when: `git -C <clone> branch --show-current` returns
  `specify/oauth-login`.
- Evidence on fail: the clone path and its actual current branch.
- Fault domain on fail: `cli-substrate` (`specify workspace
  prepare-branch` failed silently or selected the wrong branch) or
  `runner-setup` (the clone is not a git repo).

---

Rule: `baseline-merge-commit-clean`

- Subject: HEAD~1 of each routed clone — by convention the
  `specify: merge <slice>` baseline commit produced by the loop
  driver.
- Pass when: `git -C <clone> show --name-only --format= HEAD~1`
  returns only paths under `.specify/specs/` or `.specify/archive/`.
- Evidence on fail: the offending paths (capped at 5).
- Fault domain on fail: `skill-orchestration` — the loop driver split
  baseline + residue incorrectly, or generated outputs leaked into the
  baseline commit. Mirrors the `baseline-merge-commit-contains-residue`
  negative expectation in [`../scenario.md`](../scenario.md#negative-expectations).

---

Rule: `residue-commit-non-empty`

- Subject: HEAD of each routed clone — by convention the
  `specify: residue <slice>` commit produced by the loop driver.
- Pass when: HEAD touches **at least one** path AND every touched
  path is outside `.specify/specs/` and `.specify/archive/`.
- Evidence on fail: the offending paths (capped at 5) or the empty
  commit subject.
- Fault domain on fail: `skill-orchestration` — the loop driver
  produced an empty residue commit (build phase did not generate
  anything) or a residue commit that touches baseline paths.
  Mirrors the `residue-commit-contains-baseline-files` negative
  expectation in [`../scenario.md`](../scenario.md#negative-expectations).

---

Rule: `workspace-clean-before-push`

- Subject: each routed clone immediately after the loop driver
  finishes (i.e. immediately before the C11 `specify workspace push`
  step).
- Pass when: `git -C <clone> status --porcelain` is empty.
- Evidence on fail: the first three porcelain lines.
- Fault domain on fail: `skill-orchestration` — a generated artifact
  was not staged + committed before the residue commit, or the
  loop driver left the working tree dirty. C11 cannot push a dirty
  workspace.

---

## Push / Finalize Rules (C11) — Asserted

The six rules below are owned by C11. They run only when the suite is
driven through a backend that produces a `finalizeState` on the run
context (today: `scripted-finalize`); under the execute-only
`scripted-execute` backend they cleanly demote to `skip`. Cascade-skip
semantics also apply on upstream `setup-*`, `plan-*`, or `execute-*`
failure — push/finalize evidence is untrustworthy when the loop
driver consumed bad state.

The push handlers gate on `ctx.run.finalizeState.pushOutput` being
populated; the finalize handlers gate on
`ctx.run.finalizeState.finalizeOutput`; the idempotency handler gates
on `ctx.run.finalizeState.finalizeSecondOutput`; the pre-merge
negative handler gates on `ctx.run.finalizeState.finalizeRefusedPreMerge`.
Each missing slot demotes its handler to `skip`.

---

Rule: `push-opens-pr-per-project`

- Subject: `<gh-state-dir>/<repo-slug>.pr` for each routed project
  after `specify workspace push` returns.
- Pass when: every routed project has exactly one fake-`gh` PR file
  whose branch matches `specify/oauth-login` and whose PR number
  matches the value reported by the `workspace push` JSON output.
  The state field is accepted as either `OPEN` (post-push) or
  `MERGED` (post-mark-merged), because the assertion stage runs at
  end-of-run when the backend has already flipped both files.
- Evidence on fail: missing repo file, mismatched branch, or
  mismatched PR number. The shape pin for "still open immediately
  after push" lives in `push-output-json-shape-clean` — that handler
  inspects the captured push JSON, which is taken before the
  mark-merged step.
- Fault domain on fail: `cli-substrate` (push did not create the PR)
  or `external-fake-boundary` (the fake-`gh` state dir is unreadable).

---

Rule: `push-output-json-shape-clean`

- Subject: the parsed `specify --format json workspace push` JSON
  captured in `<runDir>/push-output.json`.
- Pass when: the top-level object contains a `projects` array with one
  entry per routed project, each carrying `name` (string), `status:
  "pushed"`, `branch` (string equal to `specify/oauth-login`), and
  `pr` (positive integer). The shape comes from
  `specify-cli/tests/cross_repo.rs::push_workspace`.
- Evidence on fail: the offending field path + observed value.
- Fault domain on fail: `cli-substrate` — the CLI's JSON output
  contract drifted; a regression in `--format json workspace push`
  shape is attributed here rather than to the fake-`gh` boundary.

---

Rule: `finalize-runs-before-prs-merged`

- Subject: a probe `specify change finalize` invocation issued by the
  scripted-finalize backend BEFORE the fake `gh` PR files are flipped
  to `MERGED`.
- Pass when: the CLI exits non-zero (the load-bearing RFC-14 guard
  holds — Specify must not finalize while PRs are still OPEN).
- Evidence on fail: the captured exit code and output. The negative
  expectation is asserted on by C11 but the verdict is **never a hard
  suite failure** — when the CLI accepts the call the handler reports
  a `cli-substrate` finding so the suite can land an actionable
  signal without flapping while the CLI guard is being tightened.
- Fault domain on fail: `cli-substrate` — the verb itself should
  refuse; if it does not, file a `specify-cli` follow-up against
  RFC-14.

---

Rule: `finalize-archives-plan`

- Subject: the live hub tree + the captured `specify --format json
  change finalize` JSON (first call, after PRs are marked merged).
- Pass when: every clause holds:
  - `finalized: true` in the captured JSON,
  - the live `plan.yaml` is gone from the hub root,
  - the JSON's `archived` path points at a regular file on disk,
  - `<hub>/.specify/archive/plans/` contains an entry whose name
    starts with `<change>-` (e.g. `oauth-login-20260508/`).
- Evidence on fail: the failing clauses, the live plan path, and
  the reported archived path.
- Fault domain on fail: `cli-substrate` — finalize either refused
  with the wrong reason or moved the file to the wrong location.

---

Rule: `finalize-output-json-shape-clean`

- Subject: the parsed first-call `specify --format json change
  finalize` JSON captured in `<runDir>/finalize-output.json`.
- Pass when: the top-level object carries `initiative` (string equal
  to the change name), `finalized: true`, `archived` (non-empty
  string), `projects` (array length matching the routed-project
  count, each entry with `name` (string) + `status: "merged"`), and
  `summary.merged` (number equal to the routed-project count).
- Evidence on fail: the offending field path + observed value.
- Fault domain on fail: `cli-substrate` — the CLI's JSON output
  contract drifted; a regression in `--format json change finalize`
  shape is attributed here rather than to the runner.

---

Rule: `finalize-second-call-returns-plan-not-found`

- Subject: the parsed second-call `specify --format json change
  finalize` JSON captured in `<runDir>/finalize-output.second-call.json`.
- Pass when: the JSON's `error` field equals exactly `plan-not-found`
  (mirrors `cross_repo.rs::assert_finalize_is_idempotent`).
- Evidence on fail: the observed `error` value, or a note that the
  field was absent.
- Fault domain on fail: `cli-substrate` — finalize either silently
  re-archived a missing plan or returned the wrong error reason.

---

## Define / Merge Rules (C12) — Asserted

The seven rules below are owned by C12. They run only when the suite
is driven through a backend that produces an `executeState` on the
run context (today: `scripted-execute`, `scripted-finalize`,
`agent`); under the plan-only `scripted-plan` backend they cleanly
demote to `skip`. Cascade-skip semantics also apply on upstream
`setup-*`, `plan-*`, or `execute-*` failure — define-stage evidence
is untrustworthy when the loop driver consumed bad state or split
commits incorrectly.

The handlers gate on `ctx.run.executeState.slices` being populated;
when empty (legacy backends that pre-date C12) every define-* / merge-*
handler cleanly demotes to `skip` rather than `fail`. The structural
checks read on-disk truth in the workspace clones, not driver
self-reports — the operator-results JSON is treated as a recipe for
artifact bodies, not a substitute for verification.

The same handlers run unchanged against `scripted-execute` /
`scripted-finalize` because the deterministic stub bodies are valid
artifact shapes (proposal/spec/tasks for contract slices, plus
design.md for omnia/vectis implementation slices). This is
deliberate — proving the assertions work on stub-quality artifacts
proves they will work on real `/spec:define` output without
requiring a live agent run on every CI pipeline.

---

Rule: `slice-has-proposal`

- Subject: each slice on `executeState.slices`, looked up first under
  `<clone>/.specify/specs/<slice>/proposal.md` (delta dir) and then
  under `<clone>/.specify/archive/<date>-<slice>/proposal.md`
  (post-merge dir). For projectless contract slices the lookup runs
  in the hub clone.
- Pass when: the file exists and is non-empty after stripping
  whitespace.
- Evidence on fail: the searched paths and the slice name.
- Fault domain on fail: `skill-orchestration` — `/spec:define` did
  not author a proposal, or the loop driver wrote it to an
  unexpected location.

---

Rule: `slice-has-spec`

- Subject: each slice on `executeState.slices`, looked up under
  either `<dir>/spec.md` or `<dir>/specs/main.md` (both shapes are
  legal — the legacy single-file shape and the post-RFC-9 multi-file
  shape).
- Pass when: at least one of the two candidate files exists and is
  non-empty.
- Evidence on fail: the searched paths and the slice name.
- Fault domain on fail: `skill-orchestration` — `/spec:define`
  skipped the spec phase.

---

Rule: `slice-has-design-when-required`

- Subject: each slice on `executeState.slices` whose capability brief
  requires a `design.md` artifact.
- Pass when: either (a) the brief does NOT require `design.md` (the
  rule short-circuits to `skip` for that slice), or (b) the brief
  requires it AND `<dir>/design.md` exists and is non-empty.
- Policy table (in `acceptance/assertions/define.ts`): `contracts`
  requires no design (the contract YAML IS the design); `omnia` and
  `vectis` both require design.
- Evidence on fail: the slice name, capability, and searched paths.
- Fault domain on fail: `skill-orchestration` — `/spec:define`
  skipped the design phase for an implementation slice; or
  `capability-brief` — the brief failed to surface that the slice
  needs a design document.

---

Rule: `slice-has-tasks`

- Subject: each slice on `executeState.slices`, looked up under
  `<dir>/tasks.md`.
- Pass when: the file exists and is non-empty.
- Evidence on fail: the searched paths and the slice name.
- Fault domain on fail: `skill-orchestration` — `/spec:define`
  skipped the tasks phase.

---

Rule: `slice-baseline-promoted`

- Subject: each slice on `executeState.slices` post-merge.
- Pass when: the slice's spec / proposal / tasks artifacts live
  under the routed clone's `.specify/specs/<slice>/` (delta dir) or
  `.specify/archive/<date>-<slice>/` (post-merge dir). The handler
  cleanly demotes to `skip` on backends that have not run a merge
  step yet — the merge promotion is asserted on the post-merge state.
- Evidence on fail: the searched paths.
- Fault domain on fail: `cli-substrate` (`specify slice merge run`
  failed to promote the spec) or `skill-orchestration` (the loop
  driver did not invoke the merge step).

---

Rule: `slice-archived`

- Subject: each slice on `executeState.slices` post-merge.
- Pass when: a directory matching `.specify/archive/<date>-<slice>/`
  exists in the routed clone (the date prefix is the merge-date
  stamp written by `specify slice merge run`). The handler cleanly
  demotes to `skip` on backends that have not run a merge step yet.
- Evidence on fail: the searched paths and any partial matches.
- Fault domain on fail: `cli-substrate` (`specify slice merge run`
  promoted the baseline but did not archive the delta dir) or
  `skill-orchestration` (the loop driver skipped the merge step).

---

Rule: `implementation-slice-reads-baseline-contract`

- Subject: each implementation slice on `executeState.slices` (i.e.
  every slice routed to a `project:` other than the contract slice).
- Pass when: at least one of the slice's authored artifacts
  (proposal.md / spec.md / design.md) contains a textual reference
  to the baseline contract (substring match on `contracts/` or
  `baseline contracts/`). Contract-role slices (no `project:`) are
  cleanly skipped.
- Evidence on fail: the slice name, the searched files, and a note
  that no reference was found.
- Fault domain on fail: `capability-brief` — the brief failed to
  surface the contract dependency to the implementation slice's
  define skill; or `skill-orchestration` — the live `/spec:define`
  invocation did not consume the contract baseline.

---

## Notes For Future Backends

The `scripted-finalize` backend is the C11 implementation of these
six C11 rules; the `agent` backend is the C12 implementation of the
seven C12 rules. Both backends score against the same id set. Both
backends exist to prove the **landing-path assertion plumbing** end
to end against a deterministic baseline (scripted) or operator-replayed
artifact bodies (agent); running real `/change:plan orchestrate` and
real `/spec:define` against the same fixture without operator
mediation is reserved for the deferred Cursor SDK driver (option A
in the C12 plan) — when wired up it will use the same `agent`
backend by swapping the operator-results path for a live SDK
session. Tightening the proof — variation in slice naming, real
brief interpretation, real PR creation against a hosted forge —
requires plugging the `agent` backend into the same scenario behind
the SDK driver; do not backfill it into the scripted backends.

## Define Rules (C12) — Asserted

The seven rules below are owned by C12. They run only when the suite
is driven through a backend that produces an `executeState` on the
run context (today: `scripted-execute`, `scripted-finalize`, `agent`);
under the plan-only `scripted-plan` backend they cleanly demote to
`skip`. Cascade-skip semantics also apply on upstream `setup-*` or
`plan-*` failure — the loop driver consumed a malformed plan, so
per-slice define-stage evidence is untrustworthy.

The C12 stub (`StubPhaseDriver`) writes `STUB:` bodies that satisfy
every rule below; the C12 `AgentPhaseDriver` plugs in real-quality
bodies from an `--operator-results <path>.json` file (or a future
Cursor SDK driver). Both drivers route through `driveSliceWithBodies`
so the per-slice CLI sequence and commit shape stay byte-for-byte
identical; only the artifact bodies vary.

The handlers prefer `<root>/.specify/specs/<slice>/<file>` (the real
`/spec:define` output location) and fall back to
`<root>/.specify/archive/<slice>/<file>` (the path the C10 baseline
merge commit also writes to). `<root>` is the routed clone for impl
slices and the hub root for the contract slice.

---

Rule: `slice-has-proposal`

- Subject: per plan entry, `<root>/.specify/specs/<slice>/proposal.md`
  (preferred) or `<root>/.specify/archive/<slice>/proposal.md`.
- Pass when: at least one of the two paths is a regular file.
- Evidence on fail: the slice name + both candidate paths.
- Fault domain on fail: `skill-orchestration` — the loop driver did
  not produce a proposal for the slice.

---

Rule: `slice-has-spec`

- Subject: per plan entry,
  `<root>/.specify/specs/<slice>/spec.md` or
  `<root>/.specify/specs/<slice>/specs/main.md` (the
  forward-compatible alt path) or the archive equivalents.
- Pass when: at least one of the four candidates is a regular file.
- Evidence on fail: the slice name + the candidate paths probed.
- Fault domain on fail: `skill-orchestration`.

---

Rule: `slice-has-design-when-required`

- Subject: per plan entry; controlled by the per-capability map in
  `acceptance/runner/backends/phase-driver.ts::CAPABILITY_REQUIRES_DESIGN`:
  - `contracts` → no `design` brief; rule passes regardless of
    file presence.
  - `omnia`, `vectis` → `design` brief required; rule fails when
    `design.md` is absent from both define-stage locations.
- Pass when: the capability does not require `design.md`, OR the
  capability requires it AND
  `<root>/.specify/specs/<slice>/design.md` (or the archive
  equivalent) exists.
- Evidence on fail: the slice name + capability + probed paths.
- Fault domain on fail: `capability-brief` — the slice's body
  factory dropped the design artifact a brief required.

---

Rule: `slice-has-tasks`

- Subject: per plan entry, `<root>/.specify/specs/<slice>/tasks.md`
  or the archive equivalent.
- Pass when: at least one of the two paths is a regular file.
- Evidence on fail: the slice name + both candidate paths.
- Fault domain on fail: `skill-orchestration`.

---

Rule: `slice-baseline-promoted`

- Subject: per plan entry, `<root>/.specify/specs/<slice>/`.
- Pass when: the directory exists in the baseline tree (routed clone
  for impl slices, hub for the contract slice).
- Evidence on fail: the slice name + missing directory path.
- Fault domain on fail: `skill-orchestration` — the baseline merge
  commit either did not run or did not create the spec directory.

---

Rule: `slice-archived`

- Subject: per plan entry, `<root>/.specify/archive/<slice>/`.
- Pass when: the directory exists.
- Evidence on fail: the slice name + missing directory path.
- Fault domain on fail: `skill-orchestration` — the merge step did
  not author the archive directory the C10 baseline commit reads.

---

Rule: `implementation-slice-reads-baseline-contract`

- Subject: per implementation slice (`project != null`), the bodies
  of `proposal.md`, `spec.md`, `design.md` (whichever exist).
- Pass when: at least one of those bodies references the baseline
  `contracts/` tree. Accepted markers: a `contracts/<file>.yaml`
  path reference, a backtick-quoted `contracts/...` path, or the
  exact phrase "baseline contracts".
- Evidence on fail: the slice name + the candidate file paths.
- Fault domain on fail: `capability-brief` — implementation slices
  must `depends-on` the contract slice and consume the baseline
  rather than authoring contract YAML inline. This is the **load-
  bearing RM-01 contract-first invariant** (mirrors the
  `implementation-slices-author-contract-shapes-inline` negative
  expectation in [`../scenario.md`](../scenario.md#negative-expectations)).

## Contract-Build Rules (C13) — Asserted

The five rules below are owned by C13. They run only when the suite
is driven through a backend that emits a real contract YAML bundle
under `<hub>/contracts/` — today the `contracts-build` backend
(per-slice phase driver: contract slice →
`ContractsBuildPhaseDriver`, implementation slices →
`StubPhaseDriver`). Under any other backend the contract YAML tree
is empty, so the rules cleanly demote to `skip` rather than `fail`
(the "wrong backend" signal documented in
[`../scenario.md`](../scenario.md#assertions)).

The handlers gate on `ctx.run.executeState` being populated AND on
the contracts directory containing at least one `.yaml` file.
Cascade-skip semantics mirror the define / merge family: upstream
`setup-*` or `plan-*` failure demotes every rule to `skip`. The
on-disk truth is the hub's `contracts/` tree (the contract slice is
projectless — there is no routed-clone path to inspect).

The `ContractsBuildPhaseDriver` writes byte-for-byte stable bodies
marked with a `# STUB:` header so an operator can tell at a glance
that the bundle is a deterministic fixture rather than real
`/spec:build` output, but the YAML itself is valid OpenAPI 3.1 /
JSON Schema 2020-12 and passes the contracts WASI tool unchanged —
which is what `contract-slice-yaml-validates-via-tool` proves.

---

Rule: `contract-slice-emits-yaml-artifacts`

- Subject: `<hub>/contracts/**/*.yaml` (recursive).
- Pass when: at least one regular file with a `.yaml` (or `.yml`)
  extension exists under the directory.
- Evidence on fail: the contracts directory and a note that no YAML
  was found.
- Fault domain on fail: `specialist-generation` — the contracts
  build skill produced no artifacts. (Skipped, not failed, when the
  contracts dir is empty AND no contracts-build driver ran — that
  is the wrong-backend signal.)

---

Rule: `contract-slice-yaml-validates-via-tool`

- Subject: `specify tool run contract -- <hub>/contracts --format
  json`. The handler stages a one-shot scratch project so the WASI
  tool resolves through a capability sidecar (matches the
  invocation pattern in `specify-cli/tests/contract_tool.rs`).
- Pass when: the parsed JSON's `ok` field is `true` (or, in the
  legacy v1 schema, `status` equals `clean`).
- Evidence on fail: the captured stdout (or its first 240 chars when
  the body is large).
- Fault domain on fail:
  - `specialist-generation` — the validator returned `ok: false`
    (a finding in the YAML; the contract skill produced invalid
    output);
  - `cli-substrate` — the validator exited with a code other than
    0/1 (binary mis-resolved, capability sidecar wrong, contract
    WASM binary missing). This case downgrades to `skip` when the
    handler cannot find the WASM at all (the operator-friendly
    "build the contract validator" signal).

---

Rule: `contract-slice-includes-openapi-or-asyncapi`

- Subject: `<hub>/contracts/http/` and `<hub>/contracts/messages/`.
- Pass when: at least one regular `.yaml` file exists under either
  directory.
- Evidence on fail: both subdir paths and a note that neither
  produced a YAML.
- Fault domain on fail: `specialist-generation` — the contract skill
  emitted only schemas (or only neither), missing the operator-
  facing endpoint document. Skipped, not failed, when no contract
  YAML at all was emitted.

---

Rule: `contract-slice-includes-required-schemas`

- Subject: `<hub>/contracts/schemas/oauth-token-request.yaml`,
  `<hub>/contracts/schemas/oauth-token-response.yaml`,
  `<hub>/contracts/schemas/error-response.yaml`. The list mirrors
  the `CONTRACT_YAML_PATHS` constant in
  `acceptance/runner/backends/contracts-build-driver.ts` so the
  driver remains the source of truth.
- Pass when: every required schema file exists.
- Evidence on fail: the missing schema rels.
- Fault domain on fail: `specialist-generation` — the OAuth fixture
  brief implies a token request, token response, and error
  response; missing any of them means the contract skill did not
  consume the brief faithfully. Skipped when no contract YAML at
  all was emitted.

---

Rule: `contract-baseline-files-present`

- Subject: every `<hub>/contracts/<rel>` path enumerated in
  `CONTRACT_YAML_PATHS` (one per emitted YAML).
- Pass when: every path is a regular file after the merge stage
  ran (i.e. the per-slice merge promoted the contract bundle to
  the hub baseline tree).
- Evidence on fail: the missing baseline path per record.
- Fault domain on fail: `skill-orchestration` — the baseline merge
  step dropped one or more contract YAMLs. Skipped when no
  contract YAML at all was emitted.

## Omnia-Build Rules (C14a) — Asserted

The five rules below are owned by C14a. They run only when the suite
is driven through a backend that emits a real Omnia crate skeleton
under the routed clone's `crates/<crate>/` tree — today the
`omnia-build` backend (per-slice phase driver: contract slice →
`ContractsBuildPhaseDriver`, omnia-capability slices →
`OmniaBuildPhaseDriver`, other slices → `StubPhaseDriver`). Under any
other backend no `Cargo.toml` is written, so the rules cleanly
demote to `skip` rather than `fail` (the "wrong backend" signal
documented in [`../scenario.md`](../scenario.md#assertions)).

The handlers gate on `ctx.run.executeState` being populated, on at
least one Omnia-capability slice having executed, AND on a
`crates/<crate>/Cargo.toml` existing on the routed clone (probing
`Cargo.toml` rather than the crate directory because
`StubPhaseDriver`'s residue path may incidentally create the
`crates/<crate>/src/` parent dir without writing a manifest — only
`OmniaBuildPhaseDriver` writes `Cargo.toml`). Cascade-skip
semantics mirror the contracts-build family: upstream `setup-*` or
`plan-*` failure demotes every rule to `skip`. The on-disk truth is
the routed project clone under
`<hub>/.specify/workspace/<project>/crates/<crate>/` (the Omnia
slice is project-routed — there is no hub-level path to inspect).

The `OmniaBuildPhaseDriver` writes byte-for-byte stable bodies
marked with a `# STUB:` / `// STUB:` header so an operator can tell
at a glance that the crate is a deterministic fixture rather than
real `/spec:build` output, but the TOML and Rust themselves are
syntactically valid (cargo can parse the manifest, rustc can parse
the source) and steer clear of every crate on
`plugins/omnia/references/guardrails.md` §Forbidden Crates.

---

Rule: `omnia-slice-emits-cargo-toml`

- Subject: `<hub>/.specify/workspace/<project>/crates/<crate>/
  Cargo.toml` for every Omnia-capability slice on
  `executeState.slices`. The `<crate>` mapping comes from
  `OMNIA_SLICE_TO_CRATE` in
  `acceptance/runner/backends/omnia-build-driver.ts` (with a
  kebab → snake_case fall-back), so the driver remains the
  source of truth.
- Pass when: the `Cargo.toml` exists, is non-empty, and contains
  a `[package]` table with a `name = ...` line (a crude TOML
  shape probe; full grammar is out of scope for an assertion).
- Evidence on fail: the missing path or the malformed body
  pointer.
- Fault domain on fail: `specialist-generation` — the Omnia
  build skill produced no manifest (or one cargo cannot parse).
  Skipped, not failed, when no Omnia slice ran or no manifest
  was emitted at all (the wrong-backend signal).

---

Rule: `omnia-slice-emits-lib-rs`

- Subject: `<hub>/.specify/workspace/<project>/crates/<crate>/
  src/lib.rs` (the residue file the per-slice driver writes
  through `driveSliceWithBodies`).
- Pass when: the file exists and the trimmed body is non-empty.
- Evidence on fail: the missing path or an "empty body" note.
- Fault domain on fail: `specialist-generation` — the Omnia
  build skill produced no library entrypoint. Skipped when no
  Omnia slice ran or no `Cargo.toml` was emitted (i.e. the
  build never actually fired).

---

Rule: `omnia-slice-residue-under-routed-project`

- Subject: every workspace-relative path the driver advertises
  for the slice (`Cargo.toml`, `src/lib.rs`, `src/providers.rs`).
- Pass when: every path starts with `crates/`.
- Evidence on fail: the offending paths (those outside `crates/`).
- Fault domain on fail: `specialist-generation` — the build
  skill leaked output above the crate root. Skipped when no
  Omnia slice ran.

---

Rule: `omnia-slice-no-output-outside-project`

- Subject: a small allowlist of "common-mistake" forbidden
  paths at the routed-clone root: `Cargo.toml`, `src/lib.rs`,
  `lib.rs`, `target/`. We probe a fixed list rather than
  walking the tree to keep the assertion fast.
- Pass when: none of the forbidden paths exist on disk.
- Evidence on fail: the offending root paths and their
  absolute paths under the slot.
- Fault domain on fail: `specialist-generation` — the Omnia
  build skill wrote build output outside the crate tree (a
  stray top-level manifest, source file, or `target/` dir).
  Skipped when no Omnia slice ran or no `Cargo.toml` was
  emitted at all.

---

Rule: `omnia-baseline-files-present`

- Subject: `<hub>/.specify/workspace/<project>/.specify/
  specs/<slice>/proposal.md`, `tasks.md`, plus the matching
  `.specify/archive/<slice>/` directory.
- Pass when: all three exist after merge.
- Evidence on fail: the list of missing baseline rels under the
  routed clone.
- Fault domain on fail: `skill-orchestration` — the per-slice
  merge dropped the spec dir (or the archive promotion failed).
  Skipped when no Omnia slice ran or the build never wrote a
  manifest.

## Vectis-Build Rules (C14b) — Asserted

The five rules below are owned by C14b. They run only when the suite
is driven through a backend that emits a real Vectis composition +
shell skeleton inside the routed mobile clone — today the
`vectis-build` backend (per-slice phase driver: contract slice →
`ContractsBuildPhaseDriver`, vectis-capability slices →
`VectisBuildPhaseDriver`, other slices → `StubPhaseDriver`). Under
any other backend no `composition.yaml` is written at the project
root, so the rules cleanly demote to `skip` rather than `fail`
(the "wrong backend" signal documented in
[`../scenario.md`](../scenario.md#assertions)).

The handlers gate on `ctx.run.executeState` being populated, on at
least one Vectis-capability slice having executed, AND on a
project-root `composition.yaml` existing on the routed clone
(probing `composition.yaml` rather than the residue file because
`StubPhaseDriver`'s residue path may incidentally create the
`apps/mobile/` parent dir without writing the composition — only
`VectisBuildPhaseDriver` writes `composition.yaml`). Cascade-skip
semantics mirror the contracts-build / omnia-build families:
upstream `setup-*` or `plan-*` failure demotes every rule to
`skip`. The on-disk truth is the routed project clone under
`<hub>/.specify/workspace/<project>/` (the Vectis slice is
project-routed — there is no hub-level path to inspect).

The `VectisBuildPhaseDriver` writes byte-for-byte stable bodies
marked with a `# STUB:` / `// STUB:` header so an operator can
tell at a glance that the output is a deterministic fixture rather
than real `/spec:build` output, but the YAML and Swift themselves
are syntactically valid (the YAML parses cleanly and conforms to
the `version: 1` + `screens` shape required by
`capabilities/vectis/composition.schema.json`; `swiftc -parse`
accepts the Swift body).

---

Rule: `vectis-slice-emits-composition-yaml`

- Subject: `<hub>/.specify/workspace/<project>/composition.yaml`
  for every Vectis-capability slice on `executeState.slices`.
- Pass when: the file exists, parses as YAML, has `version: 1`,
  and carries either a non-empty `screens` map or a `delta`
  document (the schema's `oneOf`).
- Evidence on fail: the missing path or the malformed-shape note
  (parse error, missing `version`, both `screens` and `delta`,
  empty `screens`).
- Fault domain on fail: `specialist-generation` — the Vectis
  build skill produced no composition (or one that does not
  match the Vectis composition shape). Skipped, not failed,
  when no Vectis slice ran or no composition was emitted at all
  (the wrong-backend signal).

---

Rule: `vectis-slice-emits-screen-files`

- Subject: `<hub>/.specify/workspace/<project>/apps/mobile/
  login_screen.swift` (the residue file the per-slice driver
  writes through `driveSliceWithBodies`).
- Pass when: the file exists and the trimmed body is non-empty.
- Evidence on fail: the missing path or an "empty body" note.
- Fault domain on fail: `specialist-generation` — the Vectis
  build skill produced no platform shell file. Skipped when no
  Vectis slice ran or no `composition.yaml` was emitted (i.e.
  the build never actually fired).

---

Rule: `vectis-slice-residue-under-routed-project`

- Subject: every workspace-relative path the driver advertises
  for the slice (`composition.yaml` at the project root,
  `apps/mobile/login_screen.swift` under `apps/`).
- Pass when: every path either equals `composition.yaml` (project
  root) or starts with `apps/`.
- Evidence on fail: the offending paths (those neither at the
  project root nor under `apps/`).
- Fault domain on fail: `specialist-generation` — the build
  skill leaked output above the platform-app root. Skipped when
  no Vectis slice ran.

---

Rule: `vectis-slice-no-output-outside-project`

- Subject: a small allowlist of "common-mistake" forbidden
  paths at the routed-clone root: `LoginScreen.swift`,
  `MainActivity.kt`, `LoginActivity.kt`, `Pods/`,
  `node_modules/`, `build/`, `DerivedData/`. We probe a fixed
  list rather than walking the tree to keep the assertion fast.
- Pass when: none of the forbidden paths exist on disk.
- Evidence on fail: the offending root paths and their absolute
  paths under the slot.
- Fault domain on fail: `specialist-generation` — the Vectis
  build skill wrote build output outside the platform-app
  layout (a stray top-level Swift file, an Android entrypoint
  at the wrong level, vendored package dirs, or a build tree).
  Skipped when no Vectis slice ran or no `composition.yaml` was
  emitted at all.

---

Rule: `vectis-baseline-files-present`

- Subject: `<hub>/.specify/workspace/<project>/.specify/
  specs/<slice>/proposal.md`, `tasks.md`, plus the matching
  `.specify/archive/<slice>/` directory.
- Pass when: all three exist after merge.
- Evidence on fail: the list of missing baseline rels under the
  routed clone.
- Fault domain on fail: `skill-orchestration` — the per-slice
  merge dropped the spec dir (or the archive promotion failed).
  Skipped when no Vectis slice ran or the build never wrote a
  composition.
