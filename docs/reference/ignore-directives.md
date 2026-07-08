# Ignore directives

Operators tolerate a single legitimate exception to an engineering-standards finding by writing an in-source `specify-ignore` directive next to the offending line. The directive carries the rule id and a non-empty rationale; the scanner picks it up unconditionally and demotes the matching finding's status to `ignored` (or `false-positive` for prefixed rationale) on the next lint run. The consumer-project scanner (`specify lint project`) retired from the operational surface; the directive grammar below is the durable contract any lint surface — today `specify lint framework`, and the consumer scanner if it earns its way back as developer tooling — honours.

This page is the single durable reference for the directive grammar, scope rules, status taxonomy, and exit-code semantics. The grammar is identical across every supported language family — the only thing that changes is the comment delimiter the directive lives inside.

## Grammar

```text
specify-ignore: <RULE-ID> <SEPARATOR> <rationale>
```

- `<RULE-ID>` is a stable codex rule id (for example `UNI-014`, `OMNIA-021`). Only one rule per directive.
- `<SEPARATOR>` is either an em-dash (`—`) or the two-character ASCII sequence `--`. Both are accepted everywhere.
- `<rationale>` is non-empty, free-form prose explaining why the finding is tolerated at this location. Rationales shorter than 16 characters are accepted by the parser but reported under `UNI-022`.

The directive is recognised only inside a comment delimiter. The `specify-ignore:` token inside an unrelated string literal is ignored.

## Comment styles

The indexer recognises a closed list of comment delimiters. Files outside this list are skipped — the directive is not a heuristic.

| Language family                                  | Directive syntax                                  |
| ------------------------------------------------ | ------------------------------------------------- |
| C-family (Rust, JS, TS, Go, Swift, Java, C, C++) | `// specify-ignore: …`, `/* specify-ignore: … */` |
| Shell, Python, YAML, TOML                        | `# specify-ignore: …`                             |
| HTML, Markdown, XML                              | `<!-- specify-ignore: … -->`                      |
| SQL, Lua                                         | `-- specify-ignore: …`                            |

## Scope rules

A directive applies to a single line. There are two forms:

- **Standalone** — the directive sits on its own line (or chain of comment-only lines) above the protected code. It applies to the **next non-blank, non-comment line**. Two adjacent directives compose: each applies to the same eventual target line, so an operator can suppress two different rules on one statement.
- **Inline trailing** — the directive sits at end-of-line on the same line as code (`let x = foo(); // specify-ignore: UNI-014 — …`). It applies to that same line.

File-scoped, block-scoped, and directory-scoped variants are deliberately not supported. Mass adoption of an exception is the baseline file's job, which is a separate (deferred) layer.

## Examples

### Standalone, Rust

```rust
// specify-ignore: UNI-014 — vendor-required tunable; lives next to the call site for review
let timeout = Duration::from_secs(120);
```

### Inline trailing, TypeScript

```typescript
const apiKey = process.env.LEGACY_API_KEY!; // specify-ignore: UNI-018 — legacy on-prem credential; rotation tracked in OPS-44
```

### `false-positive:` prefix promotes status

```python
# specify-ignore: UNI-008 — false-positive: telemetry hint mis-fires on the synchronous warm path
log.info("started")
```

A rationale prefixed `false-positive:` (lowercase, with the colon) demotes the finding to `status: false-positive` instead of `status: ignored`. Reviewers and dashboards can separate "we acknowledge the rule and chose to deviate" from "the rule mis-fired here."

### Composing directives on one line

```rust
// specify-ignore: UNI-005 — pool drains on shutdown; verified by integration test pool-leak-1
// specify-ignore: UNI-014 — vendor pool size is the documented production setting
let pool = ConnectionPool::with_size(64);
```

Both directives apply to the next non-blank, non-comment line. Each suppresses its own rule independently.

## Status taxonomy

Every emitted finding carries a `status` field on the wire. The closed enum is:

| Value            | Set by                | Meaning                                                                                                       |
| ---------------- | --------------------- | ------------------------------------------------------------------------------------------------------------- |
| `open`           | scanner (default)     | Freshly emitted finding. The only value that blocks CI by default.                                            |
| `ignored`        | directive pass        | A `specify-ignore` directive matched the finding's `(path, line, rule-id)`. Carries `disposition.directive`.  |
| `false-positive` | directive pass        | A directive matched and its rationale was prefixed `false-positive:`.                                         |
| `fixed`          | reserved              | Reserved for the cross-run baseline diff verb (deferred).                                                     |
| `accepted`       | reserved              | Reserved for explicit operator acceptance via the baseline file (deferred).                                   |

The accompanying optional `disposition` object names *who* set the status. `disposition.source` is a closed enum; `directive` is the only producer today, with the matched directive's path, line, and rationale captured under `disposition.directive`. The disposition is excluded from the finding's fingerprint, so flipping a finding from `open` to `ignored` does not change its identity.

## Exit-code semantics

Lint uses **status-aware severity** when deciding the process exit:

> Exit `2` only when there is a finding with `status: open` AND `severity ∈ {critical, important}`.

`status: ignored` and `status: false-positive` appear in every formatter and in the JSON envelope, but they do not contribute to the blocking decision. `UNI-022` (missing rationale) and `UNI-023` (orphan directive) ship at `important`, so a malformed or dead directive blocks CI by default until the operator either rationalises or removes it.

## What can go wrong

- **Missing or too-short rationale** — emits [`UNI-022`](https://github.com/augentic/specify-adapters/blob/main/codex/rules/universal/ignore-directive-missing-rationale.md). The 16-character floor is the threshold at which a rationale is long enough to be useful to a future reviewer.
- **Orphan directive** — a directive whose rule id matches no finding on its target line emits [`UNI-023`](https://github.com/augentic/specify-adapters/blob/main/codex/rules/universal/ignore-directive-orphan.md). Common causes: the rule was retired, the protected code was refactored, an intervening reformat moved the target, or the directive was copy-pasted into a context where the targeted finding never fired.
- **Shared-codex tree absent** — when the codex resolver does not produce `UNI-022` or `UNI-023` (for example, a consumer project that has not yet picked up the shared codex tree), the matching/demotion step still runs but the synthetic findings for malformed and orphan directives are silently skipped. Run `specify adapters sync` (or `specify init`) to distribute the shared codex into the out-of-tree per-project cache at `<project-cache>/codex/`, or pass `--rules-root`, to restore them.

## Related reading

- [Engineering standards layer](../explanation/standards-layer.md) — how standards enforcement fits next to workflow and artifacts
- [Shared `UNI-*` codex inventory](https://github.com/augentic/specify-adapters/blob/main/codex/rules/universal/README.md) — every shared rule, including `UNI-022` and `UNI-023`
- [Consistency checks](../contributing/checks.md) — the `specify lint framework` authoring surface
