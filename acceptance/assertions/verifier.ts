// Verifier-status helper.
//
// The contracts smoke runner needs an assertion that says "the contract
// WASI verifier exited cleanly". C13 promotes this from a placeholder
// to a real invocation: the helper now resolves the contract WASM,
// stages a scratch capability sidecar so `specify tool run contract`
// can discover the tool, runs it against a contracts/ baseline tree,
// and parses the resulting JSON.
//
// Two surfaces ship here:
//
//   * `assertVerifierStatus(input)` (sync, pre-existing) — the original
//     C05 helper. Accepts pre-captured `stdout` and turns it into a
//     pass/fail/skip record. The C13 contracts-build assertion handler
//     uses it as the JSON-parser shim AFTER `runContractValidator`
//     has captured stdout.
//
//   * `runContractValidator(opts)` (async, new in C13) — the real
//     invocation. Stages a scratch project (`.specify/project.yaml`
//     + `schemas/contracts/{capability,tools}.yaml`) so the WASI tool
//     resolves, hard-links/copies the contracts directory into it,
//     then shells out to `specify --format json tool run contract --
//     <contracts-abs-path>`. Returns the run + parsed JSON +
//     skip-rationale (when the WASM cannot be located).
//
// Design notes:
//
//   * The contract tool resolves its `read` permission against
//     `$PROJECT_DIR/contracts`. Some macOS temp roots are symlinks
//     (`/var → /private/var`); the helper canonicalises every path
//     via `Deno.realPath` before passing it to the tool, mirroring
//     the Layer 0 substrate test in `specify-cli/tests/contract_tool.rs`.
//
//   * The WASM is discovered from (in order): `CONTRACT_WASM` env
//     var, then `<dirname(specifyBin)>/../../crates/contract-validate/dist/contract-*.wasm`
//     (the local-checkout layout). When neither resolves the helper
//     returns `{ skip: "..."}` so the C13 assertion handler can
//     downgrade to a `cli-substrate` skip rather than fail.

import { copy, ensureDir } from "jsr:@std/fs@1";
import { dirname, join } from "jsr:@std/path@1";

import { fail as failRecord, pass as passRecord, skip as skipRecord } from "./types.ts";
import type { AssertionRecord } from "./types.ts";

/** Possible structured statuses a verifier can return. */
export type VerifierStatus = "clean" | "warnings" | "failures";

export interface VerifierAssertionInput {
  id: string;
  /** Path to the contracts directory the verifier should walk. */
  contractsDir: string;
  /** Captured stdout (set by the runner when calling the CLI). */
  stdout?: string;
  /** Required exit status to pass. Default: `clean`. */
  expected?: VerifierStatus;
}

/**
 * Assert the contract verifier returned the expected status. Reads
 * `stdout` (when supplied) for a JSON payload and produces a
 * structured record. The C13 contracts WASI tool emits the v2 schema
 * (`{ "schema-version": 2, "ok": true, "findings": [], "exit-code":
 * 0 }`); legacy callers may still pass v1-shaped payloads (`{ "status":
 * "clean" | "warnings" | "failures", "findings": [...] }`). Both
 * shapes are accepted — v2 is normalised onto the v1 status vocabulary
 * so a single record format flows through evidence.
 *
 * When `stdout` is empty the helper returns `skip` so a scenario can
 * declare this assertion id even when the validator is unavailable;
 * the C13 contracts-build assertion handler uses
 * `runContractValidator` to populate `stdout` before calling this.
 */
export function assertVerifierStatus(
  input: VerifierAssertionInput,
): AssertionRecord {
  const { id, stdout = "", expected = "clean" } = input;

  if (!stdout.trim()) {
    return skipRecord(
      id,
      `Verifier output not captured; pass through \`runContractValidator\` first.`,
      `no verifier stdout available`,
    );
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(stdout);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    return failRecord(
      id,
      `Verifier stdout was not valid JSON: ${msg}`,
      `bad JSON: ${stdout.slice(0, 240)}`,
      "cli-substrate",
    );
  }

  const status = readStatus(parsed);
  if (!status) {
    return failRecord(
      id,
      `Verifier JSON missing 'status' / 'ok' field`,
      `payload: ${JSON.stringify(parsed).slice(0, 240)}`,
      "cli-substrate",
    );
  }

  if (status === expected) {
    return passRecord(
      id,
      `Verifier status '${status}' matched expected '${expected}'.`,
      `status=${status}`,
    );
  }

  return failRecord(
    id,
    `Verifier status '${status}' did not match expected '${expected}'.`,
    `status=${status}; payload=${JSON.stringify(parsed).slice(0, 240)}`,
    "specialist-generation",
  );
}

