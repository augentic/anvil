<div class="hero">
<div class="eyebrow">How-to</div>
<h1 class="hero-title">Upgrade adapters</h1>

Refresh bare adapter bindings to the newest published version, and understand why day-to-day resolution never does this on its own.

<div class="meta-row">

<span class="meta-chip"><strong>Verb</strong> emery adapter upgrade</span>

<span class="meta-chip"><strong>Scope</strong> Bare bindings only</span>

</div>

</div>


<div class="when">
<strong>When to use.</strong>

Use this guide when a newer first-party adapter version has been published and you want your project to pick it up, or when you are unsure why a project keeps resolving an older installed version.
</div>


<section id="why-explicit" markdown="1">

<h2><span class="num">1</span> Why upgrades are explicit</h2>

A bare (unpinned) adapter name resolves **local-first** at every dispatch: the seeded project-cache entry when one exists, else the newest installed store version — offline, with no registry consultation. Only when nothing local exists does the launcher consult the registry and install the newest version. That keeps every plan run reproducible and offline-friendly, but it means a newer published version is never picked up implicitly. `emery adapter upgrade` is the explicit act that forces the registry check.
</section>


<section id="upgrade-one" markdown="1">

<h2><span class="num">2</span> Upgrade one adapter or all</h2>

```bash
emery adapter upgrade omnia        # one bare name
emery adapter upgrade --all       # every bare binding the project records
```

The runtime lists the first-party registry's tags (`ghcr.io/augentic/emery-adapters/<name>`), takes the newest exact-SemVer tag, and installs it into the global adapter store when it is newer than (or absent from) what is installed. `--all` collects the `project.yaml` target plus each `plan.yaml.sources.<key>` adapter; pinned bindings are skipped, and an empty set succeeds with `no bare adapter bindings to upgrade`.

Failure modes: a registry failure is the typed `adapter-latest-failed`; a repository with no SemVer tags is `adapter-latest-none`.
</section>


<section id="what-wont-move" markdown="1">

<h2><span class="num">3</span> What an upgrade won't move</h2>

- **Pinned bindings** (`emery:<name>@<semver>`) are not upgrade targets — edit the pin instead; the new pin installs through the standard pull-on-miss path on first use.
- **Seeded cache entries** always win bare-name resolution: if `emery adapter add` seeded a local component for co-development, upgrading the same name still resolves the seed (the store may still gain the newer version for other projects). Remove or re-seed the cache entry to leave co-dev mode.
</section>


<section id="init-upgrade" markdown="1">

<h2><span class="num">4</span> emery init --upgrade is a different verb</h2>

`emery init --upgrade` re-enters an initialised project: it bumps the project's Emery pin, re-scaffolds preservation-safe files only, and also refreshes bare adapter bindings the way `adapter upgrade` does. It does **not** update the installed `emery` CLI binary — update that through the channel you installed with (see [Prerequisites](../orientation/prerequisites.md#keeping-the-cli-current)).
</section>


<section id="verify" markdown="1">

<h2><span class="num">5</span> Verify what resolved</h2>

```bash
emery source resolve <name>
emery target resolve <name>
```

Each resolve emits the settled identity (`name`, `version` when pinned, `resolved-path`, `location`: `store` or `cache`); the launcher also logs every settled identity — host version, adapter version, origin — to stderr on every dispatch.
</section>


<div class="see-also">
<strong>See also</strong>

- [emery adapter](../reference/cli/adapter.md) — `add`, `upgrade`, and the resolve envelope
- [emery init](../reference/cli/init.md) — pinned-install behaviour and `--upgrade`
- [Directory layout](../reference/directory-layout.md) — the store, cache, and `EMERY_HOME`
</div>
