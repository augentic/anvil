# Client Plugin

Client-facing deliverables — Statements of Work, proposals, pricing summaries, and similar artefacts — generated from Specify artifacts.

## Skills

### /client:sow-writer

Translate Specify artifacts into a client-facing Statement of Work document.

**Synopsis:**

```text
/client:sow-writer <slice-dir> [--output <path>] [--client <name>] [--company <name>] [--pdf]
```

**Inputs:**
- `slice-dir` -- Path to the Specify slice directory containing artifacts.
- `--output` -- Output file path (defaults to `SOW-<change-name>.md`).
- `--client` -- Client name for the document.
- `--company` -- Company name for the document.
- `--pdf` -- Also generate a PDF version.

**Outputs:**
- `SOW-<name>.md` -- Markdown Statement of Work.
- Optional PDF version.

**Behavior:**
1. Validates that the slice directory contains the required artifacts.
2. Generates the SoW with the following sections:
   - Cover page
   - Introduction and background
   - Services and deliverables (derived from specs and tasks)
   - Fees and payment terms
   - Acceptance criteria (derived from spec scenarios)
   - Appendices (technical design summary)

The SoW is derived from the same artifacts that drive implementation, ensuring alignment between what is promised and what is built.
