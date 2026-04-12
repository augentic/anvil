/**
 * Regression tests for merge-specs.ts.
 *
 * Run with: deno test tests/merge-specs_test.ts
 * From:     plugins/spec/skills/merge/scripts/
 */

import { assertEquals, assert } from "jsr:@std/assert@1";

import {
  SPEC_FORMAT,
  merge,
  parseRequirementBlocks,
  parseDeltaSections,
  validateBaseline,
} from "../merge-specs.ts";

const FIXTURES = new URL("./fixtures/", import.meta.url).pathname;

function readFixture(name: string): string {
  return Deno.readTextFileSync(`${FIXTURES}${name}`);
}

// ---------------------------------------------------------------------------
// Parsing tests
// ---------------------------------------------------------------------------

Deno.test("parseRequirementBlocks - parses preamble and blocks", () => {
  const text = readFixture("baseline_simple.md");
  const [preamble, blocks] = parseRequirementBlocks(text, SPEC_FORMAT);
  assert(preamble.includes("User Authentication"));
  assertEquals(blocks.length, 2);
  assertEquals(blocks[0].reqId, "REQ-001");
  assertEquals(blocks[0].name, "Login");
  assertEquals(blocks[1].reqId, "REQ-002");
  assertEquals(blocks[1].name, "Logout");
});

Deno.test("parseRequirementBlocks - empty text", () => {
  const [preamble, blocks] = parseRequirementBlocks("", SPEC_FORMAT);
  assertEquals(preamble, "");
  assertEquals(blocks, []);
});

Deno.test("parseRequirementBlocks - preamble only", () => {
  const text = "# Title\n\nSome overview text.";
  const [preamble, blocks] = parseRequirementBlocks(text, SPEC_FORMAT);
  assert(preamble.includes("Title"));
  assertEquals(blocks, []);
});

Deno.test("parseDeltaSections - add section", () => {
  const text = readFixture("delta_add.md");
  const [renamed, removed, modified, added] = parseDeltaSections(text, SPEC_FORMAT);
  assertEquals(renamed.length, 0);
  assertEquals(removed.length, 0);
  assertEquals(modified.length, 0);
  assertEquals(added.length, 1);
  assertEquals(added[0].reqId, "REQ-003");
});

Deno.test("parseDeltaSections - modify section", () => {
  const text = readFixture("delta_modify.md");
  const [, , modified] = parseDeltaSections(text, SPEC_FORMAT);
  assertEquals(modified.length, 1);
  assertEquals(modified[0].reqId, "REQ-001");
  assert(modified[0].body.includes("MFA"));
});

Deno.test("parseDeltaSections - remove section", () => {
  const text = readFixture("delta_remove.md");
  const [, removed] = parseDeltaSections(text, SPEC_FORMAT);
  assertEquals(removed.length, 1);
  assertEquals(removed[0].reqId, "REQ-002");
});

Deno.test("parseDeltaSections - rename section", () => {
  const text = readFixture("delta_rename.md");
  const [renamed] = parseDeltaSections(text, SPEC_FORMAT);
  assertEquals(renamed.length, 1);
  assertEquals(renamed[0].reqId, "REQ-001");
  assertEquals(renamed[0].newName, "Sign In");
});

// ---------------------------------------------------------------------------
// Merge tests
// ---------------------------------------------------------------------------

Deno.test("merge - add requirement", () => {
  const baseline = readFixture("baseline_simple.md");
  const delta = readFixture("delta_add.md");
  const errors: string[] = [];
  const result = merge(baseline, delta, SPEC_FORMAT, errors);
  assertEquals(errors, []);
  assert(result.includes("REQ-001"));
  assert(result.includes("REQ-002"));
  assert(result.includes("REQ-003"));
  assert(result.includes("Password Reset"));
});

Deno.test("merge - modify requirement", () => {
  const baseline = readFixture("baseline_simple.md");
  const delta = readFixture("delta_modify.md");
  const errors: string[] = [];
  const result = merge(baseline, delta, SPEC_FORMAT, errors);
  assertEquals(errors, []);
  assert(result.includes("MFA"));
  assert(result.includes("REQ-001"));
  assert(result.includes("REQ-002"));
});

