# RFC-95 review: findings and recommendations

> Status: Pre-implementation review of [publication sets](rfc-95-publication-sets.md) and [host surface](rfc-95-host-surface.md), against the [programme spine](platform.md) and the current engine / adapters code. Not a new RFC. Fold the recommendations into those two documents (and the one `platform.md` line they share) before implementation starts.
>
> Verdict: **not implementation-ready as written.** Publication-set product policy is close. The host-surface RFC is not. They can become one sequenced Emery change after the locks below; they cannot ship as one undifferentiated cut that also retires `emery:origins` / `emery:ingest`.

## Verdict

[RFC-95 publication sets](rfc-95-publication-sets.md) is the right substitute for deleted `apply`: one member per adapter-bearing target, operator-owned Git and forge writes, archive as a whole-set observation gate, accepted CID distinct from Git SHA. The contraction kernel it names already exists (`publication-target-cycle` in `crates/project/src/plan/decomposition/contraction.rs`). Facts, `--unverified`, and the schemars golden are specified enough to build once membership, placement, rematerialize, and forge lookup are tightened.

[RFC-95 host surface](rfc-95-host-surface.md) is correctly Law 2 (no `emery:publication` / `emery:forge`; `emery:adapter` and `emery:exec-mode` stay). It is not a build spec. Its materialize sequence cannot be expressed by today's launcher (mounts are fixed at process start; the guest has already exited if the host creates the checkout afterward). It also couples the delivery gap to a git-aware blobstore in `augentic/omnia-backends`, `omnia:identity` (unwired in Emery), and a WIT-world rewrite of implemented RFC-88 / RFC-104 locator fetch. That coupling is the main reason the pair is not ready.

Adapters stay consumers. No source/target WIT or adapter prose change is required for the publication worktree or archive observation. AC7's two-repo fixture is dogfood of a two-member set, not a reason to bump `emery:adapter` for this cut.

## Findings

These are defects or holes in the current text, grounded in code that already exists.

### F1 — “One implementation cut” couples delivery to locator-fetch retirement

Publication reads the RFC-87 snapshot store and writes a filesystem mount. Survey and bind fetch are a different seam (`emery:origins` / `emery:ingest`, D9 bounds, recorded-CID skip, moved-branch warning). Folding them makes the critical-path item in `platform.md` depend on a third repository and a host-world rewrite of implemented RFC-88 / RFC-104. The product RFC's implementation requirements and the host RFC's “What this work ships together” / rejected “sequence as separate cuts” all state that coupling.

### F2 — Materialize choreography is impossible as written

Host surface steps 1–4 say the launcher decides the path, mounts it for the execute that materializes, the guest writes the CID, then the host stages the index. D11 says materialize is a side effect of `plan execute` when a target's last in-scope merge lands. Launcher mounts are composed before dispatch (`omnia::runtime!`, `launcher::Policy`). There is no mid-loop preopen and no per-target completion hook. If the host creates the checkout after the guest returns, the guest cannot write the tree. The host RFC's own edit-points list omits this.

### F3 — D1 / D11 “every entry” fights `plan drop`

`plan drop` keeps the entry and stamps `dropped_at`. Execute, refine, epoch, and gaps already use `in_scope`. “Every entry for a target must be on a committed wave” would leave a dropped-only target (or remaining dropped siblings) unable to materialize and unable to archive. Membership and the complete entry set must be in-scope entries only.

### F4 — Rematerialize and sealing are unspecified

Idempotency covers match / dirty / already-committed. It does not say what happens when a later leaf or `plan amend` produces a new final CID on an uncommitted worktree, or whether topology edits after `plan.publication.materialized` may add, remove, or rebind that target.

### F5 — Placement invents a clone the binding is forbidden to record

D2 forbids a local clone path on the target binding. D11 then says `git worktree add` from “an existing clone,” with no locator for finding one. The RFC also oscillates between a sibling checkout, `$EMERY_HOME/publication/<target>/`, and (in review) `<project-root>/.emery/publication/<target>`. Laptop search is not a deployment rule.

