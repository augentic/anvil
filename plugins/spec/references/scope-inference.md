# Scope inference — `/spec:define` specs brief

The specs brief infers which files to extract from each source by reading the plan entry's `description` for file-path hints. This replaces the former `--scope-*` flag-forwarding pipeline; the define skill no longer receives scope flags from the driver.

- **Path hints present** — the description contains path-like references (e.g. `src/common/validation/`, `src/auth/**`). The brief uses these as `include` globs on `/spec:extract`, treating bare directory names as recursive globs.
- **No path hints** — the brief runs extract on the full source tree.

The brief logs the inferred scope in the journal so operators can audit what was extracted and amend the description if the inference was wrong.

The downstream contract for how extract's native filter flags (`include` / `exclude` / `manifest`) work lives in [`../skills/extract/SKILL.md`](../skills/extract/SKILL.md) (§ Scope filters, § Sentinels always read, § Manifest shape) and [`../skills/extract/scope-filters.md`](../skills/extract/scope-filters.md).
