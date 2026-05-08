import {
  assert,
  assertEquals,
  assertExists,
  assertMatch,
  fail,
} from "jsr:@std/assert@1";
import { copy, ensureDir, exists } from "jsr:@std/fs@1";
import { dirname, fromFileUrl, join, resolve } from "jsr:@std/path@1";
import { parse as parseYaml } from "jsr:@std/yaml@1";

import { markPrMerged, readAllPrStates } from "./support/fake-gh.ts";
import { gitOutput, runGit } from "./support/git.ts";
import { setupHub } from "./support/hub.ts";
import {
  findSpecifyBin,
  runSpecify,
  runSpecifyJson,
  type SpecifyBin,
  SpecifyCommandError,
} from "./support/specify-cli.ts";
import { runWorkspaceSync } from "./support/workspace-sync.ts";

const REPO_ROOT = resolve(dirname(fromFileUrl(import.meta.url)), "..");

const HUB_NAME = "shop-platform";
const CHANGE_NAME = "oauth-login";
const BRANCH_NAME = `specify/${CHANGE_NAME}`;
const CONTRACT_SLICE = "oauth-login-contract";
const BACKEND_SLICE = "add-oauth-tokens";
const MOBILE_SLICE = "add-oauth-screens";

const PROJECTS = [
  {
    name: "shop-backend",
    capability: "omnia@v1",
    description:
      "User registration, account management, OAuth provider integration, token storage, and the authoritative HTTP API.",
  },
  {
    name: "shop-mobile",
    capability: "vectis@v1",
    description:
      "iOS and Android mobile clients with login screens, OAuth redirect handling, and token refresh flows.",
  },
] as const;

const RESIDUE_PATHS: Record<string, string> = {
  [BACKEND_SLICE]: "crates/oauth_tokens/src/lib.rs",
  [MOBILE_SLICE]: "apps/mobile/login_screen.swift",
};

interface Rm01Fixture {
  root: string;
  hubDir: string;
  env: Awaited<ReturnType<typeof setupHub>>["env"];
  fakeGhStateDir: string;
  bin: SpecifyBin;
}

Deno.test("RM-01 cross-repo happy path", async (t) => {
  const bin = await findSpecifyBin();
  if (!bin) {
    console.log(
      "[skip] RM-01 acceptance: no `specify` binary found on PATH and SPECIFY_BIN is unset.",
    );
    return;
  }

  const missingSurface = await missingRequiredSurface(bin.path);
  if (missingSurface) {
    console.log(
      `[skip] RM-01 acceptance: ${bin.path} lacks ${missingSurface}.`,
    );
    return;
  }

  const fixture = await createFixture(bin);
  let passed = false;
  try {
    await t.step("sets up hub registry and workspace", async () => {
      await runWorkspaceSync({ bin, hubDir: fixture.hubDir, env: fixture.env });
      await assertSetup(fixture);
    });

    await t.step("creates a contract-first routed plan", async () => {
      await createPlan(fixture);
      await assertPlanShape(fixture);
    });

    await t.step("executes routed slices with clean commit split", async () => {
      await executePlan(fixture);
      await assertExecutionState(fixture);
    });

    await t.step(
      "pushes through fake gh and finalizes after merge",
      async () => {
        await pushAndFinalize(fixture);
      },
    );

    passed = true;
  } finally {
    const preserve = Deno.env.get("SPECIFY_ACCEPTANCE_PRESERVE") === "1";
    if (passed && !preserve) {
      await Deno.remove(fixture.root, { recursive: true });
    } else {
      console.log(`[rm01] preserved fixture root: ${fixture.root}`);
    }
  }
});

async function createFixture(bin: SpecifyBin): Promise<Rm01Fixture> {
  const root = await Deno.makeTempDir({ prefix: "specify-rm01-" });
  const stdoutLog = join(root, "stdout.log");
  const stderrLog = join(root, "stderr.log");
  await Deno.writeTextFile(stdoutLog, "");
  await Deno.writeTextFile(stderrLog, "");

  const setup = await setupHub({
    tempDir: root,
    hubName: HUB_NAME,
    specifyBin: bin,
    capture: { stdoutLog, stderrLog },
    projects: PROJECTS.map((p) => ({ ...p })),
  });

  const briefSrc = join(
    REPO_ROOT,
    "tests",
    "fixtures",
    "rm01",
    "oauth-login.md",
  );
  const briefDst = join(setup.hubDir, "docs", "oauth-login.md");
  await ensureDir(dirname(briefDst));
  await copy(briefSrc, briefDst, { overwrite: true });

  return {
    root,
    hubDir: setup.hubDir,
    env: setup.env,
    fakeGhStateDir: setup.fakeGhStateDir,
    bin,
  };
}

