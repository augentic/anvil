// Vectis-build phase driver (RM-01 plan, C14b).
//
// Plugs into the same `PhaseDriver` interface the C12 stub / agent
// drivers and the C13 contracts-build / C14a omnia-build drivers
// use, but specialised for Vectis-capability implementation slices.
// For the RM-01 fixture the only such slice is `add-oauth-screens`
// (routed to `shop-mobile`); the per-slice dispatch on
// `ScriptedExecuteBackend` (`phaseDriverFor`) keeps every other
// slice on its existing driver (contract slice →
// `ContractsBuildPhaseDriver`, backend slice → `OmniaBuildPhaseDriver`
// when running combined coverage, otherwise → `StubPhaseDriver`).
//
// What the driver writes (mode (i) — deterministic skeleton):
//
//   <slot>/composition.yaml                   (# STUB: header — Vectis composition)
//   <slot>/apps/mobile/login_screen.swift     (// STUB: header — also the residue file)
//
// Why "deterministic but real":
//   * the bodies stay byte-for-byte stable across runs so a CI
//     re-run never produces a diff,
//   * each file carries a `# STUB:` / `// STUB:` header marking it
//     as a deterministic test fixture (not real `/spec:build`
//     output) — mirroring the C12 stub / C13 contracts-build /
//     C14a omnia-build `STUB:`-marker convention,
//   * the YAML is valid against
//     `capabilities/vectis/composition.schema.json` (`version: 1`,
//     a `screens` map with one `login` screen carrying header /
//     body / footer regions, group-based body containers, and
//     event-wired buttons),
//   * the Swift body is a syntactically valid SwiftUI view (one
//     `import`, one `struct LoginScreen: View`, one `body` computed
//     property) — `swiftc -parse` accepts it.
//
// Mode (ii) — agent-delegated `/spec:build` invocation — is
// deferred to a later amendment (the C14b plan explicitly carves it
// out). When wired up it should plug into the `AgentPhaseDriver`
// composition pattern from C12, the same way C13 / C14a reserve a
// mode (ii) for their drivers.
//
// **Commit shape note.** The shared `driveSliceWithBodies` helper
// for routed slices writes the residue file (`opts.residuePath` —
// for `add-oauth-screens` that is `apps/mobile/login_screen.swift`)
// and creates the residue commit by `git add <residuePath> && git
// commit`. To land `composition.yaml` in the same residue commit
// (so `workspace-clean-before-push` and `residue-commit-non-empty`
// continue to hold without growing the per-slice commit count —
// `baseline-merge-commit-clean` would break if HEAD~1 stopped
// pointing at `specify: merge <slice>`), this driver mirrors the
// C14a Omnia approach:
//
//   1. Pre-writes `composition.yaml` into the workspace clone
//      BEFORE calling `driveSliceWithBodies` (it lives outside
//      `.specify/`, so the baseline `git add .specify/specs
//      .specify/archive` does not pick it up — the baseline commit
//      stays clean).
//   2. Lets `driveSliceWithBodies` write `apps/mobile/login_screen.swift`
//      + add it + create the residue commit.
//   3. After `driveSliceWithBodies` returns, `git add` the extra
//      `composition.yaml` and `git commit --amend --no-edit` so HEAD
//      remains the residue commit and HEAD~1 remains the baseline
//      commit. The net effect is one residue commit touching both
//      Vectis files at once.

import { ensureDir } from "jsr:@std/fs@1";
import { dirname, join } from "jsr:@std/path@1";

import {
  driveSliceWithBodies,
  stubBodyFactory,
  type DriveSliceOpts,
  type DriveSliceResult,
  type PhaseDriver,
} from "./phase-driver.ts";
import { runGit } from "../git.ts";

