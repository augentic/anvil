#!/usr/bin/env -S deno run --allow-read --allow-write
/**
 * Deterministic spec merge tool for Specify archive workflow.
 *
 * Parses baseline and delta spec files using hard-coded heading conventions,
 * applies RENAMED -> REMOVED -> MODIFIED -> ADDED in strict order, and writes
 * the merged result.
 *
 * Exit codes:
 *   0  merge succeeded (or --validate passed)
 *   1  merge failed due to errors (missing IDs, duplicates, structure issues)
 *
 * Usage:
 *   merge-specs.ts --baseline baseline.md --delta delta.md [--output out.md]
 *   merge-specs.ts --validate merged.md [--design design.md]
 */

import { parseArgs } from "jsr:@std/cli@1/parse-args";

// ---------------------------------------------------------------------------
// Hard-coded spec format (see plugins/spec/references/spec-format.md)
// ---------------------------------------------------------------------------

export interface SpecFormat {
  requirementHeading: string;
  requirementIdPrefix: string;
  requirementIdPattern: string;
  scenarioHeading: string;
  deltaAdded: string;
  deltaModified: string;
  deltaRemoved: string;
  deltaRenamed: string;
}

export const SPEC_FORMAT: SpecFormat = {
  requirementHeading: "### Requirement:",
  requirementIdPrefix: "ID:",
  requirementIdPattern: "^REQ-[0-9]{3}$",
  scenarioHeading: "#### Scenario:",
  deltaAdded: "## ADDED Requirements",
  deltaModified: "## MODIFIED Requirements",
  deltaRemoved: "## REMOVED Requirements",
  deltaRenamed: "## RENAMED Requirements",
};

// ---------------------------------------------------------------------------
// Requirement block parser
// ---------------------------------------------------------------------------

export interface ReqBlock {
  heading: string;
  name: string;
  reqId: string;
  body: string;
}

export function parseRequirementBlocks(
  text: string,
  fmt: SpecFormat,
): [string, ReqBlock[]] {
  const lines = text.split("\n");
  const headingPrefix = fmt.requirementHeading;
  const idPrefix = fmt.requirementIdPrefix;

  const blocks: ReqBlock[] = [];
  const preambleLines: string[] = [];
  let currentLines: string[] = [];
  let currentName: string | null = null;
  let currentId: string | null = null;
  let inPreamble = true;

  function flushBlock(): void {
    if (currentName !== null) {
      const body = currentLines.join("\n");
      blocks.push({
        heading: currentLines[0] ?? "",
        name: currentName,
        reqId: currentId ?? "",
        body,
      });
    }
    currentLines = [];
    currentName = null;
    currentId = null;
  }

  for (const line of lines) {
    const stripped = line.trim();

    if (stripped.startsWith(headingPrefix)) {
      if (inPreamble) {
        inPreamble = false;
      } else {
        flushBlock();
      }
      currentName = stripped.slice(headingPrefix.length).trim();
      currentLines = [line];
      continue;
    }

    if (!inPreamble && currentName !== null && currentId === null) {
      if (stripped.startsWith(idPrefix)) {
        currentId = stripped.slice(idPrefix.length).trim();
      }
    }

    if (inPreamble) {
      if (stripped.startsWith("## ") && !stripped.startsWith(headingPrefix)) {
        inPreamble = false;
        flushBlock();
        currentLines = [line];
        currentName = null;
      } else {
        preambleLines.push(line);
      }
    } else {
      if (currentName === null && stripped.startsWith("## ")) {
        // skip stray ## headers between blocks
      }
      currentLines.push(line);
    }
  }

  flushBlock();

  const preamble = preambleLines.join("\n");
  return [preamble, blocks];
}

// ---------------------------------------------------------------------------
// Delta spec parser
// ---------------------------------------------------------------------------

export interface RenameEntry {
  reqId: string;
  newName: string;
}

