<div class="hero">
<div class="eyebrow">How-to</div>
<h1 class="hero-title">Undo a plan entry</h1>

Walk a plan entry's status backwards — one rung per call, or several with `--to` — with `emery plan undo`.

<div class="meta-row">

<span class="meta-chip"><strong>Verb</strong> emery plan undo</span>

<span class="meta-chip"><strong>Scope</strong> Per-entry status only</span>

</div>

</div>


<div class="when">
<strong>When to use.</strong>

Use this guide when a plan entry's status is ahead of reality — an entry was advanced (`in-progress`) but you want to release it, or an entry reached `done` and you need to reopen it for another pass.
</div>


<section id="semantics" markdown="1">

<h2><span class="num">1</span> Rung-by-rung semantics</h2>

Per-entry status moves forward through exactly one writer per rung (`plan add`/`amend` write `pending`, `plan advance` writes `in-progress`, `slice merge` writes `done`). `emery plan undo` is the only backward walk. By default it moves **one rung per call**; `--to <status>` walks rung by rung until the entry reaches the named status:

```bash
emery plan undo <entry>                  # one rung: done → in-progress
emery plan undo <entry>                  # one more: in-progress → pending
emery plan undo <entry> --to pending     # or both rungs in one call
```

Either way the walk never skips a rung — it fires one `plan.transition.undone` journal event per rung. An entry already at `pending` (or already at the `--to` status) has no backward rung and is refused.
</section>


<section id="what-it-does-not-touch" markdown="1">

<h2><span class="num">2</span> What undo does not touch</h2>

> [!WARNING]
> `plan undo` rewrites **plan state only**. It does not un-merge baseline specs, restore an archived slice directory, or revert generated code. If the slice already merged, undoing its entry to `in-progress` leaves the baseline as the merge left it — reconcile the artifacts yourself before re-running phases.

Also out of scope:

- **Slice lifecycle.** `metadata.yaml` keeps whatever state the slice reached; the slice orchestrations own those transitions.
</section>


<section id="typical-recoveries" markdown="1">

<h2><span class="num">3</span> Typical recoveries</h2>

| Situation | Moves |
| --------- | ----- |
| Advanced the wrong entry with `plan advance` | `emery plan undo <entry>` (releases it to `pending`), then `emery plan advance` picks the right one |
| A merged slice needs another pass | `emery plan undo <entry> --to pending` (`done → in-progress → pending`), then re-drive the slice from refine |
| Dropped a slice mid-entry and want a fresh attempt | `emery plan undo <entry>` (`in-progress → pending`), then `emery plan advance` when ready |

At most one entry may be `in-progress` at a time, so release the current entry before advancing another.
</section>


<div class="see-also">
<strong>See also</strong>

- [emery plan undo](../reference/cli/plan.md#emery-plan-undo) — reference entry with the JSON shape
- [Lifecycle](../reference/lifecycle.md) — the stacked state machines
- [Drop a slice](drop-a-slice.md) — abandoning slice work rather than rewinding plan state
</div>
