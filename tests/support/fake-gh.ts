// Fake `gh` CLI shim for the RM-01 test.
//
// Mirrors `specify-cli/tests/cross_repo.rs` (`FAKE_GH`, `FAKE_SSH`) so a
// helper that drives RM-01 sees the same substrate behaviour as the
// CLI substrate test:
//
//   * env vars: `GH_STATE_DIR` (where PR-state files live),
//     `FAKE_GITHUB_REMOTE_ROOT` (where bare remotes live; consumed by
//     fake-SSH), and the prepended `PATH` entry that holds `gh` +
//     `fake-ssh`.
//   * PR-state file: one file per repo at `${GH_STATE_DIR}/<repo>.pr`
//     with **exactly** five pipe-separated fields:
//       `number|state|merged|branch|url`
//     (e.g. `41|OPEN|false|specify/oauth-login|https://example.invalid/shop-backend/pull/41`).
//   * gh subcommand handling: `repo view`, `pr {list, create, edit, view}`.
//   * "mark merged" mutation: rewrite field 2 → `MERGED` and field 3 →
//     `true` while preserving fields 1, 4, 5.
//
// The PR-numbering policy is **not** baked into the script; it is
// rendered into the script's `case` statement from a test-supplied
// `Record<repoName, prNumber>` map (the cross-repo test had `shop-backend
// → 41`, `shop-mobile → 18` hardcoded; RM-14 suites need to pick their
// own).

import { join } from "jsr:@std/path@1";

/** Default PR numbers — match `specify-cli/tests/cross_repo.rs`. */
export const DEFAULT_PR_NUMBERS: Readonly<Record<string, number>> = Object
  .freeze({
    "shop-backend": 41,
    "shop-mobile": 18,
  });

/** Inputs for `installFakeGh`. */
export interface InstallFakeGhOptions {
  /** Bin dir to write `gh` and `fake-ssh` into (created if missing). */
  binDir: string;
  /** State dir where the script writes `<repo>.pr` files. */
  stateDir: string;
  /**
   * Optional per-repo PR numbers. Keys are bare repo names (e.g.
   * `shop-backend`), values are integers (e.g. `41`). Missing repos
   * fall through to a deterministic fallback (`99`).
   */
  prNumbers?: Record<string, number>;
}

export interface InstallFakeGhResult {
  /** Absolute path to the installed `gh` script. */
  ghPath: string;
  /** Absolute path to the installed `fake-ssh` script. */
  fakeSshPath: string;
  /** PR-state directory (created if missing). */
  stateDir: string;
  /** Effective PR-number map written into the script. */
  prNumbers: Record<string, number>;
}

/** Parsed view of a single `<repo>.pr` file. */
export interface PrState {
  /** The bare repo name (file stem after `repo_key` underscore-encoding). */
  repoKey: string;
  number: number;
  state: "OPEN" | "MERGED" | "CLOSED" | string;
  merged: boolean;
  branch: string;
  url: string;
  /** Source path of the `.pr` file. */
  sourcePath: string;
}

/**
 * Install the fake `gh` and fake-SSH scripts into `binDir`, ensure
 * `stateDir` exists, and return the absolute paths the caller should
 * thread into `GitEnv.fakeBinDir` / `fakeGhStateDir`.
 */
export async function installFakeGh(
  opts: InstallFakeGhOptions,
): Promise<InstallFakeGhResult> {
  const prNumbers: Record<string, number> = {
    ...DEFAULT_PR_NUMBERS,
    ...(opts.prNumbers ?? {}),
  };

  await Deno.mkdir(opts.binDir, { recursive: true });
  await Deno.mkdir(opts.stateDir, { recursive: true });

  const ghPath = join(opts.binDir, "gh");
  await Deno.writeTextFile(ghPath, renderFakeGh(prNumbers));
  await Deno.chmod(ghPath, 0o755);

  const fakeSshPath = join(opts.binDir, "fake-ssh");
  await Deno.writeTextFile(fakeSshPath, FAKE_SSH);
  await Deno.chmod(fakeSshPath, 0o755);

  return { ghPath, fakeSshPath, stateDir: opts.stateDir, prNumbers };
}