export function parseDeltaSections(
  text: string,
  fmt: SpecFormat,
): [RenameEntry[], ReqBlock[], ReqBlock[], ReqBlock[]] {
  const opHeadings: Record<string, string> = {
    [fmt.deltaRenamed]: "renamed",
    [fmt.deltaRemoved]: "removed",
    [fmt.deltaModified]: "modified",
    [fmt.deltaAdded]: "added",
  };

  const lines = text.split("\n");
  const sections: Record<string, string[]> = {
    renamed: [],
    removed: [],
    modified: [],
    added: [],
  };
  let currentSection: string | null = null;

  for (const line of lines) {
    const stripped = line.trim();
    let matchedSection: string | null = null;
    for (const [heading, sectionName] of Object.entries(opHeadings)) {
      if (stripped.toLowerCase() === heading.toLowerCase()) {
        matchedSection = sectionName;
        break;
      }
    }
    if (matchedSection !== null) {
      currentSection = matchedSection;
      continue;
    }
    if (currentSection !== null) {
      sections[currentSection].push(line);
    }
  }

  // Parse RENAMED section -- looks for ID: and TO: lines
  const renamed: RenameEntry[] = [];
  const idPrefix = fmt.requirementIdPrefix;
  let renameId: string | null = null;
  for (const line of sections.renamed) {
    const stripped = line.trim();
    if (stripped.startsWith(idPrefix)) {
      renameId = stripped.slice(idPrefix.length).trim();
    } else if (stripped.toUpperCase().startsWith("TO:") && renameId) {
      const newName = stripped.slice(3).trim();
      renamed.push({ reqId: renameId, newName });
      renameId = null;
    }
  }

  const [, removed] = parseRequirementBlocks(
    sections.removed.join("\n"),
    fmt,
  );
  const [, modified] = parseRequirementBlocks(
    sections.modified.join("\n"),
    fmt,
  );
  const [, added] = parseRequirementBlocks(sections.added.join("\n"), fmt);

  return [renamed, removed, modified, added];
}

// ---------------------------------------------------------------------------
// Merge algorithm
// ---------------------------------------------------------------------------

export function merge(
  baselineText: string,
  deltaText: string,
  fmt: SpecFormat,
  errors: string[],
): string {
  const isNew = !baselineText.trim();

  const [renamed, removed, modified, added] = parseDeltaSections(
    deltaText,
    fmt,
  );

  if (isNew) {
    const hasDeltaHeaders = [
      fmt.deltaAdded,
      fmt.deltaModified,
      fmt.deltaRemoved,
      fmt.deltaRenamed,
    ].some((h) => deltaText.toLowerCase().includes(h.toLowerCase()));

    if (!hasDeltaHeaders) {
      return deltaText;
    }

    const resultBlocks: string[] = [];
    for (const block of added) {
      resultBlocks.push(block.body);
    }
    return resultBlocks.length > 0
      ? resultBlocks.join("\n\n") + "\n"
      : "";
  }

  const [preamble, blocks] = parseRequirementBlocks(baselineText, fmt);
  const blocksById = new Map<string, number>();
  for (let i = 0; i < blocks.length; i++) {
    if (blocks[i].reqId) {
      blocksById.set(blocks[i].reqId, i);
    }
  }

  // Step 1: RENAMED
  for (const entry of renamed) {
    const idx = blocksById.get(entry.reqId);
    if (idx === undefined) {
      errors.push(`RENAMED: ID ${entry.reqId} not found in baseline`);
      continue;
    }
    const oldBlock = blocks[idx];
    const newHeading = `${fmt.requirementHeading} ${entry.newName}`;
    const newBody = oldBlock.body.replace(oldBlock.heading, newHeading);
    blocks[idx] = {
      heading: newHeading,
      name: entry.newName,
      reqId: oldBlock.reqId,
      body: newBody,
    };
  }

  // Step 2: REMOVED
  const idsToRemove = new Set<string>();
  for (const block of removed) {
    if (!blocksById.has(block.reqId)) {
      errors.push(`REMOVED: ID ${block.reqId} not found in baseline`);
    } else {
      idsToRemove.add(block.reqId);
    }
  }

  // Step 3: MODIFIED
  for (const modBlock of modified) {
    const idx = blocksById.get(modBlock.reqId);
    if (idx === undefined) {
      errors.push(`MODIFIED: ID ${modBlock.reqId} not found in baseline`);
      continue;
    }
    blocks[idx] = modBlock;
  }

  // Step 4: ADDED
  const existingIds = new Set(
    [...blocksById.keys()].filter((id) => !idsToRemove.has(id)),
  );
  for (const addBlock of added) {
    if (existingIds.has(addBlock.reqId)) {
      errors.push(
        `ADDED: ID ${addBlock.reqId} already exists in baseline`,
      );
      continue;
    }
    blocks.push(addBlock);
    existingIds.add(addBlock.reqId);
  }

  // Build result: preamble + surviving blocks
  const surviving = blocks.filter((b) => !idsToRemove.has(b.reqId));
  const parts: string[] = [];
  if (preamble.trim()) {
    parts.push(preamble.trimEnd());
  }
  for (const block of surviving) {
    parts.push(block.body.trim());
  }

  return parts.join("\n\n") + "\n";
}

