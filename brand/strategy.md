# Registry & Open-Source Strategy

Companion to RFC-77 (release process) · registry, open-source, brand & naming, growth · omnia, backends, emery, emery-adapters · Jul 25, 2026

> **Strategic posture: a consultancy's force multiplier, not a product.**
> Every decision below is scored against one thesis: Emery is Propellerhead's delivery force multiplier — the revenue is services engagements, and the competition is other consultancies, not spec-kit / Kiro / Tessl. 
>
> **Emery isn't competing with AI coding tools; it's the delivery system that sits on top of them**. The crowded tooling market is validation, not threat: GitHub is paying to educate the category, and the free tools don't bid on delivery contracts. The guardrail is a trace test — every engine feature should trace to a client engagement need, and the scoreboard is margin per engagement and delivery speed, not framework adoption.
>
> If the roadmap starts optimising Emery as a product, this whole strategy needs re-scoring.


## What the services business needs

Emery is not the strategy. It is the compounding delivery system for a focused services strategy:

> **Evidence-backed modernization and ongoing change for regulated or poorly understood systems.**

The valuable promise is not “agentic development,” more agents, or legacy migration by itself. It is that Propellerhead can discover what an existing system actually does, expose uncertainty instead of concealing it, produce a reviewable specification, change the system faster with agents, demonstrate which behaviour was conserved, and leave a durable record of what changed, why, and on whose authority.

### The relationship model

The offer has three connected stages.

1. **Readiness and system archaeology.** A paid entry engagement establishes repository and runtime readiness, recovers requirements and behaviour, identifies verification gaps, produces a modernization roadmap, and prices subsequent work according to measured uncertainty. This is how fixed-price risk is bounded before Propellerhead accepts it.
2. **Modernization in bounded waves.** Outcome-priced delivery proceeds against explicit acceptance boundaries. Each wave carries known evidence and protected checks or captures; unresolved intent stays visible; client stakeholders review topology and specifications; accepted results become the durable product baseline.
3. **Continuous assurance and evolution.** After modernization, Propellerhead maintains the behavioural baseline, refreshes captures and verification profiles, manages carried debt, keeps specifications and client documentation current, delivers subsequent regulatory and product changes, and periodically reassesses readiness and conservation coverage.

The third stage creates the long-lived relationship. It is not a SaaS subscription disguised as consulting: the client owns its code, artifacts, evidence, and deployment. The relationship continues because another bounded change with Propellerhead is faster, safer, and more predictable than reconstructing the estate and its decisions again.

### Stickiness without lock-in

Opening the framework means the engine cannot be the moat. That is healthy. Durable advantage comes from accumulated operational value:

- **Living behavioural baseline** — the best available account of what the system does, carried with the product rather than left in a consultant's slide deck.
- **Capture and replay corpus** — maintained evidence that future changes preserve behaviour.
- **Decision and debt history** — institutional memory of why choices were made and which gaps remain.
- **Client-specific overlays** — private domain rules, adapters, policies, and verification profiles built on the open framework.
- **Governance integration** — Emery's facts and assurance enter the client's CI, release, observability, security, and approval processes.
- **Practice velocity** — the shared rules, prompts, evaluation cases, and delivery methods improve after every engagement; a fork receives code, not the practice that keeps improving it.
- **Account continuity** — people who understand both the estate and its evidence record remain available for the next wave.
- **Reliable change cadence** — a client can begin another bounded wave without repeating discovery from zero.

These are switching benefits, not switching barriers. Clients stay because continuing creates more value, not because leaving is technically prevented.

### The scoreboard

Leadership is measured by services outcomes:

- gross margin per accepted outcome;
- elapsed time from intent to verified acceptance;
- cost per verified result;
- escaped defects and conservation failures;
- human intervention per slice;
- proportion of requirements backed by protected evidence;
- readiness improvement and carried-debt trend;
- repeat and expansion revenue;
- referenceable successful programmes.

OSS adoption, agent count, token throughput, and task concurrency are supporting signals at most. None is the business result.

### What the framework cannot substitute for

