<div class="hero">
<div class="eyebrow">Tutorial</div>
<h1 class="hero-title">Migrate a legacy service</h1>

Point Emery at an existing TypeScript codebase and drive it to an Omnia service: bind the code as a source, review the plan it surveys, then execute refine → build → merge per slice. Bring any TypeScript repository you own.

<div class="meta-row">

<span class="meta-chip"><strong>Time</strong> ~45 min</span>

<span class="meta-chip"><strong>Target</strong> Omnia</span>

<span class="meta-chip"><strong>Outcome</strong> Migrated service</span>

</div>

</div>


<section id="outcome" markdown="1">

<h2><span class="num">1</span> What you will build</h2>

An Omnia project whose specs and implementation are reconciled from a real legacy codebase rather than hand-written intent. The `typescript` source adapter surveys the legacy tree into slice-sized leads at plan time and extracts evidence from it at slice time; the migration then rides the same rhythm as every other change — only the plan row count differs.

Any TypeScript service works as the source — a small one (a single adapter or worker, not a monolith) keeps the first run short. This tutorial was developed against a small Kafka position adapter; substitute your own repository throughout.
</section>


<div class="prereq">
<strong>Prerequisites.</strong>

Complete [Prerequisites](../orientation/prerequisites.md):

- Cursor with Augentic plugins installed
- `emery` CLI (`emery --version` succeeds)
- Rust toolchain with `wasm32-wasip2` target for Omnia
- A TypeScript repository you can clone (repo root with `package.json` and `src/`)

Work in a **fresh, empty directory** — not inside a checkout of the `emery` repository — and open it in Cursor Agent chat. Everything Emery generates (`.emery/`, `plan.yaml`, `change.md`, `discovery.md`) and the `legacy/` clone is regenerable; if the directory is a git repository, gitignore them rather than committing them.
</div>


<section id="steps" markdown="1">

<h2><span class="num">2</span> Steps</h2>


<div class="tutorial-step" data-step="01">
<div class="step-label">01</div>
<h3 class="step-title">Clone the legacy source</h3>

Clone your legacy repository into `legacy/` inside the project directory:

```bash
git clone <your-legacy-repo-url> legacy/
```

The clone's root must contain `package.json` and `src/` — that is what the `typescript` source adapter surveys. The clone is an input fixture, never a deliverable: it is read at plan and slice time and is safe to delete and re-clone at any point.
</div>


<div class="tutorial-step" data-step="02">
<div class="step-label">02</div>
<h3 class="step-title">Initialise the project</h3>

Run once per project, naming it after the service you are migrating:

```text
/emery:init omnia --name my-service
```

The skill runs `emery init omnia --name my-service`, which scaffolds `.emery/{slices,specs,archive}/`, resolves the `omnia` adapter, and generates `AGENTS.md` when absent. See [Directory layout](../reference/directory-layout.md) for the full tree.

Re-running init on an already-initialized project is a no-op; `emery init --upgrade` is the re-entry path for bumping the project's Emery pin.
</div>


<div class="tutorial-step" data-step="03">
<div class="step-label">03</div>
<h3 class="step-title">Author the plan</h3>

Bind the clone as a `typescript` source:

```text
/emery:plan my-service source legacy=typescript:legacy
```

The skill elicits the one-line intent conversationally — for example `Migrate <your-legacy-repo-url> to an Omnia service`. The CLI form passes it as a flag:

```bash
emery plan author my-service \
  --intent "Migrate <your-legacy-repo-url> to an Omnia service" \
  --source "legacy=typescript:legacy"
```

The source adapter's `survey` operation scans `legacy/` and emits slice-sized leads. Planning writes three artifacts — `change.md` (operator narrative), `discovery.md` (what the survey found), and `plan.yaml` (the slice table of contents) — then exits for operator review. Nothing executes yet.
</div>


<div class="tutorial-step" data-step="04">
<div class="step-label">04</div>
<h3 class="step-title">Review the plan</h3>

