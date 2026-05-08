// Fixture backend: deterministic materialisation of expected artifacts.
//
// Purpose (RM-01 plan, C05): prove the runner + assertions composition
// works without an operator and without a live agent. The backend
// copies a known set of files from
// `acceptance/fixtures/<scenario-id>/expected/` into the run's temp
// workspace, hands the file list to the runner-owned `assertions`
// stage, and reports `passed` if every required file landed on disk.
//
// This is NOT a deterministic *workflow* stub (that is C08). It is a
// deterministic *materialisation* fixture for one scenario, used by
// `make acceptance-smoke` to give CI a real pass/fail without paying
// model cost.
//
// CLI authority: the backend never touches `.specify/` lifecycle state.
// It only writes scenario-declared `expected-artifacts:` paths into the
// workspace.

import { copy, ensureDir, exists } from "jsr:@std/fs@1";
import { dirname, fromFileUrl, join, resolve } from "jsr:@std/path@1";

import { appendLog } from "../evidence.ts";
import type {
  AssertionRecord,
  Backend,
  BackendResult,
  RunContext,
} from "../types.ts";

const REPO_ROOT = resolve(
  dirname(fromFileUrl(import.meta.url)),
  "..",
  "..",
  "..",
);

export class FixtureBackend implements Backend {
  readonly name = "fixture" as const;

  async prepare(_ctx: RunContext): Promise<void> {
    // Nothing to seed before invoke. The `invoke` step does the
    // copying so a single failure mode (missing fixture directory)
    // surfaces in one place with a clear notes string.
  }

  async invoke(ctx: RunContext): Promise<BackendResult> {
    const { scenario, paths } = ctx;
    const fixtureRoot = fixtureDirFor(scenario.frontmatter.id);

    const lines: string[] = [];
    lines.push(`=== Fixture Backend: ${scenario.frontmatter.id} ===`);
    lines.push(`Fixture root: ${fixtureRoot}`);
    lines.push(`Workspace:    ${paths.workspace}`);

    if (!(await exists(fixtureRoot))) {
      const msg =
        `fixture directory not found: ${fixtureRoot}. ` +
        `Add expected-artifact files under that path so the fixture backend ` +
        `can materialise them into the workspace. The directory tree must ` +
        `mirror the scenario's expected-artifacts list.`;
      lines.push("");
      lines.push(`error: ${msg}`);
      await appendLog(paths.stdoutLog, lines.join("\n") + "\n");
      return {
        verdict: "failed",
        faultDomain: "runner-setup",
        notes: msg,
        assertions: [],
        evidence: { materialisedPaths: [] },
      };
    }

    const declared = scenario.frontmatter["expected-artifacts"] ?? [];
    if (declared.length === 0) {
      const msg =
        `scenario '${scenario.frontmatter.id}' declared backend=fixture but ` +
        `supplied no 'expected-artifacts:' list. The fixture backend has ` +
        `nothing to materialise.`;
      lines.push("");
      lines.push(`error: ${msg}`);
      await appendLog(paths.stdoutLog, lines.join("\n") + "\n");
      return {
        verdict: "failed",
        faultDomain: "runner-setup",
        notes: msg,
        assertions: [],
        evidence: { materialisedPaths: [] },
      };
    }

    const materialised: string[] = [];
    const missingFromFixture: string[] = [];

    for (const rel of declared) {
      const src = join(fixtureRoot, rel);
      const dst = join(paths.workspace, rel);
      if (!(await exists(src))) {
        missingFromFixture.push(rel);
        continue;
      }
      await ensureDir(dirname(dst));
      await copy(src, dst, { overwrite: true });
      materialised.push(rel);
    }

    lines.push("");
    lines.push(`Materialised ${materialised.length}/${declared.length} expected-artifacts.`);
    if (missingFromFixture.length > 0) {
      lines.push(
        `Skipped ${missingFromFixture.length} file(s) absent from the fixture root:`,
      );
      for (const m of missingFromFixture) lines.push(`  - ${m}`);
    }
    await appendLog(paths.stdoutLog, lines.join("\n") + "\n");

    // The fixture backend never produces real backend assertions —
    // verdict is decided by the runner's assertion stage acting on the
    // workspace it just populated. A `pending-operator` placeholder
    // would be misleading here; we return an empty assertion list and
    // `passed` so the dispatcher can downgrade if helpers fail. Setup
    // failures above already short-circuited with `failed`.
    const assertions: AssertionRecord[] = [];

    return {
      verdict: "passed",
      faultDomain: null,
      notes:
        `Fixture backend materialised ${materialised.length} expected-artifact(s) ` +
        `from ${fixtureRoot} into the workspace. The runner's assertion stage ` +
        `decides the final verdict.`,
      assertions,
      evidence: {
        materialisedPaths: materialised,
        extras: {
          fixtureRoot,
          missingFromFixture,
        },
      },
    };
  }

  async teardown(_ctx: RunContext): Promise<void> {
    // No external resources. Workspace cleanup is the runner's job.
  }
}

/**
 * Resolve the fixture root for a scenario id. Fixtures live under
 * `acceptance/fixtures/<scenario-id>/expected/` so the on-disk shape
 * mirrors the relative paths from the scenario's
 * `expected-artifacts:` list. New scenarios opt in by adding a
 * directory here; the backend complains explicitly if a scenario asks
 * for `backend: fixture` without a fixture root.
 */
export function fixtureDirFor(scenarioId: string): string {
  return join(REPO_ROOT, "acceptance", "fixtures", scenarioId, "expected");
}
