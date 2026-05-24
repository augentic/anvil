# RFC-18: Specialized SLM Code Generation

> Status: Draft - Depends: [RFC-28](rfc-28-codex-rules.md), [RFC-10](../done/rfc-10-skills.md), [RFC-13](../done/rfc-13-extensibility.md)

## Abstract

Train a specialized Small Language Model (SLM) to generate Omnia Rust crates from Specify artifacts, with Vectis following once the Omnia path is proven. The goal is not to replace the Specify workflow. It is to make the model behind the Omnia `build/crate.md` brief cheaper, faster, more reproducible, and easier to operate at scale.

The proposed shape is conservative: keep the existing briefs, references, artifacts, and verify-repair loop as the authority; add an SLM backend behind the Omnia crate-build phase; score every generated crate with deterministic checks; and fall back to the frontier model when the SLM fails or sees an unsupported slice shape.

## Motivation

Most fine-tuning proposals struggle because the task, input shape, and reward signal are vague. Spec-driven code generation is unusually constrained:

- **Structured inputs:** `spec.md`, `design.md`, and `tasks.md` already provide predictable, reviewable inputs.
- **Bounded output:** Omnia crate generation targets Rust plus a narrow SDK surface: handlers, provider traits, error macros, WASM constraints, and standard crate layout.
- **Existing teacher:** The current frontier-model Omnia `build/crate.md` brief can generate training examples and repair failed outputs.
- **Machine-checkable scoring:** `cargo check`, `cargo clippy -- -D warnings`, `cargo test`, MockProvider replay, traceability matrices, guardrail checks, and file-layout checks can all feed an automated scorer.
- **Natural fallback:** The current frontier model remains available when the SLM fails the scorer or exceeds its supported slice shape.

### Economic Motivation

Order-of-magnitude economics are favorable if Specify is used across many downstream projects or migration slices. A single crate generation is likely to consume roughly 30k-80k input tokens and 5k-25k output tokens.

| Model class | Per-crate cost |
| --- | --- |
| Frontier model | About `$0.50-$3.00`, depending on token volume and retries. |
| Mid-tier hosted model | About `$0.10-$0.55`. |
| Self-hosted 7B-class SLM | About `$0.005-$0.02` before operational overhead. |

First-pass training should be in the low thousands of dollars or less on rented GPUs. A practical budget is roughly `$400-$1,200` for optional continual pretraining, SFT, and DPO on a rented H100-class machine. Delta retraining for SDK changes should usually be much cheaper.

At 100 crate generations per month, the economics are close but still favorable over a year once training is amortized. At 500+ crate generations per month, the SLM pays back quickly. At migration scale, where `/spec:execute` may generate or repair many crates across many slices in a reviewed change, inference cost stops being a gating concern.

### Operational Motivation

The non-monetary benefits may matter more than the direct token savings:

- **Latency:** A specialized self-hosted model should reduce multi-crate generation loops from frontier-model pacing to local batch inference pacing, often improving wall-clock time by several multiples.
- **Reproducibility:** Pinned weights make generation behavior auditable and less exposed to provider-side model changes.
- **Data locality:** Specs and designs can stay inside customer-controlled infrastructure.
- **Workflow sovereignty:** Specify becomes less exposed to provider pricing, rate limits, and terms-of-service changes.
- **Task quality:** Once enough high-quality examples exist, the SLM can internalize Omnia-specific idioms better than a general frontier model prompted at runtime.

## Design

### Scope

This RFC covers the training and operational shape for an Omnia-first SLM backend. It does not define a new slice lifecycle, replace `/spec:build`, or move deterministic verification out of the CLI and existing skills.

Vectis is intentionally second. The first implementation should prove the scoring harness, corpus pipeline, SFT loop, and dispatch boundary on Omnia before adding a Vectis adapter.

### Data Sources

Use three sources, in this order of trust:

1. **Real merged slices:** `(spec.md, design.md, tasks.md) -> crate files` from downstream Omnia projects.
2. **Extract-derived pairs:** specs and designs reconstructed by `/spec:extract`, paired with the accepted target crate.
3. **Synthetic pairs:** frontier-model outputs generated through the existing Omnia `build/crate.md` brief, kept only when they pass the scorer.

Initial corpus targets:

- 300-800 real or extract-derived pairs.
- 2,000-5,000 filtered synthetic pairs for coverage of adapter combinations and edge cases.
- A 50-slice held-out set covering single-handler, multi-handler, update-mode, matrix, and WASM guardrail cases.

### Scoring Gate

Add `score-crate <dir>` in `specify-cli` as the objective gate for training, evaluation, and production dispatch. It should emit JSON for:

- build status;
- clippy status;
- test status;
- traceability coverage;
- guardrail violations;
- file-layout conformance;
- `.env.example` completeness;
- migration notes and justified TODO markers.

This scorer becomes the reward function for synthetic filtering, reject-sampled DPO, regression testing, and fallback decisions.

### Model Strategy

Start with a strong code-oriented base model, likely Qwen3 Coder 7B Instruct or a comparable 7B-class model with good Rust and long-context behavior. Do not train a base model from scratch. The base should already know Rust syntax, async, serde, error handling, and idiomatic code layout.

The preferred framework strategy is one shared base model with separate LoRA adapters: one for Omnia first, then one for Vectis once the scorer and training loop are stable. A single adapter with a framework tag is cheaper but risks cross-pollinating Omnia and Vectis conventions.

