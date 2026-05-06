# Archive shape after the second run's `specify initiative finalize`

After the second run of `/spec:plan --orchestrate dark-mode …` (step 7
only — re-entry detects all PRs already merged on remote), the hub's
on-disk state looks like:

```text
shop-platform/
└── .specify/
    ├── project.yaml                                # unchanged
    ├── registry.yaml                               # unchanged (no mutation in `new-feature`)
    ├── archive/
    │   └── plans/
    │       ├── dark-mode-<YYYYMMDD>.yaml           # the archived plan
    │       └── dark-mode-<YYYYMMDD>/               # archived authoring trail
    │           ├── initiative.md
    │           └── plans/
    │               └── dark-mode/                  # discovery, workspace, proposal markdown
    └── workspace/
        ├── omnia-backend/                          # tier-2 clone (durable)
        └── vectis-mobile/                          # tier-2 clone (durable)
```

A third run of `/spec:plan --orchestrate dark-mode …` reports
`plan-not-found` from `specify initiative finalize` and exits zero —
the explicit "already finalized" signal.
