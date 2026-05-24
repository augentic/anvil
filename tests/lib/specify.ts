// Wrapper around the `specify` binary used by the cross-repo acceptance
// harness. Resolves `SPECIFY_BIN` (env override) first, then `specify` on
// PATH. Returns null when no binary is available so callers can skip
// CLI-replay tests cleanly on machines without a built CLI.

export interface SpecifyOptions {
  cwd?: string;
  env?: Record<string, string>;
  stdin?: string;
}

export interface SpecifyResult {
  stdout: string;
  stderr: string;
  code: number;
}

let cachedBin: string | null | undefined;

export async function resolveSpecifyBin(): Promise<string | null> {
  if (cachedBin !== undefined) return cachedBin;
  const override = (() => {
    try {
      return Deno.env.get("SPECIFY_BIN");
    } catch {
      return undefined;
    }
  })();
  if (override) {
    try {
      const stat = await Deno.stat(override);
      if (stat.isFile) {
        cachedBin = override;
        return cachedBin;
      }
    } catch {
      cachedBin = null;
      return cachedBin;
    }
  }
  // Fall back to PATH-resolved `specify`.
  const which = new Deno.Command("sh", {
    args: ["-c", "command -v specify"],
    stdout: "piped",
    stderr: "null",
  });
  const { stdout, code } = await which.output();
  if (code === 0) {
    const path = new TextDecoder().decode(stdout).trim();
    if (path.length > 0) {
      cachedBin = path;
      return cachedBin;
    }
  }
  cachedBin = null;
  return cachedBin;
}

export async function runSpecify(
  args: string[],
  opts: SpecifyOptions = {},
): Promise<SpecifyResult> {
  const bin = await resolveSpecifyBin();
  if (!bin) {
    throw new Error(
      "specify binary not resolvable; set SPECIFY_BIN or install `specify` on PATH",
    );
  }
  const cmd = new Deno.Command(bin, {
    args,
    cwd: opts.cwd,
    env: opts.env,
    stdin: opts.stdin === undefined ? "null" : "piped",
    stdout: "piped",
    stderr: "piped",
  });
  if (opts.stdin === undefined) {
    const { stdout, stderr, code } = await cmd.output();
    return {
      stdout: new TextDecoder().decode(stdout),
      stderr: new TextDecoder().decode(stderr),
      code,
    };
  }
  const child = cmd.spawn();
  const writer = child.stdin.getWriter();
  await writer.write(new TextEncoder().encode(opts.stdin));
  await writer.close();
  const { stdout, stderr, code } = await child.output();
  return {
    stdout: new TextDecoder().decode(stdout),
    stderr: new TextDecoder().decode(stderr),
    code,
  };
}
