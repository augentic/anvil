// Cross-document link consistency:
//   - relative markdown link targets resolve on disk,
//   - SKILL.md `references/` and `examples/` links resolve relative to
//     each skill directory,
//   - `<!-- skill: <plugin>:<skill> -->` directives point at a skill
//     that actually exists in the plugins/ tree.

import {
  dirname,
  fail,
  join,
  REPO_ROOT,
  relative,
  resolve,
  stripHtmlComments,
  underSymlink,
  walk,
} from "./_shared.ts";

export async function checkMarkdownLinks(): Promise<void> {
  // `rfcs/archive/` keeps the as-shipped prose of historical RFCs intact;
  // those documents intentionally retain paths that have since been
  // renamed or retired and are not part of the live workflow surface.
  // The same applies to non-archived historical RFCs (everything except
  // the current `rfc-25-*` workflow contract).
  const SKIP_DIRS = [
    /node_modules/,
    /\.git/,
    /temp/,
    /rfcs\/archive/,
    /rfcs\/rfc-(?!25)/,
  ];
  const LINK_RE = /\[[^\]]*\]\(([^)]+)\)/g;
  const FENCE_RE = /```[\s\S]*?```/g;
  const INLINE_CODE_RE = /`[^`]+`/g;

  for await (
    const entry of walk(REPO_ROOT, {
      exts: [".md"],
      includeDirs: false,
    })
  ) {
    if (SKIP_DIRS.some((re) => re.test(entry.path))) continue;
    if (await underSymlink(entry.path)) continue;

    const relFile = relative(REPO_ROOT, entry.path);
    const parent = dirname(entry.path);
    let content: string;
    try {
      content = await Deno.readTextFile(entry.path);
    } catch {
      continue;
    }

    const stripped = stripHtmlComments(content.replace(FENCE_RE, ""))
      .replace(INLINE_CODE_RE, "");

    for (const m of stripped.matchAll(LINK_RE)) {
      const target = m[1];
      if (/^(https?:\/\/|mailto:|#)/.test(target)) continue;
      const path = target.split("#")[0];
      if (!path) continue;
      if (path.startsWith("src/")) continue;
      const resolved = resolve(parent, path);
      try {
        await Deno.stat(resolved);
      } catch {
        fail(`Broken link in ${relFile}: ${target}`);
      }
    }
  }
}

export async function checkReferences(): Promise<void> {
  const REF_LINK_RE = /\[([^\]]*)\]\((references\/[^)]+|examples\/[^)]+)\)/g;
  const FENCE_RE = /```[\s\S]*?```/g;

  const PLUGINS_DIR = join(REPO_ROOT, "plugins");

  for await (
    const entry of walk(PLUGINS_DIR, {
      match: [/SKILL\.md$/],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    const skillDir = dirname(entry.path);
    const content = await Deno.readTextFile(entry.path);

    const stripped = content.replace(FENCE_RE, "");

    for (const m of stripped.matchAll(REF_LINK_RE)) {
      const refPath = m[2].split("#")[0];
      if (!refPath) continue;
      const resolved = resolve(skillDir, refPath);
      try {
        await Deno.stat(resolved);
      } catch {
        fail(
          `Skill reference missing: ${rel} links to '${refPath}' but it doesn't exist`,
        );
      }
    }
  }
}

export async function checkDirectives(): Promise<void> {
  const DIRECTIVE_RE = /<!-- skill: ([a-z][a-z0-9-]*):([a-z][a-z0-9-]*) -->/g;
  const FENCE_RE = /```[\s\S]*?```/g;
  const INLINE_CODE_RE = /`[^`]+`/g;

  const PLUGINS_DIR = join(REPO_ROOT, "plugins");

  const registry = new Map<string, Set<string>>();
  for await (
    const entry of walk(PLUGINS_DIR, {
      match: [/SKILL\.md$/],
      includeDirs: false,
    })
  ) {
    const parts = relative(PLUGINS_DIR, entry.path).split("/");
    if (parts.length >= 4 && parts[1] === "skills") {
      const plugin = parts[0];
      const skill = parts[2];
      if (!registry.has(plugin)) registry.set(plugin, new Set());
      registry.get(plugin)!.add(skill);
    }
  }

  const SKIP_DIRS = [/node_modules/, /\.git/, /temp/, /rfcs/];

  for await (
    const entry of walk(REPO_ROOT, {
      exts: [".md"],
      includeDirs: false,
    })
  ) {
    if (SKIP_DIRS.some((re) => re.test(entry.path))) continue;
    if (await underSymlink(entry.path)) continue;

    let content: string;
    try {
      content = await Deno.readTextFile(entry.path);
    } catch {
      continue;
    }
    const rel = relative(REPO_ROOT, entry.path);

    const stripped = content.replace(FENCE_RE, "").replace(INLINE_CODE_RE, "");

    for (const m of stripped.matchAll(DIRECTIVE_RE)) {
      const [, plugin, skill] = m;
      if (!registry.has(plugin)) {
        fail(`Invalid skill directive: ${rel} — plugin '${plugin}' not found`);
      } else if (!registry.get(plugin)!.has(skill)) {
        fail(
          `Invalid skill directive: ${rel} — skill '${plugin}:${skill}' not found`,
        );
      }
    }
  }
}