/**
 * Read every `<repo>.pr` file under `stateDir` and return the parsed
 * records. Useful for evidence collectors and assertion helpers.
 *
 * Throws `Error` when a file does not have the canonical 5-field shape
 * — the test invariant is that every `.pr` file is well-formed,
 * because `installFakeGh` is the only writer.
 */
export async function readAllPrStates(stateDir: string): Promise<PrState[]> {
  const out: PrState[] = [];
  try {
    for await (const entry of Deno.readDir(stateDir)) {
      if (!entry.isFile || !entry.name.endsWith(".pr")) continue;
      const path = join(stateDir, entry.name);
      const text = (await Deno.readTextFile(path)).replace(/\n+$/, "");
      out.push(parsePrFile(path, text));
    }
  } catch (e) {
    if (!(e instanceof Deno.errors.NotFound)) throw e;
  }
  out.sort((a, b) => a.repoKey.localeCompare(b.repoKey));
  return out;
}

/**
 * Mark a PR file as merged externally. Rewrites field 2 → `MERGED` and
 * field 3 → `true` while preserving fields 1 (number), 4 (branch), and
 * 5 (url). Mirrors `mark_all_prs_merged` in `cross_repo.rs`.
 *
 * `repo` is the bare repo name (e.g. `shop-backend`). The function
 * resolves it through `repo_key` to find the right `.pr` file. When the
 * PR file does not exist (e.g. push has not been called yet for that
 * repo), the function throws `Deno.errors.NotFound` — callers that want
 * "mark all" semantics should walk `readAllPrStates` first.
 */
export async function markPrMerged(opts: {
  stateDir: string;
  /** Bare repo name OR a slug like `shop/shop-backend`. */
  repo: string;
}): Promise<PrState> {
  const repoKey = repoKeyForName(opts.repo);
  const path = join(opts.stateDir, `${repoKey}.pr`);
  const before = parsePrFile(
    path,
    (await Deno.readTextFile(path)).replace(/\n+$/, ""),
  );
  const next: PrState = {
    ...before,
    state: "MERGED",
    merged: true,
  };
  await Deno.writeTextFile(path, formatPrFile(next));
  return next;
}

/** Render the PR file in the canonical 5-field shape (newline-terminated). */
export function formatPrFile(
  state: Omit<PrState, "repoKey" | "sourcePath">,
): string {
  return `${state.number}|${state.state}|${state.merged}|${state.branch}|${state.url}\n`;
}

/** Parse a `.pr` file body into a `PrState`. */
export function parsePrFile(sourcePath: string, body: string): PrState {
  const fields = body.split("|");
  if (fields.length !== 5) {
    throw new Error(
      `fake-gh PR file ${sourcePath} has ${fields.length} fields, expected 5 ` +
        `(number|state|merged|branch|url) — got: ${body}`,
    );
  }
  const [num, state, merged, branch, url] = fields;
  const repoKey = sourcePath.split("/").pop()!.replace(/\.pr$/, "");
  return {
    repoKey,
    number: Number.parseInt(num, 10),
    state,
    merged: merged.trim() === "true",
    branch,
    url,
    sourcePath,
  };
}