async function createPlan(fixture: Rm01Fixture): Promise<void> {
  const run = (args: string[]) =>
    runSpecify({
      bin: fixture.bin,
      cwd: fixture.hubDir,
      args,
      env: fixture.env,
    });

  await run(["change", "create", CHANGE_NAME]);
  await run(["change", "plan", "create", CHANGE_NAME]);
  await run([
    "change",
    "plan",
    "add",
    CONTRACT_SLICE,
    "--schema",
    "contracts@v1",
    "--description",
    "Author the shared OAuth login HTTP contract.",
    "--context",
    "docs/oauth-login.md",
  ]);
  await run([
    "change",
    "plan",
    "add",
    BACKEND_SLICE,
    "--project",
    "shop-backend",
    "--depends-on",
    CONTRACT_SLICE,
    "--description",
    "Implement OAuth provider token persistence and refresh endpoints.",
    "--context",
    "contracts/http/oauth-login.yaml",
  ]);
  await run([
    "change",
    "plan",
    "add",
    MOBILE_SLICE,
    "--project",
    "shop-mobile",
    "--depends-on",
    CONTRACT_SLICE,
    "--description",
    "Implement login UI and OAuth redirect handling.",
    "--context",
    "contracts/http/oauth-login.yaml",
  ]);
}

async function executePlan(fixture: Rm01Fixture): Promise<void> {
  const seen = new Set<string>();
  for (let i = 0; i < 16; i++) {
    const { json } = await runSpecifyJson<PlanNextJson>({
      bin: fixture.bin,
      cwd: fixture.hubDir,
      args: ["change", "plan", "next"],
      env: fixture.env,
    });

    if (!json.next) {
      assertEquals(json.reason, "all-done");
      return;
    }

    assert(!seen.has(json.next), `plan next returned ${json.next} twice`);
    seen.add(json.next);
    await driveSlice(fixture, json);
  }

  fail("plan execution did not reach all-done within 16 iterations");
}

async function driveSlice(
  fixture: Rm01Fixture,
  entry: PlanNextJson,
): Promise<void> {
  const sliceName = entry.next;
  assertExists(sliceName);
  const project = typeof entry.project === "string" ? entry.project : null;

  if (project) {
    const residuePath = RESIDUE_PATHS[sliceName];
    assertExists(residuePath, `missing residue path policy for ${sliceName}`);
    const slot = join(fixture.hubDir, ".specify", "workspace", project);

    await runSpecifyJson({
      bin: fixture.bin,
      cwd: fixture.hubDir,
      args: ["workspace", "prepare-branch", project, "--change", CHANGE_NAME],
      env: fixture.env,
    });
    await transition(fixture, sliceName, "in-progress");
    await writeSliceArtifacts(
      slot,
      sliceName,
      project === "shop-backend" ? "omnia" : "vectis",
    );
    await runGit(
      slot,
      ["add", ".specify/specs", ".specify/archive"],
      fixture.env,
    );
    await runGit(slot, [
      "commit",
      "--no-gpg-sign",
      "-m",
      `specify: merge ${sliceName}`,
    ], fixture.env);

    const residue = join(slot, residuePath);
    await ensureDir(dirname(residue));
    await Deno.writeTextFile(
      residue,
      `// Deterministic RM-01 residue for ${sliceName}.\n` +
        `// Consumes the baseline OAuth contract; does not author contract YAML.\n`,
    );
    await runGit(slot, ["add", residuePath], fixture.env);
    await runGit(slot, [
      "commit",
      "--no-gpg-sign",
      "-m",
      `specify: residue ${sliceName}`,
    ], fixture.env);
    await transition(fixture, sliceName, "done");
    return;
  }

  await transition(fixture, sliceName, "in-progress");
  await writeSliceArtifacts(fixture.hubDir, sliceName, "contracts");
  await ensureDir(join(fixture.hubDir, "contracts", "http"));
  await Deno.writeTextFile(
    join(fixture.hubDir, "contracts", "http", "oauth-login.yaml"),
    "openapi: 3.1.0\ninfo:\n  title: OAuth Login\n  version: 0.1.0\npaths: {}\n",
  );
  await transition(fixture, sliceName, "done");
}

