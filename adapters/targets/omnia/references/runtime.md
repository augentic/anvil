# Runtime Setup

Local development runtime configuration using `omnia::runtime!` macro.

For provider configuration and trait composition, see [providers/README.md](providers/README.md).

---

## Runtime Example

Create `examples/<guest-name>.rs`. Include all WASI hosts used by the guest. The example below shows all 10 available hosts:

```rust
cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use omnia_wasi_blobstore::{WasiBlobstore, BlobstoreDefault};
        use omnia_wasi_config::{WasiConfig, ConfigDefault};
        use omnia_wasi_http::{WasiHttp, HttpDefault};
        use omnia_wasi_identity::{WasiIdentity, IdentityDefault};
        use omnia_wasi_jsondb::{WasiJsonDb, JsonDbDefault};
        use omnia_wasi_keyvalue::{WasiKeyValue, KeyValueDefault};
        use omnia_wasi_messaging::{WasiMessaging, MessagingDefault};
        use omnia_wasi_otel::{WasiOtel, OtelDefault};
        use omnia_wasi_sql::{WasiSql, SqlDefault};
        use omnia_wasi_websocket::{WasiWebSocket, WebSocketDefault};

        omnia::runtime!({
            main: true,
            hosts: {
                WasiBlobstore: BlobstoreDefault,
                WasiConfig: ConfigDefault,
                WasiHttp: HttpDefault,
                WasiIdentity: IdentityDefault,
                WasiJsonDb: JsonDbDefault,
                WasiKeyValue: KeyValueDefault,
                WasiMessaging: MessagingDefault,
                WasiOtel: OtelDefault,
                WasiSql: SqlDefault,
                WasiWebSocket: WebSocketDefault,
            }
        });
    } else {
        // HACK: prevent lint error for wasm32 target
        fn main() {}
    }
}
```

## WASI Host Options

| Host            | Default            | Purpose                     |
| --------------- | ------------------ | --------------------------- |
| `WasiConfig`    | `ConfigDefault`    | Environment variable access |
| `WasiHttp`      | `HttpDefault`      | HTTP client requests        |
| `WasiIdentity`  | `IdentityDefault`  | Authentication tokens       |
| `WasiKeyValue`  | `KeyValueDefault`  | Cache/KV storage            |
| `WasiMessaging` | `MessagingDefault` | Message pub/sub             |
| `WasiOtel`      | `OtelDefault`      | OpenTelemetry tracing       |
| `WasiSql`       | `SqlDefault`       | Database connections        |
| `WasiWebSocket` | `WebSocketDefault` | WebSocket event handling    |
| `WasiBlobstore` | `BlobstoreDefault` | Binary blob storage         |
| `WasiJsonDb`    | `JsonDbDefault`    | JSON document storage       |

## Environment Variables

Create `examples/.env.example` with all required config keys documented. Use module-level `RUST_LOG` filtering to enable debug logging for the guest and Omnia subsystems:

```bash
# Logging -- use module-level filtering for debugging
RUST_LOG="info,omnia_wasi_http=debug,omnia_wasi_messaging=debug,<guest-name>=debug"

# Service config
API_URL=https://api.example.com
SERVICE_NAME=my-service
```

### Identity Environment Variables

When `WasiIdentity` is used, the runtime requires OAuth2 credentials:

```bash
# Required when WasiIdentity is enabled
IDENTITY_CLIENT_ID="<client_id>"
IDENTITY_CLIENT_SECRET="<client_secret>"
IDENTITY_TOKEN_URL="<token endpoint>"
```

Include these in `.env.example` whenever the guest's Provider implements `Identity`.

## Running

```bash
source examples/.env.example && cargo run --example <guest-name>
```

## Conditional Compilation

The `cfg_if` macro handles platform-specific compilation:

- `target_arch = "wasm32"` -- Empty main (WASM deployment)
- `not(target_arch = "wasm32")` -- Full runtime (native testing)

## References

- [providers/README.md](providers/README.md) -- Provider configuration and trait composition
- [capabilities.md](capabilities.md) -- Trait definitions and method signatures
- [guest-patterns.md](guest-patterns.md) -- Guest export patterns (HTTP, Messaging, WebSocket)