/** Mirror the shell helper `repo_key` from `cross_repo.rs`. */
export function repoKeyForName(repoOrSlug: string): string {
  // Drop trailing `.git` and the `git@github.com:` / `https://github.com/`
  // prefixes the same way the shim does, then collapse `/` and `:` to `_`.
  let slug = repoOrSlug
    .replace(/^git@github\.com:/, "")
    .replace(/^ssh:\/\/git@github\.com\//, "")
    .replace(/^https?:\/\/github\.com\//, "")
    .replace(/\.git$/, "");
  return slug.replace(/[/:]/g, "_");
}

function renderFakeGh(prNumbers: Record<string, number>): string {
  // Produce a deterministic case statement so `git diff` against the
  // installed script is meaningful when PR-number policies change.
  const cases = Object.entries(prNumbers)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([repo, num]) => `        ${repo}) number=${num} ;;`)
    .join("\n");

  return `#!/bin/sh
# Fake gh installed by tests/support/fake-gh.ts.
# Mirrors specify-cli/tests/cross_repo.rs FAKE_GH; PR-number policy is
# rendered from the test config (see DEFAULT_PR_NUMBERS in fake-gh.ts).
set -eu

state_dir="\${GH_STATE_DIR:?GH_STATE_DIR is required}"
mkdir -p "$state_dir"

repo_slug() {
  url="$(git config --get remote.origin.url 2>/dev/null || true)"
  case "$url" in
    git@github.com:*) slug="\${url#git@github.com:}" ;;
    https://github.com/*) slug="\${url#https://github.com/}" ;;
    http://github.com/*) slug="\${url#http://github.com/}" ;;
    ssh://git@github.com/*) slug="\${url#ssh://git@github.com/}" ;;
    *) slug="$url" ;;
  esac
  slug="\${slug%.git}"
  printf '%s\\n' "$slug"
}

repo_key() {
  repo_slug | tr '/:' '__'
}

pr_file() {
  printf '%s/%s.pr\\n' "$state_dir" "$(repo_key)"
}

case "\${1:-}" in
  repo)
    if [ "\${2:-}" = "view" ]; then
      name="$(basename "\${3:-unknown}" .git)"
      printf '{"name":"%s"}\\n' "$name"
      exit 0
    fi
    ;;
  pr)
    case "\${2:-}" in
      list)
        file="$(pr_file)"
        if [ -f "$file" ]; then
          number="$(cut -d '|' -f 1 "$file")"
          printf '[{"number":%s}]\\n' "$number"
        else
          printf '[]\\n'
        fi
        exit 0
        ;;
      create)
        branch=""
        while [ "$#" -gt 0 ]; do
          case "$1" in
            --head) shift; branch="\${1:-}" ;;
          esac
          shift || true
        done
        slug="$(repo_slug)"
        repo="$(basename "$slug")"
        case "$repo" in
${cases}
          *) number=99 ;;
        esac
        url="https://github.com/$slug/pull/$number"
        printf '%s|OPEN|false|%s|%s\\n' "$number" "$branch" "$url" > "$(pr_file)"
        printf '%s\\n' "$url"
        exit 0
        ;;
      edit)
        exit 0
        ;;
      view)
        file="$(pr_file)"
        if [ ! -f "$file" ]; then
          echo "no pull request" >&2
          exit 1
        fi
        IFS='|' read -r number state merged branch url < "$file"
        printf '{"state":"%s","merged":%s,"headRefName":"%s","number":%s,"url":"%s"}\\n' \\
          "$state" "$merged" "$branch" "$number" "$url"
        exit 0
        ;;
    esac
    ;;
esac

echo "unsupported fake gh invocation: $*" >&2
exit 1
`;
}

const FAKE_SSH = `#!/bin/sh
# Fake SSH installed by tests/support/fake-gh.ts.
# Mirrors specify-cli/tests/cross_repo.rs FAKE_SSH: rewrites
# git-upload-pack / git-receive-pack onto a local bare remote under
# FAKE_GITHUB_REMOTE_ROOT so 'git clone git@github.com:...' resolves
# without contacting a real network.
set -eu

remote_root="\${FAKE_GITHUB_REMOTE_ROOT:?FAKE_GITHUB_REMOTE_ROOT is required}"

if [ "$#" -lt 2 ]; then
  echo "unsupported fake ssh invocation: $*" >&2
  exit 1
fi

shift
command_line="$*"
operation="\${command_line%% *}"
repo_path="\${command_line#* }"
repo_path="\${repo_path#\\'}"
repo_path="\${repo_path%\\'}"
repo_path="\${repo_path#/}"
repo_name="\${repo_path##*/}"

case "$operation" in
  git-upload-pack|git-receive-pack)
    exec "$operation" "$remote_root/$repo_name"
    ;;
esac

echo "unsupported fake ssh git operation: $operation" >&2
exit 1
`;