### F6 — D10 is a contract sketch, not a forge API

Find/read semantics are clear; GitHub vs GitLab, how `https://github.com/org/repo@sha` becomes a REST endpoint, identity name, credential lifetime, draft / squash / rebase, and HTTP-failure vs `publication-unverified` vs `pending` are not. Lookup of “exactly one open-or-merged pull request” cannot identify an already-closed PR on retry. The engine guest exports `wasi:http/incoming-handler` and does not import outgoing HTTP; `omnia:identity` is not wired in `src/main.rs`. Archive would be the first HTTP client in the guest unless the RFC names another home.

### F7 — Branch and trailer are not unique over time

`change/<plan>` and `Emery-Change: <plan>` collide when a plan name is reused against the same repository. D10's lookup then finds the previous change's merged PR and false-verifies, or fails closed spuriously. No plan-digest or epoch disambiguator is decided.

### F8 — Archive never checks the landed tree against the accepted CID

D5 / D10 verify trailer, merged state, and landing order. An operator who amends the commit before push archives green. That may be deliberate (operator Git authority, D5 “observes rather than confirming”), but it is unstated.

### F9 — Byte-stable `order` has no algorithm

“Stable topological numbering among comparable members” is not a named sort. Equal `merged-at` on an ordered pair has no pass/fail rule.

### F10 — Drain currently implies archive will succeed

`plan status` today projects `/emery:finalize` when the plan is drained. After this RFC, archive is a publication gate. There is no publication milestone and no copy that drain does not imply archive will succeed. There is still no publication verb — that part is correct.

### F11 — Index staging, exec bits, GC roots, and vocabulary are unaddressed

One execute invocation may drain several targets; the only host hook today is process exit. `emery:exec-mode` exists because `wasi:filesystem` has no mode bits, and it is rooted at the workspaces root — the publication worktree is a new root. Archive still treats accepted CIDs as live GC roots because “checkout is never written”; after RFC-95 the product *is* written to a publication worktree, and the RFC does not say whether those CIDs stay roots. AGENTS.md, RFC-87/88 prose, and `docs/standards/workflow.md` all assert the operator checkout is never written; in-place one-member materialize writes it (new branch, tree + index).

### F12 — D8 has no public invocation surface

External records “validate against that golden's Rust wire type” and “project through the same read surface,” but D7 adds no publication subcommand. Under the integration-first posture, acceptance criterion 6 is unreachable at a public boundary as written.

### F13 — The blobstore migration does not fit what it would replace

One `WasiBlobstore` is bound today (filesystem objects at the snapshots root). Locator trees cannot share that ctx without a launcher multiplexer. `wasi:blobstore` has no channel for ingest's recorded-CID skip, moved-branch `prior` warning, or D9 `Policy` / `Meter` bounds. Those bounds are Emery product policy; a generic git blobstore that clones with ambient credentials would silently break RFC-88 D9. Container naming is an edit point; the multiplexer and policy wrap are the actual design, and they are not required to close the delivery gap.

### F14 — AC7 overstates adapter WIT breakage

Retiring origins/ingest is a `workflow`-world import change inside the published `emery:adapter` package. Source and target operations adapters export do not change. Native tests run `Ingest` / `Origins` in-process; deleting the traits is a host-world break for `emery-native` / eval, not an adapter operation bump. Forcing the three-repo release train onto publication's critical path is a packaging accident.

## Recommendations

Write these into the two RFCs (and the one `platform.md` sentence) as decided text, not as open questions. Then implement.

### R1 — Split the “one cut”

Publication worktree + archive observation ship first. Keep `emery:origins` / `emery:ingest`. Delete the lines that require a git-aware blobstore, `omnia-backends`, and origins/ingest retirement in the same implementation.

In [publication sets](rfc-95-publication-sets.md): drop “mounted worktree + git-aware blobstore, one cut” from the implementation requirements; drop the rejected alternative that forbids sequencing them.

