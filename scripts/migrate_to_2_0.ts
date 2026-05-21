// Specify 1.x -> 2.0 mechanical migration.
//
// Run once per downstream consumer project. Transforms on-disk state to
// match the 2.0 shapes (`target` instead of `adapter`, structured
// `slices[].sources`, kebab-case `specify-version`, per-axis cache, etc.).
//
// Idempotent: re-running on an already-migrated project is a no-op.
//
// Invoked by `scripts/migrate-to-2.0.sh`. See `docs/migration/2.0.md`.

import {
  parse as parseYaml,
  stringify as stringifyYaml,
} from "jsr:@std/yaml@1";
import { join, relative } from "jsr:@std/path@1";
import { walk } from "jsr:@std/fs@1/walk";

const SPECIFY_VERSION = "2.0.0";

type Json = unknown;
type YamlValue = Record<string, unknown>;

interface Options {
  projectRoot: string;
  dryRun: boolean;
}

interface Logger {
  change(file: string, msg: string): void;
  warn(msg: string): void;
  info(msg: string): void;
  summary(): { changes: number; warnings: number };
}

function makeLogger(opts: Options): Logger {
  let changes = 0;
  let warnings = 0;
  const prefix = opts.dryRun ? "would change " : "changed     ";
  return {
    change(file, msg) {
      changes++;
      const rel = relative(opts.projectRoot, file) || file;
      console.log(`${prefix}${rel}: ${msg}`);
    },
    warn(msg) {
      warnings++;
      console.log(`warning     ${msg}`);
    },
    info(msg) {
      console.log(`info        ${msg}`);
    },
    summary() {
      return { changes, warnings };
    },
  };
}

function parseArgs(argv: string[]): Options {
  let dryRun = false;
  const positional: string[] = [];
  for (const a of argv) {
    if (a === "--dry-run") dryRun = true;
    else if (a === "--help" || a === "-h") {
      console.log(
        "Usage: migrate-to-2.0.sh [--dry-run] [project-root]\n\n" +
          "Migrates a Specify 1.x project tree to 2.0 in place.\n" +
          "Defaults project-root to $PWD. Idempotent; safe to re-run.",
      );
      Deno.exit(0);
    } else if (a.startsWith("-")) {
      console.error(`unknown flag: ${a}`);
      Deno.exit(2);
    } else positional.push(a);
  }
  const projectRoot = positional[0] ?? Deno.cwd();
  return { projectRoot, dryRun };
}

async function exists(path: string): Promise<boolean> {
  try {
    await Deno.lstat(path);
    return true;
  } catch (e) {
    if (e instanceof Deno.errors.NotFound) return false;
    throw e;
  }
}

async function isDir(path: string): Promise<boolean> {
  try {
    return (await Deno.stat(path)).isDirectory;
  } catch {
    return false;
  }
}

async function readYaml(path: string): Promise<YamlValue> {
  const txt = await Deno.readTextFile(path);
  const v = parseYaml(txt);
  if (typeof v !== "object" || v === null) {
    throw new Error(`${path}: expected mapping, got ${typeof v}`);
  }
  return v as YamlValue;
}

function writeYamlText(value: Json): string {
  return stringifyYaml(value as Record<string, unknown>, {
    lineWidth: 100,
    indent: 2,
  });
}

async function writeYaml(
  path: string,
  value: Json,
  opts: Options,
): Promise<void> {
  if (opts.dryRun) return;
  await Deno.writeTextFile(path, writeYamlText(value));
}

// Rename keys in a flat mapping. Returns true if any rename occurred.
function renameKey(
  obj: Record<string, unknown>,
  oldKey: string,
  newKey: string,
): boolean {
  if (!(oldKey in obj)) return false;
  if (newKey in obj && oldKey !== newKey) {
    // Both present; trust the new key (already migrated). Drop the old.
    delete obj[oldKey];
    return true;
  }
  obj[newKey] = obj[oldKey];
  delete obj[oldKey];
  return true;
}

// -----------------------------------------------------------------------------
// Transform 1: rename `adapters/` -> `targets/` at the project root.

async function migrateAdaptersDir(opts: Options, log: Logger): Promise<void> {
  const src = join(opts.projectRoot, "adapters");
  const dst = join(opts.projectRoot, "targets");
  if (!(await isDir(src))) return;
  if (await isDir(dst)) {
    log.warn(
      `both adapters/ and targets/ exist at project root; leaving adapters/ in place for manual review`,
    );
    return;
  }
  log.change(src, `rename to ${relative(opts.projectRoot, dst) || "targets"}`);
  if (!opts.dryRun) await Deno.rename(src, dst);
}