/**
 * Per-slice workspace-relative paths the driver emits for a Vectis
 * slice. Exported so the C14b assertion handlers
 * (`vectis-slice-emits-composition-yaml`,
 * `vectis-slice-emits-screen-files`,
 * `vectis-slice-residue-under-routed-project`) can probe the same
 * canonical paths the driver wrote without re-deriving them.
 *
 * The screen file path matches the residue path policy in
 * `scripted-shared.ts::RESIDUE_PATHS` for the `add-oauth-screens`
 * slice; the composition file lives at the project root per the
 * Vectis artifact responsibilities documented in
 * `.cursor/rules/project.mdc`.
 */
export interface VectisShellPaths {
  /** Workspace-relative `composition.yaml` (project root). */
  compositionYaml: string;
  /** Workspace-relative SwiftUI screen file (the residue file). */
  loginScreen: string;
}

/** Compute the per-slice paths for the RM-01 mobile slice. */
export function vectisShellPaths(_sliceName: string): VectisShellPaths {
  // The RM-01 fixture pins one slice / one screen. The argument is
  // accepted so a future multi-slice fixture can vary by slice
  // without breaking the call sites.
  return {
    compositionYaml: "composition.yaml",
    loginScreen: "apps/mobile/login_screen.swift",
  };
}

/**
 * Body bundle the driver writes for the RM-01 mobile slice. Each
 * file is a literal string written to disk verbatim. Marked
 * `# STUB:` / `// STUB:` so a reader can tell it is a deterministic
 * fixture rather than real `/spec:build` output.
 *
 * The composition validates against
 * `capabilities/vectis/composition.schema.json`:
 *   * `version: 1` (required const),
 *   * one of `screens` / `delta` (here: `screens` with one entry),
 *   * `provenance.sources[*].kind` ∈ enum,
 *   * each region carries a `contentNodeArray` or a
 *     `headerRegion` / `bodyRegion` shape,
 *   * group `direction` / `gap` / `align` honour their enums,
 *   * every `event:` value matches `eventValue` regex (PascalCase
 *     name + optional argument list),
 *   * every `bind:` value matches `bindValue` regex (snake_case
 *     identifier with optional dotted prefix).
 *
 * The Swift body parses with `swiftc -parse`: one import, one
 * `struct ... : View` declaration, one `body` computed property.
 * Replace with real `/spec:build` output for production runs.
 */
function bodiesFor(): {
  compositionYaml: string;
  loginScreen: string;
} {
  const compositionYaml =
    `# STUB: deterministic composition.yaml fixture for the RM-01 mobile slice.
# Generated by the C14b VectisBuildPhaseDriver. Validates against
# \`capabilities/vectis/composition.schema.json\`. Replace with real
# \`/spec:build\` output for production runs.
version: 1

provenance:
  sources:
    - kind: manual

screens:
  login:
    name: "Login"
    description: "OAuth provider sign-in for the RM-01 fixture mobile clients."
    maps_to: "ViewModel::Login(LoginView)"

    header:
      title: "Sign in"

    body:
      - group:
          direction: column
          gap: medium
          align: center
          padding: medium
          items:
            - heading:
                content: "Welcome back. Sign in to continue."
                role: heading
            - button:
                label: "Continue with Google"
                event: StartProviderLogin(google)
                role: button
            - button:
                label: "Continue with Apple"
                event: StartProviderLogin(apple)
                role: button
            - text:
                content: "We never store your provider password."

    footer:
      - link:
          label: "Need help?"
          event: OpenSupport
          role: link
`;

  const loginScreen =
    `// STUB: deterministic LoginScreen fixture for the RM-01 mobile slice.
// Generated by the C14b VectisBuildPhaseDriver. Replace with real
// \`/spec:build\` output for production runs.
//
// References baseline \`contracts/oauth-login.yaml\` (the merged
// contract slice). This Vectis shell consumes the baseline
// OpenAPI 3.1 contract; it does not author HTTP shapes inline.
// The load-bearing RM-01 contract-first invariant is asserted by
// \`implementation-slice-reads-baseline-contract\` (C12).

import SwiftUI

/// SwiftUI view for the RM-01 OAuth login screen. Mirrors the
/// composition.yaml \`login\` screen entry: header title, a column
/// group of provider buttons, and a footer support link. Real
/// \`/spec:build\` output replaces this stub with a Vectis-generated
/// view that wires bindings + events through the platform shell.
struct LoginScreen: View {
    /// Provider selection callback. Stub keeps a no-op default so
    /// the file parses cleanly without a host scope.
    var onProviderTap: (String) -> Void = { _ in }

    /// Support-link callback. Stubbed for the same reason.
    var onSupportTap: () -> Void = {}

    var body: some View {
        VStack(alignment: .center, spacing: 16) {
            Text("Welcome back. Sign in to continue.")
                .font(.title2)
            Button("Continue with Google") { onProviderTap("google") }
            Button("Continue with Apple") { onProviderTap("apple") }
            Text("We never store your provider password.")
                .font(.footnote)
            Button("Need help?", action: onSupportTap)
        }
        .padding()
    }
}
`;

  return { compositionYaml, loginScreen };
}

