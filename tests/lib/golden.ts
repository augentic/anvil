// Golden-file diffing for the acceptance harness. Honours
// REGENERATE_GOLDENS=1 to overwrite the golden file in place rather than
// asserting against it; that flag is intended for one-shot regeneration
// runs after a deliberate change to fixture content.

import { dirname } from "jsr:@std/path@1";
import { walk } from "jsr:@std/fs@1/walk";
import { relative } from "jsr:@std/path@1";

function regenerate(): boolean {
  try {
    return Deno.env.get("REGENERATE_GOLDENS") === "1";
  } catch {
    return false;
  }
}

async function ensureDir(path: string): Promise<void> {
  await Deno.mkdir(path, { recursive: true }).catch(() => {});
}

export async function assertGolden(
  actual: string,
  goldenPath: string,
): Promise<void> {
  if (regenerate()) {
    await ensureDir(dirname(goldenPath));
    await Deno.writeTextFile(goldenPath, actual);
    return;
  }
  let expected: string;
  try {
    expected = await Deno.readTextFile(goldenPath);
  } catch {
    throw new Error(
      `golden missing: ${goldenPath} — re-run with REGENERATE_GOLDENS=1 to create it`,
    );
  }
  if (actual !== expected) {
    throw new Error(
      `golden mismatch: ${goldenPath}\n` +
        `--- expected ---\n${expected}\n--- actual ---\n${actual}\n` +
        `re-run with REGENERATE_GOLDENS=1 to update`,
    );
  }
}

// Recursive directory diff. Compares two directory trees file by file
// (text contents only). On mismatch, raises an Error pointing at the
// first divergence; in regenerate mode, overwrites the expected tree.
export async function assertGoldenTree(
  actualDir: string,
  expectedDir: string,
): Promise<void> {
  if (regenerate()) {
    await ensureDir(expectedDir);
    // Snapshot mode: copy actual on top of expected, drop files that no
    // longer exist on the actual side.
    const actual = await readTree(actualDir);
    const expected = await readTree(expectedDir).catch(() => new Map());
    for (const [rel, content] of actual) {
      const path = `${expectedDir}/${rel}`;
      await ensureDir(dirname(path));
      await Deno.writeTextFile(path, content);
    }
    for (const rel of expected.keys()) {
      if (!actual.has(rel)) {
        try {
          await Deno.remove(`${expectedDir}/${rel}`);
        } catch {
          // ignore
        }
      }
    }
    return;
  }

  const actual = await readTree(actualDir);
  const expected = await readTree(expectedDir);

  const actualKeys = [...actual.keys()].sort();
  const expectedKeys = [...expected.keys()].sort();
  if (actualKeys.join("\n") !== expectedKeys.join("\n")) {
    const onlyExpected = expectedKeys.filter((k) => !actual.has(k));
    const onlyActual = actualKeys.filter((k) => !expected.has(k));
    throw new Error(
      `golden tree file set mismatch under ${expectedDir}:\n` +
        `  only in expected: ${onlyExpected.join(", ") || "<none>"}\n` +
        `  only in actual:   ${onlyActual.join(", ") || "<none>"}`,
    );
  }
  for (const rel of actualKeys) {
    if (actual.get(rel) !== expected.get(rel)) {
      throw new Error(
        `golden tree mismatch at ${expectedDir}/${rel}\n` +
          `re-run with REGENERATE_GOLDENS=1 to update`,
      );
    }
  }
}

async function readTree(root: string): Promise<Map<string, string>> {
  const out = new Map<string, string>();
  for await (const entry of walk(root, { includeDirs: false })) {
    const rel = relative(root, entry.path);
    out.set(rel, await Deno.readTextFile(entry.path));
  }
  return out;
}
