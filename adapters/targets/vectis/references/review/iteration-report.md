# Iteration report template

Output format for `2g. Produce iteration report`. Use the full template on the first iteration; on subsequent iterations report only new findings and note the iteration number.

````
## Code Review Report: {app-name} (iteration {N})

**Review Team**: 3 specialists + 1 antagonist
**Confidence Level**: [high | medium | low]

### Summary
- critical: N findings
- important: N findings
- suggestion: N findings
- optional: N findings

### Critical findings

#### [CRX-001-1] Missing render() after page transition
- **rule_id**: VECTIS-002
- **File**: shared/src/app.rs, lines 384-388
- **Reviewer**: Structural Specialist
- **Antagonist**: Confirmed
- **Issue**: Navigating from Error to Loading mutates `model.page` without
  emitting `render()`. The shell may not see the Loading state.
- **Fix**: Wrap the return in `render().and(Command::event(Event::Initialize))`

... (one block per finding, ordered by severity then file)

### Important findings
...

### Suggestion findings
...

### Optional findings
...

### Adversarial Review

**Antagonist Activity Summary**:

| Action       | Count   |
| ------------ | ------- |
| Confirmed    | [count] |
| Downgraded   | [count] |
| Upgraded     | [count] |
| Disputed     | [count] |
| New Findings | [count] |

**Acceptance Rate**: [confirmed / total specialist findings]%

#### Downgraded Findings
- [ID] ORIG -> NEW: rationale

#### Upgraded Findings
- [ID] ORIG -> NEW: rationale

#### Disputed Findings
- [ID] Reported as SEVERITY: "description"
  Dispute: rationale
  Lead Decision: [Included | Excluded]

#### New Findings (Missed by Specialists)
- [NEW-1] SEVERITY: description (file:line)
  Evidence: details

### Test Gap Summary
- Missing test for: [scenario description]
- Missing test for: ...
````

Classify each finding as **mechanical** (auto-fixable) or **design-level** (requires architectural decisions). Add design-level findings to the accumulated list.

## Finding-ID conventions

- Occurrence prefixes (`CRX-1`, `LOG-1`, `IOS-1`, `AND-1`, `SWF-1`, `KTL-1`, `INT-1`, `GEN-1`, `UNI-1`, `NEW-1`) are **report-local** counters — the `id` field on a structured `LintFinding` (the `LintFinding` schema uses the equivalent `FIND-0001` shape; this report uses prefixed counters for human triage). They restart in each report and must not be confused with codex `rule-id`s.
- Codex citations carry a separate `rule_id` field (markdown prose) that maps to the kebab-case `rule-id` field on the `LintFinding` wire shape. Vectis codex ids match `^VECTIS-[0-9]{3}$` (e.g. `VECTIS-001`, `VECTIS-101`, `VECTIS-201`); shared ids match `^UNI-[0-9]{3}$`. Leave the field out for genuinely unmapped findings rather than inventing a rule id.
- Severity uses the closed `LintFinding` severity enum: `critical`, `important`, `suggestion`, `optional`. Severity reflects antagonist adjustments — upgrades and downgrades rewrite the displayed severity but preserve the original prefix and occurrence id.
- Confidence uses the closed `LintFinding` severity enum: `high`, `medium`, `low`. Required when `source: model-assisted`.
- Every finding carries a `file:line` reference and a verbatim code snippet.