async function writeSliceArtifacts(
  root: string,
  sliceName: string,
  capability: "contracts" | "omnia" | "vectis",
): Promise<void> {
  const specDir = join(root, ".specify", "specs", sliceName);
  const archiveDir = join(root, ".specify", "archive", sliceName);
  await ensureDir(specDir);
  await ensureDir(archiveDir);

  const baselineNote = capability === "contracts"
    ? "Owns the shared OAuth contract."
    : "Depends on and consumes the baseline OAuth contract.";
  await Deno.writeTextFile(
    join(specDir, "proposal.md"),
    `# ${sliceName}\n\n${baselineNote}\n`,
  );
  await Deno.writeTextFile(
    join(specDir, "spec.md"),
    `# ${sliceName} Spec\n\n${baselineNote}\n`,
  );
  await Deno.writeTextFile(
    join(specDir, "tasks.md"),
    `# ${sliceName} Tasks\n\n- [ ] Implement ${sliceName}\n`,
  );
  if (capability !== "contracts") {
    await Deno.writeTextFile(
      join(specDir, "design.md"),
      `# ${sliceName} Design\n\n${baselineNote}\n`,
    );
  }

  await Deno.copyFile(
    join(specDir, "proposal.md"),
    join(archiveDir, "proposal.md"),
  );
  await Deno.copyFile(join(specDir, "tasks.md"), join(archiveDir, "tasks.md"));
  if (capability !== "contracts") {
    await Deno.copyFile(
      join(specDir, "design.md"),
      join(archiveDir, "design.md"),
    );
  }
}

async function transition(
  fixture: Rm01Fixture,
  sliceName: string,
  status: "in-progress" | "done",
): Promise<void> {
  await runSpecify({
    bin: fixture.bin,
    cwd: fixture.hubDir,
    args: ["change", "plan", "transition", sliceName, status],
    env: fixture.env,
  });
}

async function pushAndFinalize(fixture: Rm01Fixture): Promise<void> {
  const { json: push } = await runSpecifyJson<WorkspacePushJson>({
    bin: fixture.bin,
    cwd: fixture.hubDir,
    args: ["workspace", "push"],
    env: fixture.env,
  });

  assertEquals(push.projects?.length, 2);
  for (const project of push.projects ?? []) {
    assertEquals(project.status, "pushed");
    assertEquals(project.branch, BRANCH_NAME);
    assert(["shop-backend", "shop-mobile"].includes(project.name));
    assertEquals(typeof project.pr, "number");
  }

  const prs = await readAllPrStates(fixture.fakeGhStateDir);
  assertEquals(prs.length, 2);
  for (const pr of prs) {
    assertEquals(pr.state, "OPEN");
    assertEquals(pr.merged, false);
    assertEquals(pr.branch, BRANCH_NAME);
    await markPrMerged({ stateDir: fixture.fakeGhStateDir, repo: pr.repoKey });
  }

  await runSpecifyJson({
    bin: fixture.bin,
    cwd: fixture.hubDir,
    args: ["change", "finalize"],
    env: fixture.env,
  });

  assertEquals(await exists(join(fixture.hubDir, "plan.yaml")), false);
  assert(
    await archivedPlanExists(fixture.hubDir),
    "finalize should archive plan.yaml",
  );

  try {
    await runSpecifyJson({
      bin: fixture.bin,
      cwd: fixture.hubDir,
      args: ["change", "finalize"],
      env: fixture.env,
    });
    fail("second finalize should return plan-not-found");
  } catch (e) {
    assert(e instanceof SpecifyCommandError);
    const output = JSON.parse(e.run.stdout) as { error?: string };
    assertEquals(output.error, "plan-not-found");
  }
}

async function assertSetup(fixture: Rm01Fixture): Promise<void> {
  const projectYaml = await Deno.readTextFile(
    join(fixture.hubDir, ".specify", "project.yaml"),
  );
  assertMatch(projectYaml, /^hub:\s*true$/m);
  assert(!/^capability:/m.test(projectYaml));

  const registry = await Deno.readTextFile(
    join(fixture.hubDir, "registry.yaml"),
  );
  assertMatch(registry, /name:\s*shop-backend/);
  assertMatch(registry, /name:\s*shop-mobile/);
  assertMatch(registry, /description:\s*.+/);

  await runSpecify({
    bin: fixture.bin,
    cwd: fixture.hubDir,
    args: ["registry", "validate"],
    env: fixture.env,
  });

  for (const project of PROJECTS) {
    assert(
      await exists(join(fixture.hubDir, ".specify", "workspace", project.name)),
      `workspace slot missing for ${project.name}`,
    );
  }
}

async function assertPlanShape(fixture: Rm01Fixture): Promise<void> {
  await runSpecify({
    bin: fixture.bin,
    cwd: fixture.hubDir,
    args: ["change", "plan", "validate"],
    env: fixture.env,
  });

  const plan = await loadPlan(join(fixture.hubDir, "plan.yaml"));
  assertEquals(plan.entries.length, 3);

  const contract = plan.entries.filter((e) =>
    e.schema === "contracts@v1" && e.project === null &&
    e.dependsOn.length === 0
  );
  const backend = plan.entries.filter((e) => e.project === "shop-backend");
  const mobile = plan.entries.filter((e) => e.project === "shop-mobile");

  assertEquals(contract.length, 1, "expected one projectless contracts slice");
  assertEquals(backend.length, 1, "expected one backend implementation slice");
  assertEquals(mobile.length, 1, "expected one mobile implementation slice");
  assert(backend[0].dependsOn.includes(contract[0].name));
  assert(mobile[0].dependsOn.includes(contract[0].name));
}

