# Documentation authoring standards

House rules for the Specify Developer Guide (`docs/`). The guide is built with [mdBook](https://rust-lang.github.io/mdBook/) and deployed from [`.github/workflows/docs.yaml`](../../.github/workflows/docs.yaml).

## Visual system

Custom presentation lives in [`docs/assets/theme/specify-docs.css`](../assets/theme/specify-docs.css). Reuse its component classes instead of inventing ad-hoc styling:

| Class | Use |
|-------|-----|
| `.pipeline` + `.pipeline-caption` | Architecture / workflow SVG diagrams |
| `.callout` | Posture notes, gotchas, “unchanged” reminders |
| `.audience-grid` / `.audience` | “Start here if you are…” orientation blocks |
| `.authority-widget` | Interactive authority resolution demo (adapter-anatomy only) |
| `.pill.agreed` / `.divergence` / `.conflict` | Inline requirement status chips |

Diagram assets live under [`docs/assets/diagrams/`](../assets/diagrams/). Follow [`_STYLE.md`](../assets/diagrams/_STYLE.md) for SVG authoring.

## Diagram policy

- **Explanation and orientation pages** must use committed SVG assets for workflow and architecture diagrams. Do not add new ` ```text ` pipeline diagrams or D2 blocks in those directories.
- **Reference pages** may keep D2 and ASCII command snippets where scannability matters (e.g. quick-reference per-command blocks).
- Prefer one flagship SVG per conceptual page over several small ASCII fragments.

## Raw HTML in markdown

mdBook allows inline HTML. Use it for callouts and audience grids:

```html
<div class="callout">
  <strong>Gate 1.</strong> The operator stamps <code>reviewed</code> explicitly — <code>/spec:plan</code> never writes it.
</div>
```

Keep factual prose in markdown; use HTML only for layout components the CSS targets.

## RFC citations

User-facing docs must not cite RFC numbers in visible prose except in [`docs/explanation/decision-log.md`](../explanation/decision-log.md), [`docs/explanation/release-notes.md`](../explanation/release-notes.md), and [`docs/contributing/`](../contributing/). Link targets to archived RFC paths are fine when the link text does not name the RFC. Enforced by `checkNoRfcCitationsInDocs` in `make checks`.

## Building locally

```bash
make docs        # mdbook build docs
make docs-serve  # live reload at http://localhost:3000
```

Requires `mdbook`, `mdbook-d2`, and D2 on `PATH` — see [`docs/README.md`](../README.md).

Run `make docs` before opening a documentation PR so [`docs/book/`](../book/) stays in sync with CI deploy output.
