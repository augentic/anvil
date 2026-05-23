// Per-fixture skip rules and Deno test helpers used by the
// acceptance harness. Centralised so individual category test
// files (`sources_test.ts`, `targets_test.ts`, …) stay small.

import { resolveSpecifyBin } from "./specify.ts";

export interface SkipDecision {
  skip: boolean;
  reason?: string;
}

let cachedDecision: SkipDecision | undefined;

export async function skipUnlessSpecifyBin(): Promise<SkipDecision> {
  if (cachedDecision) return cachedDecision;
  const bin = await resolveSpecifyBin();
  cachedDecision = bin
    ? { skip: false }
    : {
      skip: true,
      reason:
        "specify binary not resolvable; set SPECIFY_BIN or install `specify` on PATH",
    };
  return cachedDecision;
}

// Wrap a test body so the test always passes (printing a skip note) when
// `SPECIFY_BIN` is missing. Phase 3 replay tests use this wrapper
// for any case that shells out to the CLI, so a fresh clone without a
// built binary still completes the harness with a useful skip message
// rather than failing.
export function withSpecifyBin(
  fn: () => Promise<void>,
): () => Promise<void> {
  return async () => {
    const decision = await skipUnlessSpecifyBin();
    if (decision.skip) {
      console.log(`  skipped: ${decision.reason}`);
      return;
    }
    await fn();
  };
}
