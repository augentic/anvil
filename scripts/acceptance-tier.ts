// Acceptance target selector: map touched files to the smallest useful
// set of `make` targets before pushing.
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
// Exit code is always 0 on a successful selection. A non-zero exit means
// the script itself failed (e.g. git not available); operators should
// fall back to `make acceptance-cross-repo` in that case.

import { dirname, fromFileUrl, resolve } from "jsr:@std/path@1";

const REPO_ROOT = resolve(dirname(fromFileUrl(import.meta.url)), "..");

type Target = "checks" | "acceptance-cross-repo";

// Stable ordering used when emitting selected targets so output is
// deterministic across invocations.
const TARGET_ORDER: Target[] = [
  "checks",
  "acceptance-cross-repo",
];

const RM01: Target[] = ["checks", "acceptance-cross-repo"];

interface PathRule {
  pattern: RegExp;
  add: Target[];
  reason: string;
}

// Path → tier rule table. Order matters only for the explanatory output;
// the selector unions all matching rules' targets.
const RULES: PathRule[] = [
  {
    pattern: /^tests\/.+/,
    add: RM01,
    reason: "acceptance test or fixture change → RM-01 cross-repo test",
  },
  {
    pattern: /^plugins\/(?:spec|change)\/.+/,
    add: RM01,
    reason: "workflow skill change → RM-01 cross-repo test",
  },
  {
    pattern:
      /^capabilities\/(?:contracts|omnia|vectis)\/.+|^plugins\/(?:contract|omnia|vectis)\/.+/,
    add: RM01,
    reason: "RM-01 capability/plugin change → RM-01 cross-repo test",
  },
  {
    pattern:
      /^Makefile$|^scripts\/checks\.ts$|^scripts\/acceptance-tier\.ts$|^\.github\/workflows\/acceptance\.yml$|^\.cursor\/schemas\/scenario\.schema\.json$/,
    add: RM01,
    reason: "acceptance wiring change → RM-01 cross-repo test",
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
        console.error(
          `unknown --format: ${next} (expected 'lines' or 'make-args')`,
        );
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
    'usage: acceptance-tier.ts [--base <ref>] [--files "<a> <b> ..."] [--explain] [--format lines|make-args]',
  );
  console.log("");
  console.log("Selects the smallest sufficient set of `make` targets for a");
  console.log("change set. With no flags,");
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
        '[acceptance-tier] falling back to all targets; pass --files "" for empty diff.',
      );
      printSelection(TARGET_ORDER, args.format);
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

function printSelection(
  selected: Target[],
  format: ParsedArgs["format"],
): void {
  if (format === "make-args") {
    console.log(selected.join(" "));
  } else {
    for (const t of selected) console.log(t);
  }
}

const code = await main();
Deno.exit(code);
