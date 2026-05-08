// Acceptance aggregator (C16): runs a configured list of `make` targets
// serially, captures their output to per-target log files under a temp
// directory, and reports a single pass/skip/fail summary at the end.
//
// Aggregator does NOT fail-fast: every target runs even if an earlier one
// failed; the script exits non-zero only after the summary has been
// printed. A target whose output contains a `[skip]` line and exits 0 is
// classified as `skip` rather than `pass`.
//
// Usage:
//   deno run --allow-read --allow-write --allow-env --allow-run \
//     scripts/acceptance-aggregate.ts <target> [<target> ...] \
//       [--label <name>] [--quiet] [--show-output]
//
// Flags:
//   --label <name>     Friendly aggregator label printed in the header.
//   --quiet            Suppress per-target output even on failure.
//   --show-output      Stream full per-target output (default: stream only
//                      a per-target status line; show captured output only
//                      when the target failed).
//
// Designed to be invoked from Makefile targets like `acceptance-cross-repo`
// and `acceptance-cross-repo-deterministic`.

import { dirname, fromFileUrl, join, resolve } from "jsr:@std/path@1";

const REPO_ROOT = resolve(dirname(fromFileUrl(import.meta.url)), "..");

const RED = "\x1b[0;31m";
const GREEN = "\x1b[0;32m";
const YELLOW = "\x1b[0;33m";
const DIM = "\x1b[2m";
const NC = "\x1b[0m";

type Status = "pass" | "fail" | "skip";

interface TargetResult {
  target: string;
  status: Status;
  exitCode: number;
  durationMs: number;
  logPath: string;
}

interface ParsedArgs {
  targets: string[];
  label: string;
  quiet: boolean;
  showOutput: boolean;
}

function parseArgs(argv: string[]): ParsedArgs {
  const out: ParsedArgs = {
    targets: [],
    label: "acceptance",
    quiet: false,
    showOutput: false,
  };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--label" && i + 1 < argv.length) {
      out.label = argv[++i];
    } else if (arg === "--quiet") {
      out.quiet = true;
    } else if (arg === "--show-output") {
      out.showOutput = true;
    } else if (arg.startsWith("--")) {
      console.error(`unknown flag: ${arg}`);
      Deno.exit(2);
    } else {
      out.targets.push(arg);
    }
  }
  if (out.targets.length === 0) {
    console.error(
      "usage: acceptance-aggregate.ts <target> [<target> ...] [--label name] [--quiet] [--show-output]",
    );
    Deno.exit(2);
  }
  return out;
}

async function runTarget(
  target: string,
  logDir: string,
  showOutput: boolean,
): Promise<TargetResult> {
  const logPath = join(logDir, `${target}.log`);
  const logFile = await Deno.open(logPath, {
    create: true,
    write: true,
    truncate: true,
  });

  // Serialise writes to the shared log file so the two `tee` tasks
  // (one per child stream) do not race over a single FsFile handle —
  // Deno's WritableStream API rejects concurrent getWriter() calls.
  let writeChain: Promise<unknown> = Promise.resolve();
  const writeToLog = (bytes: Uint8Array): Promise<number> => {
    const next = writeChain.then(() => logFile.write(bytes));
    writeChain = next.catch(() => undefined);
    return next;
  };

  const started = performance.now();
  const cmd = new Deno.Command("make", {
    args: [target],
    cwd: REPO_ROOT,
    stdin: "null",
    stdout: "piped",
    stderr: "piped",
  });

  let exitCode = 0;
  let textBuffer = "";
  try {
    const child = cmd.spawn();

    const tee = async (
      src: ReadableStream<Uint8Array>,
      mirror: typeof Deno.stdout | typeof Deno.stderr | null,
    ) => {
      const decoder = new TextDecoder();
      const reader = src.getReader();
      try {
        while (true) {
          const { value, done } = await reader.read();
          if (done) break;
          if (value) {
            textBuffer += decoder.decode(value, { stream: true });
            await writeToLog(value);
            if (mirror) {
              try {
                await mirror.write(value);
              } catch {
                // mirror may close mid-stream (operator Ctrl+C); the
                // log still captures the bytes for post-mortem review.
              }
            }
          }
        }
      } finally {
        reader.releaseLock();
      }
    };

    // We always tee stderr+stdout to the log file. When --show-output is
    // set, also mirror to the live console so the operator can watch
    // progress; otherwise we keep the aggregator console quiet and only
    // re-emit the buffered log on failure (default behavior).
    const stdoutMirror = showOutput ? Deno.stdout : null;
    const stderrMirror = showOutput ? Deno.stderr : null;

    const [, , status] = await Promise.all([
      tee(child.stdout, stdoutMirror),
      tee(child.stderr, stderrMirror),
      child.status,
    ]);
    exitCode = status.code;
    await writeChain;
  } catch (e) {
    const msg = e instanceof Error ? `${e.name}: ${e.message}` : String(e);
    const note = `\n[aggregator] failed to spawn make ${target}: ${msg}\n`;
    try {
      await writeToLog(new TextEncoder().encode(note));
      await writeChain;
    } catch {
      // best-effort
    }
    textBuffer += note;
    exitCode = 2;
  } finally {
    try {
      logFile.close();
    } catch {
      // already closed
    }
  }

  const durationMs = Math.round(performance.now() - started);

  // Skip detection: smoke drivers print a `[skip]` (or `[c<NN> skip]`,
  // `[c<NN>-suffix skip]`) line and exit 0 when their preconditions are
  // not satisfied (no `specify` on PATH, missing operator results,
  // missing recorded trace, etc.). Treat that as skip rather than pass
  // so the aggregator summary distinguishes "ran clean" from "did
  // nothing".
  let status: Status;
  if (exitCode === 0) {
    status = /^\s*\[(?:[a-z0-9][a-z0-9 -]*\s+)?skip\]/m.test(textBuffer)
      ? "skip"
      : "pass";
  } else {
    status = "fail";
  }

  return { target, status, exitCode, durationMs, logPath };
}

