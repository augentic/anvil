# Brand & Open-Source Strategy

Companion to RFC-77 (release process) · registry, open-source, brand & naming, growth · omnia, backends, emery, emery-adapters · Jul 25, 2026

> **Our core strategy: We help organizations safely build and modernize critical software.**
> Propellerhead delivers clear, bounded outcomes for new and existing critical systems. Modernizing old systems is our primary way in, but we also build new ones. Emery (our delivery system) makes our work repeatable and transparent, while Omnia (our runtime) keeps application logic separate from infrastructure choices. We sell expert services, not software licenses.
>
> **Emery doesn't compete with AI coding tools; it manages them.** Coding agents can write and test code, and open-source models will only get better. Emery connects that AI-generated work to real evidence, human decisions, and verified results. Our success is measured by client confidence and project margins, not how many people download our framework.

## What our business needs

### Vision

> **Critical software should evolve without losing the trust and knowledge built into it.**

Old systems need to change without breaking what the business relies on. New systems should be built with clear intent and flexible infrastructure, so they don't become tomorrow's legacy headaches.

The problem isn't a lack of code generation. For old systems, it's that nobody knows exactly what the code does anymore. For new systems, it's the rush to build before understanding the long-term consequences. Both lead to expensive mistakes.

### Mission and promise

Propellerhead figures out what a critical system actually needs to do, and turns that into a solid foundation for safe, continuous change.

> **We modernize critical systems carefully. We figure out how they really work, agree on what to keep, and deliver change in safe, clear steps. For new systems, we make sure the goals and infrastructure choices are clear before they become permanent.**

Every change we make can be traced back to real evidence, a human decision, and a test. That traceability is why clients trust us.

### Emery's role

Emery isn't our strategy—it's the tool that powers our delivery.

> **Deliberate, evidence-backed delivery for critical systems.**

Clients don't buy "agentic development" or an audit trail. They buy the assurance that we understand their system, we don't hide risks, we use AI safely, and we leave a clear record of what changed and why.

AI makes coding faster. Emery makes it accountable.

### Human judgment is our edge

As writing code gets cheaper, the real value moves to human skills: picking the right problems, understanding consequences, resolving confusion, and deciding what "good" looks like. These aren't roadblocks to automation; they are the most important parts of the job.

Our review process exists so clients and engineers can make smart decisions together. AI agents do the heavy lifting; humans decide what gets built.

### Open source broadens our reach

Open-source AI tools are making software development cheaper and more accessible. We welcome this. Saying "we use advanced AI" won't make us special for long.

Our real advantage is our practice: our judgment, delivery discipline, and ability to learn from every project. By keeping Omnia open and opening Emery, clients can see how we work and avoid vendor lock-in. We compete on how well we deliver, not by hiding our tools.

### Omnia keeps infrastructure flexible

Omnia is a key part of what we offer clients. It lets us write application logic (using WebAssembly) completely separate from the infrastructure it runs on (like databases, messaging, or identity services).

This means a client can swap out their cloud provider or database later without rewriting their core application. It makes testing easier and gives clients long-term flexibility. It doesn't make migrations entirely automatic, but it makes them much less painful.

Clients love this. We should highlight it in our pitches and case studies.

### The relationship model

Our work happens in three stages:

1. **Definition and readiness.** For old systems, we figure out what it does and where the gaps are. For new systems, we define the goals, constraints, and infrastructure needs. This lets us accurately price the first phase of work.
2. **Build and modernize in clear steps.** We deliver work in bounded phases with clear acceptance criteria. We review plans with the client, keep uncertainty visible, and build a solid foundation.
3. **Continuous assurance and evolution.** We stick around to maintain the system, manage future changes, and keep the documentation and tests up to date.

The third stage is where long-term relationships are built. It's not a SaaS subscription—the client owns everything. They stay with us because it's safer and faster than starting over with someone else.

### Stickiness without lock-in

Because our tools are open, clients aren't locked in technically. They stay because of the value we provide:

- **A living baseline** — an always-accurate picture of what their system does.
- **Decision history** — a clear record of why choices were made.
- **Governance integration** — our tools plug right into their existing security and release processes.
- **Account continuity** — our team understands their business and their code.
- **Reliable cadence** — they can start new projects quickly without repeating discovery.

### The scoreboard

We measure success by business outcomes:

- Margin per project
- Time from idea to accepted delivery
- Cost per verified result
- Few bugs and no lost functionality
- Smooth human reviews
- Successful infrastructure swaps without rewriting code
- Repeat business and happy references

We don't measure success by GitHub stars, AI token usage, or how many agents we run.

### What the framework cannot substitute for

Emery makes us faster and safer, but it doesn't run the business. We still need great sales qualification, strong project management, deep industry knowledge, and excellent client relationships. Those matter just as much as the tech.

## Where the four repos stand today

