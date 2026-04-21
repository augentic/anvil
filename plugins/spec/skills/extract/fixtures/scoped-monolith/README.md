# scoped-monolith fixture

Demonstrates `/spec:extract` with `--include` shrinking the read set without
disturbing language / dependency detection.

`source/` is a tiny TypeScript monolith with two capabilities (`a`, `b`) and
a shared util. Running extract with `--include 'src/a/**'` should:

- Still detect TypeScript as the source language and read `package.json` +
  top-level `README.md` (sentinels).
- Emit specs + design covering capability `a` only.
- Skip `src/b/` and `src/common/` from business-logic extraction.

Compare `expected/` with what extract actually produces against `source/`.
The expected artifacts are illustrative — they pin the scoping behaviour,
not byte-for-byte reproducibility.

## Tree

```text
scoped-monolith/
├── README.md            # this file
├── invocation.md        # shell command the operator runs
├── source/              # monolith under extraction
│   ├── package.json
│   ├── README.md
│   └── src/
│       ├── a/handler.ts
│       ├── b/handler.ts
│       └── common/util.ts
└── expected/            # illustrative extract output for the invocation
    ├── design.md
    └── specs/
        └── a/spec.md
```
