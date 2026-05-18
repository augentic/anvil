# Init and capability issues

Use this page when `/spec:init` cannot resolve a capability or skills appear to be running against stale capability content.

## Prerequisites

- A repo where you have just run, or are about to run, `/spec:init <capability>`.
- The capability identifier or URL you supplied (and the error message, if any).

## Capability resolution failure

**Symptom:** `/spec:init` fails to resolve the capability identifier.

**Cause:** Invalid identifier or URL, network error, or the `@ref` suffix does not exist.

**Resolution:**
1. Verify the identifier format: a bare name (e.g. `omnia`), an `https://github.com/augentic/specify/capabilities/<name>[@<ref>]` URL, or a `file:///…` URI.
2. Check network connectivity.
3. Try without a ref suffix to use the latest version.

## Cache stale after capability update

**Symptom:** Skills use outdated brief content.

**Cause:** The capability was updated upstream but the local cache was not refreshed.

**Resolution:** Re-run `/spec:init <capability>` to refresh the cache.
