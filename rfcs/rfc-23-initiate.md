# RFC-23: Rehome the Change Umbrella as `/change:initiate`

> Status: Draft - Depends: [RFC-9](archive/rfc-9-platform.md), [RFC-13](archive/rfc-13-extensibility.md) - See also: [RFC-20](rfc-20-survey.md)

## Abstract

Rehome today's `/change:plan <name> orchestrate` mode as a dedicated top-level skill at `/change:initiate <name>`. The cross-repo umbrella sequence (brief → registry → plan → execute → push → PR handoff → finalize) becomes a peer of `/change:plan`, not a sub-mode of it.

The change is a pure rename + relocation: byte-identical seven-step internals, identical halt classifications, identical recovery sequences, identical `--dry-run` semantics. No new CLI verbs, no new on-disk state, no new behaviour. The existing `/change:plan <name> orchestrate` invocation continues to work for one release window via a thin compatibility forward that prints a deprecation notice on stderr and execs the new skill.

The win is conceptual: `/change:plan` returns to being purely about authoring `plan.yaml` via its five-step loop; `/change:initiate` owns end-to-end orchestration; `/change:execute` keeps driving the per-slice loop; the slash surface gains a clean bookend with `specify change finalize`.

## Motivation

When the cross-repo umbrella landed (RFC-9), `/change:plan` was the only Layer 2 skill ready to host it, so the seven-step sequence shipped as the `orchestrate` positional. That was pragmatic, but two years on it has accumulated three observable problems:

- **Double duty obscures layering.** `/change:plan <name>` authors `plan.yaml`. `/change:plan <name> orchestrate` authors `plan.yaml`, runs `/change:execute`, pushes branches, observes PR state, and runs `specify change finalize`. New operators reading "the plan skill" reasonably expect it to plan; they do not expect it to drive PRs to merge.
- **Documentation overhead.** The plan skill's `SKILL.md`, `references/runbook.md`, and `orchestration.md` together describe two distinct surfaces (default mode, orchestrate mode) with overlapping pre-flight, mode handling, and re-entry semantics. Splitting the umbrella into its own skill removes that overlap.
- **Asymmetric bookend.** The lifecycle's closing verb is `specify change finalize`. The opening verb that runs the matching umbrella is `/change:plan <name> orchestrate` — a positional on a different surface. Operators learning the system have to discover the bookend rather than read it off the slash menu.

The umbrella is composition-only today (`orchestration.md` §"Composition discipline") and stays composition-only after rehoming. There is no behavioural debt to migrate — only a slash-command rename and a documentation move.

## Design

### Principles

1. **Composition only.** The new skill is a rename + rehoming of today's orchestration mode. It introduces no new CLI verbs, no new on-disk state, no new halt classifications, no new recovery sequences. Any temptation to add behaviour during the rehoming is a bug — file it against `/change:plan`, `/change:execute`, or the underlying CLI verb instead.
2. **Byte-identical user-visible behaviour.** Every halt the underlying skills surface (`stuck`, `halted`, `driver-interrupted`, `registry-amendment-required`, `pending-checks`, `failed-checks`, `branch-pattern-mismatch`, `dirty` workspace) flows through unchanged. Diagnostics are not paraphrased.
3. **Single-release deprecation window.** `/change:plan <name> orchestrate` keeps working for one release; the deprecation notice names the removal release explicitly. After that release, the positional is removed and `/change:plan` returns to a single mode.
4. **Bookend symmetry.** `/change:initiate` ↔ `specify change finalize`. The opening and closing surfaces both live on the change noun and read as a deliberate pair.
5. **Plan stays the planner.** After the rename, `/change:plan` has exactly one job — author `plan.yaml` via its five-step loop. The `orchestrate` positional is gone.

### Manual vs umbrella flow

Specify supports two flows for moving a change through its lifecycle. RFC-23 only changes the *name* of the umbrella — both flows existed before the rehoming and both remain after.

| Flow | Invocation | When to use |
| --- | --- | --- |
| **Manual** | `/change:plan <name> [...]` → operator review → `/change:execute loop` → (optional) `specify change finalize` | Hands-on work where the operator wants to inspect or edit `plan.yaml` (or `discovery.md` / `survey.md` / `workspace.md`) between authoring and execution; single-repo runs that don't need PR observation; teams that hand the plan off between people. |
| **Umbrella** | `/change:initiate <name> [...]` | Multi-repo cross-cutting work where the operator wants one command to drive every step from brief through merged PRs and finalize. Halts at any of the well-defined halt classifications and is re-entrant — recovery is "fix the cause, re-run `/change:initiate <name>`". |

