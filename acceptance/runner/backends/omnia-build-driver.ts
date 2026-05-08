// Omnia-build phase driver (RM-01 plan, C14a).
//
// Plugs into the same `PhaseDriver` interface the C12 stub / agent
// drivers and the C13 contracts-build driver use, but specialised
// for Omnia-capability implementation slices. For the RM-01 fixture
// the only such slice is `add-oauth-tokens` (routed to
// `shop-backend`); the per-slice dispatch on
// `ScriptedExecuteBackend` (`phaseDriverFor`) keeps every other
// slice on its existing driver (contract slice →
// `ContractsBuildPhaseDriver`, mobile slice → `StubPhaseDriver`).
//
// What the driver writes (mode (i) — deterministic skeleton):
//
//   <slot>/crates/<crate>/Cargo.toml         (# STUB: header)
//   <slot>/crates/<crate>/src/lib.rs         (// STUB: header — also the residue file)
//   <slot>/crates/<crate>/src/providers.rs   (// STUB: header)
//
// Why "deterministic but real":
//   * the bodies stay byte-for-byte stable across runs so a CI
//     re-run never produces a diff,
//   * each file carries a `# STUB:` / `// STUB:` header marking it
//     as a deterministic test fixture (not real `/spec:build`
//     output) — mirroring the C12 stub / C13 contracts-build
//     `STUB:`-marker convention,
//   * the Cargo.toml is valid TOML cargo can parse, the .rs files
//     are valid Rust rustc can parse, and the implementation steers
//     clear of every crate on
//     `plugins/omnia/references/guardrails.md` §Forbidden Crates.
//
// Mode (ii) — agent-delegated `/spec:build` invocation — is
// deferred to a later amendment (the C14a plan explicitly carves it
// out). When wired up it should plug into the `AgentPhaseDriver`
// composition pattern from C12, the same way C13 reserves a mode
// (ii) for contracts.
//
// **Commit shape note.** The shared `driveSliceWithBodies` helper
// for routed slices writes the residue file (`opts.residuePath` —
// for `add-oauth-tokens` that is `crates/oauth_tokens/src/lib.rs`)
// and creates the residue commit by `git add <residuePath> && git
// commit`. To land Cargo.toml + providers.rs in the same residue
// commit (so `workspace-clean-before-push` and `residue-commit-non-
// empty` continue to hold without growing the per-slice commit
// count — `baseline-merge-commit-clean` would break if HEAD~1
// stopped pointing at `specify: merge <slice>`), this driver:
//
//   1. Pre-writes Cargo.toml + providers.rs into the workspace
//      clone BEFORE calling `driveSliceWithBodies` (they live
//      outside `.specify/`, so the baseline `git add .specify/specs
//      .specify/archive` does not pick them up — the baseline
//      commit stays clean).
//   2. Lets `driveSliceWithBodies` write lib.rs + add it + create
//      the residue commit.
//   3. After `driveSliceWithBodies` returns, `git add` the extra
//      files and `git commit --amend --no-edit` so HEAD remains the
//      residue commit and HEAD~1 remains the baseline commit. The
//      net effect is one residue commit touching all three Omnia
//      files at once.

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
 * Slice → crate name policy for the RM-01 cross-repo fixture.
 *
 * `add-oauth-tokens` (routed to `shop-backend`, capability `omnia`)
 * → `oauth_tokens` (the same crate dir name the residue path
 * `crates/oauth_tokens/src/lib.rs` already pins). Backends that mix
 * RM-01 with other suites can extend the map; the driver falls
 * back to deriving a crate name from the slice (kebab → snake)
 * when the slice is not pre-declared.
 */
export const OMNIA_SLICE_TO_CRATE: Record<string, string> = {
  "add-oauth-tokens": "oauth_tokens",
};

/**
 * Per-crate workspace-relative paths the driver emits for an Omnia
 * slice. Exported so the C14a assertion handlers
 * (`omnia-slice-emits-cargo-toml`, `omnia-slice-emits-lib-rs`,
 * `omnia-slice-residue-under-routed-project`) can probe the same
 * canonical paths the driver wrote without re-deriving them.
 */
export interface OmniaCratePaths {
  /** Workspace-relative crate root (e.g. `crates/oauth_tokens`). */
  crateRoot: string;
  /** Workspace-relative `Cargo.toml`. */
  cargoToml: string;
  /** Workspace-relative `src/lib.rs`. */
  libRs: string;
  /** Workspace-relative `src/providers.rs`. */
  providersRs: string;
}