function statusBadge(status: Status): string {
  switch (status) {
    case "pass":
      return `${GREEN}PASS${NC}`;
    case "skip":
      return `${YELLOW}SKIP${NC}`;
    case "fail":
      return `${RED}FAIL${NC}`;
  }
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
}

async function main(): Promise<number> {
  const args = parseArgs(Deno.args);

  const logRoot = await Deno.makeTempDir({
    prefix: `specify-acceptance-${args.label}-`,
  });

  console.log(`[aggregator] label:   ${args.label}`);
  console.log(`[aggregator] targets: ${args.targets.length}`);
  console.log(`[aggregator] logs:    ${logRoot}`);
  console.log("");

  const results: TargetResult[] = [];
  for (const target of args.targets) {
    process_stdout_write(`  → ${target} ... `);
    const result = await runTarget(target, logRoot, args.showOutput);
    results.push(result);
    console.log(`${statusBadge(result.status)} (${formatDuration(result.durationMs)})`);
    if (result.status === "fail" && !args.quiet && !args.showOutput) {
      // Re-emit captured output so the failure is debuggable from the
      // aggregator's own console without having to open the log file.
      try {
        const captured = await Deno.readTextFile(result.logPath);
        console.log(`${DIM}--- ${result.target} output (exit ${result.exitCode}) ---${NC}`);
        console.log(captured.trimEnd());
        console.log(`${DIM}--- end ${result.target} output ---${NC}`);
      } catch {
        // log file unreadable; nothing to mirror
      }
    }
  }

  const passes = results.filter((r) => r.status === "pass").length;
  const skips = results.filter((r) => r.status === "skip").length;
  const fails = results.filter((r) => r.status === "fail").length;

  console.log("");
  console.log(`[aggregator] summary (${args.label}):`);
  for (const r of results) {
    console.log(
      `  ${statusBadge(r.status)}  ${r.target.padEnd(48)} ` +
        `${formatDuration(r.durationMs).padStart(8)}  ` +
        `${DIM}${r.logPath}${NC}`,
    );
  }
  console.log("");
  console.log(
    `[aggregator] totals: ${passes} pass, ${skips} skip, ${fails} fail ` +
      `(${results.length} target${results.length === 1 ? "" : "s"})`,
  );

  if (fails > 0) {
    console.log(
      `${RED}[aggregator] ${fails} target(s) failed. ` +
        `Logs preserved under ${logRoot}.${NC}`,
    );
    return 1;
  }
  console.log(
    `${GREEN}[aggregator] all targets ${
      skips > 0 ? `passed or skipped (${skips} skipped)` : "passed"
    }.${NC}`,
  );
  return 0;
}

// Tiny shim so we don't pull in node:process for a single sync write.
function process_stdout_write(s: string): void {
  Deno.stdout.writeSync(new TextEncoder().encode(s));
}

const code = await main();
Deno.exit(code);
