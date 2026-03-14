#!/usr/bin/env python3
"""Deterministic spec merge tool for Specify archive workflow.

Parses baseline and delta spec files using hard-coded heading conventions,
applies RENAMED -> REMOVED -> MODIFIED -> ADDED in strict order, and writes
the merged result.

Exit codes:
  0  merge succeeded (or --validate passed)
  1  merge failed due to errors (missing IDs, duplicates, structure issues)

Usage:
  merge-specs.py --baseline baseline.md --delta delta.md [--output out.md]
  merge-specs.py --validate merged.md [--design design.md]
"""

import argparse
import os
import re
import sys
from typing import Dict, List, NamedTuple, Optional, Tuple


# ---------------------------------------------------------------------------
# Hard-coded spec format (see plugins/spec/references/spec-format.md)
# ---------------------------------------------------------------------------

class SpecFormat(NamedTuple):
    requirement_heading: str
    requirement_id_prefix: str
    requirement_id_pattern: str
    scenario_heading: str
    delta_added: str
    delta_modified: str
    delta_removed: str
    delta_renamed: str


SPEC_FORMAT = SpecFormat(
    requirement_heading="### Requirement:",
    requirement_id_prefix="ID:",
    requirement_id_pattern=r"^REQ-[0-9]{3}$",
    scenario_heading="#### Scenario:",
    delta_added="## ADDED Requirements",
    delta_modified="## MODIFIED Requirements",
    delta_removed="## REMOVED Requirements",
    delta_renamed="## RENAMED Requirements",
)


# ---------------------------------------------------------------------------
# Requirement block parser
# ---------------------------------------------------------------------------

class ReqBlock(NamedTuple):
    heading: str      # full heading line including ###
    name: str         # display name after "### Requirement:"
    req_id: str       # e.g. "REQ-001"
    body: str         # everything from heading through end of block


def parse_requirement_blocks(
    text: str, fmt: SpecFormat
) -> Tuple[str, List[ReqBlock]]:
    """Split a baseline spec into (preamble, [requirement blocks]).

    The preamble is all text before the first requirement heading or ## header.
    """
    lines = text.split("\n")
    heading_prefix = fmt.requirement_heading  # e.g. "### Requirement:"
    id_prefix = fmt.requirement_id_prefix     # e.g. "ID:"

    blocks: List[ReqBlock] = []
    preamble_lines: List[str] = []
    current_lines: List[str] = []
    current_name: Optional[str] = None
    current_id: Optional[str] = None
    in_preamble = True

    def flush_block() -> None:
        nonlocal current_lines, current_name, current_id
        if current_name is not None:
            body = "\n".join(current_lines)
            blocks.append(ReqBlock(
                heading=current_lines[0] if current_lines else "",
                name=current_name,
                req_id=current_id or "",
                body=body,
            ))
        current_lines = []
        current_name = None
        current_id = None

    for line in lines:
        stripped = line.strip()

        if stripped.startswith(heading_prefix):
            if in_preamble:
                in_preamble = False
            else:
                flush_block()
            current_name = stripped[len(heading_prefix):].strip()
            current_lines = [line]
            continue

        if not in_preamble and current_name is not None and current_id is None:
            if stripped.startswith(id_prefix):
                current_id = stripped[len(id_prefix):].strip()

        if in_preamble:
            if stripped.startswith("## ") and not stripped.startswith(heading_prefix):
                in_preamble = False
                flush_block()
                current_lines = [line]
                current_name = None
            else:
                preamble_lines.append(line)
        else:
            if current_name is None and stripped.startswith("## "):
                pass  # skip stray ## headers between blocks
            current_lines.append(line)

    flush_block()

    preamble = "\n".join(preamble_lines)
    return preamble, blocks


# ---------------------------------------------------------------------------
# Delta spec parser
# ---------------------------------------------------------------------------

class RenameEntry(NamedTuple):
    req_id: str
    new_name: str