In [host surface](rfc-95-host-surface.md): status stays Design until R2–R10 are in; “What this work ships together” becomes the publication mount + index staging + forge projector. Locator-fetch retirement moves to a follow-on section (R14).

In [platform.md](platform.md): RFC-95 host line becomes Omnia generic interfaces, mounted worktree, no `emery:publication` / `emery:forge`; git-aware blobstore is not this staffing cut.

### R2 — Lock materialize choreography

Replace the contradictory host sequence with:

1. At `plan execute` start, the launcher provisions every in-scope member path (in-place `.` when D11 allows it, otherwise `$EMERY_HOME/publication/<target>/`).
2. The guest writes files with the existing snapshot kernel (`Store::materialize`). It does not run Git and does not encode Git objects.
3. After the guest returns, the launcher stages the index (`git add`) on each provisioned worktree that materializes in that invocation. No commit.
4. Dirty / already-committed detection is host Git state (index / `HEAD`), surfaced as the typed errors D11 already names.

Do not add a publication-specific `emery:*` WIT. Do not wait on a generic Omnia command-completion hook unless one already exists at the pinned Omnia version.

### R3 — Membership is in-scope only

Change D1 / D11 “every entry for a target” to **in-scope entries** (the same `plan::in_scope` execute, refine, epoch, and gaps already use). A dropped-only target is not a member and does not block archive. Contraction for materialize and archive uses the in-scope graph. Author-time cycle validation may keep the full graph; cycles cannot appear from drop alone.

### R4 — Say what rematerialize does

- Tree still matches the CID → no-op.
- Operator has uncommitted edits → `publication-worktree-dirty`.
- Operator has committed → do not rewind.
- A later leaf or `plan amend` produces a **new** accepted CID on an **uncommitted** worktree → rematerialize.
- After `plan.publication.materialized`, topology edits that would add, remove, or rebind that target's in-scope entries are rejected until archive.

### R5 — Collapse placement to two cases

- In-place, one member, product repo clean and at the recorded parent → that repository on `change/<plan>`.
- Otherwise → reuse or create `$EMERY_HOME/publication/<target>/`.

No “existing clone” search. No clone path on the binding. No `<project-root>/.emery/publication/`. First-time clone root is the `$EMERY_HOME` path, not an example.

### R6 — GitHub-only v1 for D10, host-side projector

Specify in D10 / host surface:

- How `https://github.com/org/repo@sha` becomes the REST repository.
- Token via `omnia:identity` (identity name, lifetime, redaction; public/unauthenticated reads when no token is needed). Confirm `omnia:identity` exists at the pinned Omnia version before coding; if it does not, name the substitute in the host RFC.
- Lookup by `(repository, branch)` across **open, merged, and closed** pull requests.
- Exactly one matching trailer succeeds; zero → `unpublished`; several → fail closed.
- Draft, squash, and rebase count as merged if the forge says merged.
- Transport / auth failure is not `publication-unverified`. `pending`, `unverified`, and HTTP failure are three different outcomes.

Put archive HTTP in a small **host-side projector** that uses `wasi:http` + `omnia:identity`. The guest consumes the typed result. No `emery:forge`. No `gh` subprocess. No forge writes. RM-17 stays out.

### R7 — Disambiguate reused plan names

Keep `change/<plan>` and `Emery-Change: <plan>` as the operator-facing markers. Lookup also matches the covering plan digest (or the `plan.execute.started` epoch digest) so a later change with the same plan name cannot false-verify against the previous merged pull request. Put that digest in the trailer **or** in the find contract — pick one in D3 / D10 and use it consistently.

### R8 — Explicit non-check: landed tree vs accepted CID

D5 verifies trailer, merged, and landing order. It does **not** require the merge-commit tree to equal the accepted CID. Operator Git is authoritative. Write that sentence into D5 so it is not a silent hole. Do not alias accepted CID to Git SHA.

### R9 — Name the order algorithm

Ranks are Kahn topological order over the contracted DAG with a **sorted** ready set. Unrelated members omit `order`. An ordered pair with equal `merged-at` fails verification; do not invent a tie-break. Unchanged plan, facts, and forge state remain byte-stable because the algorithm is closed.