The two flows share every underlying skill and CLI verb. The umbrella is composition over the same surfaces an operator would invoke by hand; nothing is reachable through `/change:initiate` that isn't also reachable through the manual flow. Picking between them is an operator preference about how much of the lifecycle to drive in one step.

**Where operator review fits.** Both flows surface plan-time slice decisions through `/change:plan`'s propose brief — an interactive accept/edit/reject loop per proposed slice (see [`propose.md`](../plugins/change/skills/plan/propose.md)). The umbrella does **not** add a second, post-plan review pause; once propose's loop exits cleanly and `specify plan validate` passes, the umbrella proceeds into `/change:execute loop` immediately. Operators who want a hard pause between authoring and execution (to review `plan.yaml` out of band, run `specify plan amend` against entries, or hand the plan to a teammate) should use the manual flow. Whether the umbrella should grow an opt-in pause is tracked under §Open Questions.

**Composite shape.** The umbrella treats `/change:plan` (step 3) and `/change:execute` (step 4) as units and observes their halt codes; it does not know their internal shape. With RFC-20's survey and synthesise briefs landed, the full picture is:

```text
/change:initiate <name>
├─ 1. brief                 specify change create
├─ 2. registry              specify registry validate
├─ 3. plan                  /change:plan <name>
│  ├─ 3(a) discovery        /change:analyze per input → discovery.md
│  ├─ 3(b) sync-workspace   specify workspace sync → workspace.md  (multi-repo only)
│  ├─ 3(b.5) survey         [RFC-20] DAG decomposition → survey.md
│  ├─ 3(b.6) synthesise     [RFC-20] reconciliation → discovery.md §Reconciliation
│  ├─ 3(c) propose          accept/edit/reject loop → specify plan add per slice
│  └─ 3(d) assignment       specify plan amend --project per entry  (multi-repo only)
├─ 4. execute               /change:execute loop
│  └─ per slice             /spec:define → /spec:build → /spec:merge
├─ 5. push                  specify workspace push
├─ 6. PR handoff            gh pr list (operator merges externally)
└─ 7. finalize              specify change finalize
```

Adding new sub-steps to either child skill (RFC-20 to plan; future RFCs to execute) does not change the umbrella's own surface — composition discipline is what keeps `/change:initiate` itself stable.

### Skill shape

| Aspect | Today (`/change:plan orchestrate`) | After RFC-23 (`/change:initiate`) |
| --- | --- | --- |
| Skill location | `plugins/change/skills/plan/orchestration.md` | `plugins/change/skills/initiate/SKILL.md` |
| Invocation | `/change:plan <name> orchestrate [...]` | `/change:initiate <name> [...]` |
| Bookend | Asymmetric (`orchestrate` ↔ `specify change finalize`) | Symmetric (`/change:initiate` ↔ `specify change finalize`) |
| Owns logic? | No (composition only) | No (composition only — same constraint, same verb-hygiene table) |
| Owns on-disk state? | No | No |
| Drives `/change:plan`? | Recursively (delegates to default mode) | As a peer skill (step 3 of the umbrella) |
| Pre-flight | hub presence, `specify` binary, kebab-case `<name>`, shape resolution | identical |
| `--dry-run` | observation-only; reads registry + plan + workspace state, invokes `/change:plan dry-run`, prints preview | identical |
| Re-entry | idempotent; on-disk state is the source of truth; never tracks own progress | identical |

