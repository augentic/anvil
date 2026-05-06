# clean-start — self-heal no-op

The plan has no `in-progress` entries. Self-heal's scan finds nothing to reconcile and falls through to step 4 of the supervised run (`specify plan next`). No plan transitions, no journal entries, no phase invocations — the only side effect is a single diagnostic line on stdout.

```text
Self-heal: no in-progress entries found.
```

The driver proceeds to `specify plan next`, which in this fixture returns `email-verification` (the first eligible `pending` entry after the `done` `user-registration` dependency is satisfied). Self-heal is not involved in that selection.
