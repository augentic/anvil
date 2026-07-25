# CLI Naming Brief

Companion to [registry-and-open-source-strategy.md](./registry-and-open-source-strategy.md) · shorter / likeable internal names · Jul 25, 2026

> **Posture for this exercise:** the CLI is Propellerhead's delivery force multiplier. Clients buy Propellerhead; engineers run the tool. Trademark for the CLI is **not required** unless (or until) the binary becomes a marketed open-source platform brand.
>
> That relaxes the §3 clearance bar in the registry strategy. Optimize for mouthfeel, shortness, and daily CLI use — not Nice classes 9/42.

## The fork

The registry strategy assumed **one name** for binary, repo, GHCR, WIT, and the public platform — so trademark was load-bearing. For an internal delivery tool, split the layers:


| Layer              | Needs a “real” brand? | Naming job                                      |
| ------------------ | --------------------- | ----------------------------------------------- |
| Firm               | Yes — Propellerhead   | Wins the deal                                   |
| Client deliverables| Their brand / none    | Invisible                                       |
| Internal CLI       | Likeable + usable     | Engineers say it 50×/day                        |
| Optional public OSS later | Then revisit trademark | Can rename or keep the codename            |


Under that split, score **mouthfeel, shortness, metaphor, PATH friction**. Spec-kit collision still matters if people install both. Augentic / Excava-style clearance becomes nice-to-have, not a knockout — unless you later open the platform under the same name.

## What still knocks names out (even internally)

- Ubiquitous PATH tools (`dig`, and anything that shadows a daily Unix command).
- Famous same-niche binaries (GitHub spec-kit's `specify`; Go's `delve` debugger).
- Tone that fights the audit story (`scrape`, `pry` as the marketed verb).

Different-vertical software (e.g. excava.app for field archaeology) is a **strategic demote** for a public archaeology metaphor, not an automatic internal kill. Revisit with counsel only if the CLI name becomes the public platform face.

## Naming brief (copy-paste)

Use this prompt when generating candidates:

---

**Naming brief — internal delivery CLI (not a marketed platform brand)**

Role: Propellerhead engineers use this daily to deliver client software. Clients rarely see the name. Propellerhead stays the firm brand. Trademark for the CLI is **not required**; avoid only painful collisions (PATH, famous dev tools).

Job of the name: feel like a sharp tool for *legacy discovery → modern, evidence-backed delivery*. Prefer short (≤6 letters ideal, ≤8 max), easy to say, slightly witty or tactile — not corporate compounds (`-holt` / `-mere` / `-ford` unless they earn it).

Surfaces to imagine:

- binary: `<name> init`
- dir: `.<name>/`
- skills: `/<abbr>:*` ok if binary is longer
- sentence: “We’ll run it through `<name>` and show the evidence trail.”

Constraints:

- Must work as a CLI verb-object: `<name> plan`, `<name> refine`
- No collision with ubiquitous PATH tools
- Avoid `specify` / `spec` as the binary (GitHub spec-kit)
- OK if the name is a real English word, coinage, or light metaphor
- NZ/AU ear welcome; no need to sound Nordic/enterprise

Generate 30 candidates in three buckets:

1. **Tactile / tool** (wrench-in-hand)
2. **Archaeology / uncover** (without copying Excava)
3. **Proof / gate / trail** (audit story)

For each: 1-line why it’s likeable, CLI example, and any obvious collision. Then pick a top 8 “say it in the kitchen” shortlist — gut likeability over legal purity.

---

## Useful follow-ups

| Ask | What you get |
| --- | --- |
| **Kitchen shortlist** | ~25 names, no legal research; react with 👍/👎 only |
| **Compress Crossholt** | Same meaning as the coined shortlist, 4–6 letters |
| **Codename energy** | More Linear/Vercel (warp, chalk, forge) than consultancy |
| **Dual name** | Cute internal CLI + boring public repo later if OSS happens |
| **Slash namespace separate** | e.g. binary changes, skills stay `/spec:*` for Cursor muscle memory |

When kicking off a run, state tone: **tool**, **uncover**, **proof**, or a mix — and whether slash commands may stay `/spec:*` while the binary changes.

## Scoring under the relaxed plan


| Keep scoring | Drop or demote |
| ------------ | -------------- |
| Length, spell, say-aloud | Trademark / Nice 9/42 |
| `<name> init` naturalness | “Can we own search?” |
| PATH / famous-tool clashes | Coined uniqueness for counsel |
| Fits archaeology *or* audit | One name must carry the whole growth brand |


## Caution if OSS returns

If the engine is later opened and a third-party adapter ecosystem is encouraged, a throwaway-cute CLI name becomes the public face whether you trademark it or not.

- **Internal-only** → freer naming.
- **Public platform later** → either pick a name that ages well, or accept a second rename before the flip (still cheap pre-1.0).

The registry strategy’s Crossholt / Sureholt / Gateholt / Scarpmere shortlist remains the right list when binary = marketed platform brand. This brief is the alternate track when Propellerhead stays the only client-facing name.

## Related

- [Registry & open-source strategy](./registry-and-open-source-strategy.md) — §3 brand architecture (trademark-first platform shortlist)
- RFC-77 — release process; rename before tags and the launcher’s compiled GHCR constant bake names in
