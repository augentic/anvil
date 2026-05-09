# Spec-Driven Code Generation with a Specialized SLM

## Purpose

This RFC proposes training a specialized Small Language Model (SLM) to generate Omnia Rust crates from Specify artifacts, with Vectis following once the Omnia path is proven. The goal is not to replace the existing Specify workflow. It is to make the model behind `/omnia:crate-writer` cheaper, faster, more reproducible, and easier to operate at scale.

## Why This Is Plausible

Most fine-tuning proposals struggle because the task, input shape, and reward signal are vague. Spec-driven code generation is unusually constrained:

- **Structured inputs:** `spec.md`, `design.md`, and `tasks.md` already provide predictable, reviewable inputs.
- **Bounded output:** Omnia crate generation targets Rust plus a narrow SDK surface: handlers, provider traits, error macros, WASM constraints, and standard crate layout.
- **Existing teacher:** The current frontier-model `crate-writer` skill can generate training examples and repair failed outputs.
- **Machine-checkable scoring:** `cargo check`, `cargo clippy -- -D warnings`, `cargo test`, MockProvider replay, traceability matrices, guardrail checks, and file-layout checks can all feed an automated scorer.
- **Natural fallback:** The current frontier model remains available when the SLM fails the scorer or exceeds its supported slice shape.

## Proposed Approach

Start with Omnia crate generation. Train one base model with an Omnia LoRA adapter, then add a Vectis adapter only after the scoring harness and training loop are stable.

The SLM should not memorize the whole reference corpus. Keep `crate-writer/references/*.md` and examples in retrieval, then train the model to follow retrieved references and emit the expected crate shape. This keeps SDK changes cheap: update the reference docs first, run a delta fine-tune only when behavior actually drifts.

Recommended first pass:

1. Build `score-crate <dir>` in `specify-cli` as the objective gate. It should emit JSON for build status, clippy, tests, traceability, guardrails, layout, `.env.example`, and migration notes.
2. Inventory real training pairs from merged Omnia slices: `(spec.md, design.md, tasks.md) -> crate files`.
3. Add extract-derived and synthetic pairs, but only keep outputs that pass the scorer.
4. Fine-tune a strong code-oriented base model, likely Qwen3 Coder 7B Instruct or a comparable 7B-class model with good Rust and long-context behavior.
5. Use reject-sampled DPO only after SFT produces plausible crates. Pair high-scoring and low-scoring outputs from the same prompt.
6. Wire the SLM as an alternative `crate-writer` backend behind a config flag. Keep the existing verify-repair loop and frontier-model fallback.

Initial targets:

- 300-800 real or extract-derived pairs.
- 2,000-5,000 filtered synthetic pairs for coverage.
- A 50-slice held-out set covering single-handler, multi-handler, update-mode, matrix, and WASM guardrail cases.

## Workflow Fit

`/spec:build` continues to invoke `/omnia:crate-writer`. The only new behavior is dispatch: `crate-writer` chooses either the frontier model or the SLM backend. The existing repair loop remains unchanged:

```text
generate -> score -> repair -> score -> fallback if still failing
```

This makes the SLM an operational optimization, not a new delivery process. Specify artifacts, skills, references, and CLI checks remain the authority.

## Expected Benefits

Order-of-magnitude economics are favorable if Specify is used across many downstream projects or migration slices. A frontier model generation can cost roughly `$0.50-$3.00` per crate depending on token volume and retry count. A self-hosted 7B-class SLM can plausibly land around `$0.005-$0.02` per crate before operational overhead, with first-pass training in the low thousands of dollars or less on rented GPUs.

The non-monetary benefits may matter more:

- **Latency:** A specialized self-hosted model should reduce multi-crate generation loops from frontier-model pacing to local batch inference pacing.
- **Reproducibility:** Pinned weights make generation behavior auditable and less exposed to provider-side model changes.
- **Data locality:** Specs and designs can stay inside customer-controlled infrastructure.
- **Workflow sovereignty:** Specify becomes less exposed to provider pricing, rate limits, and terms-of-service changes.
- **Task quality:** Once enough high-quality examples exist, the SLM can internalize Omnia-specific idioms better than a general frontier model prompted at runtime.

