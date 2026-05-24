# Generated type conventions

The codegen binary produces two sets of Kotlin files:

1. **Bincode types** in `generated/com/example/app/` -- `Event`, `ViewModel`, `Effect`, `Request`, `Requests`, and all view structs, enums, and capability types (`HttpRequest`, `HttpResponse`, `SseRequest`, `SseResponse`, `KeyValueOperation`, `TimeRequest`, `TimeResponse`, `Filter`, etc.)
2. **UniFFI bindings** in `generated/uniffi/shared/` -- the `CoreFfi` class that bridges to the Rust native library.

## Import rules for all hand-written Kotlin files

Every `.kt` file that references generated types MUST have explicit imports:

```kotlin
// For bincode types (Event, ViewModel, Effect, etc.)
import com.example.app.Event
import com.example.app.ViewModel
import com.example.app.Effect
// ... import each type individually

// For the CoreFfi bridge (only in Core.kt)
import uniffi.shared.CoreFfi
```

**NEVER** assume these types are in the same package as the hand-written code. The hand-written code lives in `com.vectis.{appname}` but the generated types are in `com.example.app` and `uniffi.shared`.

## Enum class naming conventions

Simple Rust enums without payloads (e.g., `Filter { All, Active, Completed }`, `SyncStatus { Idle, Syncing, Offline }`, `SseState`) are generated as Kotlin `enum class` with **UPPER_CASE** values:

```kotlin
// Generated as:
enum class Filter { ALL, ACTIVE, COMPLETED }
enum class SyncStatus { IDLE, SYNCING, OFFLINE }
enum class SseState { DISCONNECTED, CONNECTING, CONNECTED }
```

Pattern match with `==` equality, NOT `is`:

```kotlin
// CORRECT:
when (filter) {
    Filter.ALL -> ...
    Filter.ACTIVE -> ...
    Filter.COMPLETED -> ...
}

// WRONG (will not compile):
when (filter) {
    is Filter.All -> ...    // ← enum values are not types
}
```

## Sealed interface naming conventions

Rust enums WITH payloads (e.g., `Event`, `ViewModel`, `Effect`) are generated as Kotlin `sealed interface` with nested `data class` or `data object` variants:

```kotlin
// Generated as:
sealed interface Event {
    data class Navigate(val value: Route) : Event
    data class SetNewTitle(val value: String) : Event
    data object ClearCompleted : Event     // unit variant → data object
}
```

Pattern match with `is` for data classes, direct reference for data objects:

```kotlin
when (event) {
    is Event.Navigate -> event.value       // data class
    is Event.SetNewTitle -> event.value    // data class
    Event.ClearCompleted -> ...            // data object (no `is`)
}
```

## Numeric type mapping

| Rust type | Kotlin generated type | Notes |
|---|---|---|
| `usize` / `u64` | `ULong` | Use `.toLong()` when passing to Compose UI that expects `Long` |
| `u32` | `UInt` | Effect IDs are `UInt` |
| `u16` | `UShort` | HTTP status codes |
| `Vec<u8>` | `List<UByte>` | Use `.toUByteArray().toList()` to convert from `ByteArray` |

## KeyValue types

- `Value.Bytes` takes `List<UByte>` (not `List<Byte>`) -- convert with `byteArray.toUByteArray().toList()`
- `KeyValueOperation.Set.value` is `List<UByte>` -- convert back with `op.value.map { it.toByte() }.toByteArray()`
- `KeyValueResponse.ListKeys` takes `(keys: List<String>, nextCursor: ULong)` -- pass `0UL` for no more keys, NOT a `String`
- `KeyValueError` is a sealed interface with variants `Io`, `Timeout`, `CursorNotFound`, `Other` -- use `KeyValueError.Other(msg)`, NOT `KeyValueError(msg)`

## Time types

- `Duration` has a single field `nanos: ULong` (total nanoseconds), NOT separate `secs`/`nanos` fields
- `TimeRequest` variants: `Now`, `NotifyAt(id, instant)`, `NotifyAfter(id, duration)`, `Clear(id)` -- each has a `TimerId` field
- `TimeResponse` variants: `Now(instant)`, `InstantArrived(id)`, `DurationElapsed(id)`, `Cleared(id)` -- NOT `DURATIONREACHED`
- `NotifyAfter` and `NotifyAt` handlers must store their coroutine `Job` in a `timerJobs` map keyed by `TimerId`. `Clear` must cancel and remove the stored job before responding with `Cleared`. Without job tracking, cleared timers continue to fire stale events into the core.

## @OptIn annotations

Classes that call `.toUByteArray()` need:

```kotlin
@OptIn(ExperimentalUnsignedTypes::class)
class SseClient { ... }
```
