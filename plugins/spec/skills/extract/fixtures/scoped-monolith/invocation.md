# Invocation

```bash
/spec:extract \
  plugins/spec/skills/extract/fixtures/scoped-monolith/source \
  plugins/spec/skills/extract/fixtures/scoped-monolith/out \
  --include 'src/a/**'
```

- `--include 'src/a/**'` narrows the business-logic read set to `src/a/handler.ts`.
- `src/b/handler.ts` and `src/common/util.ts` are **not** analyzed in Step 2 even though they live under `source/`.
- `package.json` and top-level `README.md` are always read (see §*Sentinels always read* in `../../SKILL.md`); language detection and dependency pinning run unchanged.

Expected output shape under `out/`:

```text
out/
├── design.md
└── specs/
    └── a/spec.md
```

Expected artifacts are checked in under `expected/`.
