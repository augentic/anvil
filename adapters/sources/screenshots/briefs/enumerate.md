# `screenshots.enumerate`

Walk `$SOURCE_DIR` (a read-only preopen of an operator-bound directory of screen images), identify one candidate per screen via vision inference, and return one candidate block per screen for the CLI to append under `## Candidate inventory` in `discovery.md`. The CLI persists the result; this brief returns the candidate-block payload only.

## Inputs

- `$SOURCE_DIR` — read-only directory holding the bound screen-image set. Never write here.
- `<source-key>` — the plan-level binding key under `plan.yaml.sources.<key>`; the CLI passes it in and this brief embeds it in every `sources:` list.
- `$SCRATCH_DIR` — per-slice write-only scratch space; use only for unavoidable intermediate state (e.g. cropped staging files when chrome cropping is required to disambiguate a screen).

## Vision prerequisite

The brief assumes the agent runtime can inspect attached images. The check is **positive**: at least one of the input image paths MUST be successfully read through the runtime's native attachment / file-read mechanism. The brief MUST NOT consult a host-provided "vision adapter" flag (those are announced inconsistently across runtimes), and it MUST NOT fall back to filename-based or metadata-only inference.

When the check fails, exit `1` with a single-line message naming the supported runtimes:

```text
screenshots.enumerate requires a runtime that can read attached images
(Cursor IDE, Claude Code, cursor-agent CLI, or any host that exposes image
attachments to the agent). Verify the runtime can open the image at <path>
and re-run.
```

## Accepted formats

PNG and JPEG only. HEIC, TIFF, PDF, SVG, WebP, and GIF MUST be converted before invocation; the brief MUST NOT invent a conversion step or call out to a hosted service.

## What is a candidate

One discrete screen the source images describe. Recognition rules, in order:

1. **One image, one screen.** When `$SOURCE_DIR` holds multiple screen images that depict visually distinct screens, treat each as a candidate.
2. **State variants.** When several images depict the same screen in different states (`loading`, `empty`, `populated`, `error`), group them into a single candidate by visual similarity (matching header / chrome / dominant content). The state variants stay attached to the one screen candidate for `extract` to fold into `states.<name>` regions later. Do not emit one candidate per state.
3. **Platform variants.** When several images depict the same screen on different platforms (iOS / Android / web), they also collapse into a single candidate. `extract` recovers per-platform shape under `platforms.<platform>.*` later.

Triage authority: explicit `state <slug>:<name>=<path>` and `group <slug>:<paths>` mappings (passed through as optional source-binding metadata) beat visual similarity; if no operator hints are supplied, group by visual similarity alone.

Skip images that contain no application content (orphan splash screens, full-screen brand marks, internal QA cards). When in doubt, emit the candidate — `propose` and the operator at Gate 1 reconcile false positives.

## Candidate id and summary

- `id`: kebab-case slug derived from the screen's vision-inferred title (visible app-bar title, prominent heading) or, when no title is legible, from the input filename stem with `-` substituted for non-kebab characters. Lowercase, strip punctuation, replace whitespace with `-`. Example: visible header "Task list" → `task-list`; filename `Settings Detail.png` → `settings-detail`. Re-enumerating the same source replaces by `id`, so stability matters more than prettiness.
- `summary`: a one-line description of the screen — typically `<screen-title>: <one-sentence content summary>` lifted from visible cues (e.g. "Task list: today's open tasks for the signed-in user."). Keep it under 200 characters. Do not invent content the screens do not show.

## Output

Return one block per candidate, in alphabetical `id` order. The CLI appends them under the existing `## Candidate inventory` heading in `discovery.md`; this brief never writes the heading itself.

```markdown
### task-list

- id: task-list
- sources: [<source-key>]
- summary: Task list: today's open tasks for the signed-in user.
```

Field order is fixed (`id`, `sources`, `summary`). `sources:` always carries exactly the supplied `<source-key>` for this adapter; cross-source merging is `/spec:plan`'s `propose` sub-step, not this brief's job. Do not set `tentative`.

## Worked example

Bound directory layout (relative to `$SOURCE_DIR`):

```text
task-list-populated.png   # visible header: "Today"; rows of task items
task-list-empty.png       # same header / chrome; empty-state illustration
archive.png               # visible header: "Archive"; archived tasks list
```

Expected output (alphabetically by `id`; `task-list-populated.png` and `task-list-empty.png` collapse into a single candidate by visual similarity):

```markdown
### archive

- id: archive
- sources: [<source-key>]
- summary: Archive: completed tasks the user has archived.

### task-list

- id: task-list
- sources: [<source-key>]
- summary: Task list: today's open tasks for the signed-in user.
```

A full input / output fixture for this example lives at [`tests/fixtures/sources/screenshots/task-list-two-screen/`](../../../../tests/fixtures/sources/screenshots/task-list-two-screen/) in the repo.

## Determinism

- Emit candidates sorted alphabetically by `id`.
- Field order inside each block is fixed: `id`, `sources`, `summary`.
- No timestamps, host paths, or other run-state in the output — re-running against unchanged inputs produces byte-identical blocks.
- Triage of state variants into the same candidate MUST be reproducible. When two images are equally plausible as the dominant variant of a screen, pick the one whose filename sorts first lexicographically.

## Guardrails

- `$SOURCE_DIR` is read-only. Reads outside it surface as `source-enumerate-path-denied`; never attempt to widen the preopen.
- Never crop or extract production assets out of screenshots. Cropping platform chrome (status bars, navigation bars, browser chrome, emulator frames) into `$SCRATCH_DIR` is permitted only as a triage aid; cropped pixels never leave the brief.
- Do not write or rewrite the `## Candidate inventory` heading — the CLI owns the section frame.
- Do not emit Evidence here. Per-screen spatial extraction is `screenshots.extract`'s job, run once per candidate at slice time.
- Do not invent a candidate the screens do not depict. Empty inventories (`$SOURCE_DIR` parseable but no application screens) are valid output.
- Do not fall back to filename-based inference when the vision prerequisite fails — exit `1` per the prerequisite block.
