// First-party tool surface enforcement (RM-09):
//   - first-party WASM tools declared in adapter tools.yaml are
//     exact scalar wasm-pkg package requests matching the release version,
//   - retired host helpers (specify-vectis, specify-contract, …) must
//     be re-routed through their declared-tool equivalent in active
//     briefs and skill bodies.

import {
  TARGETS_DIR,
  fail,
  join,
  parseYaml,
  relative,
  REPO_ROOT,
  underSymlink,
  walk,
} from "./_shared.ts";

interface ToolManifest {
  tools?: unknown[];
}

interface ExpectedToolDeclaration {
  adapter: string;
  name: string;
  package: string;
}

const VERSION_RE = /^(\d+\.\d+\.\d+)$/;

const EXPECTED_FIRST_PARTY_TOOLS: ExpectedToolDeclaration[] = [
  {
    adapter: "contracts",
    name: "contract",
    package: "specify:contract@0.3.0",
  },
  {
    adapter: "vectis",
    name: "vectis",
    package: "specify:vectis@0.3.0",
  },
];

// Resolve the per-target tool declaration map keyed by tool name and yielding
// the canonical `specify:<name>@<version>` package request for comparison
// against the expected first-party list.
//
// First-party tools are declared inline in `targets/<name>/adapter.yaml` under
// `tools[]` ({ name, version } objects validated by `target.schema.json`). The
// 1.x `adapters/<name>/tools.yaml` sidecar shape was retired with RFC-25.
async function resolveAdapterDeclarations(
  adapter: string,
): Promise<{ rel: string; declarations: Map<string, string> } | null> {
  const targetManifestPath = join(TARGETS_DIR, adapter, "adapter.yaml");
  let stat: Deno.FileInfo;
  try {
    stat = await Deno.stat(targetManifestPath);
  } catch {
    return null;
  }
  if (!stat.isFile) return null;

  const rel = relative(REPO_ROOT, targetManifestPath);
  const manifest = parseYaml(
    await Deno.readTextFile(targetManifestPath),
  ) as ToolManifest;
  const tools = Array.isArray(manifest.tools) ? manifest.tools : [];
  const declarations = new Map<string, string>();
  for (const tool of tools) {
    if (typeof tool !== "object" || tool === null) {
      fail(
        `First-party tool declaration: ${rel} — \`tools[]\` entries must be { name, version } objects under target.schema.json`,
      );
      continue;
    }
    const entry = tool as Record<string, unknown>;
    const name = entry.name;
    const version = entry.version;
    if (typeof name !== "string" || typeof version !== "string") {
      fail(
        `First-party tool declaration: ${rel} — tool object must carry string \`name\` and \`version\` fields`,
      );
      continue;
    }
    if (!VERSION_RE.test(version)) {
      fail(
        `First-party tool declaration: ${rel} — tool '${name}' version '${version}' must be \`<major>.<minor>.<patch>\` without prerelease metadata`,
      );
      continue;
    }
    declarations.set(name, `specify:${name}@${version}`);
  }
  return { rel, declarations };
}

export async function checkFirstPartyToolDeclarations(): Promise<void> {
  const cache = new Map<
    string,
    { rel: string; declarations: Map<string, string> } | null
  >();

  for (const expected of EXPECTED_FIRST_PARTY_TOOLS) {
    let resolved = cache.get(expected.adapter);
    if (resolved === undefined) {
      resolved = await resolveAdapterDeclarations(expected.adapter);
      cache.set(expected.adapter, resolved);
    }
    if (!resolved) continue;

    const packageRequest = resolved.declarations.get(expected.name);
    if (!packageRequest) {
      fail(
        `First-party tool declaration: ${resolved.rel} — missing tool '${expected.name}'`,
      );
      continue;
    }

    if (packageRequest !== expected.package) {
      fail(
        `First-party tool declaration: ${resolved.rel} — '${expected.name}' package must be '${expected.package}'`,
      );
    }
  }
}

interface RetiredHelperPattern {
  token: string;
  pattern: RegExp;
  replacement: string;
}

const DECLARED_TOOL_EQUIVALENT_RULE =
  "skill.invokes-host-binary-with-declared-tool-equivalent";

const RETIRED_HELPER_PATTERNS: RetiredHelperPattern[] = [
  {
    token: "specify-contract-validate",
    pattern: /\bspecify-contract-validate\b/,
    replacement: "specify tool run contract -- <BASELINE_DIR> --format json",
  },
  {
    token: "specify-contract",
    pattern: /\bspecify-contract\b(?!-validate)/,
    replacement: "specify tool run contract -- <BASELINE_DIR> --format json",
  },
  {
    token: "specify-vectis validate",
    pattern: /\bspecify-vectis\s+validate\b/,
    replacement: "specify tool run vectis -- validate <mode> [path]",
  },
  {
    token: "specify vectis validate",
    pattern: /\bspecify\s+vectis\s+validate\b/,
    replacement: "specify tool run vectis -- validate <mode> [path]",
  },
  {
    token: "specify-vectis init",
    pattern: /\bspecify-vectis\s+init\b/,
    replacement: "specify tool run vectis -- scaffold core <app-name>",
  },
  {
    token: "specify vectis init",
    pattern: /\bspecify\s+vectis\s+init\b/,
    replacement: "specify tool run vectis -- scaffold core <app-name>",
  },
  {
    token: "specify-vectis add-shell",
    pattern: /\bspecify-vectis\s+add-shell\b/,
    replacement: "specify tool run vectis -- scaffold ios|android <app-name>",
  },
  {
    token: "specify vectis add-shell",
    pattern: /\bspecify\s+vectis\s+add-shell\b/,
    replacement: "specify tool run vectis -- scaffold ios|android <app-name>",
  },
];

async function activeBriefAndSkillFiles(): Promise<string[]> {
  const files: string[] = [];

  try {
    const stat = await Deno.stat(TARGETS_DIR);
    if (stat.isDirectory) {
      for await (
        const entry of walk(TARGETS_DIR, {
          exts: [".md"],
          includeDirs: false,
        })
      ) {
        if (await underSymlink(entry.path)) continue;
        const parts = relative(TARGETS_DIR, entry.path).split("/");
        if (parts.length >= 3 && parts[1] === "briefs") files.push(entry.path);
      }
    }
  } catch {
    // Optional root.
  }

  const pluginsDir = join(REPO_ROOT, "plugins");
  for await (
    const entry of walk(pluginsDir, {
      exts: [".md"],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
    const parts = relative(pluginsDir, entry.path).split("/");
    if (parts.length >= 3 && parts[1] === "skills") files.push(entry.path);
  }

  return files.sort();
}

export async function checkDeclaredToolEquivalentInvocations(): Promise<void> {
  for (const path of await activeBriefAndSkillFiles()) {
    const rel = relative(REPO_ROOT, path);
    let content: string;
    try {
      content = await Deno.readTextFile(path);
    } catch {
      continue;
    }
    const lines = content.split("\n");
    for (let i = 0; i < lines.length; i++) {
      for (const helper of RETIRED_HELPER_PATTERNS) {
        if (!helper.pattern.test(lines[i])) continue;
        fail(
          `${DECLARED_TOOL_EQUIVALENT_RULE}: ${rel}:${
            i + 1
          } -- '${helper.token}' has a declared-tool equivalent; use \`${helper.replacement}\``,
        );
      }
    }
  }
}
