# Driver-supplied arguments — `/spec:define`

When invoked by `/change:execute` from a plan entry, `/spec:define` accepts:

```text
/spec:define <name> \
    [source <key>=<path-or-url>...]
```

- **`source <key>=<path-or-url>`** — a resolved entry from the plan's top-level `sources` map. The key is the kebab-case identifier used in the plan entry's `sources` list; the value is either a local filesystem path or a git URL. `/change:execute` has already validated that the key exists in the plan's top-level `sources` map; the skill treats the value as opaque and forwards it to whichever define brief invokes `/spec:extract` (which inlines a guarded `git clone` snippet for URL values — see the *Cloning a source tree* subsection in [`../skills/analyze/SKILL.md`](../skills/analyze/SKILL.md)). The driver never clones; that stays inside the brief pipeline.

The plan entry's `description` field provides the scoping and delta-targeting context that the specs brief uses to infer extract filters and baseline targets. See [scope-inference.md](scope-inference.md) and [delta-target-inference.md](delta-target-inference.md).

The authoritative contract for how `/change:execute` builds these flag values lives in [`../../change/skills/execute/SKILL.md` § Argument resolution (`sources`)](../../change/skills/execute/SKILL.md). The downstream contract for how extract's native filter flags work lives in [`../skills/extract/SKILL.md`](../skills/extract/SKILL.md) (§ Scope filters, § Sentinels always read, § Manifest shape).
