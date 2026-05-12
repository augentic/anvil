// First-party tool surface enforcement (RM-09):
//   - first-party WASM tools declared in capability tools.yaml are
//     exact scalar wasm-pkg package requests matching the release version,
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

interface ToolManifest {
  tools?: unknown[];
}

interface ExpectedToolDeclaration {
  capability: string;
  name: string;
  package: string;
}

const PACKAGE_RE = /^specify:([a-z][a-z0-9-]*)@(\d+\.\d+\.\d+)$/;

const EXPECTED_FIRST_PARTY_TOOLS: ExpectedToolDeclaration[] = [
  {
    capability: "contracts",
    name: "contract",
    package: "specify:contract@0.3.0",
  },
  {
    capability: "vectis",
    name: "vectis",
    package: "specify:vectis@0.3.0",
  },
];

export async function checkFirstPartyToolDeclarations(): Promise<void> {
  const declarationsByCapability = new Map<
    string,
    Map<string, string>
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
    const declarations = new Map<string, string>();
    for (const tool of tools) {
      if (typeof tool !== "string") {
        fail(
          `First-party tool declaration: ${rel} — entries must be scalar package requests`,
        );
        continue;
      }
      const match = PACKAGE_RE.exec(tool);
      if (!match) {
        fail(
          `First-party tool declaration: ${rel} — '${tool}' must be an exact specify:*@<semver> package request without prerelease metadata`,
        );
        continue;
      }
      declarations.set(match[1], tool);
    }
    declarationsByCapability.set(
      expected.capability,
      declarations,
    );
  }

  for (const expected of EXPECTED_FIRST_PARTY_TOOLS) {
    const rel = `capabilities/${expected.capability}/tools.yaml`;
    const packageRequest = declarationsByCapability
      .get(expected.capability)
      ?.get(expected.name);
    if (!packageRequest) {
      fail(
        `First-party tool declaration: ${rel} — missing tool '${expected.name}'`,
      );
      continue;
    }

    if (packageRequest !== expected.package) {
      fail(
        `First-party tool declaration: ${rel} — '${expected.name}' package must be '${expected.package}'`,
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