async function assertExecutionState(fixture: Rm01Fixture): Promise<void> {
  for (const project of ["shop-backend", "shop-mobile"]) {
    const slot = join(fixture.hubDir, ".specify", "workspace", project);
    assertEquals(
      await gitOutput(slot, ["branch", "--show-current"], fixture.env),
      BRANCH_NAME,
    );
    assertEquals(
      await gitOutput(slot, ["status", "--porcelain"], fixture.env),
      "",
    );

    const subjects =
      (await gitOutput(slot, ["log", "--format=%s", "-2", "HEAD"], fixture.env))
        .split("\n");
    assertMatch(subjects[0], /^specify: residue /);
    assertMatch(subjects[1], /^specify: merge /);

    const residueFiles = await changedFiles(slot, "HEAD", fixture);
    const baselineFiles = await changedFiles(slot, "HEAD~1", fixture);
    assert(
      residueFiles.length > 0,
      `${project} residue commit should be non-empty`,
    );
    assert(residueFiles.every((p) => !p.startsWith(".specify/")));
    assert(
      baselineFiles.every((p) =>
        p.startsWith(".specify/specs/") || p.startsWith(".specify/archive/")
      ),
      `${project} baseline commit should only touch .specify specs/archive`,
    );
  }
}

async function changedFiles(
  cwd: string,
  rev: string,
  fixture: Rm01Fixture,
): Promise<string[]> {
  const run = await runGit(
    cwd,
    ["show", "--name-only", "--format=", rev],
    fixture.env,
  );
  return run.stdout.split("\n").map((s) => s.trim()).filter(Boolean);
}

async function loadPlan(path: string): Promise<{ entries: PlanEntry[] }> {
  const raw = parseYaml(await Deno.readTextFile(path)) as Record<
    string,
    unknown
  >;
  const entriesRaw = Array.isArray(raw.changes) ? raw.changes : [];
  return {
    entries: entriesRaw.map((item): PlanEntry => {
      const entry = (item ?? {}) as Record<string, unknown>;
      const dependsOn = Array.isArray(entry["depends-on"])
        ? entry["depends-on"].filter((value): value is string =>
          typeof value === "string"
        )
        : [];
      return {
        name: typeof entry.name === "string" ? entry.name : "",
        project: typeof entry.project === "string" && entry.project.length > 0
          ? entry.project
          : null,
        schema: typeof entry.schema === "string" ? entry.schema : null,
        dependsOn,
      };
    }),
  };
}

async function archivedPlanExists(hubDir: string): Promise<boolean> {
  const archive = join(hubDir, ".specify", "archive", "plans");
  try {
    for await (const entry of Deno.readDir(archive)) {
      if (
        entry.isFile &&
        entry.name.startsWith(`${CHANGE_NAME}-`) &&
        entry.name.endsWith(".yaml")
      ) return true;
    }
  } catch {
    return false;
  }
  return false;
}

async function missingRequiredSurface(bin: string): Promise<string | null> {
  const checks: Array<[string, string[], string]> = [
    ["`specify init --hub`", ["init", "--help"], "--hub"],
    ["`specify change plan`", ["change", "plan", "--help"], "create"],
    [
      "`specify change plan next`",
      ["change", "plan", "next", "--help"],
      "eligible",
    ],
    ["`specify workspace prepare-branch`", [
      "workspace",
      "prepare-branch",
      "--help",
    ], "--change"],
    ["`specify workspace push`", ["workspace", "push", "--help"], "push"],
    ["`specify change finalize`", ["change", "finalize", "--help"], "finalize"],
  ];

  for (const [label, args, needle] of checks) {
    if (!(await helpHas(bin, args, needle))) return label;
  }
  return null;
}

async function helpHas(
  bin: string,
  args: string[],
  needle: string,
): Promise<boolean> {
  try {
    const { code, stdout } = await new Deno.Command(bin, {
      args,
      stdout: "piped",
      stderr: "null",
    }).output();
    return code === 0 && new TextDecoder().decode(stdout).includes(needle);
  } catch {
    return false;
  }
}

interface PlanNextJson {
  next: string | null;
  reason: string | null;
  project?: string | null;
}

interface PlanEntry {
  name: string;
  project: string | null;
  schema: string | null;
  dependsOn: string[];
}

interface WorkspacePushJson {
  projects?: Array<{
    name: string;
    status: string;
    branch: string;
    pr?: number;
  }>;
}
