# Change Manifest

Produce the classified, ordered change plan consumed by the execution
pipeline.

## Instructions

1. Read specs (the deltas), design, and current-state
2. For each change item, classify using this decision tree:

   Is the item entirely new (not in current state)?
   → ADDITIVE

   Is the item being removed (in current state, marked - in specs)?
   → SUBTRACTIVE

   Has the item's name, location, or structural role changed?
   → STRUCTURAL

   Has the item's content/behavior changed?
   → MODIFYING

3. Assess overall complexity:
   - low: only additive changes
   - medium: mix of additive + modifying, or limited subtractive
   - high: structural changes, significant subtractive, cross-target
   - recommend-regeneration: >50% handler logic rewritten, domain model
     restructured, or crate purpose changed

4. For each change item, predict affected files in the crate

5. Order changes: structural → subtractive → modifying → additive

6. Flag cross-target interactions from pipeline.toml dependencies

## Output Format

# Change Manifest

- Change: <openspec change name>
- Generated: <timestamp>

## Targets

### Target: <crate-name>

- Complexity: <low|medium|high|recommend-regeneration>
- Route: <crate-updater|tdd-gen>

#### STRUCTURAL

- STR-N: <description>
  - Source: <spec ref or design section>
  - Scope: <number of files, references>

#### SUBTRACTIVE

- SUB-N: <description>
  - Source: <spec ref>
  - Current location: <file path>
  - Dependents: <list>

#### MODIFYING

- MOD-N: <description>
  - Source: <spec ref>
  - Affected: <handler names, type names>

#### ADDITIVE

- ADD-N: <description>
  - Source: <spec ref>
  - New files: <predicted paths>

## Cross-Target Dependencies

- <from-target> → <to-target>: <what contract is shared>

## Risks

- <interaction risks between changes>