# Documentation authoring standards

House rules for the Specify Developer Guide (`docs/`). The guide is built with [mdBook](https://rust-lang.github.io/mdBook/) and deployed from [`.github/workflows/docs.yaml`](../../.github/workflows/docs.yaml). The visual system lives in [`docs/assets/theme/specify-docs.css`](../assets/theme/specify-docs.css).

## Document types (Diátaxis)

Organise chapters by reader intent. The `docs/SUMMARY.md` section order follows this model:

| Type | Directory | Reader goal | Structure |
| ---- | --------- | ----------- | --------- |
| **Tutorial** | `tutorials/` | Learn by doing — first success guaranteed | Numbered steps; minimal forward references; link out for depth |
| **How-to** | `how-to/` | Solve one specific task | Prerequisites (one line) → steps → See also |
| **Explanation** | `explanation/`, `orientation/` | Understand concepts and design | Prose + SVG diagrams; no step-by-step commands as primary content |
| **Reference** | `reference/`, `appendices/` | Look up precise facts | Synopsis first; tables; link to explanation for rationale |

Getting Started (`orientation/`) is setup and path selection only — vocabulary lives under Understanding Specify (`explanation/concepts.md`).

Contributing and standards docs (`contributing/`, `standards/`) follow reference conventions unless they are explicitly procedural how-tos.

## Visual system

The book ships a forked mdbook theme ([`docs/theme/`](../theme/)) plus the cross-cutting stylesheet at [`docs/assets/theme/specify-docs.css`](../assets/theme/specify-docs.css). Reuse the component classes instead of inventing ad-hoc styling:

| Class | Use |
|-------|-----|
| `.hero` + `.eyebrow` + `.hero-title` + `.meta-row` + `.meta-chip` | Top-of-chapter hero block with breadcrumb + lede + meta chips |
| `section` + `h2 .num` | Numbered top-level sections (RFC-style outline) |
| `.decisions` + `.decision[data-d="D1"..."D8"]` + `.tag` | Decision cards (D1 = behaviour, D2 = documentation, …) |
| `.audience-grid` / `.audience` | "Start here if you are…" orientation blocks |
| `.questions` / `.question` + `.qnum` + `.q` + `.answer` | Open-question cards |
| `.matrix` + `.dot` + `.blocker` + `.scenario-id` + `.blocker-flag` | Acceptance / scenario matrices |
| `details.alt` + `.alt-tag` + `.body` | Alternatives accordion (Rejected / Partial) |
| `.pipeline` + `.pipeline-caption` | Architecture / workflow SVG diagrams |
| `.callout` | Posture notes, gotchas, "unchanged" reminders |
| `.status-pill` | Inline status badge (e.g. Draft) |
| `.pill.agreed` / `.divergence` / `.conflict` | Inline requirement status chips |
| `.authority-widget` | Interactive authority resolution demo (adapter-anatomy only) |
| `.waves` | Wave / timeline wrapper |

## Authoring partials

Reusable component scaffolds live in [`docs/templates/`](../templates/) and are expanded by the [`mdbook-template`](https://github.com/sgoudham/mdbook-template) preprocessor. Authors call them with:

```markdown
\{{#template <relative-path>.md key=value …}}
```

The path is **relative to the chapter file** (so chapters in `docs/` use `templates/<partial>.md` and chapters under `docs/reference/` use `../templates/<partial>.md`). Argument values are everything between `=` and the next `<space>key=` pattern — do **not** quote values, mdbook-template keeps the quote characters literal.

### Hero block

```markdown
\{{#template templates/hero-open.md eyebrow=Specify Developer Guide title=From prompts to durable specs}}
Specify 2.0 turns ad-hoc AI prompting into a repeatable, auditable workflow…

\{{#template templates/meta-row-open.md}}
\{{#template templates/meta-chip.md label=Version value=2.0}}
\{{#template templates/meta-chip.md label=Status value=Released}}
\{{#template templates/meta-row-close.md}}
\{{#template templates/hero-close.md}}
```

The first paragraph inside the hero is styled as the lede automatically; subsequent paragraphs fall back to normal weight.

### Numbered section

```markdown
\{{#template templates/section-open.md id=goals num=A title=Goals}}
…body markdown…
\{{#template templates/section-close.md}}
```

### Decision card

```markdown
\{{#template templates/decisions-open.md}}

\{{#template templates/decision-open.md id=D1 tag=behaviour title=Authority hierarchy}}
Body paragraph(s) describing the decision.
\{{#template templates/decision-close.md}}

\{{#template templates/decisions-close.md}}
```

Available `data-d` values are `D1`–`D8`; they drive the tag colour. Pick a tag word that matches the decision intent (`behaviour`, `documentation`, `intent`, …).

### Pipeline diagram

```markdown
\{{#template templates/pipeline-open.md}}

![Workflow poster](../assets/diagrams/quick-reference/workflow-poster.svg)

\{{#template templates/pipeline-close.md caption=init → plan → Gate 1 → execute → finalize.}}
```

### Audience cards

```markdown
\{{#template templates/audience-grid-open.md}}

\{{#template templates/audience-open.md who=New to Specify}}
Read [What is Specify?](orientation/index.md), install the [Prerequisites](orientation/prerequisites.md).
\{{#template templates/audience-close.md}}

\{{#template templates/audience-grid-close.md}}
```

### Question / FAQ card

```markdown
\{{#template templates/questions-open.md}}
\{{#template templates/question-open.md qnum=Q1 q=How do candidates fuse across sources?}}
Behaviour wins on the closed authority enum.
\{{#template templates/question-close.md}}
\{{#template templates/questions-close.md}}
```

### Alternatives accordion

```markdown
\{{#template templates/alt-open.md status=rejected title=Status property on Evidence files tag=Rejected}}
Why we ruled this out.
\{{#template templates/alt-close.md}}
```

`status` accepts `rejected` (default) or `partial`; the tag colour follows.

### Callout

```markdown
\{{#template templates/callout-open.md}}
**Gate 1.** The operator stamps `reviewed` explicitly — `/spec:plan` never writes it.
\{{#template templates/callout-close.md}}
```

### Status pill (inline)

```markdown
This document is \{{#template templates/status-pill.md label=Draft}}.
```

## Palette and theme picker

Light / dark mode is driven by mdbook's theme picker (the paintbrush icon in the menu bar). The relevant CSS variable buckets live at the top of [`specify-docs.css`](../assets/theme/specify-docs.css):

- `html.light`, `html.rust` → light palette
- `html.coal`, `html.navy`, `html.ayu` → dark palette

OS-level `prefers-color-scheme` is **no longer consulted directly** — `book.toml` sets `default-theme = "light"` and `preferred-dark-theme = "navy"`, then mdbook handles user toggling and persistence. If you add a new colour, declare it in every bucket so all five themes stay readable.

## Right-rail per-page TOC

[`mdbook-pagetoc`](https://github.com/slowsage/mdbook-pagetoc) injects a scrollspy table of contents on every chapter at viewports ≥ 1440 px. Authors don't need to opt in — every `## H2` and `### H3` heading appears automatically. The widget hides on narrow viewports.

## Diagram policy

- **Explanation, orientation, tutorial, and how-to pages** must use committed SVG assets for workflow and architecture diagrams. Do not add new ` ```text ` pipeline diagrams or `d2` blocks in those directories.
- **Reference pages** may keep D2 and ASCII command snippets where scannability matters (e.g. quick-reference per-command blocks).
- Prefer one flagship SVG per conceptual page over several small ASCII fragments.
- The [`mdbook-d2`](https://github.com/danieleades/mdbook-d2) preprocessor renders fenced ` ```d2 ` blocks inline; the [`d2`](https://d2lang.com/) binary must be on `PATH` for the build to succeed.

Diagram assets live under [`docs/assets/diagrams/`](../assets/diagrams/). Follow `_STYLE.md` in that folder for SVG authoring.

## Raw HTML in markdown

mdBook allows inline HTML. Use it for callouts and audience grids when the partial syntax doesn't fit (e.g. tight inline composition):

```html
<div class="callout">
  <strong>Gate 1.</strong> The operator stamps <code>reviewed</code> explicitly — <code>/spec:plan</code> never writes it.
</div>
```

Keep factual prose in markdown; use HTML only for layout components the CSS targets.

## Link gate

`mdbook-linkcheck` validates every internal link on every `mdbook build`. The relevant settings live in [`docs/book.toml`](../book.toml) under `[output.linkcheck]`:

- `follow-web-links = false` — local links only.
- `warning-policy = "error"` — broken refs fail the build.
- `traverse-parent-directories = true` — refs outside `docs/` (e.g. `../../adapters/…`) are allowed.
- `exclude = [ … ]` — regexes for intentional out-of-tree refs and retired stubs.

`make docs` runs the full pipeline, including linkcheck. If a build fails on a new ref, the cleanest fix is to retarget the link to the canonical 2.0 page; only add to `exclude` if the path is intentionally external.

## RFC citations

User-facing docs must not cite RFC numbers in visible prose except in [`../explanation/decision-log.md`](../explanation/decision-log.md), [`../explanation/release-notes.md`](../explanation/release-notes.md), and [`../contributing/`](../contributing/). Link targets to archived RFC paths are fine when the link text does not name the RFC. Enforced by `checkNoRfcCitationsInDocs` in `make check`.

## Building locally

```bash
make docs        # mdbook build docs (HTML + linkcheck)
make docs-serve  # live reload at http://localhost:3000
```

Requires the pinned versions of `mdbook`, `mdbook-d2`, `mdbook-linkcheck`, `mdbook-pagetoc`, `mdbook-template`, and `D2` — see [`docs/README.md`](../README.md) for install commands and the version table.

Run `make docs` before opening a documentation PR so `docs/book/` stays in sync with CI deploy output.