## Risks and Mitigations

| Risk | Mitigation |
| ---- | ---------- |
| The SLM passes syntax checks but misses behavior | Keep MockProvider replay, traceability, and substance checks in `score-crate`. |
| Novel capabilities regress | Fall back to the frontier model after failed repair attempts. |
| Synthetic data erodes conventions | Filter synthetic pairs through the scorer and preserve the Specify authority hierarchy. |
| SDK changes cause drift | Retrieve current references at inference and run cheap delta fine-tunes only when needed. |
| Update-mode generation is weaker than create-mode | Keep update-mode as a first-class training category, with existing crate inventory in the prompt. |
| Customer data leaks into training | Hash, dedupe, segregate customer-derived pairs, and support project-level opt-out. |

## Suggested 90-Day Plan

| Phase | Weeks | Outcome |
| ----- | ----- | ------- |
| Set the floor | 1-3 | Implement `score-crate`; inventory real and extract-derived training pairs. |
| First trainable run | 3-6 | Train an Omnia adapter on 500-1,000 pairs; target useful `cargo check` pass rates on held-out slices. |
| Expand and align | 6-10 | Generate filtered synthetic data, re-run SFT, and add DPO from scorer-ranked outputs. |
| Production trial | 10-12 | Quantize, wire behind a config flag, retain frontier fallback, and start measuring real slice outcomes. |

## Decision Requested

Approve a short discovery and prototype effort around Omnia crate generation:

1. Add `score-crate` as the shared evaluation gate.
2. Build the first training corpus from existing Omnia slices.
3. Run one QLoRA prototype and compare it against the current frontier-model `crate-writer` on a held-out set.

If the prototype does not clear the scoring threshold, stop there. If it does, graduate the SLM backend behind the existing `crate-writer` dispatch and continue with Vectis as a second adapter.
# Spec-Driven Code Generation with a Specialized SLM

## Purpose

This RFC proposes training a specialized Small Language Model (SLM) to generate Omnia Rust crates from Specify artifacts, with Vectis following once the Omnia path is proven. The goal is not to replace the existing Specify workflow. It is to make the model behind `/omnia:crate-writer` cheaper, faster, more reproducible, and easier to operate at scale.

## Why This Is Plausible

Most fine-tuning proposals struggle because the task, input shape, and reward signal are vague. Spec-driven code generation is unusually constrained:

- **Structured inputs:** `spec.md`, `design.md`, and `tasks.md` already provide predictable, reviewable inputs.
- **Bounded output:** Omnia crate generation targets Rust plus a narrow SDK surface: handlers, provider traits, error macros, WASM constraints, and standard crate layout.
- **Existing teacher:** The current frontier-model `crate-writer` skill can generate training examples and repair failed outputs.
- **Machine-checkable scoring:** `cargo check`, `cargo clippy -- -D warnings`, `cargo test`, MockProvider replay, traceability matrices, guardrail checks, and file-layout checks can all feed an automated scorer.
- **Natural fallback:** The current frontier model remains available when the SLM fails the scorer or exceeds its supported slice shape.

## Proposed Approach

Start with Omnia crate generation. Train one base model with an Omnia LoRA adapter, then add a Vectis adapter only after the scoring harness and training loop are stable.

The SLM should not memorize the whole reference corpus. Keep `crate-writer/references/*.md` and examples in retrieval, then train the model to follow retrieved references and emit the expected crate shape. This keeps SDK changes cheap: update the reference docs first, run a delta fine-tune only when behavior actually drifts.

Recommended first pass:

