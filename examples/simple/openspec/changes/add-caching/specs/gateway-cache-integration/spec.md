# gateway-cache-integration spec

## MODIFIED: request pipeline
Added cache lookup step before upstream dispatch.
Respects `Cache-Control: no-cache` to bypass.
