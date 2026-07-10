#!/bin/bash
# Operator replay driver for the `guest-execute-loop` scenario: the
# inverted loop driven by the one `specify` binary itself, through
# either shim.
#
#   bash evals/drivers/guest-execute-loop.sh            # composed wasm deployment
#   SPECIFY_SHIM=native bash evals/drivers/guest-execute-loop.sh
#
# Guest mode builds the workflow guest and the `specify` binary, seeds
# the sandbox project (init from the release-built omnia component),
# writes the deployment manifest (workflow guest + the eight
# release-built adapter components from the sibling specify-adapters
# checkout) at the sandbox root — the forwarding guest leg honors a
# project-root omnia.toml over the generated manifest in the project
# cache — then drives `plan author` and `plan execute` through
# `specify` with the live cursor backend, stamping Gate 1 natively in
# between.
#
# Native mode builds `specify-dev` (the harness/native shim: in-process
# adapter dispatch, cursor model, ephemeral MCP shelves) and drives the
# same loop with no wasm builds and no deployment manifest.
#
# Post-run probes per the scenario's assertion ids run at the end in
# both modes; grading and the run record stay operator-owned.
#
# Not CI. Requires `cursor-agent` on PATH (logged in) and the sibling
# augentic/specify-adapters checkout release-built (`cargo make release`
# there; native mode needs only the omnia component for init). Makes
# real model calls.
set -eu

root="$(cd "$(dirname "$0")/../.." && pwd)"
adapters="${SPECIFY_ADAPTERS:-$root/../specify-adapters}"
release="$adapters/target/wasm32-wasip2/release"
shim="${SPECIFY_SHIM:-guest}"
sandbox="${SPECIFY_SANDBOX:-$root/evals/.sandbox}/guest-execute-loop"
[ "$shim" = native ] && sandbox="$sandbox-native"

command -v cursor-agent >/dev/null || {
  echo "cursor-agent not found on PATH; see evals/scenarios/guest-execute-loop.md" >&2
  exit 2
}
command -v jq >/dev/null || { echo "jq not found on PATH" >&2; exit 2; }
[ -f "$release/omnia.wasm" ] || {
  echo "release-built adapter components not found at $release; run \`cargo make release\` in the sibling specify-adapters checkout (override the root with SPECIFY_ADAPTERS=)" >&2
  exit 2
}

# Build the binary under test. Guest mode also builds a fresh workflow
# guest (the manifest below points at the debug guest so the loop under
# test is the branch head, not a published core); native mode links the
# same verb handlers and the sibling adapter crates in-process.
if [ "$shim" = native ]; then
  (cd "$root" && cargo build -q -p specify-dev)
  specify="$root/target/debug/specify-dev"
else
  (
    cd "$root"
    cargo build -q -p specify-cli
    cargo build -q -p specify-cli --lib --target wasm32-wasip2
  )
  specify="$root/target/debug/specify"
  workflow_wasm="$root/target/wasm32-wasip2/debug/specify.wasm"
  # Pin the freshly built guest as the core (development override)
  # so init never hydrates `specify:core` from the registry — the loop
  # under test is the branch head, not a published core.
  export SPECIFY_CORE_PATH="$workflow_wasm"
fi

# Seed the sandbox. Guest mode inits from the release-built omnia
# component (mirrored into the project component cache, so target
# resolution works both natively and inside the guest). The native shim
# carries only the scaffold leg of init (provisioning stays with the
# shipped path), so native mode scaffolds against the bare `omnia` name
# and stages the component at the resolver's development probe.
rm -rf "$sandbox"
mkdir -p "$sandbox"
cd "$sandbox"
if [ "$shim" = native ]; then
  mkdir -p "$sandbox/target/wasm32-wasip2/release"
  cp "$release/omnia.wasm" "$sandbox/target/wasm32-wasip2/release/omnia.wasm"
  "$specify" init omnia --name guest-demo --scaffold-only
else
  "$specify" init "$release/omnia.wasm" --name guest-demo
fi

# The deployment manifest (guest mode only): the checked-in repo-root
# omnia.toml shape with the "." mount re-pointed at the sandbox. The
# forwarding guest leg picks this up from the project root instead of
# the generated manifest, which lets `plan author` dispatch to the
# source:intent guest before plan.yaml binds it. The native shim needs
# none of this — adapters are linked in and the MCP shelves ride an
# ephemeral listener the shim spawns itself.
if [ "$shim" != native ]; then
  {
    printf '[[guest]]\nid = "workflow"\nsource.path = "%s"\n' "$workflow_wasm"
    printf 'link = ["specify:adapter/source@0.1.0", "specify:adapter/target@0.1.0"]\n\n'
    for id in source:intent source:documentation source:typescript source:screenshots \
              source:captures target:contracts target:omnia target:vectis; do
      name="${id#*:}"
      printf '[[guest]]\nid = "%s"\nsource.path = "%s/%s.wasm"\n\n' "$id" "$release" "$name"
    done
    printf '[[mount]]\nname = "."\npath = "%s"\nwritable = true\n\n' "$sandbox"
    for id in source:intent source:documentation source:typescript source:screenshots \
              source:captures target:contracts target:omnia target:vectis; do
      name="${id#*:}"
      printf '[[route.http]]\nprefix = "/mcp/%s"\nguest = "%s"\n\n' "$name" "$id"
    done
    printf '[transport]\ndefault = "in-process"\n'
  } > "$sandbox/omnia.toml"
fi

log="$sandbox/guest-execute-loop.log"
echo "guest-execute-loop ($shim): sandbox=$sandbox log=$log"

# A pinned port keeps the runtime's HTTP trigger (the MCP shelves the
# cursor backend advertises to the spawned agent) from colliding with
# other local deployments. Guest-mode only; the native shim binds its
# own ephemeral listener.
addr="${HTTP_ADDR:-127.0.0.1:8094}"

drive() {
  echo "==> specify $*" | tee -a "$log"
  HTTP_ADDR="$addr" "$specify" "$@" 2>&1 | tee -a "$log"
}

# The inverted loop: author the plan (survey through the intent source,
# lead reconciliation through the judgment leg), stamp Gate 1 natively,
# then the execute loop to drained.
drive plan author guest-demo \
  --intent "Provide a greeting service with one operation that returns a fixed greeting string."
"$specify" plan transition guest-demo approved
drive plan execute

# Post-run probes (see evals/shared/assertions.md#guest-execute-loop);
# grading against them is the operator's. Identical across shims.
echo "--- probes ---"
"$specify" plan status --format json | jq -c .
grep -c 'status: done' plan.yaml
"$specify" journal show --filter plan.entry.advanced | jq -c .payload
"$specify" journal show --filter slice.merge.succeeded | jq -c .payload
"$specify" journal show --filter slice.archive.created | jq -c .payload
if test ! -f .specify/guest.lock; then echo "guest.lock released"; else echo "guest.lock STILL HELD"; fi
ls crates/ 2>/dev/null || echo "no generated crates/ directory"
echo "guest-execute-loop ($shim): done — grade per the scenario and file evals/runs/guest-execute-loop.<result>.md"
