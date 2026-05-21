// Golden-diff and idempotency test for scripts/migrate-to-2.0.sh.
//
// Run via:
//   deno test --allow-read --allow-write --allow-run --allow-env \
//     tests/migration_test.ts
//
// Or (preferred): `make test-migration`.

import { assertEquals } from "jsr:@std/assert@1";
import { walk } from "jsr:@std/fs@1/walk";
import { copy } from "jsr:@std/fs@1/copy";
import { join, relative, resolve } from "jsr:@std/path@1";

const REPO_ROOT = resolve(
  new URL("..", import.meta.url).pathname,
);
const FIXTURE_1X = join(REPO_ROOT, "tests/fixtures/migration/1.x");
const FIXTURE_2X = join(REPO_ROOT, "tests/fixtures/migration/2.0");
const EXPECTED_DRY_RUN = join(
  REPO_ROOT,
  "tests/fixtures/migration/expected-dry-run.txt",
);
const SCRIPT = join(REPO_ROOT, "scripts/migrate-to-2.0.sh");

async function runMigration(
  projectRoot: string,
  args: string[] = [],
): Promise<{ stdout: string; stderr: string; code: number }> {
  const cmd = new Deno.Command("bash", {
    args: [SCRIPT, ...args, projectRoot],
    stdout: "piped",
    stderr: "piped",
  });
  const { stdout, stderr, code } = await cmd.output();
  return {
    stdout: new TextDecoder().decode(stdout),
    stderr: new TextDecoder().decode(stderr),
    code,
  };
}

async function copyFixture(dst: string): Promise<void> {
  await copy(FIXTURE_1X, dst, { overwrite: true });
}

async function listTree(root: string): Promise<Map<string, string>> {
  const out = new Map<string, string>();
  for await (const entry of walk(root, { includeDirs: false })) {
    const rel = relative(root, entry.path);
    out.set(rel, await Deno.readTextFile(entry.path));
  }
  return out;
}

function normaliseDryRun(text: string, projectRoot: string): string {
  return text.replaceAll(projectRoot, "<PROJECT>");
}

Deno.test("migrate-to-2.0: dry-run matches golden", async () => {
  const tmp = await Deno.makeTempDir({ prefix: "specify-migration-dryrun-" });
  try {
    await copyFixture(tmp);
    const result = await runMigration(tmp, ["--dry-run"]);
    assertEquals(result.code, 0, `stderr:\n${result.stderr}`);

    const expected = await Deno.readTextFile(EXPECTED_DRY_RUN);
    const actual = normaliseDryRun(result.stdout, tmp);
    assertEquals(actual.trim(), expected.trim());

    // Dry-run must not write anything.
    const original = await listTree(FIXTURE_1X);
    const after = await listTree(tmp);
    assertEquals(
      [...after.keys()].sort(),
      [...original.keys()].sort(),
      "dry-run added or removed files",
    );
    for (const [k, v] of after) {
      assertEquals(v, original.get(k), `dry-run mutated ${k}`);
    }
  } finally {
    await Deno.remove(tmp, { recursive: true });
  }
});

Deno.test("migrate-to-2.0: apply produces the 2.0 golden tree", async () => {
  const tmp = await Deno.makeTempDir({ prefix: "specify-migration-apply-" });
  try {
    await copyFixture(tmp);
    const result = await runMigration(tmp);
    assertEquals(result.code, 0, `stderr:\n${result.stderr}`);

    const expected = await listTree(FIXTURE_2X);
    const actual = await listTree(tmp);

    assertEquals(
      [...actual.keys()].sort(),
      [...expected.keys()].sort(),
      "migrated tree has unexpected file set",
    );
    for (const [k, v] of expected) {
      assertEquals(actual.get(k), v, `mismatch in ${k}`);
    }
  } finally {
    await Deno.remove(tmp, { recursive: true });
  }
});

Deno.test("migrate-to-2.0: re-run is a no-op", async () => {
  const tmp = await Deno.makeTempDir({ prefix: "specify-migration-idem-" });
  try {
    await copyFixture(tmp);
    const first = await runMigration(tmp);
    assertEquals(first.code, 0);

    const before = await listTree(tmp);
    const second = await runMigration(tmp);
    assertEquals(second.code, 0);
    const after = await listTree(tmp);

    assertEquals(
      [...after.keys()].sort(),
      [...before.keys()].sort(),
      "second run added or removed files",
    );
    for (const [k, v] of after) {
      assertEquals(v, before.get(k), `second run mutated ${k}`);
    }
    if (!second.stdout.includes("already on specify")) {
      throw new Error(
        `second run missing 'already on specify' confirmation:\n${second.stdout}`,
      );
    }
  } finally {
    await Deno.remove(tmp, { recursive: true });
  }
});