// -----------------------------------------------------------------------------
// Transform 2: rewrite .specify/project.yaml.

async function migrateProjectYaml(opts: Options, log: Logger): Promise<void> {
  const path = join(opts.projectRoot, ".specify", "project.yaml");
  if (!(await exists(path))) {
    throw new Error(
      `not a Specify project: missing ${relative(opts.projectRoot, path) || path}`,
    );
  }
  const before = await Deno.readTextFile(path);
  const doc = (parseYaml(before) ?? {}) as YamlValue;

  const notes: string[] = [];
  if (renameKey(doc, "adapter", "target")) notes.push("adapter -> target");
  if (renameKey(doc, "specify_version", "specify-version")) {
    notes.push("specify_version -> specify-version");
  }
  if (renameKey(doc, "hub", "workspace")) notes.push("hub -> workspace");

  if (doc["specify-version"] !== SPECIFY_VERSION) {
    doc["specify-version"] = SPECIFY_VERSION;
    notes.push(`specify-version = ${SPECIFY_VERSION}`);
  }

  const after = writeYamlText(doc);
  if (after !== before) {
    log.change(path, notes.join("; ") || "normalised");
    if (!opts.dryRun) await Deno.writeTextFile(path, after);
  }
}

// -----------------------------------------------------------------------------
// Transform 3: rewrite .specify/registry.yaml.

async function migrateRegistryYaml(opts: Options, log: Logger): Promise<void> {
  const path = join(opts.projectRoot, ".specify", "registry.yaml");
  if (!(await exists(path))) return;
  const before = await Deno.readTextFile(path);
  const doc = (parseYaml(before) ?? {}) as YamlValue;

  let touched = false;
  const projects = doc["projects"];
  if (Array.isArray(projects)) {
    for (const p of projects) {
      if (typeof p === "object" && p !== null) {
        const obj = p as Record<string, unknown>;
        if (renameKey(obj, "adapter", "target")) touched = true;
        if (renameKey(obj, "proposed-adapter", "proposed-target")) {
          touched = true;
        }
        if (renameKey(obj, "proposed_adapter", "proposed-target")) {
          touched = true;
        }
      }
    }
  } else if (typeof projects === "object" && projects !== null) {
    for (const v of Object.values(projects)) {
      if (typeof v === "object" && v !== null) {
        const obj = v as Record<string, unknown>;
        if (renameKey(obj, "adapter", "target")) touched = true;
        if (renameKey(obj, "proposed-adapter", "proposed-target")) {
          touched = true;
        }
      }
    }
  }

  const after = writeYamlText(doc);
  if (after !== before) {
    log.change(path, touched ? "adapter -> target on projects" : "normalised");
    if (!opts.dryRun) await Deno.writeTextFile(path, after);
  }
}

// -----------------------------------------------------------------------------
// Transform 4: rewrite plan.yaml (live + archived).

interface ReshapeStats {
  slicesReshaped: number;
  candidatesLifted: number;
  statusesSanitised: number;
  adapterRenamed: number;
}

function reshapePlanSlices(
  plan: YamlValue,
  stats: ReshapeStats,
): void {
  const slices = plan["slices"];
  if (!Array.isArray(slices)) return;
  for (const s of slices) {
    if (typeof s !== "object" || s === null) continue;
    const slice = s as Record<string, unknown>;

    if (renameKey(slice, "adapter", "target")) stats.adapterRenamed++;

    // Lift standalone candidate.
    const standalone = slice["candidate"];
    const sliceName = typeof slice["name"] === "string"
      ? (slice["name"] as string)
      : undefined;
    const candidateForBare = typeof standalone === "string"
      ? (standalone as string)
      : sliceName;

    const srcs = slice["sources"];
    if (Array.isArray(srcs)) {
      let touched = false;
      const next: Array<Record<string, unknown>> = [];
      for (const item of srcs) {
        if (typeof item === "string") {
          if (!candidateForBare) {
            throw new Error(
              `plan.yaml slice "${sliceName ?? "<unnamed>"}" has bare-string sources but no candidate to lift`,
            );
          }
          next.push({ key: item, candidate: candidateForBare });
          touched = true;
        } else if (typeof item === "object" && item !== null) {
          const obj = item as Record<string, unknown>;
          if (typeof obj["key"] === "string" && "candidate" in obj) {
            next.push(obj);
          } else if (
            typeof obj["key"] === "string" && candidateForBare !== undefined
          ) {
            next.push({ key: obj["key"], candidate: candidateForBare });
            touched = true;
          } else {
            next.push(obj);
          }
        }
      }
      if (touched) {
        slice["sources"] = next;
        stats.slicesReshaped++;
      }
    }

    if ("candidate" in slice) {
      delete slice["candidate"];
      stats.candidatesLifted++;
    }

    // Per-entry status sanitisation.
    const status = slice["status"];
    if (status === "blocked" || status === "failed" || status === "skipped") {
      slice["status"] = "pending";
      stats.statusesSanitised++;
    }
  }
}

