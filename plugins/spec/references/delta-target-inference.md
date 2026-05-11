# Delta-target inference — `/spec:define` specs brief

The specs brief infers which existing baselines this slice modifies by reading the plan entry's `description` for references to prior change names (e.g. "delta-target user-registration", "modifies email-verification"). For each referenced name, the brief checks whether a baseline exists at `.specify/specs/<name>/spec.md` and applies the DELTA composition pass on confirmed matches.

The brief logs the inferred delta targets in the journal. If the description does not reference any existing baselines, all extracted specs remain in fresh new-crate form.

The artifact-side delta conventions (how spec authors write `### REMOVED` / `### CHANGED` / `### NEW` blocks) live in [artifact-conventions.md](artifact-conventions.md).
