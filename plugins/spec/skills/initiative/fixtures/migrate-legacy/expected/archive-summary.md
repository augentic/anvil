# Archive shape after `specify initiative finalize`

After step 7 succeeds, the hub's on-disk state looks like:

```text
shop-platform/
└── .specify/
    ├── project.yaml                                # unchanged
    ├── registry.yaml                               # foo-backend + foo-mobile (from step 3)
    ├── archive/
    │   └── plans/
    │       ├── migrate-foo-<YYYYMMDD>.yaml         # the archived plan
    │       └── migrate-foo-<YYYYMMDD>/             # archived authoring trail
    │           ├── initiative.md
    │           └── plans/
    │               └── migrate-foo/                # discovery, workspace, proposal markdown
    └── workspace/
        ├── foo-backend/                            # tier-2 clone (durable)
        └── foo-mobile/                             # tier-2 clone (durable)
```

`plan.yaml` and `initiative.md` no longer live at their pre-finalize paths
— `specify initiative finalize` moved them atomically. Re-running
`/spec:initiative create migrate-foo …` reports `plan-not-found` (the
explicit "already finalized" signal) and exits zero.

If the operator had passed `--auto-merge --clean`-equivalent (i.e. run
`specify initiative finalize --clean` by hand), the workspace clones
under `.specify/workspace/` would be removed too. This fixture pins the
default `--auto-merge` path, which leaves them on disk for the next
initiative.
