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

Chapters that use a hero block should **omit** the duplicate `# Title` H1 — the hero owns the page title. See the exemplar pages listed under [Page-type scaffolds](#page-type-scaffolds).

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
| Native admonitions (`> [!NOTE]`, `> [!IMPORTANT]`, …) | Posture notes, Gate reminders, gotchas, completion (see [Admonitions](#admonitions)) |
| `.tutorial-step` + `.step-label` + `.step-title` | Numbered tutorial step panels |
| `.prereq` / `.when` | Prerequisites block (tutorials) / when-to-use opener (how-tos) |
| `.rhythm` + `.rhythm-step` + `.rhythm-num` + `.rhythm-label` | Change-rhythm summary cards (explanation pages) |
| `.card-grid` + `.card` + `.card-time` | Section landing page link cards |
| `.synopsis` | Reference page scannable lead |
| `.see-also` | Consistent footer link block |
| `.platform` + `.platform-product[data-active=true]` | Augentic product stack strip (Specify / Omnia / Vectis) |
| `.status-pill` | Inline status badge (e.g. Draft) |
| `.pill.agreed` / `.divergence` / `.conflict` | Inline requirement status chips |
| `.authority-widget` | Interactive authority resolution demo (adapter-anatomy only) |
| `.waves` | Wave / timeline wrapper |

## HTML component blocks

Copy HTML scaffolds from [`docs/authoring-snippets/`](../authoring-snippets/) or from the exemplar chapters listed under [Page-type scaffolds](#page-type-scaffolds). Paste the markup directly into chapter markdown — mdBook passes it through to the HTML renderer. Styles live in [`specify-docs.css`](../assets/theme/specify-docs.css); do not invent one-off CSS.

### Hero block

Omit the duplicate `# Title` H1 when using a hero — the hero owns the page title. See [`index.md`](../index.md).

```html
<div class="hero">
<div class="eyebrow">Specify Developer Guide</div>
<h1 class="hero-title">From prompts to durable specs</h1>

First paragraph is styled as the lede automatically.

<div class="meta-row">
<span class="meta-chip"><strong>Version</strong> 2.0</span>
<span class="meta-chip"><strong>Status</strong> Released</span>
</div>
</div>
```

### Numbered section

```html
<section id="goals" markdown="1">

<h2><span class="num">A</span> Goals</h2>

Body markdown…
</section>
```

The `markdown="1"` attribute lets mdBook render markdown inside the section.

### Decision card

```html
<div class="decisions">

<div class="decision" data-d="D1">
  <span class="tag">behaviour</span>
  <h4>Authority hierarchy</h4>
Body paragraph(s) describing the decision.
</div>

</div>
```

Available `data-d` values are `D1`–`D8`; they drive the tag colour.

### Pipeline diagram

```html
<div class="pipeline">

![Workflow poster](../assets/diagrams/quick-reference/workflow-poster.svg)

<p class="pipeline-caption">init → plan → Gate 1 → execute → finalize.</p>
</div>
```

### Audience cards

```html
<div class="audience-grid">

<div class="audience">
<h4>New to Specify</h4>
Read [What is Specify?](orientation/index.md)…
</div>

</div>
```

### Tutorial step

```html
<div class="tutorial-step">
<span class="step-label">Step 01</span>
<h3 class="step-title">Initialise the project</h3>

Run once per project: `/spec:init omnia`
</div>
```

### Card grid (section landing)

```html
<div class="card-grid">
<a class="card" href="quick-start.md">
<div class="card-head">
<h3 class="card-title">Quick start</h3>
<span class="card-time">~30 min</span>
</div>
<div class="card-body">
Run a one-slice Omnia change from intent through finalize.
</div>
</a>
</div>
```

### Synopsis and see-also

```html
<div class="synopsis">
Agent-driven orchestrator. Deterministic work delegates to `specrun plan *`.
</div>

<div class="see-also">
<strong>See also</strong>

- [Core concepts](../explanation/concepts.md)
</div>
```

Optional `variant=` values: `gate`, `gotcha`, `success`, `unchanged`. Omit `variant` for the default accent bar.

### Tutorial step

```markdown
\{{#template templates/tutorial-step-open.md num=01 title=Initialise the project}}
Run once per project: `/spec:init omnia`
\{{#template templates/tutorial-step-close.md}}
```

### Prerequisites

```markdown
\{{#template templates/prereq-open.md}}
Complete [Prerequisites](../orientation/prerequisites.md): Cursor, `specify` CLI, …
\{{#template templates/prereq-close.md}}
```

### When to use (how-to)

```markdown
\{{#template templates/when-open.md}}
Use this guide when `/spec:execute` parks on build or merge failure.
\{{#template templates/when-close.md}}
```

### Workflow rhythm cards

```markdown
\{{#template templates/rhythm-open.md}}
\{{#template templates/rhythm-step-open.md num=01 label=Plan title=Define the change}}
`/spec:plan` writes `plan.yaml` and exits at `pending`.
\{{#template templates/rhythm-step-close.md}}
\{{#template templates/rhythm-close.md}}
```

### Card grid (section landing)

```markdown
\{{#template templates/card-grid-open.md}}
\{{#template templates/card-open.md title=Quick start time=~30 min href=quick-start.md}}
Run a one-slice Omnia change from intent through finalize.
\{{#template templates/card-close.md}}
\{{#template templates/card-grid-close.md}}
```

### Synopsis (reference)

```markdown
\{{#template templates/synopsis-open.md}}
Agent-driven orchestrator. Deterministic work delegates to `specrun plan *`.
\{{#template templates/synopsis-close.md}}
```

### See also

```markdown
\{{#template templates/see-also-open.md}}
- [Core concepts](../explanation/concepts.md)
- [Quick reference](../reference/quick-reference.md)
\{{#template templates/see-also-close.md}}
```

### Platform stack

```markdown
\{{#template templates/platform-open.md}}
\{{#template templates/platform-product-open.md name=Specify role=Workflow engine active=true}}
Enforces the spec-first rhythm documented in this guide.
\{{#template templates/platform-product-close.md}}
\{{#template templates/platform-close.md}}
```

Set `active=true` on the current product; omit on sibling products.

### Decision consequence (optional)

```markdown
\{{#template templates/decision-consequence.md text=Losers survive as inline commentary.}}
```

Place before `decision-close.md` when a decision card needs a consequence line.

### Status pill (inline)

```html
This document is <span class="status-pill">Draft</span>.
```

## Admonitions

Use mdBook 0.5 [native admonitions](https://rust-lang.github.io/mdBook/format/markdown.html#admonitions) for posture notes instead of custom callout HTML:

| Kind | Syntax | Use |
| ---- | ------ | --- |
| Gate / operator stamp | `> [!IMPORTANT]` | Gate 1 reminders, required human steps |
| Gotcha | `> [!WARNING]` | Pitfalls and failure modes |
| Success / completion | `> [!TIP]` | Tutorial outcomes, “you’re done” notes |
| Unchanged behaviour | `> [!NOTE]` | Behaviour that did not change |

```markdown
> [!IMPORTANT]
> **Gate 1.** The operator stamps `approved` explicitly — `/spec:plan` never writes it.
```

Multi-line bodies prefix every line with `> `.

## Page-type scaffolds

Copy the exemplar chapter for each Diátaxis type when authoring or migrating pages:

| Type | Exemplar | Key components |
| ---- | -------- | -------------- |
| **Tutorial** | [`tutorials/quick-start.md`](../tutorials/quick-start.md) | hero, meta-chip, prereq, tutorial-step, pipeline, `> [!TIP]`, see-also |
| **How-to** | [`how-to/drive-slice-manually.md`](../how-to/drive-slice-manually.md) | hero, when, numbered section, `> [!IMPORTANT]`, see-also |
| **Explanation** | [`explanation/concepts.md`](../explanation/concepts.md) | hero, audience-grid, rhythm, pipeline, admonition, see-also |
| **Reference** | [`reference/change-skills/plan.md`](../reference/change-skills/plan.md) | hero, synopsis, tables below fold, see-also |
| **Section landing** | [`tutorials/index.md`](../tutorials/index.md) | hero, card-grid, see-also |

## Palette and theme picker

Light / dark mode is driven by mdbook's theme picker (the paintbrush icon in the menu bar). The relevant CSS variable buckets live at the top of [`specify-docs.css`](../assets/theme/specify-docs.css):

- `html.light`, `html.rust` → light palette
- `html.coal`, `html.navy`, `html.ayu` → dark palette

OS-level `prefers-color-scheme` is available via the **Auto** entry in mdBook's theme picker. `book.toml` sets `default-theme = "light"` and `preferred-dark-theme = "navy"`. If you add a new colour, declare it in every bucket so all five themes stay readable.

## Sidebar heading navigation

mdBook 0.5 adds an **On this page** block to the left sidebar while you read a chapter. Every `##` and `###` heading appears automatically — no author opt-in. Style overrides live in [`specify-docs.css`](../assets/theme/specify-docs.css) and [`theme/css/chrome.css`](../theme/css/chrome.css).

## Diagram policy

- **Explanation, orientation, tutorial, and how-to pages** must use committed SVG assets for workflow and architecture diagrams. Do not add new ` ```text ` pipeline diagrams in those directories.
- **Reference and contributing pages** may use SVG or ASCII command snippets where scannability matters.
- Prefer one flagship SVG per conceptual page over several small ASCII fragments.

Diagram assets live under [`docs/assets/diagrams/`](../assets/diagrams/). Follow `_STYLE.md` in that folder for SVG authoring.

## Raw HTML in markdown

mdBook allows inline HTML for layout components the CSS targets (hero, cards, sections with `markdown="1"`, etc.). Keep factual prose in markdown where possible; use admonitions for callouts.

## Link gate

[`mdbook-linkcheck2`](https://crates.io/crates/mdbook-linkcheck2) validates every internal link on every `mdbook build`. Settings live in [`docs/book.toml`](../book.toml) under `[output.linkcheck2]`:

- `follow-web-links = false` — local links only.
- `warning-policy = "error"` — broken refs fail the build.
- `traverse-parent-directories = true` — refs outside `docs/` (e.g. `../../adapters/…`) are allowed.
- `exclude = [ … ]` — regexes for intentional out-of-tree refs and retired stubs.

`mdbook build docs` runs the full pipeline (HTML + linkcheck2). If a build fails on a new ref, the cleanest fix is to retarget the link to the canonical 2.0 page; only add to `exclude` if the path is intentionally external.

## RFC citations

User-facing docs must not cite RFC numbers in visible prose except in [`../explanation/decision-log.md`](../explanation/decision-log.md), [`../explanation/release-notes.md`](../explanation/release-notes.md), and [`../contributing/`](../contributing/). Link targets to archived RFC paths are fine when the link text does not name the RFC. Enforced by `checkNoRfcCitationsInDocs` in `make check`.

## Building locally

```bash
mdbook build docs   # HTML + linkcheck2
mdbook serve docs   # live reload at http://localhost:3000
```

Requires mdBook **0.5.1+** (linkcheck2 compatibility) and `mdbook-linkcheck2` — see [`docs/README.md`](../README.md).

Run `mdbook build docs` before opening a documentation PR so `docs/book/` stays in sync with CI deploy output.