def parse_delta_sections(
    text: str, fmt: SpecFormat
) -> Tuple[
    List[RenameEntry],   # renamed
    List[ReqBlock],      # removed
    List[ReqBlock],      # modified
    List[ReqBlock],      # added
]:
    """Parse a delta spec into its four operation sections."""
    op_headings = {
        fmt.delta_renamed: "renamed",
        fmt.delta_removed: "removed",
        fmt.delta_modified: "modified",
        fmt.delta_added: "added",
    }

    lines = text.split("\n")
    sections: Dict[str, List[str]] = {
        "renamed": [], "removed": [], "modified": [], "added": []
    }
    current_section: Optional[str] = None

    for line in lines:
        stripped = line.strip()
        matched_section = None
        for heading, section_name in op_headings.items():
            if stripped.lower() == heading.lower():
                matched_section = section_name
                break
        if matched_section is not None:
            current_section = matched_section
            continue
        if current_section is not None:
            sections[current_section].append(line)

    # Parse RENAMED section -- looks for ID: and TO: lines
    renamed: List[RenameEntry] = []
    id_prefix = fmt.requirement_id_prefix
    current_id: Optional[str] = None
    for line in sections["renamed"]:
        stripped = line.strip()
        if stripped.startswith(id_prefix):
            current_id = stripped[len(id_prefix):].strip()
        elif stripped.upper().startswith("TO:") and current_id:
            new_name = stripped[3:].strip()
            renamed.append(RenameEntry(req_id=current_id, new_name=new_name))
            current_id = None

    # Parse requirement blocks from other sections
    removed_text = "\n".join(sections["removed"])
    _, removed = parse_requirement_blocks(removed_text, fmt)

    modified_text = "\n".join(sections["modified"])
    _, modified = parse_requirement_blocks(modified_text, fmt)

    added_text = "\n".join(sections["added"])
    _, added = parse_requirement_blocks(added_text, fmt)

    return renamed, removed, modified, added


# ---------------------------------------------------------------------------
# Merge algorithm
# ---------------------------------------------------------------------------

def merge(
    baseline_text: str,
    delta_text: str,
    fmt: SpecFormat,
    errors: List[str],
) -> str:
    """Apply delta operations to baseline and return the merged result."""
    is_new = not baseline_text.strip()

    renamed, removed, modified, added = parse_delta_sections(delta_text, fmt)

    if is_new:
        has_delta_headers = any(
            h.lower() in delta_text.lower()
            for h in [fmt.delta_added, fmt.delta_modified,
                       fmt.delta_removed, fmt.delta_renamed]
        )
        if not has_delta_headers:
            return delta_text

        result_blocks: List[str] = []
        for block in added:
            result_blocks.append(block.body)
        return "\n\n".join(result_blocks) + "\n" if result_blocks else ""

    preamble, blocks = parse_requirement_blocks(baseline_text, fmt)
    blocks_by_id: Dict[str, int] = {}
    for i, b in enumerate(blocks):
        if b.req_id:
            blocks_by_id[b.req_id] = i

    # Step 1: RENAMED
    for entry in renamed:
        idx = blocks_by_id.get(entry.req_id)
        if idx is None:
            errors.append(
                f"RENAMED: ID {entry.req_id} not found in baseline"
            )
            continue
        old_block = blocks[idx]
        new_heading = f"{fmt.requirement_heading} {entry.new_name}"
        new_body = old_block.body.replace(old_block.heading, new_heading, 1)
        blocks[idx] = ReqBlock(
            heading=new_heading,
            name=entry.new_name,
            req_id=old_block.req_id,
            body=new_body,
        )

    # Step 2: REMOVED
    ids_to_remove = set()
    for block in removed:
        if block.req_id not in blocks_by_id:
            errors.append(
                f"REMOVED: ID {block.req_id} not found in baseline"
            )
        else:
            ids_to_remove.add(block.req_id)

    # Step 3: MODIFIED
    for mod_block in modified:
        idx = blocks_by_id.get(mod_block.req_id)
        if idx is None:
            errors.append(
                f"MODIFIED: ID {mod_block.req_id} not found in baseline"
            )
            continue
        blocks[idx] = mod_block

    # Step 4: ADDED
    existing_ids = set(blocks_by_id.keys()) - ids_to_remove
    for add_block in added:
        if add_block.req_id in existing_ids:
            errors.append(
                f"ADDED: ID {add_block.req_id} already exists in baseline"
            )
            continue
        blocks.append(add_block)
        existing_ids.add(add_block.req_id)

    # Build result: preamble + surviving blocks
    surviving = [b for b in blocks if b.req_id not in ids_to_remove]
    parts = []
    if preamble.strip():
        parts.append(preamble.rstrip())
    for block in surviving:
        parts.append(block.body.strip())

    return "\n\n".join(parts) + "\n"


