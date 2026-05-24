# Iteration report template

Output format for `2g. Produce iteration report`. Use the full template on the first iteration; on subsequent iterations report only new findings and note the iteration number.

````
## Code Review Report: {app-name} (iteration {N})

**Review Team**: 3 specialists + 1 antagonist
**Confidence Level**: [HIGH | MEDIUM | LOW]

### Summary
- Critical: N findings
- Warning: N findings
- Info: N findings

### Critical Findings

#### [CRX-001-1] Missing render() after page transition
- **Rule ID**: VECTIS-002
- **File**: shared/src/app.rs, lines 384-388
- **Reviewer**: Structural Specialist
- **Antagonist**: Confirmed
- **Issue**: Navigating from Error to Loading mutates `model.page` without
  emitting `render()`. The shell may not see the Loading state.
- **Fix**: Wrap the return in `render().and(Command::event(Event::Initialize))`

... (one block per finding, ordered by severity then file)

### Warning Findings
...

### Info Findings
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

- Severity reflects antagonist adjustments — upgrades and downgrades rewrite the displayed severity but preserve the original prefix and occurrence ID.
- Every finding carries a `file:line` reference and a verbatim code snippet.