// ---------------------------------------------------------------------------
// Validation (post-merge coherence checks)
// ---------------------------------------------------------------------------

export function validateBaseline(
  text: string,
  fmt: SpecFormat,
  designText?: string,
): string[] {
  const errors: string[] = [];
  const [, blocks] = parseRequirementBlocks(text, fmt);

  // (a) No duplicate requirement IDs
  const seenIds = new Map<string, number>();
  for (const block of blocks) {
    if (!block.reqId) continue;
    if (seenIds.has(block.reqId)) {
      errors.push(`Duplicate ID: ${block.reqId}`);
    }
    seenIds.set(block.reqId, 1);
  }

  // (b) No duplicate requirement names
  const seenNames = new Map<string, number>();
  for (const block of blocks) {
    if (seenNames.has(block.name)) {
      errors.push(`Duplicate requirement name: ${block.name}`);
    }
    seenNames.set(block.name, 1);
  }

  // (c) Heading structure valid
  const idPattern = new RegExp(fmt.requirementIdPattern);
  for (const block of blocks) {
    if (!block.reqId) {
      errors.push(
        `Requirement '${block.name}' has no ${fmt.requirementIdPrefix} line`,
      );
    } else if (!idPattern.test(block.reqId)) {
      errors.push(
        `Requirement '${block.name}' has invalid ID '${block.reqId}' ` +
          `(expected pattern: ${fmt.requirementIdPattern})`,
      );
    }
    if (!block.body.includes(fmt.scenarioHeading.replace(/:$/, ""))) {
      errors.push(
        `Requirement '${block.name}' (${block.reqId}) has no ` +
          `${fmt.scenarioHeading} section`,
      );
    }
  }

  // (d) No orphaned design references
  // NOTE: Uses the anchored pattern as-is. Because the pattern has ^ and $,
  // finditer/matchAll only matches whole-line occurrences with the "m" flag.
  if (designText) {
    const refPattern = new RegExp(fmt.requirementIdPattern, "gm");
    const baselineIds = new Set(seenIds.keys());
    for (const match of designText.matchAll(refPattern)) {
      const refId = match[0];
      if (!baselineIds.has(refId)) {
        errors.push(
          `Design references ${refId} which does not exist in baseline`,
        );
      }
    }
  }

  return errors;
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

function die(msg: string): never {
  console.error(`ERROR: ${msg}`);
  Deno.exit(1);
}

function fileExists(path: string): boolean {
  try {
    const info = Deno.statSync(path);
    return info.isFile;
  } catch {
    return false;
  }
}

if (import.meta.main) {
  const args = parseArgs(Deno.args, {
    string: ["baseline", "delta", "validate", "output", "design", "o"],
    alias: { o: "output" },
  });

  if (!args.delta && !args.validate) {
    die("Either --delta or --validate is required");
  }
  if (args.delta && args.validate) {
    die("--delta and --validate are mutually exclusive");
  }

  const fmt = SPEC_FORMAT;

  // --- Validate mode ---
  if (args.validate) {
    if (!fileExists(args.validate)) {
      die(`File not found: ${args.validate}`);
    }
    const text = Deno.readTextFileSync(args.validate);
    let designText: string | undefined;
    if (args.design) {
      if (!fileExists(args.design)) {
        die(`Design file not found: ${args.design}`);
      }
      designText = Deno.readTextFileSync(args.design);
    }

    const errs = validateBaseline(text, fmt, designText);
    if (errs.length > 0) {
      for (const e of errs) {
        console.error(`FAIL: ${e}`);
      }
      Deno.exit(1);
    } else {
      console.log("All coherence checks passed.");
      Deno.exit(0);
    }
  }

  // --- Merge mode ---
  if (!args.delta) {
    die("--delta is required in merge mode");
  }

  if (!fileExists(args.delta)) {
    die(`Delta file not found: ${args.delta}`);
  }

  let baselineText = "";
  if (args.baseline && fileExists(args.baseline)) {
    baselineText = Deno.readTextFileSync(args.baseline);
  }

  const deltaText = Deno.readTextFileSync(args.delta);

  const errors: string[] = [];
  const result = merge(baselineText, deltaText, fmt, errors);

  if (errors.length > 0) {
    for (const e of errors) {
      console.error(`ERROR: ${e}`);
    }
    Deno.exit(1);
  }

  if (args.output) {
    Deno.writeTextFileSync(args.output, result);
  } else {
    Deno.stdout.writeSync(new TextEncoder().encode(result));
  }
}
