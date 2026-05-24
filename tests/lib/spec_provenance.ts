// Lightweight provenance-block parser for `spec.md` files. Mirrors the
// W1.3 parser in `crates/domain/src/spec/provenance.rs`: every requirement
// block is introduced by a `### Requirement: <title>` heading and carries
// `ID:`, `Sources:`, and `Status:` lines. The Status enum is closed.

// Closed `Status:` enum from `crates/domain/src/spec/provenance.rs` (W1.3).
export const STATUS_VALUES = new Set([
  "agreed",
  "divergence",
  "conflict",
  "unknown",
]);

export interface RequirementBlock {
  title: string;
  id: string;
  sources: string[];
  status: string;
  inlineTags: string[];
  startLine: number;
}

export interface ParseResult {
  requirements: RequirementBlock[];
  errors: string[];
}

const HEADING_RE = /^### Requirement: (.+?)(\s*\[(?<tag>[a-z-]+)\])?\s*$/;
const ID_RE = /^ID:\s*(\S+)\s*$/;
const SOURCES_RE = /^Sources:\s*\[([^\]]*)\]\s*$/;
const STATUS_RE = /^Status:\s*([a-z-]+)\s*$/;

export function parseSpec(content: string): ParseResult {
  const lines = content.split("\n");
  const requirements: RequirementBlock[] = [];
  const errors: string[] = [];

  for (let i = 0; i < lines.length; i++) {
    const m = HEADING_RE.exec(lines[i]);
    if (!m) continue;
    const title = m[1].trim();
    const inlineTags: string[] = [];
    if (m.groups?.tag) inlineTags.push(m.groups.tag);

    let id: string | null = null;
    let sources: string[] | null = null;
    let status: string | null = null;
    for (let j = i + 1; j < Math.min(i + 8, lines.length); j++) {
      const idM = ID_RE.exec(lines[j]);
      if (idM) {
        id = idM[1];
        continue;
      }
      const srcM = SOURCES_RE.exec(lines[j]);
      if (srcM) {
        sources = srcM[1]
          .split(",")
          .map((s) => s.trim())
          .filter(Boolean);
        continue;
      }
      const stM = STATUS_RE.exec(lines[j]);
      if (stM) {
        status = stM[1];
      }
    }

    if (!id) {
      errors.push(`line ${i + 1}: requirement '${title}' missing ID:`);
      continue;
    }
    if (!sources) {
      errors.push(`line ${i + 1}: requirement '${title}' missing Sources:`);
      continue;
    }
    if (!status) {
      errors.push(`line ${i + 1}: requirement '${title}' missing Status:`);
      continue;
    }
    if (!STATUS_VALUES.has(status)) {
      errors.push(
        `line ${i + 1}: requirement '${title}' Status: '${status}' is not in ${
          [...STATUS_VALUES].join(" | ")
        }`,
      );
    }
    requirements.push({
      title,
      id,
      sources,
      status,
      inlineTags,
      startLine: i + 1,
    });
  }
  return { requirements, errors };
}
