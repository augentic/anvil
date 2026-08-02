<div class="hero">
<div class="eyebrow">How-to</div>
<h1 class="hero-title">Drop a slice</h1>

Abandon a slice without merging its specs into the baseline — the rollback counterpart to merge.

<div class="meta-row">

<span class="meta-chip"><strong>Skill</strong> /emery:drop</span>

<span class="meta-chip"><strong>Verb</strong> emery slice drop</span>

</div>

</div>


<div class="when">
<strong>When to use.</strong>

Use this guide when an in-progress slice should not land: the lead turned out to be wrong-sized, the requirement was overtaken by events, or refine surfaced evidence that invalidates the slice's premise.
</div>


<section id="drop-vs-merge" markdown="1">

<h2><span class="num">1</span> Drop vs merge</h2>

Merge and drop are the two terminal slice states. Merge applies the slice's spec deltas to the baseline, archives the slice, and stamps the plan entry `done`. Drop archives the slice **without** touching the baseline and stamps nothing on the plan:

| | `merged` | `dropped` |
| --- | --- | --- |
| Baseline specs | Updated with the slice's deltas | Untouched |
| Slice directory | Archived | Archived |
| Plan entry | Stamped `done` | Unchanged |

Drop is legal from any active slice state (`refining`, `refined`, `built`) — you do not need to finish a phase first.
</section>


<section id="run-it" markdown="1">

<h2><span class="num">2</span> Run it</h2>

```text
/emery:drop <slice>
```

or directly:

```bash
emery slice drop <name> --reason "lead was duplicate of account-registration"
```

The `--reason` string lands in the archive's `metadata.yaml` and the journal, so the audit trail says why the slice never merged.
</section>


<section id="what-is-discarded" markdown="1">

<h2><span class="num">3</span> What is and isn't discarded</h2>

Dropping moves `.emery/slices/<name>/` — proposal, specs, design, tasks, `model.yaml`, evidence — to `.emery/archive/YYYY-MM-DD-<name>/`. The artifacts survive for audit; they just never reach the baseline.

> [!WARNING]
> Drop does not revert your working tree. If `/emery:build` already ran, the generated code changes sit in your project as ordinary uncommitted (or committed) edits — clean them up with your normal Git tooling if the slice's code should go too.
</section>


<section id="plan-follow-up" markdown="1">

<h2><span class="num">4</span> Follow up on the plan</h2>

Dropping does not rewrite plan state. If the entry was `in-progress`, the execute loop stops with `slice-dropped` when it next looks. Choose one:

- **Retry the entry fresh** — `emery plan undo <entry>` walks it back to `pending`; a later `emery plan advance` re-advances it and refine starts a new slice directory.
- **Remove the entry** — only while the plan is still replaceable (everything `pending`): `emery plan remove <entry>`.
- **Leave it and re-plan** — for larger regroupings, re-run `/emery:plan` (confirms before replacing a pending plan).
</section>


<div class="see-also">
<strong>See also</strong>

- [emery slice drop](../reference/cli/slice.md#emery-slice-drop) — reference entry
- [Undo a plan entry](undo-a-plan-entry.md) — releasing the dropped slice's plan entry
- [Lifecycle](../reference/lifecycle.md) — `dropped` as a terminal state
</div>