The skill ships with the standard layout: `SKILL.md` (orientation surface — Critical Path, Reference table, Guardrails) plus `references/runbook.md` (the verbatim seven-step procedural body lifted from today's `orchestration.md`). This matches `/change:plan`'s and `/change:execute`'s structure.

### The seven-step internal sequence

Lifted verbatim from `plugins/change/skills/plan/orchestration.md` §"Internal sequence":

| Step | Invocation | Owner skill / verb | Halts |
| --- | --- | --- | --- |
| 1 Brief | `specify change create <name>` (when `change.md` is absent) | CLI | none — runs or no-ops |
| 2 Registry | `specify registry validate` | CLI | validation failures (description-missing, kebab violations, etc.) |
| 3 Plan | `/change:plan <name> [from ...] [against ...] [source ...]` (default mode) | sibling skill | propose-loop abort, `specify plan validate` failure |
| 4 Execute | `/change:execute loop` | sibling skill | `stuck`, `halted`, `driver-interrupted`, `registry-amendment-required` |
| 5 Push | `specify workspace push` | CLI | per-project `failed` |
| 6 PR handoff | `gh pr list` (read-only); operator merges externally | CLI / forge | any PR not yet `MERGED` |
| 7 Finalize | `specify change finalize` | CLI | guard refusals (plan absent, non-terminal entries, dirty workspace, unmerged PR) |

The umbrella never merges PRs. Step 6 observes remote PR state and waits; step 7 only runs after every PR is `MERGED`. This is unchanged from today.

### Naming

Skill identifier: `change-initiate` (matching the existing `change-plan`, `change-execute`, `change-analyze` pattern in SKILL.md frontmatter). Slash command: `/change:initiate`. The choice is grounded in three properties:

1. **Bookend symmetry with `specify change finalize`.** Initiate ↔ finalize reads as a deliberate open/close pair on the change surface.
2. **No collision with existing names.** `initiate` is not used as a verb anywhere on the slash, CLI, or artefact surface today. Specifically:
   - `/change:open` would overload PR vocabulary (and the everyday "open the file" sense).
   - `/change:start` is too generic and clashes with no specific surface, but reads as informal next to `finalize`.
   - `/change:draft` implies the artefact is provisional indefinitely.
   - `/change:scope` describes the brief's contents but lacks lifecycle resonance.
   - `/change:brief` collides with the `change.md` artefact name *and* with the brief-pipeline terminology inside `/change:plan` (where `briefs/<capability>/{discovery,propose,…}.md` are the per-capability briefs).
3. **Verb register.** "Initiate" reads as a deliberate, formal start — appropriate for a command that, on a populated multi-repo hub, can move dozens of slices through to merged PRs in one invocation.

### Backwards compatibility

For one release window, `/change:plan <name> orchestrate [...]` prints a deprecation notice on stderr and forwards to `/change:initiate <name> [...]` with the same positional inputs. The forwarding is mechanical: the positional flags and modes (`from`, `against`, `source`, `dry-run`, `extend`, `shape`) pass through unchanged.

Deprecation notice shape:

```text
deprecated: /change:plan <name> orchestrate is deprecated; use /change:initiate <name>.
            forwarding to /change:initiate; this forward will be removed in <release>.
```

The forward exits with the new skill's exit code unmodified. After the named release, the `orchestrate` positional is removed and `/change:plan` accepts only its default-mode invocation.

The `specify change` CLI verbs are untouched. `specify change create` (step 1), `specify change show`, and `specify change finalize` (step 7) keep their current shapes.

### Survey and synthesise (RFC-20 interaction)

If RFC-20 has landed by the time this RFC is implemented, `/change:initiate` does **not** see survey or synthesise directly. They are sub-steps of `/change:plan`'s brief pipeline (steps 3(b.5) and 3(b.6) per RFC-20 §"Pipeline ordering"); the umbrella only knows about the plan skill as a unit.

If RFC-20 has *not* landed, this RFC is unaffected — the umbrella stays exactly the seven steps in the table above. RFC-23 does not depend on RFC-20.

## Implementation Plan

1. **Scaffold the skill directory.** Create `plugins/change/skills/initiate/` with:
   - `SKILL.md` (orientation surface — name, description, argument-hint, Critical Path, Reference table, Guardrails). Mirror the shape of `/change:plan`'s `SKILL.md`.
   - `references/runbook.md` containing the verbatim seven-step body lifted from `plugins/change/skills/plan/orchestration.md`. The body is moved, not duplicated.
   - `references/re-entry.md` and `references/shapes.md` moved from the plan skill if they are umbrella-specific (audit during the move; if they cover both modes, split into shared + umbrella-specific halves).
2. **Move fixtures.** Migrate `plugins/change/skills/plan/fixtures/migrate-legacy/`, `fixtures/new-feature/`, and `fixtures/update-existing/` to `plugins/change/skills/initiate/fixtures/`. Update fixture transcripts so the invocation line reads `/change:initiate <name> [...]` instead of `/change:plan <name> orchestrate [...]`.
3. **Add the deprecation forward in `/change:plan`.** When the orchestrate positional is supplied, print the deprecation notice on stderr and exec `/change:initiate <name> [...]` with the same positional inputs. Add a regression fixture asserting the notice text and the exit-code passthrough.
4. **Trim the plan skill's documentation.** Remove the orchestration mode from `plugins/change/skills/plan/SKILL.md` (Orientation paragraph), `references/runbook.md` (mode deltas section), and any other plan-skill doc that mentions the umbrella. Replace those references with a short pointer to `/change:initiate` and RFC-23.
5. **Move `orchestration.md`.** Delete `plugins/change/skills/plan/orchestration.md`; its content lives at `plugins/change/skills/initiate/references/runbook.md` after step 1. Add a stub at the old path for one release that redirects readers to the new location, then remove the stub when the deprecation forward is removed.
6. **Update tutorials.** Rewrite `docs/tutorials/cross-repo-change.md`, `docs/tutorials/landing-a-change.md`, and `docs/tutorials/cross-repo-execute.md` to use `/change:initiate` instead of `/change:plan <name> orchestrate`. Update any inline `--orchestrate` flag references in `docs/how-to/` and `docs/explanation/` (notably `layered-stack.md` and `drop-down-a-layer.md`).
7. **Update CLI / framework reference docs.** Update `docs/reference/change-skills/index.md`, `docs/reference/change-component.md`, `AGENTS.md`, `.cursor/rules/project.mdc`, and `.cursor-plugin/marketplace.json` to mention `/change:initiate` alongside (or in place of) the `orchestrate` mode. The marketplace description currently reads "drives the cross-repo umbrella under `--orchestrate`" — rewrite to "drives the cross-repo umbrella as `/change:initiate`".
8. **CHANGELOG.** Add an entry recording the rename, the deprecation window, and the release in which the forward will be removed.
9. **Acceptance.** Add a regression to the cross-repo Deno acceptance suite asserting that `/change:initiate <name>` and `/change:plan <name> orchestrate` produce byte-identical output streams except for the deprecation notice on stderr (same plan, same execute trail, same push outcome, same finalize result).

## Migration

This RFC is **purely a rename + relocation**. There is no new behaviour to learn and no on-disk format change.

For operators:

- Replace `/change:plan <name> orchestrate [...]` invocations with `/change:initiate <name> [...]`. The positional inputs (`from`, `against`, `source`, `dry-run`, `extend`, `shape`) are unchanged.
- For one release window the old invocation continues to work; it prints a deprecation notice on stderr and forwards to the new skill. The notice names the removal release explicitly.
- `/change:plan` (without `orchestrate`) is unchanged. Operators who only author `plan.yaml` and run `/change:execute` separately need no migration.
- The seven-step internal sequence, halt classifications, recovery sequences, and `--dry-run` semantics are byte-identical to today's `orchestrate` mode — the only operator-visible change is the slash-command name.

For skill / fixture authors:

- Fixtures that invoke `/change:plan <name> orchestrate` move to the new skill's fixture directory and update their invocation line.
- The `orchestration.md` file at `plugins/change/skills/plan/` is deleted; its content lives at `plugins/change/skills/initiate/references/runbook.md`. Update cross-references accordingly.
- Internal docs that say "the orchestration mode" should say "the `/change:initiate` umbrella" instead.

For tutorial / how-to authors:

- All command snippets that read `/change:plan <name> orchestrate ...` become `/change:initiate <name> ...`.
- Layering diagrams that show `/change:plan` straddling Layer 2 authoring and Layer 2 orchestration get a third box: `/change:initiate` (orchestration), `/change:plan` (authoring), `/change:execute` (driver).

There is no breaking change in this release. The breaking change lands in the release named by the deprecation notice, when the forward is removed.

## Alternatives Considered

**Keep the umbrella as `/change:plan orchestrate` rather than rehoming it.** Rejected. The double-duty problem outlined in §Motivation does not get smaller as the umbrella accretes capabilities; rehoming is cheaper now than after RFC-20's survey/synthesise inserts more steps into the plan-skill brief pipeline.

**Pick `/change:open`, `/change:start`, `/change:draft`, `/change:scope`, or `/change:brief` instead of `/change:initiate`.** Considered. `open` overloads PR vocabulary and the everyday "open the file" sense; `start` is too generic and reads informal next to `finalize`; `draft` implies the artefact is provisional indefinitely; `scope` describes the brief's contents but lacks lifecycle resonance; `brief` collides with the `change.md` artefact name and with brief-pipeline terminology inside `/change:plan`. `initiate` was chosen for its bookend symmetry with `specify change finalize` and its absence of collisions.

**Make `/change:initiate` a pass-through wrapper around a new CLI verb `specify change initiate`.** Rejected. The umbrella is composition over Layer 2 skills (`/change:plan`, `/change:execute`) plus Layer 1 CLI verbs; a new top-level CLI verb would duplicate the seven-step orchestration outside the skill surface. Composition discipline stays the same as today's orchestration mode: the skill is the orchestrator; the CLI verbs underneath stay focused on single state-transition operations.

**Bundle the rehoming into RFC-20 (Survey-to-Plan Pipeline).** Considered. RFC-20 already touches the plan skill's brief pipeline, so a combined RFC would land both renames in one operator-facing release. Rejected because RFC-20 is scoped to "Survey-to-Plan Pipeline" and the umbrella rehoming is independent of survey — a reader of RFC-20 should not also have to consume an unrelated rename. The two RFCs land independently; if both land in the same release, the CHANGELOG combines the operator-facing notes.

**Drop `/change:plan orchestrate` immediately without a deprecation forward.** Rejected. Tutorials, operator scripts, and CI pipelines reference the old invocation; a hard removal in the same release as the rename would break consumers. One release window is the minimum acceptable courtesy; longer windows are an open question (see §Open Questions).

**Move the umbrella to `/change:execute orchestrate` instead of a new skill.** Rejected. The executor's job is "drive `plan.yaml` through the per-slice loop"; adding the umbrella's pre-execute steps (brief, registry, plan author) would replicate the same double-duty problem the rehoming exists to solve. A peer skill is the cleanest layout.

**Subsume `/change:plan` into `/change:initiate` so the slash menu reads as a single linear workflow (`/change:initiate` to author + drive, `/change:execute` to run the slice loop, with no separate planner verb).** Considered. The argument is conceptual symmetry: if `/change:initiate` is the planning *and* orchestration entry point, `/change:plan` looks like an internal step that doesn't need its own slash verb. Rejected for v1 because (a) the **manual flow loses its planning entry point** — operators who want to author `plan.yaml` and review it out of band before executing would have to invoke `/change:initiate --plan-only` (or some equivalent), inventing a new semi-mode this RFC tries to avoid; (b) `/change:plan` has its own complete brief pipeline (discovery → sync-workspace → [survey, synthesise] → propose → assignment) that is meaningful as a standalone surface for operators driving the plan loop without committing to the umbrella; and (c) the rename is independent of the rehoming and would inflate this RFC well beyond a pure rename. If the symmetry argument grows teeth (operator confusion about three change verbs vs two), it's a follow-up RFC against the slash surface, not part of RFC-23.

## Non-Goals

- **New behaviour inside `/change:initiate`.** The skill is composition only — pure rename and rehoming. New halt classifications, new recovery sequences, new pre-flight checks, new on-disk state, and forge-merge automation are all out of scope. Any such gap belongs in `/change:plan`, `/change:execute`, or the underlying CLI verb.
- **Removing `/change:plan orchestrate` immediately.** The deprecation forward stays for one release window so operator scripts and tutorials migrate without breakage. Removing the forward is a follow-up CHANGELOG entry, not part of this RFC's landing.
- **Renaming `specify change create` or `specify change finalize`.** The CLI surface stays stable; only the slash skill that wraps the umbrella is renamed.
- **Multi-plan orchestration.** RFC-3a's single `plan.yaml` invariant is preserved; `/change:initiate` drives one change at a time. Multi-plan output and parallel changes remain a separate concern (deferred to RFC-21 / RFC-22 territory).
- **Forge-agnostic land step.** Step 6 still uses `gh`; non-GitHub forges still fall back to the manual fallback path (merge by hand, re-run the umbrella to finalize). Forge abstraction is a separate concern.
- **Re-thinking the seven-step sequence.** The steps and their owners are unchanged. If the steps need to change, that is a separate RFC against the orchestration sequence itself.
- **Collapsing `/change:plan` into `/change:initiate`.** The two skills stay distinct: `/change:plan` owns the brief pipeline as a standalone surface for the manual flow; `/change:initiate` composes it as step 3. See §Alternatives for the trade-off.
- **Adding a post-plan / pre-execute pause to the umbrella.** `/change:initiate` proceeds straight from `specify plan validate` into `/change:execute loop`; an explicit `--pause-after-plan` gate is tracked under §Open Questions and is not part of this RFC's landing.

## Open Questions

1. **Deprecation window length.** This RFC proposes one release window (deprecation notice + forward, then removal). Is one release enough given how many tutorials, operator scripts, and CI configurations reference the old invocation? Current preference: one release, with the deprecation notice naming the removal release explicitly so consumers have a hard date to migrate by. Revisit only if usage telemetry (CHANGELOG feedback, GitHub issues) shows the window is too short.
2. **Runbook filename.** The seven-step body moves from `plugins/change/skills/plan/orchestration.md` to a new location under `plugins/change/skills/initiate/`. Should it land at `references/runbook.md` (matching `/change:plan`'s and `/change:execute`'s split between SKILL orientation and runbook procedure) or keep its existing `orchestration.md` filename? Current preference: `references/runbook.md` for consistency with sibling skills.
3. **Stub-redirect at the old path.** Should `plugins/change/skills/plan/orchestration.md` keep a one-line stub for the deprecation window pointing readers at the new location, or be deleted immediately? Current preference: keep the stub for the deprecation window, then delete with the forward.
4. **Backporting tutorials in the same release.** Should the tutorial rewrites land in the same release as the new skill (so day-1 documentation is consistent), or in the *next* release (so the deprecation notice in the old invocation lines up with the docs that mention it)? Current preference: same release; the deprecation notice already names `/change:initiate` so docs and notice agree.
5. **Should the new skill carry an `--orchestrate` flag for forward-compatibility with future modes?** Current preference: no. The skill exists *because* it is the orchestrator; a flag would suggest there is a non-orchestrating mode, which there is not.
6. **Explicit plan-review pause between umbrella step 3 and step 4.** Today (and after this RFC) the umbrella proceeds straight from `specify plan validate` into `/change:execute loop` without a second operator confirmation; operator review of slices happens inside `/change:plan`'s propose accept/edit/reject loop, not at the umbrella seam. Should `/change:initiate` grow an opt-in `--pause-after-plan` (or default-on-with-`--no-pause`) gate so operators can inspect `plan.yaml` out of band — running `specify plan amend`, comparing against `survey.md`, handing the plan to a teammate — before the executor runs? Current preference: no. Operators who want a hard pause use the manual flow (`/change:plan <name>` then `/change:execute loop`); adding a flag to the umbrella would invent a third semi-mode that has to be explained alongside the existing manual/umbrella split. Revisit if usage patterns show operators consistently re-entering the umbrella with `specify plan amend` edits between plan and execute (which would indicate the propose-loop review surface is insufficient and a post-plan gate is doing real work).

## References

- [RFC-9: Platform](archive/rfc-9-platform.md) — orchestration umbrella and shape inference; the original landing of the seven-step sequence.
- [RFC-13: Extensibility](archive/rfc-13-extensibility.md) — Layer 2 skill composition rules.
- [RFC-20: Survey-to-Plan Pipeline](rfc-20-survey.md) — peer RFC; if both land together, the umbrella's step 3 internally grows survey + synthesise sub-steps without affecting the umbrella surface.
- [`/change:plan orchestration.md`](../plugins/change/skills/plan/orchestration.md) — the seven-step body this RFC moves.
- [`/change:plan` SKILL.md](../plugins/change/skills/plan/SKILL.md) — the skill the orchestrate positional is removed from.
- [`/change:execute` SKILL.md](../plugins/change/skills/execute/SKILL.md) — the executor `/change:initiate` invokes at step 4.
- [`specify change` CLI reference](../docs/reference/cli/change.md) — `specify change create` (step 1) and `specify change finalize` (step 7), the bookends `/change:initiate` wraps.
- [`docs/explanation/layered-stack.md`](../docs/explanation/layered-stack.md) — where the umbrella sits in Specify's layered architecture (Layer 2).
- [`docs/tutorials/cross-repo-change.md`](../docs/tutorials/cross-repo-change.md) — the canonical worked example to update with the new skill name.