Read `change.md`, `discovery.md`, and `plan.yaml`. For a migration, check that the slice breakdown matches how you think the service decomposes, and that `discovery.md`'s lead inventory reflects the legacy tree you expected it to survey — an empty survey usually means the source binding pointed at the wrong directory.

> [!IMPORTANT]
> **Review.** This pause is the operator review step. Running `emery plan execute` is your approval of the plan — `/emery:plan` never starts execution itself. To adjust scope first, see [Amend a plan before executing](../how-to/amend-a-plan.md).

At any point, `emery plan status` (or `/emery:status`) is the read-only "where am I / what next" probe — it prints the plan's state and the literal next command, and never writes anything.
</div>


<div class="tutorial-step" data-step="05">
<div class="step-label">05</div>
<h3 class="step-title">Execute</h3>

```text
/emery:execute
```

Running execute is your approval; it drives each slice through **refine → build → merge** until every plan entry is done:

<div class="pipeline">


![Per-slice loop](../assets/diagrams/concepts/slice-loop.svg)

<p class="pipeline-caption">refine synthesizes artifacts from legacy evidence; build implements tasks; merge folds specs into the baseline.</p>
</div>


During refine, the adapter's `extract` operation produces evidence YAML from the legacy code; that evidence carries `authority: behaviour`, the lowest class, so your intent wins any disagreement — see [Legacy migration at scale](../explanation/legacy-migration.md).

If execute stops, the stop card prints a `hint:` and a `resume:` command — run the resume command. To drive one slice at a time instead, use the breakout skills `/emery:refine`, `/emery:build`, `/emery:merge` (or `emery slice refine|build|merge`); abandon a slice with `emery slice drop`. Both `emery plan status` and `emery slice list` are read-only checks between steps.
</div>


<div class="tutorial-step" data-step="06">
<div class="step-label">06</div>
<h3 class="step-title">Finalize</h3>

Publish the generated work through your normal Git and review workflow first — Emery performs no Git or forge operations. Then confirm the plan is drained and close it:

```bash
emery plan status    # every entry should be done
emery plan archive
```

```text
/emery:finalize my-service
```
</div>


</section>


> [!TIP]
> **Done looks like:** archived slice artifacts under `.emery/archive/`, Omnia output in the project tree, and the merged spec baseline under `.emery/specs/` — a durable trail from legacy code to generated service.


<section id="troubleshooting" markdown="1">

<h2><span class="num">3</span> Troubleshooting</h2>

Every CLI failure prints a kebab-case error code and usually a `hint:` line naming the recovery command — follow the hint first. Common cases:

| Symptom                            | What to try                                                                              |
| ---------------------------------- | ---------------------------------------------------------------------------------------- |
| `init` says already initialized    | Use `emery init --upgrade`, or skip straight to plan/execute                             |
| Source path missing / empty survey | Confirm `legacy/package.json` exists; the source binding must be `legacy=typescript:legacy` |
| `emery-version-too-old` (exit 3)   | The project pin is newer than the installed binary — reinstall the CLI (the hint prints the literal command), then `emery init --upgrade` |
| `adapter-cli-too-old` (exit 3)     | Reinstall the CLI; if the adapter is the stale side, `emery adapter upgrade <name>`      |
| Execute stopped mid-slice          | Run the stop card's `resume:` command; `emery plan status` reprints it                   |
| Not sure what to do next           | `emery plan status` (or `/emery:status`) — read-only, prints the literal next command    |

</section>


<div class="see-also">
<strong>See also</strong>

- [Legacy migration at scale](../explanation/legacy-migration.md) — why authority matters when code and docs disagree
- [Bind multiple sources](../how-to/bind-multiple-sources.md) — combine legacy code with design notes at plan time
- [Drive a slice manually](../how-to/drive-slice-manually.md) — when execute parks on a failure
- [Quick reference card](../reference/quick-reference.md) — source binding grammar and command cheat sheet
</div>