/** Compute the per-crate paths for a slice. */
export function omniaCratePaths(
  sliceName: string,
  crateName?: string,
): OmniaCratePaths {
  const name = crateName ?? OMNIA_SLICE_TO_CRATE[sliceName] ??
    sliceName.replace(/-/g, "_");
  const crateRoot = `crates/${name}`;
  return {
    crateRoot,
    cargoToml: `${crateRoot}/Cargo.toml`,
    libRs: `${crateRoot}/src/lib.rs`,
    providersRs: `${crateRoot}/src/providers.rs`,
  };
}

/**
 * Body bundle the driver writes for the RM-01 OAuth tokens crate.
 * Each file is a literal string written to disk verbatim. Marked
 * `# STUB:` / `// STUB:` so a reader can tell it is a deterministic
 * fixture rather than real `/spec:build` output.
 *
 * Constraints honoured (per `plugins/omnia/references/guardrails.md`):
 *   * No forbidden crate (`reqwest`, `tokio` runtime, `redis`,
 *     `sqlx`, `diesel`, `mongodb`, `azure_storage_blobs`,
 *     `aws-sdk-s3`, `hyper`, `dotenv`, `dotenvy`, `rand`, `uuid`,
 *     `lazy_static`).
 *   * No `unwrap()` / `expect()` in the stub bodies.
 *   * No `unsafe` blocks.
 *   * No `println!` / `dbg!` / `eprintln!`.
 *   * `tokio` only appears under `[dev-dependencies]`.
 *
 * The Cargo.toml uses pinned versions (rather than `workspace =
 * true`) because the fixture lives in an isolated repo with no
 * surrounding workspace; pinning keeps the file syntactically valid
 * for `cargo` to parse without requiring the operator to land an
 * `[workspace]` table. The choice mirrors the C13 contracts-build
 * driver's "valid OpenAPI YAML, but a deterministic fixture
 * shape" pattern.
 */
function bodiesFor(crateName: string): {
  cargoToml: string;
  libRs: string;
  providersRs: string;
} {
  const cargoToml =
    `# STUB: deterministic Cargo.toml fixture for the RM-01 backend slice.
# Generated by the C14a OmniaBuildPhaseDriver. Replace with real
# \`/spec:build\` output (and \`workspace = true\` dependencies) for
# production runs. The forbidden-crate list in
# \`plugins/omnia/references/guardrails.md\` was honoured when
# picking the dependency set below — see the driver source for
# rationale.

[package]
name = "${crateName}"
description = "OAuth token persistence and refresh endpoints (RM-01 fixture)."
edition = "2021"
publish = false
version = "0.1.0"

[lib]
crate-type = ["lib"]

[dependencies]
anyhow = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
tracing = "0.1"

[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt"] }
`;

  const libRs =
    `// STUB: deterministic OAuth token storage stub for the RM-01 backend slice.
// Generated by the C14a OmniaBuildPhaseDriver. Replace with real
// \`/spec:build\` output for production runs.
//
// References baseline \`contracts/oauth-login.yaml\` (the merged
// contract slice). This Omnia crate consumes the baseline OpenAPI
// 3.1 contract; it does not author HTTP shapes inline. The
// load-bearing RM-01 contract-first invariant is asserted by
// \`implementation-slice-reads-baseline-contract\` (C12).

#![allow(dead_code)]

pub mod providers;

use serde::{Deserialize, Serialize};

/// Domain model for an OAuth-issued Shop session token. Mirrors the
/// \`OauthTokenResponse\` JSON Schema in
/// \`contracts/schemas/oauth-token-response.yaml\` so callers can
/// deserialize a provider response straight into the struct.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OauthToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
}

/// Storage entrypoint surfaced to the WASM guest. Real
/// implementations dispatch through a provider trait
/// (\`providers::TokenStore\`) the host runtime supplies; this stub
/// stays no-op so the fixture parses cleanly without bringing in a
/// runtime dependency.
pub struct OauthTokenStore;

impl OauthTokenStore {
    /// Persist a freshly issued OAuth token. The provider trait
    /// performs the actual write; this stub is a placeholder until
    /// real \`/spec:build\` output replaces it.
    pub fn store(
        _provider: &impl providers::TokenStore,
        _key: &str,
        _token: &OauthToken,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
`;

  const providersRs =
    `// STUB: provider trait surface for the RM-01 backend slice.
// Generated by the C14a OmniaBuildPhaseDriver. Replace with real
// \`/spec:build\` output for production runs.
//
// Mirrors the Omnia provider pattern documented in
// \`plugins/omnia/references/capabilities.md\` (one trait per
// provider capability; host runtime injects an implementation at
// guest wire-up time).

use crate::OauthToken;

/// Provider-injected token storage trait. Production
/// implementations live in the WASM host runtime; this stub keeps
/// the trait shape so downstream code can compile against it
/// without a real backend.
pub trait TokenStore {
    /// Persist a token under the supplied key.
    fn put(&self, key: &str, token: &OauthToken) -> anyhow::Result<()>;
    /// Retrieve a previously stored token, or \`Ok(None)\` when the
    /// key is absent.
    fn get(&self, key: &str) -> anyhow::Result<Option<OauthToken>>;
}
`;

  return { cargoToml, libRs, providersRs };
}