Emery can improve delivery economics and assurance; it cannot create a leading services firm by itself. Propellerhead also needs rigorous deal qualification, fixed-price commercial governance, senior programme and delivery leadership, vertical domain expertise, account ownership, protected-data governance, lighthouse case studies, and a repeatable way to train delivery teams. Those capabilities may matter more commercially than the entire concurrency and fleet programme.


## Where the four repos stand today


| Repo                        | Visibility | Registry state                                                     | Notes                                                                                                                      |
| --------------------------- | ---------- | ------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------- |
| `augentic/omnia`            | Public     | crates.io — 0.34.0 published (both repos pin 0.35.0 via git patch) | MIT / Apache-2.0. Runtime layer, already open and publishing.                                                              |
| `augentic/backends`         | Public     | crates.io — omnia-cursor 0.29.0 declared, git-patched to main      | Follows omnia's posture; already open.                                                                                     |
| `augentic/emery`          | Private    | No crates published — publishable crates already carry `emery-*` names; only the root binary and guest / launcher / mock / probe / wasi-exec-bits stay `publish = false` | Metadata already declares MIT / Apache-2.0 and a public repo URL. The posture is open source that hasn't been switched on. |
| `augentic/emery-adapters` | Private    | GHCR wasm components only (`ghcr.io/augentic/emery-adapters`)    | Consumes engine crates as unpinned git dependencies, held by `Cargo.lock`. Holds the rule corpus and prompt prose.         |




## 1 · Cargo registry for the engine crates

**Recommendation:** No registry yet — pin the git dependencies by release tag instead.

The `adapter` / `native` / `probe` / `prose` seam currently floats on emery's default branch, held only by the committed `Cargo.lock`. That is exactly what RFC-77 D9 warns about: adapters that only build against an unpublished seam state. Once Phase A gives durable `release-X.Y.Z` lines, pinning by tag makes D9 enforceable in `Cargo.toml` with zero infrastructure:

```toml
# emery-adapters/Cargo.toml — after RFC-77 Phase A
adapter = { git = "https://github.com/augentic/emery.git", tag = "v0.28.0" }
```



### Why a registry doesn't pay for itself today


| Concern                                 | Detail                                                                                                                                                                    |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| crates.io is public-only, names collide | `adapter`, `native`, `probe`, `prose` are generic on crates.io. **Retired:** the publishable crates already carry `emery-*` publish names in Cargo metadata (short `[lib]` names keep Rust paths `adapter::` / `native::`), so publishing no longer forces a rename. |
| Forces omnia pin hygiene first          | Published crates can't carry `[patch.crates-io]`. Both workspaces pin omnia 0.35.0, but crates.io is at 0.34.0 — every engine publish would be gated on an omnia publish. |
| Private registry is ops burden          | Kellnr / Cloudsmith / JFrog add hosting and auth for a small team with exactly one internal consumer.                                                                     |
| One consumer, same team                 | Git deps + tags + the committed sibling `[patch]` block already cover co-development.                                                                                     |


> **Trigger to revisit.** The moment a third-party adapter author needs the SDK (RM-21), git dependencies into a private repo stop working. Then publish the SDK-facing crates (`adapter`, plus `native` / `probe` for the test harness) to crates.io under `emery-`* names. RFC-77's "no crates.io" non-goal is right for this cut — reword it as *deferred until third-party SDK consumers exist*, not rejected.



## 2 · Open vs closed source

**Recommendation:** Open the platform (omnia, backends, emery). Open emery-adapters after an IP pass, keeping client-derived overlays private.

You're already half committed: omnia and backends are public and on crates.io, and both private workspaces declare MIT / Apache-2.0 with public repository URLs in their metadata. The strategic thesis — the advantage is the thinking, not the replicable software — argues for finishing the move.

### Why opening emery is close to forced


| Driver                                          | Argument                                                                                                                                                            |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Your own roadmap requires it                    | RM-21 (third-party adapters, descriptor registry, trust policy) cannot exist around a private engine. Nobody builds against a WIT contract and SDK they can't read. |
| Distribution friction fights the services model | Clients run the `emery` binary and pull adapter components from GHCR pull-on-miss. Private repos mean token provisioning for every client machine and CI runner.  |
| The software is proof of the thinking           | For a consultancy, an open platform is compounding marketing: sales credibility, no lock-in objection, inbound interest, hiring.                                    |




