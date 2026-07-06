---
id: UNI-023
title: Ignore Directive Orphan
severity: important
trigger: A `specify-ignore` directive names a rule id that does not match any finding on its target line.
---

## Rule

A `specify-ignore` directive must suppress a real finding. When a directive's rule id does not match any finding emitted for the next non-blank, non-comment line, the directive is an orphan — it does nothing today, but it leaves the reader with the impression that an exception is in force at that location. Orphan directives accumulate over time as the underlying rule, hint, or code path drifts, and they silently fail to suppress whatever finding the operator may have meant to suppress next.

`important` is the defensible tier here because a dead directive is an authored statement of intent that no longer matches reality: unlike ordinary dead code (UNI-013, `suggestion`), it actively misleads the next reader about what is being tolerated at that location, and the fix is mechanical — delete the directive or restore the underlying finding it was meant to suppress.

## Look For

- A directive whose rule id belongs to a rule that has since been retired or renumbered.
- A directive left behind after the offending code was refactored so the finding no longer fires.
- A directive whose target line is empty or comment-only because of an intervening reformat (the directive now applies to a line that emits no finding).
- A copy-pasted directive carried into a file where the targeted finding never fired in the first place.
- A directive whose rule id does not belong to any rule resolved by the current scan (typo, wrong namespace, or a rule from a codex tree not loaded in this project).

## See Also

- [Ignore directives reference](../../../../../docs/reference/ignore-directives.md) — full grammar, comment-style table, and exit semantics.
