// Acceptance tier selector (C16): map a list of touched files to the
// smallest sufficient set of `make` targets the operator (or CI) should
// run before pushing.
//
// Usage:
//   deno run --allow-read --allow-run scripts/acceptance-tier.ts
//   deno run --allow-read --allow-run scripts/acceptance-tier.ts --base origin/main
//   deno run --allow-read scripts/acceptance-tier.ts --files "path/a path/b ..."
//
// Outputs (stdout):
//   - One `make` target per line, in the order they should run.
//   - Only `make checks` is emitted when the diff is empty or touches
//     only neutral files (docs, RFCs, READMEs unrelated to acceptance).
//
// The tier mapping follows the C16 amendment in the implementation plan.
// New mappings should be added there first, then mirrored here.
//
// Exit code is always 0 on a successful selection. A non-zero exit means
// the script itself failed (e.g. git not available); operators should
// fall back to `make acceptance-cross-repo` in that case.

import { dirname, fromFileUrl, resolve } from "jsr:@std/path@1";

const REPO_ROOT = resolve(dirname(fromFileUrl(import.meta.url)), "..");

type Target =
  | "checks"
  | "acceptance-smoke"
  | "acceptance-stub-smoke"
  | "acceptance-cross-repo-recorded-smoke"
  | "acceptance-cross-repo-setup-smoke"
  | "acceptance-cross-repo-plan-smoke"
  | "acceptance-cross-repo-execute-smoke"
  | "acceptance-cross-repo-finalize-smoke"
  | "acceptance-cross-repo-contracts-build-smoke"
  | "acceptance-cross-repo-omnia-build-smoke"
  | "acceptance-cross-repo-vectis-build-smoke";

// Stable ordering used when emitting selected targets so the output is
// deterministic across invocations. Mirrors the tier order in the plan.
const TARGET_ORDER: Target[] = [
  "checks",
  "acceptance-smoke",
  "acceptance-stub-smoke",
  "acceptance-cross-repo-recorded-smoke",
  "acceptance-cross-repo-setup-smoke",
  "acceptance-cross-repo-plan-smoke",
  "acceptance-cross-repo-execute-smoke",
  "acceptance-cross-repo-finalize-smoke",
  "acceptance-cross-repo-contracts-build-smoke",
  "acceptance-cross-repo-omnia-build-smoke",
  "acceptance-cross-repo-vectis-build-smoke",
];

const TIER_0: Target[] = ["checks"];
const TIER_1: Target[] = [
  "acceptance-smoke",
  "acceptance-stub-smoke",
  "acceptance-cross-repo-recorded-smoke",
];
const TIER_2: Target[] = [
  "acceptance-cross-repo-setup-smoke",
  "acceptance-cross-repo-plan-smoke",
  "acceptance-cross-repo-execute-smoke",
  "acceptance-cross-repo-finalize-smoke",
];
const TIER_3_CONTRACTS: Target[] = ["acceptance-cross-repo-contracts-build-smoke"];
const TIER_3_OMNIA: Target[] = ["acceptance-cross-repo-omnia-build-smoke"];
const TIER_3_VECTIS: Target[] = ["acceptance-cross-repo-vectis-build-smoke"];
const TIER_3_ALL: Target[] = [
  ...TIER_3_CONTRACTS,
  ...TIER_3_OMNIA,
  ...TIER_3_VECTIS,
];

interface PathRule {
  pattern: RegExp;
  add: Target[];
  reason: string;
}

