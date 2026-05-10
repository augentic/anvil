// First-party tool surface enforcement (RM-09):
//   - first-party WASM tools declared in capability tools.yaml are
//     pinned at the release version with sha256 + permissions matching
//     the expected shape,
//   - retired host helpers (specify-vectis, specify-contract, …) must
//     be re-routed through their declared-tool equivalent in active
//     briefs and skill bodies.

import {
  CAPABILITIES_DIR,
  fail,
  join,
  parseYaml,
  relative,
  REPO_ROOT,
  underSymlink,
  walk,
} from "./_shared.ts";

interface ToolDeclaration {
  name?: unknown;
  version?: unknown;
  source?: unknown;
  sha256?: unknown;
  permissions?: {
    read?: unknown;
    write?: unknown;
  };
}

interface ToolManifest {
  tools?: ToolDeclaration[];
}

interface ExpectedToolDeclaration {
  capability: string;
  name: string;
  source: string;
  read: string[];
  write: string[];
}

const SHA256_RE = /^[a-f0-9]{64}$/;

const EXPECTED_FIRST_PARTY_TOOLS: ExpectedToolDeclaration[] = [
  {
    capability: "contracts",
    name: "contract",
    source:
      "https://github.com/augentic/specify-cli/releases/download/v0.2.0/contract.wasm",
    read: ["$PROJECT_DIR/contracts"],
    write: [],
  },
  {
    capability: "vectis",
    name: "vectis-validate",
    source:
      "https://github.com/augentic/specify-cli/releases/download/v0.2.0/vectis-validate.wasm",
    read: ["$PROJECT_DIR/.specify", "$PROJECT_DIR/design-system"],
    write: [],
  },
  {
    capability: "vectis",
    name: "vectis-scaffold",
    source:
      "https://github.com/augentic/specify-cli/releases/download/v0.2.0/vectis-scaffold.wasm",
    read: ["$PROJECT_DIR", "$CAPABILITY_DIR"],
    write: ["$PROJECT_DIR"],
  },
];

function stringArray(value: unknown): string[] | null {
  if (!Array.isArray(value)) return null;
  if (!value.every((entry) => typeof entry === "string")) return null;
  return value;
}

function sameStringArray(actual: unknown, expected: string[]): boolean {
  const values = stringArray(actual);
  if (!values || values.length !== expected.length) return false;
  return values.every((value, index) => value === expected[index]);
}

export async function checkFirstPartyToolDeclarations(): Promise<void> {
  const declarationsByCapability = new Map<
    string,
    Map<string, ToolDeclaration>
  >();

  for (const expected of EXPECTED_FIRST_PARTY_TOOLS) {
    if (declarationsByCapability.has(expected.capability)) continue;
    const manifestPath = join(
      CAPABILITIES_DIR,
      expected.capability,
      "tools.yaml",
    );
    const rel = relative(REPO_ROOT, manifestPath);
    let manifest: ToolManifest;
    try {
      manifest = parseYaml(
        await Deno.readTextFile(manifestPath),
      ) as ToolManifest;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      fail(`First-party tool declaration: ${rel} — cannot read or parse: ${msg}`);
      continue;
    }

    const tools = Array.isArray(manifest.tools) ? manifest.tools : [];
    declarationsByCapability.set(
      expected.capability,
      new Map(
        tools
          .filter((tool) => tool && typeof tool === "object")
          .map((
            tool,
          ) => [String((tool as ToolDeclaration).name), tool as ToolDeclaration]),
      ),
    );
  }

  for (const expected of EXPECTED_FIRST_PARTY_TOOLS) {
    const rel = `capabilities/${expected.capability}/tools.yaml`;
    const tool = declarationsByCapability
      .get(expected.capability)
      ?.get(expected.name);
    if (!tool) {
      fail(
        `First-party tool declaration: ${rel} — missing tool '${expected.name}'`,
      );
      continue;
    }

    if (tool.version !== "0.2.0") {
      fail(
        `First-party tool declaration: ${rel} — '${expected.name}' must use release version 0.2.0`,
      );
    }
    if (tool.source !== expected.source) {
      fail(
        `First-party tool declaration: ${rel} — '${expected.name}' source must be '${expected.source}'`,
      );
    }
    if (typeof tool.sha256 !== "string" || !SHA256_RE.test(tool.sha256)) {
      fail(
        `First-party tool declaration: ${rel} — '${expected.name}' must include a lowercase SHA-256 pin`,
      );
    }
    if (!sameStringArray(tool.permissions?.read, expected.read)) {
      fail(
        `First-party tool declaration: ${rel} — '${expected.name}' read permissions must be ${
          JSON.stringify(expected.read)
        }`,
      );
    }
    if (!sameStringArray(tool.permissions?.write, expected.write)) {
      fail(
        `First-party tool declaration: ${rel} — '${expected.name}' write permissions must be ${
          JSON.stringify(expected.write)
        }`,
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
    replacement: "specify tool run vectis-validate -- <mode> [path]",
  },
  {
    token: "specify vectis validate",
    pattern: /\bspecify\s+vectis\s+validate\b/,
    replacement: "specify tool run vectis-validate -- <mode> [path]",
  },
  {
    token: "specify-vectis init",
    pattern: /\bspecify-vectis\s+init\b/,
    replacement: "specify tool run vectis-scaffold -- core <app-name>",
  },
  {
    token: "specify vectis init",
    pattern: /\bspecify\s+vectis\s+init\b/,
    replacement: "specify tool run vectis-scaffold -- core <app-name>",
  },
  {
    token: "specify-vectis add-shell",
    pattern: /\bspecify-vectis\s+add-shell\b/,
    replacement: "specify tool run vectis-scaffold -- ios|android <app-name>",
  },
  {
    token: "specify vectis add-shell",
    pattern: /\bspecify\s+vectis\s+add-shell\b/,
    replacement: "specify tool run vectis-scaffold -- ios|android <app-name>",
  },
];

async function activeBriefAndSkillFiles(): Promise<string[]> {
  const files: string[] = [];

  for await (
    const entry of walk(CAPABILITIES_DIR, {
      exts: [".md"],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
    const parts = relative(CAPABILITIES_DIR, entry.path).split("/");
    if (parts.length >= 3 && parts[1] === "briefs") files.push(entry.path);
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
