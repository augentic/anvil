<div class="hero">
<div class="eyebrow">How-to</div>
<h1 class="hero-title">Recover from a stale guest lock</h1>

Clear a leftover `.emery/guest.lock` marker after a driver session died without releasing it.

<div class="meta-row">

<span class="meta-chip"><strong>Symptom</strong> guest-marker-held</span>

<span class="meta-chip"><strong>Fix</strong> Verify, then remove the marker</span>

</div>

</div>


<div class="when">
<strong>When to use.</strong>

Use this guide when `emery plan refine` or `emery plan execute` exits with `guest-marker-held` (exit 2) but you are certain no other driver session is running — typically after a crashed terminal, a killed process, or a machine restart mid-run.
</div>


<section id="understand" markdown="1">

<h2><span class="num">1</span> Understand the marker</h2>

The guest-routed drivers (`emery plan refine`, `emery plan execute`) each create and hold the `.emery/guest.lock` marker for the run's lifetime, so two driver runs cannot interleave writes. A second driver session refuses with `guest-marker-held`.

Because only the run that created the marker removes it, a driver that dies without cleanup leaves it behind, and every later `emery plan refine` / `emery plan execute` refuses until it is removed.
</section>


<section id="verify" markdown="1">

<h2><span class="num">2</span> Verify the holder is really gone</h2>

Before touching the marker, confirm no driver session is still alive:

1. Check your other terminals and agent sessions for a running `emery plan refine` or `emery plan execute`.
2. Check the process table:

```bash
ps aux | rg 'emery plan (refine|execute)' | rg -v rg
```

If a live session shows up, do not remove the marker — wait for it to finish or stop it cleanly.
</section>


<section id="remove" markdown="1">

<h2><span class="num">3</span> Remove the marker</h2>

> [!WARNING]
> Removing `.emery/guest.lock` while a driver loop is still running lets a second loop start against the same plan, and the two will interleave lifecycle writes. Only remove the marker after step 2 confirms the holder is gone.

```bash
rm .emery/guest.lock
```

This is the one sanctioned hand-edit inside `.emery/` — the marker carries no state beyond its own existence.
</section>


<section id="resume" markdown="1">

<h2><span class="num">4</span> Resume</h2>

```bash
emery plan status
emery plan execute
```

Execute reads on-disk lifecycle state and resumes from the active entry, skipping phases already complete. `plan status` first tells you exactly where the dead run got to.
</section>


<div class="see-also">
<strong>See also</strong>

- [Drop down a layer](drop-down-a-layer.md) — recovery when execute parks on a real failure
- [emery plan execute](../reference/cli/plan.md#emery-plan-execute) — the marker's lifetime and stop conditions
- [Glossary — Guest](../appendices/glossary.md#g) — engine guest vs adapter guest
</div>
