#!/usr/bin/env bash
# Idempotent wasm-pkg publish leg: probe, then build, then publish.
#
# Usage: wkg-publish-idempotent.sh <package-ref> <artifact-path> <build-command...>
#
# Probes the registry for <package-ref> FIRST; only when the identity is
# definitively absent does it run <build-command...> and `wkg publish`
# <artifact-path>. Registry identities are immutable, and skip-if-present
# is the immutability enforcement (DECISIONS.md §"Publishing and
# distribution: one transport, idempotent legs"): it is what prevents a
# re-tag from mutating an already-published version.
#
# The load-bearing invariant is the absent-vs-unreachable distinction:
# the probe treats only a definitive not-found as permission to publish.
# Any other probe failure (network unreachable, auth, timeout, missing
# namespace mapping) aborts the leg non-zero — guessing "absent" on an
# unreachable registry would re-push into an immutable identity.
#
# Requirements:
#   - `wkg` on PATH (CI pins `cargo install wkg@0.15.0 --locked`: the
#     not-found fingerprints below are coupled to wkg's error text and
#     were validated against 0.15.0 — revalidate them when bumping)
#   - a wasm-pkg config mapping the package's namespace to its registry
#     host (CI writes one; see .github/workflows/release-binaries.yaml)
#
# A sibling copy of this script lives in the specify-adapters repo
# (scripts/wkg-publish-idempotent.sh); the probe/fingerprint/abort logic
# must stay semantically identical — apply fixes to both.
set -euo pipefail

if [ "$#" -lt 3 ]; then
    echo "usage: $0 <package-ref> <artifact-path> <build-command...>" >&2
    exit 2
fi

package_ref="$1"
artifact="$2"
shift 2

probe_dir="$(mktemp -d)"
trap 'rm -rf "${probe_dir}"' EXIT

# Probe: `wkg get` succeeds only when the exact identity resolves in the
# registry. Three outcomes:
#   1. success                → already published; skip and exit 0.
#   2. definitive not-found   → proceed to build + publish.
#   3. anything else          → abort: absent and unreachable are
#                               indistinguishable, so publishing is unsafe.
echo "probing registry for ${package_ref}"
if probe_output="$(wkg get "${package_ref}" --output "${probe_dir}/probe.wasm" --overwrite 2>&1)"; then
    echo "${package_ref} already published; skipping (immutable identity)"
    exit 0
fi

# Case-insensitive not-found fingerprints, deliberately registry/OCI
# specific. "manifest unknown" is what a missing tag actually surfaces
# as through the OCI distribution API (e.g. GHCR); the rest cover
# warg/HTTP phrasings. A bare "failed to resolve" is NOT matched — it
# overlaps with DNS/connection failures, which must take the abort path
# below; only the package-scoped phrasing counts as a definitive miss.
if printf '%s' "${probe_output}" | grep -qiE 'manifest unknown|not found|404|no such (tag|manifest|package|version)|does not exist|failed to resolve package'; then
    echo "${package_ref} not present in registry; building and publishing"
else
    {
        echo "error: registry probe for ${package_ref} failed for a reason other"
        echo "than not-found (network unreachable? auth? timeout?). This leg"
        echo "cannot distinguish absent from unreachable, so it aborts rather"
        echo "than risk re-publishing an immutable identity."
        echo "--- probe output ---"
        printf '%s\n' "${probe_output}"
    } >&2
    exit 1
fi

"$@"

if [ ! -s "${artifact}" ]; then
    echo "error: build produced no artifact at ${artifact}" >&2
    exit 1
fi

wkg publish "${artifact}" --package "${package_ref}"
echo "published ${package_ref}"
