// Plugin layout sanity:
//   - every symlink under plugins/ resolves,
//   - every plugin directory listed in `.cursor-plugin/marketplace.json`
//     has the expected `skills/` and `.cursor-plugin/plugin.json`
//     surface, and conversely every plugin with a manifest is declared
//     in the marketplace.

import {
  fail,
  join,
  relative,
  REPO_ROOT,
  walk,
} from "./_shared.ts";

export async function checkSymlinks(): Promise<void> {
  const PLUGINS_DIR = join(REPO_ROOT, "plugins");

  for await (
    const entry of walk(PLUGINS_DIR, {
      includeDirs: true,
      includeFiles: true,
    })
  ) {
    let info: Deno.FileInfo;
    try {
      info = await Deno.lstat(entry.path);
    } catch {
      continue;
    }
    if (!info.isSymlink) continue;

    try {
      await Deno.stat(entry.path);
    } catch {
      fail(`Broken symlink: ${relative(REPO_ROOT, entry.path)}`);
    }
  }
}

export async function checkPluginConsistency(): Promise<void> {
  const manifestPath = join(REPO_ROOT, ".cursor-plugin", "marketplace.json");
  let manifest: {
    plugins: { name: string; source: string }[];
  };
  try {
    manifest = JSON.parse(await Deno.readTextFile(manifestPath));
  } catch {
    fail("Cannot read .cursor-plugin/marketplace.json");
    return;
  }

  const declaredSources = new Set(manifest.plugins.map((p) => p.source));

  const PLUGINS_DIR = join(REPO_ROOT, "plugins");
  for await (
    const entry of walk(PLUGINS_DIR, {
      maxDepth: 3,
      match: [/plugin\.json$/],
      includeDirs: false,
    })
  ) {
    const relParts = relative(PLUGINS_DIR, entry.path).split("/");
    if (
      relParts.length === 3 &&
      relParts[1] === ".cursor-plugin" &&
      relParts[2] === "plugin.json"
    ) {
      const pluginDir = relParts[0];
      if (!declaredSources.has(pluginDir)) {
        fail(
          `Plugin '${pluginDir}' has .cursor-plugin/plugin.json but is not in marketplace.json`,
        );
      }
    }
  }

  for (const p of manifest.plugins) {
    const pluginDir = join(PLUGINS_DIR, p.source);
    const skillsDir = join(pluginDir, "skills");
    let hasSkillsDir = false;
    try {
      const stat = await Deno.stat(skillsDir);
      if (!stat.isDirectory) {
        fail(`Plugin '${p.name}' has no skills/ directory`);
      } else {
        hasSkillsDir = true;
      }
    } catch {
      fail(
        `Plugin '${p.name}' declared in marketplace.json but skills/ not found`,
      );
    }

    if (hasSkillsDir) {
      const pluginManifestPath = join(
        pluginDir,
        ".cursor-plugin",
        "plugin.json",
      );
      try {
        const stat = await Deno.stat(pluginManifestPath);
        if (!stat.isFile) {
          fail(
            `Plugin '${p.name}' has skills/ but .cursor-plugin/plugin.json is not a file`,
          );
        }
      } catch {
        fail(
          `Plugin '${p.name}' has skills/ but .cursor-plugin/plugin.json not found`,
        );
      }
    }
  }
}
