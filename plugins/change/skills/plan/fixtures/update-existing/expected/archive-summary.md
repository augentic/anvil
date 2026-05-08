# Archive shape after `specify change finalize`

After step 7 succeeds, the hub's on-disk state looks like:

```text
shop-platform/
└── .specify/
    ├── project.yaml                                    # unchanged
    ├── registry.yaml                                   # unchanged
    ├── archive/
    │   └── plans/
    │       ├── polish-pass-<YYYYMMDD>.yaml             # the archived plan
    │       └── polish-pass-<YYYYMMDD>/                 # archived authoring trail
    │           ├── change.md
    │           └── plans/
    │               └── polish-pass/                    # discovery, workspace, proposal markdown
    └── workspace/
        ├── omnia-backend/                              # tier-2 clone (durable)
        └── vectis-mobile/                              # tier-2 clone (durable)
```

A second run of `/change:plan <name> orchestrate polish-pass shape
update-existing` reports `plan-not-found` from `specify change
finalize` and exits zero — the explicit "already finalized" signal.
