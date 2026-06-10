# `typescript` source adapter fixture

Tiny Express service mirroring a small legacy monolith. It documents the expected `typescript` source-adapter shape for acceptance scenario `4` (`code, multi-slice` decomposes the source tree into leads).

## Layout

```
source/                          # the read-only tree the adapter walks ($SOURCE_DIR)
├── package.json                 # express dependency declaration
├── tsconfig.json
└── src/
    ├── server.ts                # app.post("/users", registerUser)
    ├── users/
    │   ├── register.ts          # handler + email validation
    │   └── repository.ts        # User interface + insertUser
expected/
├── discovery.md                 # the lead block survey appends to discovery.md
└── evidence/legacy-monolith.yaml # Evidence for the user-registration lead
```

The source tree has one route registration. The survey brief's size check (Decision 2) collapses it into a single source-level lead named `user-registration`. The extract brief returns one `excerpt`, one `type`, and one `call` claim against the same tree.

Source key used in the expected outputs: `legacy-monolith`. The slice that binds this source is also named `user-registration` (so the bare-string `sources: [legacy-monolith]` shorthand is legal at the plan level).