/**
 * Vectis-build phase driver. For Vectis implementation slices it
 * pre-writes `composition.yaml` into the routed clone, delegates to
 * `driveSliceWithBodies` for the standard define+build+merge
 * lifecycle (which writes the SwiftUI residue file and the residue
 * commit), then folds the composition into the same residue commit
 * via `git commit --amend --no-edit`.
 *
 * For non-Vectis slices the driver delegates straight through to
 * the stub body factory — mirroring the defensive fall-through in
 * C13's `ContractsBuildPhaseDriver` and C14a's
 * `OmniaBuildPhaseDriver`. The C14b backend uses `phaseDriverFor`
 * so this driver only ever runs for Vectis slices in practice; the
 * fall-through means a misconfigured backend still produces a
 * sensible action log instead of crashing.
 */
export class VectisBuildPhaseDriver implements PhaseDriver {
  readonly name = "vectis-build" as const;

  async driveSlice(opts: DriveSliceOpts): Promise<DriveSliceResult> {
    const isVectisImplSlice = opts.project !== null &&
      opts.capabilityName === "vectis";

    if (!isVectisImplSlice) {
      return driveSliceWithBodies(opts, stubBodyFactory);
    }

    if (!opts.workspaceProjectDir) {
      throw new Error(
        `VectisBuildPhaseDriver: routed Vectis slice '${opts.sliceName}' ` +
          `requires workspaceProjectDir.`,
      );
    }

    const paths = vectisShellPaths(opts.sliceName);
    const bodies = bodiesFor();

    // Pre-write the composition. It lives outside `.specify/`, so
    // the baseline `git add .specify/specs .specify/archive` step
    // does not pick it up.
    const slot = opts.workspaceProjectDir;
    const compositionAbs = join(slot, paths.compositionYaml);
    await ensureDir(dirname(compositionAbs));
    await Deno.writeTextFile(compositionAbs, bodies.compositionYaml);

    // Standard lifecycle (define + baseline commit + residue
    // commit + transition done). The residue body becomes our
    // login_screen.swift.
    const result = await driveSliceWithBodies(opts, (innerOpts) => {
      const stub = stubBodyFactory(innerOpts);
      return { ...stub, residue: bodies.loginScreen };
    });

    // Fold composition.yaml into the residue commit. The
    // `--amend --no-edit` keeps HEAD = residue commit (so HEAD~1
    // remains the `specify: merge <slice>` baseline commit; the
    // C10 baseline-merge-commit-clean assertion still holds).
    await runGit(slot, ["add", paths.compositionYaml], opts.env);
    await runGit(
      slot,
      ["commit", "--no-gpg-sign", "--amend", "--no-edit"],
      opts.env,
    );

    result.actions.push({
      ts: new Date().toISOString(),
      phase: "merge",
      slice: opts.sliceName,
      action: "vectis-build-amend-residue",
      command: ["git", "commit", "--amend", "--no-edit"],
      artifacts: [paths.compositionYaml],
    });

    return result;
  }
}
