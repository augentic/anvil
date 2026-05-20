// Adapter manifest validation:
//   - adapter.yaml validates against adapter.schema.json,
//   - pipeline brief paths exist, ids are unique, frontmatter ids
//     match, and the `needs` graph is acyclic,
//   - operator-facing instruction files declare an output location
//     preamble.

import {
  Ajv2020,
  PROFILES_DIR,
  dirname,
  fail,
  join,
  parseYaml,
  relative,
  REPO_ROOT,
  walk,
} from "./_shared.ts";

interface PipelineEntry {
  id: string;
  brief: string;
}

interface AdapterYaml {
  name: string;
  version?: number;
  description?: string;
  pipeline: {
    define: PipelineEntry[];
    build: PipelineEntry[];
    merge: PipelineEntry[];
  };
}

export async function validateAdapterYaml(): Promise<void> {
  const ajv = new Ajv2020({ allErrors: true });

  const adapterSchema = JSON.parse(
    await Deno.readTextFile(join(PROFILES_DIR, "adapter.schema.json")),
  );

  const validate = ajv.compile(adapterSchema);

  for await (
    const entry of walk(PROFILES_DIR, {
      maxDepth: 2,
      includeDirs: false,
      match: [/adapter\.yaml$/],
    })
  ) {
    const rel = relative(REPO_ROOT, entry.path);
    const data = parseYaml(await Deno.readTextFile(entry.path));
    if (!validate(data)) {
      for (const err of validate.errors ?? []) {
        fail(
          `Adapter validation failed: ${rel} — ${err.instancePath} ${err.message}`,
        );
      }
    }
  }
}

async function parseBriefFrontmatter(
  briefPath: string,
): Promise<Record<string, unknown> | null> {
  let content: string;
  try {
    content = await Deno.readTextFile(briefPath);
  } catch {
    return null;
  }
  const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
  if (!fmMatch) return null;
  try {
    return parseYaml(fmMatch[1]) as Record<string, unknown>;
  } catch {
    return null;
  }
}

export async function checkAdapterIntegrity(): Promise<void> {
  for await (
    const entry of walk(PROFILES_DIR, {
      maxDepth: 2,
      includeDirs: false,
      match: [/adapter\.yaml$/],
    })
  ) {
    const dirPath = dirname(entry.path);
    const name = dirPath.split("/").pop()!;
    const manifest = parseYaml(
      await Deno.readTextFile(entry.path),
    ) as AdapterYaml;

    const pipeline = manifest.pipeline;
    if (!pipeline) continue;

    // Post-RFC-13 §3.11 the manifest carries only the slice phases
    // (define, build, merge); planning is owned by the change-draft
    // skill and `pipeline.plan` is rejected by `adapter.schema.json`.
    const allEntries: PipelineEntry[] = [
      ...(pipeline.define ?? []),
      ...(pipeline.build ?? []),
      ...(pipeline.merge ?? []),
    ];

    const ids = new Set<string>();
    for (const pe of allEntries) {
      if (ids.has(pe.id)) {
        fail(
          `Adapter integrity: ${name}/adapter.yaml: duplicate pipeline entry id '${pe.id}'`,
        );
      }
      ids.add(pe.id);
    }

    for (const pe of allEntries) {
      try {
        await Deno.stat(join(dirPath, pe.brief));
      } catch {
        fail(
          `Adapter integrity: ${name}/adapter.yaml: brief not found for '${pe.id}': ${pe.brief}`,
        );
        continue;
      }

      const fm = await parseBriefFrontmatter(join(dirPath, pe.brief));
      if (!fm) {
        fail(
          `Adapter integrity: ${name}/adapter.yaml: brief '${pe.id}' has no valid frontmatter: ${pe.brief}`,
        );
        continue;
      }

      if (fm.id !== pe.id) {
        fail(
          `Adapter integrity: ${name}/adapter.yaml: pipeline id '${pe.id}' does not match brief frontmatter id '${fm.id}'`,
        );
      }

      const needs = fm.needs as string[] | undefined;
      if (needs) {
        for (const dep of needs) {
          if (!ids.has(dep)) {
            fail(
              `Adapter integrity: ${name}/adapter.yaml: brief '${pe.id}' needs undeclared '${dep}'`,
            );
          }
        }
      }
    }

    // Cycle detection via Kahn's algorithm on needs graph
    const inDeg = new Map<string, number>();
    const adj = new Map<string, string[]>();
    for (const id of ids) {
      inDeg.set(id, 0);
      adj.set(id, []);
    }
    for (const pe of allEntries) {
      const fm = await parseBriefFrontmatter(join(dirPath, pe.brief));
      const needs = (fm?.needs as string[] | undefined) ?? [];
      for (const dep of needs) {
        if (ids.has(dep)) {
          adj.get(dep)!.push(pe.id);
          inDeg.set(pe.id, (inDeg.get(pe.id) ?? 0) + 1);
        }
      }
    }
    const queue = [...ids].filter((id) => inDeg.get(id) === 0);
    let visited = 0;
    while (queue.length > 0) {
      const n = queue.shift()!;
      visited++;
      for (const nb of adj.get(n) ?? []) {
        const deg = (inDeg.get(nb) ?? 1) - 1;
        inDeg.set(nb, deg);
        if (deg === 0) queue.push(nb);
      }
    }
    if (visited < ids.size) {
      fail(
        `Adapter integrity: ${name}/adapter.yaml: cycle in brief needs graph`,
      );
    }
  }
}

export async function checkInstructionPreambles(): Promise<void> {
  const OUTPUT_LOCATION_RE =
    /^> \*\*Output location\*\*: `\.specify\/slices\//m;

  for await (
    const entry of walk(PROFILES_DIR, {
      maxDepth: 3,
      includeDirs: false,
      match: [/instructions\/[a-z]+\.md$/],
    })
  ) {
    const rel = relative(REPO_ROOT, entry.path);
    let content: string;
    try {
      content = await Deno.readTextFile(entry.path);
    } catch {
      continue;
    }
    if (!OUTPUT_LOCATION_RE.test(content)) {
      fail(
        `Missing output location preamble: ${rel} — instruction files must declare output location to prevent cross-plugin path contamination`,
      );
    }
  }
}