### R10 — `plan status` after drain

Status grows a publication milestone: per-member materialized / committed / pull request open / merged, plus the next operator Git step (`commit` / `push` / open PR / land). No publication verb. Drain must not imply that `emery plan archive` / `/emery:finalize` will succeed.

### R11 — Index, exec bits, GC, and vocabulary

- One execute may drain several targets; index staging runs once per provisioned worktree after that invocation returns.
- Either widen `emery:exec-mode` to the publication root or restore executable bits during index staging. Pick one in the host RFC.
- After materialize, accepted CIDs **remain** snapshot GC roots. The publication worktree is not the store; `plan archive` sweep policy does not change because a checkout now exists.
- D11 in-place writes the operator checkout on a new branch. Implementation requirements must list AGENTS.md, `docs/standards/workflow.md`, and the RFC-87/88 “checkout is never written” sentences as same-change prose.

### R12 — D7 / D8 and archive crash/retry

External records validate against `crates/project/answers/publication.schema.json` in crate-level tests. This cut adds no publication subcommand and no `--record` flag. Rewrite acceptance criterion 6 to that boundary.

Archive mutation order: project → verify → journal `plan.publication.projected` / `plan.publication.member-landed` → mutate archive → sweep. A crash after journal and before mutation is resume-safe. `--unverified` still journals `plan.publication.unverified-archive` first. `--force` still skips only the outstanding-work ladder.

### R13 — Implementation sequence once the documents agree

One Emery change, two conflict domains that close the `platform.md` delivery gap. RFC-96 is not a prerequisite (serial execute can materialize when a target's last in-scope merge lands). RFC-96 later widens wave membership without changing the fact shape. RM-17 stays out.

| Cut | Ships | Depends on |
| --- | --- | --- |
| **A — export** | Publication worktree, D11 placement, `plan.publication.materialized`, in-scope membership, contraction reuse, status milestone, operator Git docs. Keep `emery:origins` / `emery:ingest`. | Existing RFC-87 store + `Store::materialize` + launcher Git porcelain + one publication mount. |
| **B — observe** | Projector golden, GitHub-only D10 reads, archive gate, `--unverified`, `projected` / `member-landed` facts. | Cut A facts. Host-side HTTP projector (R6). |

Integration tests use temp Git repositories and HTTP doubles. They do not need a git blobstore backend.

### R14 — Host-surface follow-on (write it; do not build it in this staffing cut)

Keep Law 2: no `emery:publication` / `emery:forge`. If origins/ingest ever retire:

- One `wasi:blobstore` import needs a launcher multiplexer (snapshot filesystem vs locator git).
- D9 bounds stay Emery product policy wrapping the backend, not a generic credentialed clone in `omnia-backends`.
- Ingest's recorded-CID skip and moved-branch warning need an explicit blobstore equivalent, or ingest stays.
- Prefer splitting the engine `workflow` world out of the published `emery:adapter` WIT package so a later host cut is host-only per `docs/release.md`.
- AC7 remains a two-member publication fixture; it does not gate RFC completion on a live adapter-train release.

## What not to change

- Operator owns every Git commit and every forge write. No `plan materialize` / `commit` / `publish` verb.
- Members remain distinct adapter-bearing `slices[].target` keys. Composite / read-only / unused targets are not members.
- Order comes only from contracted leaf `depends-on`. No second decomposition reader.
- Publication worktree is not an RFC-87 workspace and is not a forge write.
- Workers do not write the worktree. RFC-100's fence stands.
- `--unverified` vs `--force` composition as already written in D5.
- First-party adapters remain consumers of the engine pin.

## Done when

Both RFCs, plus the RFC-95 sentence in `platform.md`, contain R1–R12 as closed decisions (R14 as an explicit later section, not as this cut). Host surface status can then move from “Design. Edit this file” to the same Active follow-on status as publication sets. Implementation starts from that text, as cuts A then B.