function readStatus(p: unknown): VerifierStatus | null {
  if (typeof p !== "object" || p === null) return null;
  const obj = p as Record<string, unknown>;
  // C13 v2 shape: `{ "schema-version": 2, "ok": true, "findings": [...] }`.
  if (typeof obj.ok === "boolean") {
    if (obj.ok) return "clean";
    const findings = Array.isArray(obj.findings) ? obj.findings : [];
    if (findings.length === 0) return "failures";
    // The v2 tool emits `severity: warning | error` per finding;
    // any error finding flips the status to `failures`, otherwise
    // `warnings`.
    const hasError = findings.some((f) => {
      if (!f || typeof f !== "object") return false;
      const sev = (f as { severity?: unknown }).severity;
      return sev === "error" || sev === undefined;
    });
    return hasError ? "failures" : "warnings";
  }
  // Legacy v1 shape: `{ "status": "clean" | "warnings" | "failures" }`.
  const status = obj.status;
  if (status === "clean" || status === "warnings" || status === "failures") {
    return status;
  }
  return null;
}

// ---------------------------------------------------------------------------
// Live tool invocation (C13).
// ---------------------------------------------------------------------------

/** Outcome of `runContractValidator`. */
export type ValidatorRun =
  | {
    kind: "ran";
    /** Captured stdout from `specify tool run contract`. */
    stdout: string;
    /** Captured stderr. */
    stderr: string;
    /** Process exit code. `0` for clean, `1` for findings, other for substrate-level errors. */
    exitCode: number;
    /** Parsed JSON payload (`null` when stdout was not parseable JSON). */
    parsed: unknown | null;
    /** Absolute path the validator received as its first positional arg. */
    contractsArg: string;
    /** Scratch project directory the helper materialised. Useful for evidence. */
    scratchDir: string;
  }
  | {
    kind: "skip";
    /** Operator-readable rationale; surfaced as the assertion record evidence. */
    reason: string;
  };

export interface RunContractValidatorOpts {
  /** Absolute path to the `specify` binary. */
  specifyBin: string;
  /** Absolute path to the baseline `contracts/` tree to validate. */
  contractsDir: string;
  /** Optional override for the contract WASM path. Falls back to env + auto-discovery. */
  contractWasmPath?: string;
  /** Optional pre-resolved tools cache directory (otherwise made under tmp). */
  toolsCacheDir?: string;
  /** Optional pre-existing scratch root (otherwise made under tmp). */
  scratchRoot?: string;
  /** Optional logger for sub-process output. */
  logger?: (line: string) => void;
}

/**
 * Stage a scratch capability sidecar and run `specify tool run
 * contract` against the supplied contracts tree. Returns either the
 * captured run output (kind=`ran`) or a skip rationale (kind=`skip`)
 * when the contract WASM cannot be located. Assertion handlers turn
 * the skip rationale into a `cli-substrate` skip record rather than
 * failing.
 */