// Path → tier rule table. Order matters only for the explanatory output;
// the selector unions all matching rules' targets.
const RULES: PathRule[] = [
  {
    // Runner / assertion changes can affect every cross-repo smoke.
    pattern: /^acceptance\/runner\/.+|^acceptance\/assertions\/.+/,
    add: [...TIER_0, ...TIER_1, ...TIER_2, ...TIER_3_ALL],
    reason: "runner or assertion change → all cross-repo smokes (Tier 1+2+3)",
  },
  {
    pattern: /^capabilities\/contracts\/.+|^plugins\/contract\/.+|^acceptance\/recorded\/.+/,
    add: [...TIER_0, ...TIER_1, ...TIER_3_CONTRACTS],
    reason: "contracts capability / plugin / recorded trace → Tier 1 + contracts-build",
  },
  {
    pattern: /^capabilities\/omnia\/.+|^plugins\/omnia\/.+/,
    add: [...TIER_0, ...TIER_1, ...TIER_3_OMNIA],
    reason: "omnia capability or plugin → Tier 1 + omnia-build",
  },
  {
    pattern: /^capabilities\/vectis\/.+|^plugins\/vectis\/.+/,
    add: [...TIER_0, ...TIER_1, ...TIER_3_VECTIS],
    reason: "vectis capability or plugin → Tier 1 + vectis-build",
  },
  {
    pattern: /^plugins\/spec\/.+|^plugins\/change\/.+/,
    add: [...TIER_0, ...TIER_1, ...TIER_2],
    reason: "spec/change plugin (skill prose) → Tier 1 + Tier 2 (deterministic flows)",
  },
  {
    // General acceptance/ changes (READMEs, suites, fixtures, schemas)
    // and Makefile / scripts/checks.ts edits are framework-level: keep
    // Tier 0 and Tier 1 to catch obvious regressions cheaply. Tier 2/3
    // are only triggered when the runner or specific capability code
    // changes (the more specific rules above).
    pattern:
      /^Makefile$|^scripts\/checks\.ts$|^scripts\/acceptance-(?:tier|aggregate)\.ts$|^acceptance\/.+|^\.cursor\/schemas\/(?:scenario|operator-results)\.schema\.json$/,
    add: [...TIER_0, ...TIER_1],
    reason: "framework / Makefile / acceptance docs → Tier 0 + Tier 1",
  },
];

interface ParsedArgs {
  base: string | null;
  files: string[] | null;
  explain: boolean;
  format: "lines" | "make-args";
}

function parseArgs(argv: string[]): ParsedArgs {
  const out: ParsedArgs = {
    base: null,
    files: null,
    explain: false,
    format: "lines",
  };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--base" && i + 1 < argv.length) {
      out.base = argv[++i];
    } else if (arg === "--files" && i + 1 < argv.length) {
      out.files = argv[++i]
        .split(/\s+/)
        .map((s) => s.trim())
        .filter((s) => s.length > 0);
    } else if (arg === "--explain") {
      out.explain = true;
    } else if (arg === "--format" && i + 1 < argv.length) {
      const next = argv[++i];
      if (next !== "lines" && next !== "make-args") {
        console.error(`unknown --format: ${next} (expected 'lines' or 'make-args')`);
        Deno.exit(2);
      }
      out.format = next;
    } else if (arg === "-h" || arg === "--help") {
      printHelp();
      Deno.exit(0);
    } else {
      console.error(`unknown argument: ${arg}`);
      printHelp();
      Deno.exit(2);
    }
  }
  return out;
}

function printHelp(): void {
  console.log(
    "usage: acceptance-tier.ts [--base <ref>] [--files \"<a> <b> ...\"] [--explain] [--format lines|make-args]",
  );
  console.log("");
  console.log("Selects the smallest sufficient set of `make` targets for a");
  console.log("change set, based on the C16 tier mapping. With no flags,");
  console.log("derives the file list from `git diff --name-only` against");
  console.log("`origin/main` (falling back to `HEAD~1`).");
}

async function deriveBase(): Promise<string> {
  for (const candidate of ["origin/main", "HEAD~1"]) {
    const ok = await refExists(candidate);
    if (ok) return candidate;
  }
  return "HEAD";
}

async function refExists(ref: string): Promise<boolean> {
  try {
    const proc = new Deno.Command("git", {
      args: ["rev-parse", "--verify", "--quiet", ref],
      cwd: REPO_ROOT,
      stdout: "null",
      stderr: "null",
    });
    const status = await proc.output();
    return status.code === 0;
  } catch {
    return false;
  }
}

