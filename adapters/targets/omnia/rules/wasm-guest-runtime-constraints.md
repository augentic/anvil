---
id: OMNIA-002
title: WASM Guest Runtime Constraints
severity: critical
trigger: An Omnia guest relies on process, thread, filesystem, environment, randomness, mutable global state, blocking I/O, or native runtime behavior unavailable in wasm32-wasip2.
rule_hints:
  - kind: path-pattern
    value: '**/*.rs'
    description: Restrict the forbidden-std scan to Rust source files.
  - kind: regex
    value: '\bstd::(env|fs|net|process|thread::spawn)\b'
    description: Forbidden std namespace in Omnia WASM guest code.
---

## Rule

Omnia crates must run as stateless `wasm32-wasip2` guest components. Production code must not rely on native process features, background threads, direct filesystem/network/environment access, mutable global state, runtime singletons, blocking I/O, or crates that require an OS-backed runtime. State that must outlive one invocation belongs in host providers, not component memory.

Compile-time constants and immutable `LazyLock` lookup tables are acceptable when they do not cache request data, credentials, provider results, or runtime state.

## Look For

- `std::env`, `std::fs`, `std::net`, `std::process`, `std::thread::spawn`, or synchronous I/O used in production guest code.
- Runtime dependencies such as `tokio` as an application runtime, native `hyper` servers, `rand`, `uuid` generation that depends on runtime randomness, or dotenv-style configuration loaders.
- `static mut`, `OnceCell`, `OnceLock`, `lazy_static!`, or `Mutex<HashMap<...>>` used as caches or mutable singleton state.
- Background task, timer, circuit-breaker, worker-pool, retry daemon, or startup-loader patterns copied from a server process.
- Assumptions that data loaded once will remain available across handler invocations.
- Direct use of system clocks or sleeps for scheduling instead of invocation-time logic and provider-backed state.

## Spec Guidance

When source behavior depends on process lifetime, translate the requirement into an Omnia runtime pattern. Startup caches usually become cache-aside reads through `StateStore` plus the original data provider; periodic refresh usually becomes TTL-backed cache expiry; token caches usually become `Identity` provider calls.