export async function runContractValidator(
  opts: RunContractValidatorOpts,
): Promise<ValidatorRun> {
  const wasmPath = await resolveContractWasm(opts);
  if (!wasmPath) {
    return {
      kind: "skip",
      reason:
        "Contract WASM not found. Set CONTRACT_WASM=/path/to/contract-*.wasm " +
        "or build `cargo build -p contract-validate --release` in specify-cli " +
        "(expected at <specify-bin>/../../crates/contract-validate/dist/).",
    };
  }

  const scratchRoot = opts.scratchRoot ??
    await Deno.makeTempDir({ prefix: "specify-contract-validator-" });
  await ensureDir(scratchRoot);
  const scratchDir = await Deno.realPath(scratchRoot);
  const cacheDir = opts.toolsCacheDir ??
    join(scratchDir, ".tools-cache");
  await ensureDir(cacheDir);

  // 1. Stage the synthetic project + capability sidecar.
  await ensureDir(join(scratchDir, ".specify"));
  await Deno.writeTextFile(
    join(scratchDir, ".specify", "project.yaml"),
    "name: rm01-contracts-validator-scratch\ncapability: contracts\nrules: {}\n",
  );
  const sidecarDir = join(scratchDir, "schemas", "contracts");
  await ensureDir(sidecarDir);
  await Deno.writeTextFile(
    join(sidecarDir, "capability.yaml"),
    [
      "name: contracts",
      "version: 1",
      "description: RM-01 contracts-validator scratch sidecar (C13).",
      "pipeline:",
      "  define: []",
      "  build: []",
      "  merge: []",
      "",
    ].join("\n"),
  );
  const sha = await sha256Hex(wasmPath);
  await Deno.writeTextFile(
    join(sidecarDir, "tools.yaml"),
    [
      "tools:",
      "  - name: contract",
      "    version: 0.2.0",
      `    source: "file://${wasmPath}"`,
      `    sha256: "${sha}"`,
      "    permissions:",
      '      read:',
      '        - "$PROJECT_DIR/contracts"',
      "      write: []",
      "",
    ].join("\n"),
  );

  // 2. Copy the supplied contracts tree into the scratch project so
  //    the WASI `read: $PROJECT_DIR/contracts` permission lets the
  //    tool actually see the files. Uses `realPath` to flatten any
  //    symlinks the temp-root layout introduced.
  const stagedContracts = join(scratchDir, "contracts");
  await ensureDir(stagedContracts);
  for await (const entry of Deno.readDir(opts.contractsDir)) {
    const src = join(opts.contractsDir, entry.name);
    const dst = join(stagedContracts, entry.name);
    await copy(src, dst, { overwrite: true });
  }
  const contractsArg = await Deno.realPath(stagedContracts);

  // 3. Invoke. The contract tool emits a JSON payload to stdout; we
  //    capture it for the assertion handler to parse.
  // Argument order matches `specify-cli/tests/contract_tool.rs`: the
  // contracts directory is the first positional after `--`, then
  // `--format json` is a tool-level flag the WASI binary parses.
  const cmd = new Deno.Command(opts.specifyBin, {
    cwd: scratchDir,
    args: [
      "tool",
      "run",
      "contract",
      "--",
      contractsArg,
      "--format",
      "json",
    ],
    env: { SPECIFY_TOOLS_CACHE: cacheDir },
    stdout: "piped",
    stderr: "piped",
  });
  const { code, stdout, stderr } = await cmd.output();
  const stdoutText = new TextDecoder().decode(stdout);
  const stderrText = new TextDecoder().decode(stderr);
  if (opts.logger) {
    opts.logger(`[contract-validator] exit=${code}`);
    if (stdoutText) opts.logger(`[contract-validator stdout] ${stdoutText}`);
    if (stderrText) opts.logger(`[contract-validator stderr] ${stderrText}`);
  }
  let parsed: unknown | null = null;
  try {
    parsed = JSON.parse(stdoutText);
  } catch {
    parsed = null;
  }
  return {
    kind: "ran",
    stdout: stdoutText,
    stderr: stderrText,
    exitCode: code,
    parsed,
    contractsArg,
    scratchDir,
  };
}

/**
 * Resolve the contract WASM. Order:
 *   1. `opts.contractWasmPath` override.
 *   2. `CONTRACT_WASM` env var.
 *   3. `<dirname(specifyBin)>/../../crates/contract-validate/dist/contract-*.wasm`
 *      (local-checkout layout matching `specify-cli/tests/contract_tool.rs`).
 * Returns `null` when none resolves so the caller can downgrade to a
 * `cli-substrate` skip.
 */
async function resolveContractWasm(
  opts: RunContractValidatorOpts,
): Promise<string | null> {
  if (opts.contractWasmPath) {
    return await pathIfReadable(opts.contractWasmPath);
  }
  const env = Deno.env.get("CONTRACT_WASM");
  if (env) {
    const resolved = await pathIfReadable(env);
    if (resolved) return resolved;
  }
  const binDir = dirname(opts.specifyBin);
  // <bin>/../../crates/contract-validate/dist/
  const distDir = join(binDir, "..", "..", "crates", "contract-validate", "dist");
  try {
    for await (const entry of Deno.readDir(distDir)) {
      if (!entry.isFile) continue;
      if (!entry.name.endsWith(".wasm")) continue;
      if (!entry.name.startsWith("contract-")) continue;
      const candidate = join(distDir, entry.name);
      const resolved = await pathIfReadable(candidate);
      if (resolved) return resolved;
    }
  } catch {
    // dist dir missing — fall through to null
  }
  return null;
}

async function pathIfReadable(p: string): Promise<string | null> {
  try {
    const real = await Deno.realPath(p);
    const stat = await Deno.stat(real);
    return stat.isFile ? real : null;
  } catch {
    return null;
  }
}

async function sha256Hex(path: string): Promise<string> {
  const bytes = await Deno.readFile(path);
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return Array.from(new Uint8Array(digest))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}
