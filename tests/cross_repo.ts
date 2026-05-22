// Cross-repo acceptance harness entry point.
//
// Replays the fixture trees under `tests/fixtures/` end-to-end against the
// deterministic CLI surface and structurally validates synthesised goldens.
// Run via `make test` (which invokes `deno test tests/cross_repo.ts`).
//
// The harness deliberately does NOT pin LLM-emitted prose. Skill bodies are
// agent-driven markdown (`plugins/spec/skills/<name>/SKILL.md`,
// `sources/<name>/briefs/{enumerate,extract}.md`,
// `targets/<name>/briefs/{shape,build,merge}.md`); byte-exact synthesis
// replay belongs in a separate (deferred) RFC. What this harness does cover:
//
//   - source fixtures   — schema-validate emitted Evidence + structural
//                         shape of `expected/discovery.md`.
//   - target fixtures   — provenance-parse `expected/spec.md`,
//                         schema-validate `expected/composition.yaml` (when
//                         a `specify` binary is available), and confirm the
//                         shape-evidence checklist appears in the synthesised
//                         artifacts.
//   - skill/refine      — every requirement block in `expected/spec.md`
//                         parses with a recognised `Status:` enum value,
//                         every Evidence input schema-validates.
//   - skill/execute|build|merge|finalize
//                       — golden-diff transcripts, stop-hints, and
//                         expected-trace files.
//
// REGENERATE_GOLDENS=1 overwrites goldens in place where the harness uses
// the `golden.ts` helpers.

import "./cross_repo/sources_test.ts";
import "./cross_repo/targets_test.ts";
import "./cross_repo/skills_refine_test.ts";
import "./cross_repo/skills_loop_test.ts";