### emery-adapters — the real decision

The distilled "thinking" — `codex/rules/`, review-team protocols, synthesis and build prompt corpora — lives here. But it is **already shipped to every client**: the prose is compiled into the `.wasm` components clients pull, and prompt text is trivially extractable from a component. Closing the repo protects the git history, not the content. The moat is velocity — rules and prompts improve every engagement; a fork inherits a stale snapshot without the practice that produced it.


| Open                                                                                                                                               | Private                                                                                                                                                                                                                                                                                                    |
| -------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **First-party adapters + rule corpus.** Your best portfolio piece, and effectively source-available anyway. Ship as-is after a one-time IP review. | **Client-derived and vertical prose.** Client-specific rule overlays, engagement adapters, domain playbooks. A private adapter repo publishes components the same way — `emery adapter add` and private GHCR pins already handle closed distribution. The boundary is per adapter/overlay, not per repo. |


> **Licensing.** Keep the permissive MIT / Apache-2.0 already declared. BSL/FSL guards against a hyperscaler productizing your software — the thesis explicitly says the software isn't the product, and restrictive licenses would cost the credibility that motivates opening up.



## 3 · Brand architecture & naming

**Recommendation:** Propellerhead stays the firm. One new coined brand carries the open-source platform — shortlist: Crossholt, Sureholt, Gateholt, Scarpmere, pending legal clearance. No new consulting brand. Kitchen lean (not clearance): **Scarpmere** for the org/umbrella (replaces `augentic`); short CLI candidates and a wider inventory live in [naming-brief.md](./naming-brief.md#kitchen-inventory-jul-25-2026) (current short-word lean: **emery**).

Propellerhead keeps the contracts, references, and 25 years of trust — the buyers who sign are the least moved by brand freshness. The open platform does the innovation signalling as *evidence*, not assertion (the Thoughtworks pattern). A separate consulting brand ("DarkTarn Consulting, powered by Propellerhead") is the worst of both worlds: zero inherited equity, and the tie-back tagline defeats the distancing it was invented for. Market evidence agrees — Emery is already winning clients under the Propellerhead name.

### Why both original names must go (the CLI rename to `emery` has since landed; `augentic` is still in place)


| Name       | Blocker                                                                                                                                                                                                                                                                                                                                                    |
| ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `augentic` | Augentic GmbH (Munich, est. 2020) holds a **registered EU trademark** on "Augentic" since 2021 — identity, biometrics, and digital-currency software for governments. A senior user with a registered mark in adjacent classes; under a permissive OSS license the trademark is the only retained IP, so an unenforceable name is strategically untenable. |
| `specify`  | GitHub's spec-kit is a **triple identical collision**: a CLI binary named `specify`, a `specify init` first command, and a `.specify/` scaffold directory — same niche, GitHub-backed, 35 agent integrations, on PyPI. A user with spec-kit installed cannot even have both binaries on PATH.                                                              |




### Candidate ranking

Primary shortlist (knockout-clear for software; file classes 9/42 in NZ, AU, EU, US):


| Rank | Name          | Verdict                                                                                                                                                                                                                                                                                                      |
| ---- | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1    | **Crossholt** | Cross + holt (refuge / solid ground): the proven place to cross from legacy chaos to a modern system. Nine letters, said as spelled (`crossholt init`). No Tarn*, no Crawford phone collision, no "dark" tax. Nearest priors are a dissolved UK shell and an unrelated NY corp — different classes.        |
| 2    | **Sureholt**  | Sure + holt: says the auditability differentiator out loud (evidence-backed certainty, not just speed). Same CLI shape as Crossholt. Slightly more marketing-literal — some will hear "SureSoftware" — but knockout-clean.                                                                                 |
| 3    | **Gateholt**  | Gate + holt: native to the product vocabulary (Gate 1, merge gates) without colliding with `specify`. Exact compound looks clear; watch the dense "gate" neighbourhood in software (gateways, Gatehouse projects) — phonetic neighbours for counsel.                                                       |
| 4    | **Scarpmere** | Scarp (geological cut / cliff face) + mere (clear water): best semantic fit for the system-archaeology modernization wedge. Hallway risk: "scarp" is unfamiliar and may come back as *sharp-mere* / *scar-meer*.                                                                                           |


Demoted (kept for counsel fallback; do not lead with these):


| Rank | Name     | Verdict                                                                                                                                                                                                                                                                                                                                      |
| ---- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 5    | Cragford | Crag + ford still etymologically strong, but **Crawford Software** (ERP consulting) fails the phone test — "Cragford / Crawford?" is a permanent sales tax. Also Cragsoft. Only revisit if Crossholt / Sureholt / Gateholt fail clearance and counsel clears the Crawford neighbourhood.                                                   |
| 6    | Cragmere | Crag + mere: solidity plus clarity. Demoted for fuzzy adjacency — CragSoftware (AI studio), The Cragmere Group (NJ construction), and **Rockmere Partners** (Dallas AI transformation consultancy). Watch the "mere / mear" spelling in the say-it-aloud test.                                                                              |
| 7    | Tarnholm | Tarn + holm: clean Nordic sound, but **kill for AU expansion** — [Tarn](https://tarn.au/) is a Melbourne AI/software studio in the growth market, and Sierra Nevada holds **TARNHUS** in class 42. "Holm" misheard as "home" remains a flattering failure mode if counsel somehow clears the Tarn neighbourhood.                           |
| 8    | Darktarn | Strong imagery, clean search — but "dark" pulls against the transparency / audit story, and Darktrace sits one fuzzy syllable away in enterprise ears. US marks TARNHELM / TARNHUS for lawyer review. Last resort only.                                                                                                                     |
| 9    | Emend    | Latin *emendare* — correct a text; strong fit for evidence-backed legacy rewrite (`emend plan`). Demoted for [lucaswiman/emend](https://github.com/lucaswiman/emend): live Python refactoring CLI on PyPI with the same binary and an `.emend/` scaffold. Brew / `cargo binstall` / git install avoid the PyPI name fight, but PATH shadow and `.emend/` clash remain if an engineer has both. Fine as an internal CLI if the scaffold dir does not copy `.emend/`; revisit before any public platform face. |


Checked and eliminated: Stillmere (active UK civil-engineering consultancy), Stonebeck (tech-strategy consultancy), Quoin (Quoin.ai + Quoin Technologies), Lintel (Lintel Technologies + LintelAI), Holdfast (holdfast.dev + Hold Fast Ai Ltd), Attestor (multiple AI control-plane projects + GCP Binary Authorization term), Torford (Clarks shoe line + Tom Ford phonetic collision), Clearford (Clearford Water Systems, registered marks in 9/42), Trueford (TrueFords apparel), Fellford (Fellford Co Pty Ltd, AU 2025), **Excava** (liked for the dig-the-legacy metaphor, but hard-killed: [excava.app](https://excava.app/) is a live archaeological-excavation SaaS — exact name, software class, and the same archaeology metaphor Propellerhead would want; also Excava.com.au construction supplies in AU, Excava.cl mining vehicle, Excavaite IP-AI neighbour, Taiwan class-007 machinery mark). Wildcard: Cairnford / Cairnmere — semantically the strongest of all (cairns are waymarkers), but three software companies already mine the cairn root (Cairnsoft, CairnSoft, Cairn Software); a lawyer's call on operating in that neighbourhood.

### Repo map after the rename (crossholt shown as placeholder — substitute the cleared winner)


| Today                       | Proposed                   | Notes                                                                                                                                                                      |
| --------------------------- | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `augentic/emery`          | `crossholt/crossholt`      | Engine + CLI carries the product name: binary `crossholt`, scaffold `.crossholt/`, WIT `crossholt:adapter`. Pre-1.0 hard-cut posture makes the rename a grep-and-replace. |
| `augentic/emery-adapters` | `crossholt/adapters`       | GHCR becomes `ghcr.io/crossholt/adapters/<name>`.                                                                                                                          |
| `augentic/omnia`            | `crossholt/omnia`          | Name kept — see callout below.                                                                                                                                             |
| `augentic/backends`         | `crossholt/omnia-backends` | `backends` is meaningless as a standalone public repo name; prefix with the substrate it belongs to (the crate is already `omnia-cursor`).                                 |


Principles: every public repo name self-describing without org context; repo name = binary = crate prefix = OCI namespace; decide before RFC-77 tags and the launcher's compiled GHCR constant bake the names in.

> **Omnia stays.** "Omnia" is generic Latin and commercially crowded, but it is the substrate, not the marketed brand — and the bare `omnia` crate name on crates.io is an irreplaceable asset already held. Renaming would ripple through published crates and both workspaces' macros for no audience gain. If the runtime is ever marketed standalone, rebrand at the prose level first ("the Crossholt runtime"); the crate names never have to move. Precedent: Kubernetes/etcd, Deno/V8.

## 4 · Growth: NZD $10m → $50m in three years

**Recommendation:** Keep this document as the platform layer, and put a market wedge on top — outcome-priced legacy modernization, sold on auditability, expanding into Australia — with M&A as an optional accelerant if organic growth undershoots.

Everything above is a margin-and-credibility strategy; a force multiplier alone does not 5x a services firm. The growth layer is a market, pricing, and geography decision.

### The arithmetic constraint

$50m from $10m is 5x — roughly 71% CAGR. At conventional T&M rates (~$250–300k/FTE) $50m means ~170–200 delivery heads; scaling headcount ~5x in 36 months while holding quality is the classic services-firm failure mode. The safer path to the 5x is **revenue per head**: fixed-price, outcome-priced work where Emery's speed advantage is captured as margin rather than passed on as cheaper hours, so the headcount curve stays well below the revenue curve.

### The wedge market: legacy modernization

Emery's source adapters — `captures` (runtime behavior with replay digests), `screenshots`, `documentation`, `typescript` — plus evidence, provenance, and authority resolution constitute a **system-archaeology engine**: it recovers a provable specification from a running system nobody understands. That is the missing capability in the largest under-served enterprise market in Australasia: every bank, insurer, utility, and agency holds 20–40-year-old systems where every previous rewrite failed because *nobody could specify what the old system does*. Deal economics fit the arithmetic: modernization programs are $3–20m fixed-price engagements; five to eight concurrent programs is $50m.

### The differentiator: accountability, not speed

AI-accelerated delivery is already noise. **AI delivery with an audit trail** — every requirement traceable to evidence, conflicts resolved by declared authority, merges gated and journaled — is a category of one, structurally hard to copy, and it is the unlock that lets regulated buyers (government, banking, insurance, health) say yes to AI-built systems at all. Lead with "AI-built, evidence-backed, auditable." Fixed-price delivery risk is underwritten by the same evidence trail.

The beachhead already exists: two central-government clients and one local-government client are on the books today. Year 1 is not about winning government logos — it is about converting existing relationships into the first fixed-price modernization anchors and the public case studies that carry the audit story into Australia.

### The wedge is contested — the differentiator still holds

Well-capitalized AI coding platforms now also sell brownfield modernization on speed and cost. **"We do legacy modernization with AI" is no longer differentiating on its own.** Lead with the audit trail and behavioural conservation, not the automation.

Emery's commercial artifact remains the joined, digest-bound delivery record: source evidence, authority resolution, `[unknown]` / `[conflict]` gaps, exact execution coverage, verification, and accepted state. An outer agent may drive typed commands and projections ([platform.md § Operator identity](../rfcs/platform.md#operator-identity-an-agent-may-drive-the-engine), [RM-24](../rfcs/roadmap.md#rm-24-operator-control-surface)); the engine stays a deterministic state machine over the fact log. Readiness as a paid entry engagement ([RFC-94](../rfcs/rfc-94-target-readiness.md)) and conservation under host-attested verification ([RFC-97](../rfcs/rfc-97-native-verification.md), [RFC-98](../rfcs/rfc-98-behavioural-conservation.md)) underwrite fixed-price delivery more directly than speculative concurrency. Sequencing authority is the [services programme](../rfcs/platform.md), not competitor feature checklists.

### Geography and M&A

NZ is the proving ground; the $50m lives in Australia (5–6x the market, same timezone, trans-Tasman procurement familiarity). AU entity plus a senior local face in year one, anchored by a lighthouse deal won from NZ. From a $10m base, strong organic growth (40–50% CAGR) reaches ~$27–34m — well short of $50m. The gap closes either with one or two more concurrent modernization programs than the organic plan assumes, or by acquiring a small AU/NZ consultancy (0.5–1x revenue) in year 2–3 and retooling it onto the platform; the open-source flip is what makes acquired engineers onboardable. M&A is the contingency lever, not the plan's foundation.

### Deprioritized

The RM-21 ecosystem play (third-party adapter authors, certification, marketplace) is the product-company growth loop and would arm competitors in the modernization market. Keep the open core; exercise the ecosystem option from strength in year 4.

### Three-year shape

| Year | Revenue target | Focus |
| --- | --- | --- |
| 1 | ~$15–17m | Convert the existing government relationships (two central, one local) into two or three fixed-price modernization anchors; add one financial-services anchor; AU entity + first AU deal; publish the provenance/audit story hard — it is the marketing. |
| 2 | ~$25–30m | 4–6 concurrent programs, majority outcome-priced; hire delivery leadership ahead of the constraint; decide by mid-year whether the M&A lever is needed. |
| 3 | ~$50m | Mostly organic programs; one acquired-and-retooled consultancy only if the year-2 checkpoint called for it. |

> **Honest caveat.** 5x in three years is a top-decile outcome for a services firm even with a genuine margin edge. The likely landing zone if the thesis holds is $30–40m. The principal risk is fixed-price delivery risk, mitigated by the same evidence trail that differentiates the offer.

## Sequencing


| #   | Step                                                                                            | Depends on                            | Status                                                                                                                       |
| --- | ----------------------------------------------------------------------------------------------- | ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| 0a  | Legal knockout + trademark filing for the platform name (classes 9/42 — NZ, AU, EU, US)         | —                                     | Open                                                                                                                         |
| 0b  | Secure the name everywhere: GitHub org, GHCR, domains, crates.io / PyPI namespaces              | Step 0a                               | Open                                                                                                                         |
| 0c  | Rename org, product, binary, scaffold dir, and WIT package while everything is still private    | Step 0b                               | Partly done — binary, `.emery/` scaffold, and WIT package renamed to `emery` ahead of sequence; the org rename off `augentic` remains |
| 1   | Land RFC-77 Phase A: release lines, tags, published WIT pins                                    | Step 0c                               | Done — release tags exist (`v0.37.0` at last check)                                                                          |
| 2   | Flip the engine repo public                                                                     | Step 1                                | Open — repo still private                                                                                                    |
| 3   | Switch adapter git deps to `tag =` pins (satisfies RFC-77 D9 in Cargo.toml)                     | Steps 1–2                             | Open — adapter git deps are still unpinned, held by `Cargo.lock`                                                             |
| 4   | IP pass over `codex/rules/` and prompt corpora; open the adapters repo                          | Step 2                                | Open                                                                                                                         |
| 5   | Publish SDK crates to crates.io under the new product prefix                                    | First external adapter author (RM-21) | Waiting on trigger                                                                                                           |

> **Sequence caveat.** Steps 0c and 1 ran ahead of the 0a/0b gate: the CLI rename to `emery` and the release tags landed while the org is still `augentic`. The trademark-blocked org name is therefore already baked into the launcher's compiled GHCR constant (`ghcr.io/augentic/emery-adapters`), so the eventual org rename forces a launcher constant change, a component re-publish, and a client re-pin — the cost step 0c existed to avoid grows with every release shipped before it.


Source: augentic repo visibility via GitHub API, crates.io publish state, EUIPO record for Augentic GmbH, github/spec-kit docs, name-collision web searches, and RFC-76 / RFC-77 / RM-21 as of Jul 25, 2026.