# ---------------------------------------------------------------------------
# Validation (post-merge coherence checks)
# ---------------------------------------------------------------------------

def validate_baseline(
    text: str,
    fmt: SpecFormat,
    design_text: Optional[str] = None,
) -> List[str]:
    """Run coherence checks on a merged baseline. Returns error messages."""
    errors: List[str] = []
    _, blocks = parse_requirement_blocks(text, fmt)

    # (a) No duplicate requirement IDs
    seen_ids: Dict[str, int] = {}
    for block in blocks:
        if not block.req_id:
            continue
        if block.req_id in seen_ids:
            errors.append(f"Duplicate ID: {block.req_id}")
        seen_ids[block.req_id] = 1

    # (b) No duplicate requirement names
    seen_names: Dict[str, int] = {}
    for block in blocks:
        if block.name in seen_names:
            errors.append(f"Duplicate requirement name: {block.name}")
        seen_names[block.name] = 1

    # (c) Heading structure valid
    id_pattern = re.compile(fmt.requirement_id_pattern)
    for block in blocks:
        if not block.req_id:
            errors.append(
                f"Requirement '{block.name}' has no {fmt.requirement_id_prefix} line"
            )
        elif not id_pattern.match(block.req_id):
            errors.append(
                f"Requirement '{block.name}' has invalid ID '{block.req_id}' "
                f"(expected pattern: {fmt.requirement_id_pattern})"
            )
        if fmt.scenario_heading.rstrip(":") not in block.body:
            errors.append(
                f"Requirement '{block.name}' ({block.req_id}) has no "
                f"{fmt.scenario_heading} section"
            )

    # (d) No orphaned design references
    if design_text:
        ref_pattern = re.compile(fmt.requirement_id_pattern)
        baseline_ids = set(seen_ids.keys())
        for match in ref_pattern.finditer(design_text):
            ref_id = match.group(0)
            if ref_id not in baseline_ids:
                errors.append(
                    f"Design references {ref_id} which does not exist in baseline"
                )

    return errors


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def die(msg: str) -> None:
    print(f"ERROR: {msg}", file=sys.stderr)
    sys.exit(1)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Deterministic spec merge tool for Specify"
    )

    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument(
        "--delta", help="Path to delta spec (merge mode)"
    )
    group.add_argument(
        "--validate", metavar="MERGED",
        help="Path to merged baseline to validate (validate mode)"
    )

    parser.add_argument(
        "--baseline", help="Path to baseline spec (empty/missing = new capability)"
    )
    parser.add_argument(
        "--output", "-o", help="Output file (default: stdout)"
    )
    parser.add_argument(
        "--design", help="Path to design.md for orphaned-reference checking"
    )

    args = parser.parse_args()

    fmt = SPEC_FORMAT

    # --- Validate mode ---
    if args.validate:
        if not os.path.isfile(args.validate):
            die(f"File not found: {args.validate}")
        text = open(args.validate, encoding="utf-8").read()
        design_text = None
        if args.design:
            if not os.path.isfile(args.design):
                die(f"Design file not found: {args.design}")
            design_text = open(args.design, encoding="utf-8").read()

        errs = validate_baseline(text, fmt, design_text)
        if errs:
            for e in errs:
                print(f"FAIL: {e}", file=sys.stderr)
            sys.exit(1)
        else:
            print("All coherence checks passed.")
            sys.exit(0)

    # --- Merge mode ---
    if not args.delta:
        die("--delta is required in merge mode")

    if not os.path.isfile(args.delta):
        die(f"Delta file not found: {args.delta}")

    baseline_text = ""
    if args.baseline and os.path.isfile(args.baseline):
        baseline_text = open(args.baseline, encoding="utf-8").read()

    delta_text = open(args.delta, encoding="utf-8").read()

    errors: List[str] = []
    result = merge(baseline_text, delta_text, fmt, errors)

    if errors:
        for e in errors:
            print(f"ERROR: {e}", file=sys.stderr)
        sys.exit(1)

    if args.output:
        with open(args.output, "w", encoding="utf-8") as f:
            f.write(result)
    else:
        sys.stdout.write(result)


if __name__ == "__main__":
    main()