async function changedFiles(base: string): Promise<string[]> {
  const proc = new Deno.Command("git", {
    args: ["diff", "--name-only", `${base}..HEAD`],
    cwd: REPO_ROOT,
    stdout: "piped",
    stderr: "piped",
  });
  const out = await proc.output();
  if (out.code !== 0) {
    const err = new TextDecoder().decode(out.stderr);
    throw new Error(`git diff failed (exit ${out.code}): ${err.trim()}`);
  }
  const text = new TextDecoder().decode(out.stdout);
  return text
    .split("\n")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

interface SelectionExplanation {
  file: string;
  matchedRules: { reason: string; add: Target[] }[];
}

interface Selection {
  selected: Target[];
  explanations: SelectionExplanation[];
}

function selectTargets(files: string[]): Selection {
  const selected = new Set<Target>();
  const explanations: SelectionExplanation[] = [];

  // Always emit Tier 0 (`make checks`) as the floor: it stays cheap and
  // catches the markdown / scenario-frontmatter regressions that any PR
  // can introduce regardless of touched paths.
  selected.add("checks");

  for (const file of files) {
    const matched: { reason: string; add: Target[] }[] = [];
    for (const rule of RULES) {
      if (rule.pattern.test(file)) {
        for (const t of rule.add) selected.add(t);
        matched.push({ reason: rule.reason, add: rule.add });
      }
    }
    if (matched.length > 0) {
      explanations.push({ file, matchedRules: matched });
    }
  }

  const ordered: Target[] = TARGET_ORDER.filter((t) => selected.has(t));
  return { selected: ordered, explanations };
}

async function main(): Promise<number> {
  const args = parseArgs(Deno.args);

  let files: string[];
  if (args.files !== null) {
    files = args.files;
  } else {
    let base = args.base;
    if (!base) base = await deriveBase();
    try {
      files = await changedFiles(base);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      console.error(`[acceptance-tier] ${msg}`);
      console.error(
        "[acceptance-tier] falling back to all targets; pass --files \"\" for empty diff.",
      );
      // Conservative fallback: include every cross-repo smoke so the
      // operator does not silently under-test a change just because git
      // misbehaved.
      const allTargets: Target[] = [
        ...TIER_0,
        ...TIER_1,
        ...TIER_2,
        ...TIER_3_ALL,
      ];
      const ordered = TARGET_ORDER.filter((t) => allTargets.includes(t));
      printSelection(ordered, args.format);
      return 0;
    }
  }

  const { selected, explanations } = selectTargets(files);

  if (args.explain) {
    console.error(`[acceptance-tier] base files: ${files.length}`);
    if (files.length === 0) {
      console.error(
        "[acceptance-tier] no changed files; emitting Tier 0 floor (`make checks`) only.",
      );
    } else {
      const matched = new Set(explanations.map((e) => e.file));
      const unmatched = files.filter((f) => !matched.has(f));
      for (const exp of explanations) {
        console.error(`[acceptance-tier] ${exp.file}`);
        for (const m of exp.matchedRules) {
          console.error(
            `[acceptance-tier]   + ${m.add.join(", ")}  (${m.reason})`,
          );
        }
      }
      if (unmatched.length > 0) {
        console.error(
          `[acceptance-tier] ${unmatched.length} file(s) had no rule match (Tier 0 only):`,
        );
        for (const f of unmatched) console.error(`[acceptance-tier]   - ${f}`);
      }
    }
    console.error(`[acceptance-tier] selected: ${selected.join(" ")}`);
  }

  printSelection(selected, args.format);
  return 0;
}

function printSelection(selected: Target[], format: ParsedArgs["format"]): void {
  if (format === "make-args") {
    console.log(selected.join(" "));
  } else {
    for (const t of selected) console.log(t);
  }
}

const code = await main();
Deno.exit(code);