Deno.test("merge - remove requirement", () => {
  const baseline = readFixture("baseline_simple.md");
  const delta = readFixture("delta_remove.md");
  const errors: string[] = [];
  const result = merge(baseline, delta, SPEC_FORMAT, errors);
  assertEquals(errors, []);
  assert(result.includes("REQ-001"));
  assert(!result.includes("REQ-002"));
  assert(!result.includes("Logout"));
});

Deno.test("merge - rename requirement", () => {
  const baseline = readFixture("baseline_simple.md");
  const delta = readFixture("delta_rename.md");
  const errors: string[] = [];
  const result = merge(baseline, delta, SPEC_FORMAT, errors);
  assertEquals(errors, []);
  assert(result.includes("Sign In"));
  assert(result.includes("REQ-001"));
});

Deno.test("merge - new spec without delta headers (passthrough)", () => {
  const delta = readFixture("delta_new_spec.md");
  const errors: string[] = [];
  const result = merge("", delta, SPEC_FORMAT, errors);
  assertEquals(errors, []);
  assert(result.includes("Brand New Feature"));
  assert(result.includes("REQ-001"));
});

Deno.test("merge - new spec with delta headers (extract ADDED)", () => {
  const delta = readFixture("delta_add.md");
  const errors: string[] = [];
  const result = merge("", delta, SPEC_FORMAT, errors);
  assertEquals(errors, []);
  assert(result.includes("REQ-003"));
});

Deno.test("merge - add duplicate ID errors", () => {
  const baseline = readFixture("baseline_simple.md");
  const delta = readFixture("delta_add.md").replaceAll("REQ-003", "REQ-001");
  const errors: string[] = [];
  merge(baseline, delta, SPEC_FORMAT, errors);
  assert(errors.some((e) => e.includes("REQ-001") && e.includes("already exists")));
});

Deno.test("merge - remove missing ID errors", () => {
  const baseline = readFixture("baseline_simple.md");
  const delta = readFixture("delta_remove.md").replaceAll("REQ-002", "REQ-999");
  const errors: string[] = [];
  merge(baseline, delta, SPEC_FORMAT, errors);
  assert(errors.some((e) => e.includes("REQ-999") && e.includes("not found")));
});

Deno.test("merge - modify missing ID errors", () => {
  const baseline = readFixture("baseline_simple.md");
  const delta = readFixture("delta_modify.md").replaceAll("REQ-001", "REQ-999");
  const errors: string[] = [];
  merge(baseline, delta, SPEC_FORMAT, errors);
  assert(errors.some((e) => e.includes("REQ-999") && e.includes("not found")));
});

Deno.test("merge - rename missing ID errors", () => {
  const baseline = readFixture("baseline_simple.md");
  const delta = readFixture("delta_rename.md").replaceAll("REQ-001", "REQ-999");
  const errors: string[] = [];
  merge(baseline, delta, SPEC_FORMAT, errors);
  assert(errors.some((e) => e.includes("REQ-999") && e.includes("not found")));
});

// ---------------------------------------------------------------------------
// Validation tests
// ---------------------------------------------------------------------------

Deno.test("validateBaseline - valid baseline passes", () => {
  const text = readFixture("baseline_simple.md");
  const errors = validateBaseline(text, SPEC_FORMAT);
  assertEquals(errors, []);
});

Deno.test("validateBaseline - duplicate ID detected", () => {
  const text = readFixture("baseline_simple.md").replaceAll("REQ-002", "REQ-001");
  const errors = validateBaseline(text, SPEC_FORMAT);
  assert(errors.some((e) => e.includes("Duplicate ID")));
});

Deno.test("validateBaseline - missing scenario detected", () => {
  const text = readFixture("baseline_simple.md")
    .replace("#### Scenario: Valid credentials", "")
    .replace("#### Scenario: Active session", "");
  const errors = validateBaseline(text, SPEC_FORMAT);
  assert(errors.some((e) => e.includes("Scenario")));
});

Deno.test("validateBaseline - design orphaned reference (fixed with multiline flag)", () => {
  // The TypeScript port uses the "m" flag with RegExp, fixing the pre-existing
  // Python bug where ^$ anchors without re.MULTILINE prevented matching.
  const text = readFixture("baseline_simple.md");
  const design = "This references\nREQ-999\nwhich does not exist.";
  const errors = validateBaseline(text, SPEC_FORMAT, design);
  assert(errors.some((e) => e.includes("REQ-999")));
});