function adjustPlanLifecycle(plan: YamlValue, archive: boolean): string | null {
  const slices = plan["slices"];
  const allDone = Array.isArray(slices) && slices.length > 0 &&
    slices.every((s) =>
      typeof s === "object" && s !== null &&
      (s as Record<string, unknown>)["status"] === "done"
    );
  const anyTouched = Array.isArray(slices) && slices.some((s) =>
    typeof s === "object" && s !== null &&
    ((s as Record<string, unknown>)["status"] === "in-progress" ||
      (s as Record<string, unknown>)["status"] === "done")
  );

  const before = plan["lifecycle"];
  let next: string;
  if (archive) {
    // Archive lifecycle stays as-was (historical record), but the value
    // must be one of the legal 2.0 names. Collapse in-progress/drained.
    if (before === "drained" || before === "in-progress") next = "reviewed";
    else if (typeof before === "string") next = before as string;
    else next = "reviewed";
  } else {
    if (before === "drained") next = "reviewed";
    else if (before === "in-progress") next = "reviewed";
    else if (before === "reviewed") next = "reviewed";
    else if (before === "pending") next = "pending";
    else if (allDone) next = "reviewed";
    else if (anyTouched) next = "reviewed";
    else next = "pending";
  }
  if (before === next) return null;
  plan["lifecycle"] = next;
  return `lifecycle: ${typeof before === "string" ? before : "<unset>"} -> ${next}`;
}

async function migratePlanFile(
  path: string,
  opts: Options,
  log: Logger,
  archive: boolean,
): Promise<void> {
  const before = await Deno.readTextFile(path);
  const doc = (parseYaml(before) ?? {}) as YamlValue;
  const stats: ReshapeStats = {
    slicesReshaped: 0,
    candidatesLifted: 0,
    statusesSanitised: 0,
    adapterRenamed: 0,
  };
  reshapePlanSlices(doc, stats);
  const lifecycleNote = adjustPlanLifecycle(doc, archive);

  const after = writeYamlText(doc);
  if (after === before) return;

  const notes: string[] = [];
  if (stats.adapterRenamed) {
    notes.push(`adapter -> target on ${stats.adapterRenamed} slice(s)`);
  }
  if (stats.slicesReshaped) {
    notes.push(`reshaped ${stats.slicesReshaped} slice sources binding(s)`);
  }
  if (stats.candidatesLifted) {
    notes.push(`lifted ${stats.candidatesLifted} standalone candidate(s)`);
  }
  if (stats.statusesSanitised) {
    notes.push(`sanitised ${stats.statusesSanitised} status value(s)`);
  }
  if (lifecycleNote) notes.push(lifecycleNote);
  log.change(path, notes.join("; ") || "normalised");
  if (!opts.dryRun) await Deno.writeTextFile(path, after);
}

async function migratePlanYamls(opts: Options, log: Logger): Promise<void> {
  const live = join(opts.projectRoot, ".specify", "plan.yaml");
  if (await exists(live)) await migratePlanFile(live, opts, log, false);

  const archiveDir = join(opts.projectRoot, ".specify", "archive", "plans");
  if (await isDir(archiveDir)) {
    for await (
      const entry of walk(archiveDir, {
        includeDirs: false,
        match: [/plan\.yaml$/],
      })
    ) {
      await migratePlanFile(entry.path, opts, log, true);
    }
  }
}

// -----------------------------------------------------------------------------
// Transform 5: rewrite sources.yaml.

async function migrateSourcesYaml(opts: Options, log: Logger): Promise<void> {
  const path = join(opts.projectRoot, ".specify", "sources.yaml");
  if (!(await exists(path))) return;
  const before = await Deno.readTextFile(path);
  const doc = (parseYaml(before) ?? {}) as YamlValue;

  const sources = doc["sources"];
  let touched = false;
  if (typeof sources === "object" && sources !== null) {
    for (const v of Object.values(sources as Record<string, unknown>)) {
      if (typeof v !== "object" || v === null) continue;
      const obj = v as Record<string, unknown>;
      if (renameKey(obj, "value", "path")) touched = true;
    }
  }
  const after = writeYamlText(doc);
  if (after !== before) {
    log.change(path, touched ? "value -> path on sources" : "normalised");
    if (!opts.dryRun) await Deno.writeTextFile(path, after);
  }
}

