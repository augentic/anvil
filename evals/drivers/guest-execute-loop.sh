#!/bin/bash
# Operator replay driver for the `guest-execute-loop` scenario: the
# RFC-61 Step 4 inverted loop through the composed Omnia deployment.
#
#   bash evals/drivers/guest-execute-loop.sh
#
# Builds the workflow guest and the runtime binary, seeds the sandbox
# project (vendored omnia target adapter, one-slice intent plan stamped
# approved at Gate 1), writes the deployment manifest (workflow guest +
# the eight committed adapter guests from the sibling specify-adapters
# checkout), then drives `source survey intent` and `plan execute`
# through `specify-runtime` with the live cursor backend. Post-run
# probes per the scenario's assertion ids run at the end; grading and
# the run record stay operator-owned.
#
# Not CI. Requires `cursor-agent` on PATH (logged in) and the sibling
# augentic/specify-adapters checkout. Makes real model calls.
set -eu

root="$(cd "$(dirname "$0")/../.." && pwd)"
engine="$root/engine"
adapters="${SPECIFY_ADAPTERS:-$root/../specify-adapters}"
sandbox="${SPECIFY_SANDBOX:-$root/evals/.sandbox}/guest-execute-loop"

command -v cursor-agent >/dev/null || {
  echo "cursor-agent not found on PATH; see evals/scenarios/guest-execute-loop.md" >&2
  exit 2
}
command -v jq >/dev/null || { echo "jq not found on PATH" >&2; exit 2; }
[ -d "$adapters/targets/omnia" ] || {
  echo "sibling specify-adapters checkout not found at $adapters (override with SPECIFY_ADAPTERS=)" >&2
  exit 2
}

# Build the binaries under test and the workflow guest.
(
  cd "$engine"
  cargo build -q -p specify -p specify-runtime
  cargo build -q -p specify-workflow-guest --target wasm32-wasip2
)
specify="$engine/target/debug/specify"
runtime="$engine/target/debug/specify-runtime"
workflow_wasm="$engine/target/wasm32-wasip2/debug/specify_workflow_guest.wasm"

# Seed the sandbox: vendored target adapter (symlinks dereferenced — the
# adapter's reference symlinks point into the checkout's shared/ tree,
# which the vendored copy does not carry), init, one-slice intent plan,
# operator Gate-1 stamp. The relative adapter path matters: it resolves
# against the guest's "." preopen at execute time, where the out-of-tree
# manifest cache and absolute host paths do not exist.
rm -rf "$sandbox"
mkdir -p "$sandbox/adapters/targets"
cp -RL "$adapters/targets/omnia" "$sandbox/adapters/targets/omnia"
cd "$sandbox"
"$specify" init ./adapters/targets/omnia --name guest-demo
"$specify" plan create guest-demo \
  --intent "Provide a greeting service with one operation that returns a fixed greeting string."
"$specify" plan add greeting-service --sources intent
"$specify" plan transition guest-demo approved

# The deployment manifest: the checked-in engine/omnia.toml shape with the
# "." mount re-pointed at the sandbox.
addr="${HTTP_ADDR:-127.0.0.1:8094}"
{
  printf '[[guest]]\nid = "workflow"\nsource.path = "%s"\n' "$workflow_wasm"
  printf 'link = ["augentic:specify/source@0.1.0", "augentic:specify/target@0.1.0"]\n\n'
  for id in source:intent source:documentation source:typescript source:screenshots \
            source:captures target:contracts target:omnia target:vectis; do
    axis="${id%%:*}"; name="${id#*:}"
    case "$axis" in source) dir=sources ;; *) dir=targets ;; esac
    printf '[[guest]]\nid = "%s"\nsource.path = "%s/%s/%s/guest.wasm"\n\n' \
      "$id" "$adapters" "$dir" "$name"
  done
  printf '[[mount]]\nname = "."\npath = "%s"\nwritable = true\n\n' "$sandbox"
  for id in source:intent source:documentation source:typescript source:screenshots \
            source:captures target:contracts target:omnia target:vectis; do
    name="${id#*:}"
    printf '[[route.http]]\nprefix = "/mcp/%s"\nguest = "%s"\n\n' "$name" "$id"
  done
  printf '[transport]\ndefault = "in-process"\n'
} > "$sandbox/omnia.toml"

log="$sandbox/guest-execute-loop.log"
echo "guest-execute-loop: sandbox=$sandbox log=$log"

drive() {
  echo "==> specify-runtime -- $*" | tee -a "$log"
  HTTP_ADDR="$addr" \
  SPECIFY_INTENT_MCP_URL="http://$addr/mcp/intent" \
  SPECIFY_OMNIA_MCP_URL="http://$addr/mcp/omnia" \
    "$runtime" run --config "$sandbox/omnia.toml" -- "$@" 2>&1 | tee -a "$log"
}

# The inverted loop: survey through the source:intent guest, then the
# guest execute loop to drained.
drive source survey intent
drive plan execute

# Post-run probes (see evals/shared/assertions.md#guest-execute-loop);
# grading against them is the operator's.
echo "--- probes ---"
"$specify" plan status --format json | jq -c .
grep -c 'status: done' plan.yaml
"$specify" journal show --filter plan.entry.advanced | jq -c .payload
"$specify" journal show --filter slice.merge.succeeded | jq -c .payload
"$specify" journal show --filter slice.archive.created | jq -c .payload
if test ! -f .specify/guest.lock; then echo "guest.lock released"; else echo "guest.lock STILL HELD"; fi
ls crates/ 2>/dev/null || echo "no generated crates/ directory"
echo "guest-execute-loop: done — grade per the scenario and file evals/runs/guest-execute-loop.<result>.md"
