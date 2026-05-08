# RM-01 Test Support

This directory contains helper modules used by
[`../cross_repo.ts`](../cross_repo.ts). It is deliberately
small support code, not a pluggable framework.

The helpers provide:

- deterministic Git commands and identity,
- a fake `gh` CLI plus fake SSH transport backed by local bare remotes,
- fixture source repositories,
- hub and registry setup through the real `specify` CLI,
- workspace sync/status wrappers,
- `specify` subprocess helpers with stdout/stderr capture.

The test owns the assertions directly. If RM-01 needs a new invariant, add it to
the Deno test instead of creating a backend or assertion registry.