/**
 * Omnia-build phase driver. For Omnia implementation slices it
 * pre-writes Cargo.toml + providers.rs into the routed clone,
 * delegates to `driveSliceWithBodies` for the standard
 * define+build+merge lifecycle (which writes the lib.rs residue
 * file and the residue commit), then folds the extra crate files
 * into the same residue commit via `git commit --amend --no-edit`.
 *
 * For non-Omnia slices the driver delegates straight through to
 * the stub body factory — mirroring the defensive fall-through in
 * C13's `ContractsBuildPhaseDriver`. The C14a backend uses
 * `phaseDriverFor` so this driver only ever runs for Omnia slices
 * in practice; the fall-through means a misconfigured backend
 * still produces a sensible action log instead of crashing.
 */
export class OmniaBuildPhaseDriver implements PhaseDriver {
  readonly name = "omnia-build" as const;

  async driveSlice(opts: DriveSliceOpts): Promise<DriveSliceResult> {
    const isOmniaImplSlice = opts.project !== null &&
      opts.capabilityName === "omnia";

    if (!isOmniaImplSlice) {
      return driveSliceWithBodies(opts, stubBodyFactory);
    }

    if (!opts.workspaceProjectDir) {
      throw new Error(
        `OmniaBuildPhaseDriver: routed Omnia slice '${opts.sliceName}' ` +
          `requires workspaceProjectDir.`,
      );
    }

    const paths = omniaCratePaths(opts.sliceName);
    const bodies = bodiesFor(crateNameFromPaths(paths));

    // Pre-write the extras. They live outside `.specify/`, so the
    // baseline `git add .specify/specs .specify/archive` step does
    // not pick them up.
    const slot = opts.workspaceProjectDir;
    const cargoTomlAbs = join(slot, paths.cargoToml);
    const providersRsAbs = join(slot, paths.providersRs);
    await ensureDir(dirname(cargoTomlAbs));
    await ensureDir(dirname(providersRsAbs));
    await Deno.writeTextFile(cargoTomlAbs, bodies.cargoToml);
    await Deno.writeTextFile(providersRsAbs, bodies.providersRs);

    // Standard lifecycle (define + baseline commit + residue
    // commit + transition done). The residue body becomes our
    // lib.rs.
    const result = await driveSliceWithBodies(opts, (innerOpts) => {
      const stub = stubBodyFactory(innerOpts);
      return { ...stub, residue: bodies.libRs };
    });

    // Fold Cargo.toml + providers.rs into the residue commit. The
    // `--amend --no-edit` keeps HEAD = residue commit (so HEAD~1
    // remains the `specify: merge <slice>` baseline commit; the
    // C10 baseline-merge-commit-clean assertion still holds).
    await runGit(slot, ["add", paths.cargoToml, paths.providersRs], opts.env);
    await runGit(
      slot,
      ["commit", "--no-gpg-sign", "--amend", "--no-edit"],
      opts.env,
    );

    result.actions.push({
      ts: new Date().toISOString(),
      phase: "merge",
      slice: opts.sliceName,
      action: "omnia-build-amend-residue",
      command: ["git", "commit", "--amend", "--no-edit"],
      artifacts: [paths.cargoToml, paths.providersRs],
    });

    return result;
  }
}

function crateNameFromPaths(paths: OmniaCratePaths): string {
  // `crates/<crate>` → `<crate>`; safe because `omniaCratePaths`
  // always emits exactly two segments.
  return paths.crateRoot.split("/")[1];
}
