<div class="hero">
<div class="eyebrow">How-to</div>
<h1 class="hero-title">Publish a change</h1>

Review, commit, and push the publication worktrees `emery plan execute` materializes, with the pull-request trailers finalize verification looks for.

<div class="meta-row">

<span class="meta-chip"><strong>Symptom</strong> plan status shows a materialized publication member</span>

<span class="meta-chip"><strong>Fix</strong> Ordinary Git: diff, commit, push, pull request</span>

</div>

</div>


<div class="when">
<strong>When to use.</strong>

Use this guide when `emery plan execute` has drained and `emery plan status` lists one or more publication members as `materialized`. Publication is operator-owned: Emery stages each target's accepted result as an ordinary Git worktree, and everything from review to merge runs through your normal Git and forge tooling.
</div>


<section id="find" markdown="1">

<h2><span class="num">1</span> Find the worktree</h2>

`emery plan status` names each member's branch and node-local worktree path:

```text
publication:
  default: materialized — review, commit, and push branch change/checkout-v2 from /…/publication/checkout-v2/default
```

Each member is one target repository. The branch is always `change/<plan>`. Placement follows one rule: a single-member change whose product checkout is clean at the recorded parent revision is materialized **in place** (your own checkout switches to the publication branch); every other member lands in a dedicated slot under `$EMERY_HOME/publication/<plan>/<target>/`.

The content arrives staged in the Git index — `git status` shows the whole delta as changes to be committed, and the nested `.emery/change/` home is excluded automatically.
</section>


<section id="review" markdown="1">

<h2><span class="num">2</span> Review and commit</h2>

```bash
cd "$EMERY_HOME/publication/<plan>/<target>"   # or your checkout when in place
git diff --staged
git commit
git push -u origin change/<plan>
```

Your commit is authoritative. Once `HEAD` moves off the recorded parent, Emery never rewinds or restates the branch — a re-run of `emery plan execute` leaves your commit exactly as it is. Conversely, uncommitted edits you leave in the worktree stop a re-materialization with `publication-worktree-dirty` rather than being overwritten.
</section>


<section id="pull-request" markdown="1">

<h2><span class="num">3</span> Open the pull request with both trailers</h2>

Open a pull request from `change/<plan>` and include both trailer lines in the pull-request body:

```text
Emery-Change: <plan>
Emery-Change-Digest: sha256:…
```

The digest is the covering `plan.yaml` content digest, recorded on the member's `plan.publication.materialized` journal fact (`emery journal show --filter plan.publication.materialized`). It disambiguates a reused plan name against the same repository over time, so a later change cannot false-verify against an earlier change's merged pull request.

When the change spans several repositories, land the pull requests in the plan's dependency order.
</section>


<section id="finalize" markdown="1">

<h2><span class="num">4</span> Finalize</h2>

After every member's pull request has merged, run `/emery:finalize` (or `emery plan archive` directly). Archive verifies publication before mutating anything: each member must have exactly one merged pull request carrying both trailers.
</section>


## See also

- [Drop down a layer](drop-down-a-layer.md) — manual CLI control when automation is blocked
- [Understanding Emery](../explanation/concepts.md) — where publication sits in the workflow