The SLM should not memorize the whole reference corpus. Keep `adapters/targets/omnia/references/*.md` and examples in retrieval, then train the model to follow retrieved references and emit the expected crate shape. This keeps SDK changes cheap: update the reference docs first, run a delta fine-tune only when behavior actually drifts.

### Training Pipeline

The first training loop has four stages:

1. **Optional continual pretraining:** Run a short pass over Omnia SDK Rust, merged crate outputs, Omnia adapter references and examples, and existing Specify artifacts. This is for vocabulary and idiom familiarity, not behavior.
2. **Supervised fine-tuning:** Run QLoRA on real, extract-derived, and filtered synthetic examples.
3. **Preference optimization:** Once SFT produces plausible crates, use reject-sampled DPO. Pair high-scoring and low-scoring outputs from the same prompt.
4. **Quantized deployment:** Quantize the resulting adapter or merged model and re-run the held-out scorer on the deployed artifact.

Reasonable first hyperparameters are QLoRA rank 32-64, alpha at roughly twice rank, dropout 0.05, learning rate around 1e-4 to 2e-4, 2-3 epochs, 16k-32k context, and loss masked to assistant tokens.

### Training Example Shape

The SFT examples should mirror the current Omnia `build/crate.md` invocation:

```text
SYSTEM:
  Distilled Omnia crate-build rules, authority hierarchy, handler pattern,
  error handling, and guardrails.

USER:
  <crate name>
  <retrieved reference chunks>
  <spec.md>
  <design.md>
  <existing crate inventory when updating>

ASSISTANT:
  <derived adapters, mode, Side-Effect Matrix, Outbound Message Matrix,
   Transaction Boundary Matrix>
  <Cargo.toml and source files in deterministic path order>
```

The assistant output starts with the implementation plan because the existing Omnia brief already requires derived adapters, the three matrices, and traceability checks before handoff. Keeping that structure in the training target makes the generated crate easier to score and repair.

### Workflow Integration

`/spec:build` continues to drive the Omnia `build/crate.md` brief. The only new behavior is dispatch: the brief chooses either the frontier model or the SLM backend. The existing repair loop remains unchanged:

```text
generate -> score -> repair -> score -> fallback if still failing
```

This makes the SLM an operational optimization, not a new delivery process. Specify artifacts, skills, references, and CLI checks remain the authority.

## Risks

| Risk | Mitigation |
| --- | --- |
| The SLM passes syntax checks but misses behavior | Keep MockProvider replay, traceability, and substance checks in `score-crate`. |
| Novel adapters regress | Fall back to the frontier model after failed repair attempts. |
| Synthetic data erodes conventions | Filter synthetic pairs through the scorer and preserve the Specify authority hierarchy. |
| SDK changes cause drift | Retrieve current references at inference and run cheap delta fine-tunes only when needed. |
| Update-mode generation is weaker than create-mode | Keep update-mode as a first-class training category, with existing crate inventory in the prompt. |
| Customer data leaks into training | Hash, dedupe, segregate customer-derived pairs, and support project-level opt-out. |

## Rollout Plan

| Phase | Weeks | Outcome |
| --- | --- | --- |
| Set the floor | 1-3 | Implement `score-crate`; inventory real and extract-derived training pairs. |
| First trainable run | 3-6 | Train an Omnia adapter on 500-1,000 pairs; target useful `cargo check` pass rates on held-out slices. |
| Expand and align | 6-10 | Generate filtered synthetic data, re-run SFT, and add DPO from scorer-ranked outputs. |
| Production trial | 10-12 | Quantize, wire behind a config flag, retain frontier fallback, and start measuring real slice outcomes. |

## Generalization

The Omnia/Vectis plan is a specific instance of a more general SLM adaptation pattern:

1. **Classify the goal.** Use RAG for knowledge, SFT for behavior and format, DPO or ORPO for preference alignment, distillation for shrinking a stronger model, and guardrails for hard policy constraints.
2. **Pick a base model that already knows the hard general skill.** For code generation, that means a code-oriented instruct model with good tokenizer fit, context length, license, and serving support.
3. **Try prompt, retrieval, and constrained decoding before training.** Keep volatile knowledge in retrieval so doc changes do not require retraining.
4. **Build the eval harness first.** A 50-500 item golden set plus task-specific checks prevents training from optimizing for vibes.
5. **Curate data aggressively.** A few hundred high-quality real examples beat thousands of noisy ones. Synthetic examples are useful only when filtered.
6. **Start with LoRA or QLoRA.** Full fine-tuning is a later option if adapters plateau.
7. **Quantize and evaluate the quantized model.** GGUF, AWQ, GPTQ, EXL2, or MLX can shift behavior enough that the deployed artifact needs its own score.
8. **Close the loop.** Log failures, harvest corrected outputs, maintain a model card, and retrain on a deliberate cadence.

## Recommendation

Approve a short discovery and prototype effort around Omnia crate generation:

1. Add `score-crate` as the shared evaluation gate.
2. Build the first training corpus from existing Omnia slices.
3. Run one QLoRA prototype and compare it against the current frontier-model Omnia crate-build phase on a held-out set.

If the prototype does not clear the scoring threshold, stop there. If it does, graduate the SLM backend behind the existing Omnia crate-build phase and continue with Vectis as a second adapter.

## References

- [RFC-28: Codex Resolution and Structured Review Findings](rfc-28-codex-rules.md)
- [RFC-10: Skills](archive/rfc-10-skills.md)
- [RFC-13: Extensibility](archive/rfc-13-extensibility.md)
- [`adapters/targets/omnia/briefs/build/crate.md`](../adapters/targets/omnia/briefs/build/crate.md)