// -----------------------------------------------------------------------------
// Transform 6: cache layout migration.

async function migrateCacheLayout(opts: Options, log: Logger): Promise<void> {
  const oldCache = join(opts.projectRoot, ".specify", ".cache", "adapters");
  const newCache = join(opts.projectRoot, ".specify", ".cache", "targets");
  if (!(await isDir(oldCache))) return;
  if (await isDir(newCache)) {
    log.warn(
      `both .specify/.cache/adapters/ and .cache/targets/ exist; leaving adapters/ in place for manual review`,
    );
    return;
  }
  log.change(oldCache, `rename to .specify/.cache/targets/`);
  if (!opts.dryRun) await Deno.rename(oldCache, newCache);
}

// -----------------------------------------------------------------------------
// Transform 7: retire baseline layout.yaml under specs/.

async function migrateLayoutYaml(opts: Options, log: Logger): Promise<void> {
  const specsDirs = [
    join(opts.projectRoot, ".specify", "specs"),
    join(opts.projectRoot, ".specify", "slices"),
  ];
  for (const root of specsDirs) {
    if (!(await isDir(root))) continue;
    for await (
      const entry of walk(root, {
        includeDirs: false,
        match: [/(^|\/)layout\.yaml$/],
      })
    ) {
      const target = `${entry.path.replace(/layout\.yaml$/, "")}.layout.yaml.deprecated`;
      if (await exists(target)) {
        log.info(
          `layout.yaml already retired alongside ${relative(opts.projectRoot, target)}`,
        );
        continue;
      }
      log.change(
        entry.path,
        "rename to .layout.yaml.deprecated (retired; re-emit via screenshots.extract)",
      );
      if (!opts.dryRun) await Deno.rename(entry.path, target);
    }
  }
}

// -----------------------------------------------------------------------------
// Transform 8: warn on composition.yaml.

async function warnCompositionYaml(opts: Options, log: Logger): Promise<void> {
  const root = join(opts.projectRoot, ".specify", "specs");
  if (!(await isDir(root))) return;
  for await (
    const entry of walk(root, {
      includeDirs: false,
      match: [/(^|\/)composition\.yaml$/],
    })
  ) {
    log.warn(
      `${relative(opts.projectRoot, entry.path)} is now a build output regenerated by targets/vectis/build; delete after the first 2.0 /spec:execute (kept for diff)`,
    );
  }
}

// -----------------------------------------------------------------------------
// Transform 9: warn on retired plugin skill if a local plugin clone exists.

async function warnRetiredPluginSkill(
  opts: Options,
  log: Logger,
): Promise<void> {
  const skill = join(
    opts.projectRoot,
    "plugins",
    "vectis",
    "skills",
    "image-layout-inferer",
  );
  if (!(await isDir(skill))) return;
  log.warn(
    `plugins/vectis/skills/image-layout-inferer/ is retired in 2.0; its body lifted into sources/screenshots/. Re-sync your plugin marketplace.`,
  );
}

// -----------------------------------------------------------------------------
// Main.

async function main(): Promise<number> {
  const opts = parseArgs(Deno.args);
  const log = makeLogger(opts);

  log.info(
    `${opts.dryRun ? "[dry-run] " : ""}migrating ${opts.projectRoot} to specify 2.0`,
  );

  // Refuse silently if not a Specify project; this lives inside the first
  // transform (project.yaml) which throws when the marker is missing.
  await migrateProjectYaml(opts, log);
  await migrateAdaptersDir(opts, log);
  await migrateRegistryYaml(opts, log);
  await migratePlanYamls(opts, log);
  await migrateSourcesYaml(opts, log);
  await migrateCacheLayout(opts, log);
  await migrateLayoutYaml(opts, log);
  await warnCompositionYaml(opts, log);
  await warnRetiredPluginSkill(opts, log);

  const { changes, warnings } = log.summary();
  if (changes === 0) {
    log.info(
      `already on specify ${SPECIFY_VERSION}${warnings ? ` (${warnings} warning(s))` : ""}; nothing to do`,
    );
  } else if (opts.dryRun) {
    log.info(`${changes} change(s) pending; re-run without --dry-run to apply`);
  } else {
    log.info(`${changes} change(s) applied; ${warnings} warning(s)`);
  }
  return 0;
}

try {
  Deno.exit(await main());
} catch (e) {
  console.error(`migrate-to-2.0: ${e instanceof Error ? e.message : e}`);
  Deno.exit(1);
}