| Repo | Visibility | Registry state | Notes |
| --- | --- | --- | --- |
| `augentic/omnia` | Public | crates.io — 0.34.0 published | MIT / Apache-2.0. Runtime layer, already open and publishing. |
| `augentic/backends` | Public | crates.io — omnia-cursor 0.29.0 declared | Follows omnia's posture; already open. |
| `augentic/emery` | Private | No crates published | Metadata already declares MIT / Apache-2.0. Open source that hasn't been switched on yet. |
| `augentic/emery-adapters` | Private | GHCR wasm components only | Holds the rule corpus and prompt prose. |

## 1 · Cargo registry for the engine crates

**Recommendation:** No registry yet — pin the git dependencies by release tag instead.

Once Phase A gives durable `release-X.Y.Z` lines, pinning by tag makes dependencies enforceable in `Cargo.toml` with zero infrastructure. A private registry is an unnecessary ops burden right now.

> **Trigger to revisit:** The moment a third-party adapter author needs the SDK, publish the SDK-facing crates to crates.io under `emery-*` names.

## 2 · Open vs closed source

**Recommendation:** Open the platform (omnia, backends, emery). Open emery-adapters after an IP pass, keeping client-derived overlays private.

We are already half committed. The strategic thesis—our advantage is our thinking, not the replicable software—argues for finishing the move. Open source is changing the category, and a transparent delivery system helps clients adopt AI-assisted coding without vendor lock-in.

### emery-adapters — the real decision

The distilled "thinking" (rules, review protocols, prompts) lives here. But it is **already shipped to every client** inside the `.wasm` components. Closing the repo protects the git history, not the content. Our moat is our velocity—our rules and prompts improve with every engagement.

## 3 · Brand architecture & naming

**Recommendation:** Propellerhead stays the firm name. One new coined brand carries the open-source platform (e.g., Crossholt, Sureholt, Gateholt, Scarpmere). No new consulting brand.

Propellerhead keeps the contracts, references, and 25 years of trust. The open platform does the innovation signalling as *evidence*, not assertion. A separate consulting brand is the worst of both worlds: zero inherited equity.

### Candidate ranking

Primary shortlist for the platform name:

1. **Crossholt** (Refuge / solid ground to cross to)
2. **Sureholt** (Auditability and certainty)
3. **Gateholt** (Native to product vocabulary)
4. **Scarpmere** (Geological cut + clear water)

> **Omnia stays.** "Omnia" is the substrate, not the marketed brand. Renaming it would ripple through published crates for no audience gain.

## 4 · Growth: NZD $10m → $50m in three years

**Recommendation:** Keep this document as the platform layer, and put a market wedge on top — outcome-priced legacy modernization, sold on accountable change and infrastructure optionality, expanding into Australia. M&A is an optional accelerant.

### The arithmetic constraint

Scaling headcount 5x in 36 months while holding quality is a classic failure mode. The safer path is **revenue per head**: fixed-price, outcome-priced work where Emery's speed advantage is captured as margin, so headcount stays well below revenue.

### The wedge market: legacy modernization

Emery's source adapters act as a **system-archaeology engine**: they recover a provable specification from a running system nobody understands. This is the missing capability in the largest under-served enterprise market in Australasia.

### Modernization is the wedge, not the boundary

The same deliberate-delivery method applies when a client needs a new critical system. New builds begin from intent and constraints rather than recovered legacy behaviour, but they still benefit from explicit authority, reflection, and reviewable boundaries.

### The differentiator: accountable change, not faster coding

Buyers do not purchase an audit trail or an agent fleet; they purchase a critical system they can commission and accept without betting the business. Lead with the outcome: understand what matters, create room for human judgment, deliver in bounded waves, and preserve infrastructure options.

### Three-year shape

| Year | Revenue target | Focus |
| --- | --- | --- |
| 1 | ~$15–17m | Convert existing government relationships into fixed-price modernization anchors; add one financial-services anchor; AU entity + first AU deal. |
| 2 | ~$25–30m | 4–6 concurrent programs, majority outcome-priced; hire delivery leadership; decide on M&A lever. |
| 3 | ~$50m | Mostly organic programs; one acquired-and-retooled consultancy only if needed. |

## Sequencing

| # | Step | Depends on | Status |
| --- | --- | --- | --- |
| 0a | Legal knockout + trademark filing for platform name | — | Open |
| 0b | Secure the name everywhere | Step 0a | Open |
| 0c | Rename org, product, binary, scaffold dir, and WIT package | Step 0b | Partly done (`emery` binary renamed) |
| 1 | Land RFC-77 Phase A: release lines, tags, published WIT pins | Step 0c | Done |
| 2 | Flip the engine repo public | Step 1 | Open |
| 3 | Switch adapter git deps to `tag =` pins | Steps 1–2 | Open |
| 4 | IP pass over `codex/rules/` and prompts; open adapters repo | Step 2 | Open |
| 5 | Publish SDK crates to crates.io | First external adapter author | Waiting |