1. Build `score-crate <dir>` in `specify-cli` as the objective gate. It should emit JSON for build status, clippy, tests, traceability, guardrails, layout, `.env.example`, and migration notes.
2. Inventory real training pairs from merged Omnia slices: `(spec.md, design.md, tasks.md) -> crate files`.
3. Add extract-derived and synthetic pairs, but only keep outputs that pass the scorer.
4. Fine-tune a strong code-oriented base model, likely Qwen3 Coder 7B Instruct or a comparable 7B-class model with good Rust and long-context behavior.
5. Use reject-sampled DPO only after SFT produces plausible crates. Pair high-scoring and low-scoring outputs from the same prompt.
6. Wire the SLM as an alternative `crate-writer` backend behind a config flag. Keep the existing verify-repair loop and frontier-model fallback.

Initial targets:

- 300-800 real or extract-derived pairs.
- 2,000-5,000 filtered synthetic pairs for coverage.
- A 50-slice held-out set covering single-handler, multi-handler, update-mode, matrix, and WASM guardrail cases.

## Workflow Fit

`/spec:build` continues to invoke `/omnia:crate-writer`. The only new behavior is dispatch: `crate-writer` chooses either the frontier model or the SLM backend. The existing repair loop remains unchanged:

```text
generate -> score -> repair -> score -> fallback if still failing
```

This makes the SLM an operational optimization, not a new delivery process. Specify artifacts, skills, references, and CLI checks remain the authority.

## Expected Benefits

Order-of-magnitude economics are favorable if Specify is used across many downstream projects or migration slices. A frontier model generation can cost roughly `$0.50-$3.00` per crate depending on token volume and retry count. A self-hosted 7B-class SLM can plausibly land around `$0.005-$0.02` per crate before operational overhead, with first-pass training in the low thousands of dollars or less on rented GPUs.

The non-monetary benefits may matter more:

- **Latency:** A specialized self-hosted model should reduce multi-crate generation loops from frontier-model pacing to local batch inference pacing.
- **Reproducibility:** Pinned weights make generation behavior auditable and less exposed to provider-side model changes.
- **Data locality:** Specs and designs can stay inside customer-controlled infrastructure.
- **Workflow sovereignty:** Specify becomes less exposed to provider pricing, rate limits, and terms-of-service changes.
- **Task quality:** Once enough high-quality examples exist, the SLM can internalize Omnia-specific idioms better than a general frontier model prompted at runtime.

## Risks and Mitigations

| Risk | Mitigation |
| ---- | ---------- |
| The SLM passes syntax checks but misses behavior | Keep MockProvider replay, traceability, and substance checks in `score-crate`. |
| Novel capabilities regress | Fall back to the frontier model after failed repair attempts. |
| Synthetic data erodes conventions | Filter synthetic pairs through the scorer and preserve the Specify authority hierarchy. |
| SDK changes cause drift | Retrieve current references at inference and run cheap delta fine-tunes only when needed. |
| Update-mode generation is weaker than create-mode | Keep update-mode as a first-class training category, with existing crate inventory in the prompt. |
| Customer data leaks into training | Hash, dedupe, segregate customer-derived pairs, and support project-level opt-out. |

## Suggested 90-Day Plan

| Phase | Weeks | Outcome |
| ----- | ----- | ------- |
| Set the floor | 1-3 | Implement `score-crate`; inventory real and extract-derived training pairs. |
| First trainable run | 3-6 | Train an Omnia adapter on 500-1,000 pairs; target useful `cargo check` pass rates on held-out slices. |
| Expand and align | 6-10 | Generate filtered synthetic data, re-run SFT, and add DPO from scorer-ranked outputs. |
| Production trial | 10-12 | Quantize, wire behind a config flag, retain frontier fallback, and start measuring real slice outcomes. |

## Decision Requested

Approve a short discovery and prototype effort around Omnia crate generation:

1. Add `score-crate` as the shared evaluation gate.
2. Build the first training corpus from existing Omnia slices.
3. Run one QLoRA prototype and compare it against the current frontier-model `crate-writer` on a held-out set.

If the prototype does not clear the scoring threshold, stop there. If it does, graduate the SLM backend behind the existing `crate-writer` dispatch and continue with Vectis as a second adapter.