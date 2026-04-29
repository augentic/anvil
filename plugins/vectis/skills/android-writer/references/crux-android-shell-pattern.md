# Crux Android Shell Pattern (0.17+ API)

The Android shell is a thin Kotlin/Jetpack Compose layer that renders the `ViewModel` from the Crux core and sends user-initiated `Event` values back. All business logic lives in the shared Rust crate; the shell only handles platform I/O (HTTP, KV, SSE, Time, Platform) and UI rendering.

## Architecture

```
┌──────────────────────────────────────────────────┐
│  Jetpack Compose UI                              │
│  ┌────────────┐  ┌────────────┐  ┌───────────┐  │
│  │ MainScreen │  │ ErrorScreen│  │ Loading   │  │
│  └──────┬─────┘  └──────┬─────┘  └─────┬─────┘  │
│         │               │               │        │
│         └───────┬───────┘───────────────┘        │
│                 ▼                                 │
│          Root Composable                         │
│           when (state) { ... }                   │
│                 │                                 │
│                 ▼                                 │
│          Core (wrapper)                          │
│          ┌────────────────────────────────────┐  │
│          │ viewModel: StateFlow<ViewModel>    │  │
│          │ fun update(event: Event)           │  │
│          │ fun processRequest(req: Request)   │  │
│          └──────────────┬─────────────────────┘  │
│                         │                        │
│                         ▼                        │
│               CoreFfi (UniFFI bridge)            │
│               .update(data) → effects            │
│               .resolve(id, data) → effects       │
│               .view() → viewModel                │
└──────────────────────────────────────────────────┘
                          │
                          ▼
               ┌────────────────────┐
               │  Rust shared crate │
               │  (dynamic library) │
               └────────────────────┘
```

## Core.kt

The `Core` class is the bridge between Compose and the Rust core. Two patterns are supported based on complexity.

### Pattern 1: Simple Core (Render only)

For apps with only the `Render` effect. `Core` extends `androidx.lifecycle.ViewModel` and uses Compose `mutableStateOf`.

```kotlin
package com.vectis.counter.core

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.example.app.Effect
import com.example.app.Event
import com.example.app.Request
import com.example.app.Requests
import com.example.app.ViewModel
import uniffi.shared.CoreFfi

open class Core : androidx.lifecycle.ViewModel() {
    private var coreFfi: CoreFfi = CoreFfi()

    var view: ViewModel by mutableStateOf(
        ViewModel.bincodeDeserialize(coreFfi.view())
    )
        private set

    fun update(event: Event) {
        val effects = coreFfi.update(event.bincodeSerialize())
        val requests = Requests.bincodeDeserialize(effects)
        for (request in requests) {
            processRequest(request)
        }
    }

    private fun processRequest(request: Request) {
        when (val effect = request.effect) {
            is Effect.Render -> {
                this.view = ViewModel.bincodeDeserialize(coreFfi.view())
            }
        }
    }
}
```

### Pattern 2: Full Core (with side effects)

For apps with HTTP, SSE, KV, Time, or Platform effects. Uses coroutines and `StateFlow`. Injected via Koin.

**CRITICAL**: All `scope.launch` blocks for async effects (SSE, Time) MUST wrap their body in `try/catch` to prevent unhandled exceptions from crashing the app. Always rethrow `CancellationException`. Catch blocks MUST call `resolveAndHandleEffects` with a fallback response (e.g., `SseResponse.Done` for SSE, `TimeResponse.DurationElapsed` / `TimeResponse.InstantArrived` for timers) so the core request ID is never left unresolved.

```kotlin
package com.vectis.myapp.core

import android.util.Log
import com.example.app.*
import uniffi.shared.CoreFfi
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

private const val TAG = "Core"

class Core(
    private val httpClient: HttpClient,
    private val sseClient: SseClient,
) {
    private val coreFfi = CoreFfi()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private val timerJobs = mutableMapOf<TimerId, Job>()

    private val _viewModel: MutableStateFlow<ViewModel> =
        MutableStateFlow(getViewModel())
    val viewModel: StateFlow<ViewModel> = _viewModel.asStateFlow()

    fun update(event: Event) {
        scope.launch {
            val effects = coreFfi.update(event.bincodeSerialize())
            handleEffects(effects)
        }
    }

    private suspend fun handleEffects(effects: ByteArray) {
        val requests = Requests.bincodeDeserialize(effects)
        for (request in requests) {
            processRequest(request)
        }
    }

    private suspend fun processRequest(request: Request) {
        when (val effect = request.effect) {
            is Effect.Render -> {
                _viewModel.value = getViewModel()
            }
            is Effect.Http -> {
                val result = httpClient.request(effect.value)
                resolveAndHandleEffects(request.id, result.bincodeSerialize())
            }
            is Effect.ServerSentEvents -> {
                scope.launch {
                    try {
                        sseClient.request(effect.value) { response ->
                            resolveAndHandleEffects(request.id, response.bincodeSerialize())
                        }
                    } catch (e: CancellationException) { throw e }
                    catch (e: Exception) {
                        Log.e(TAG, "SSE error: ${e.message}", e)
                        resolveAndHandleEffects(request.id, SseResponse.Done.bincodeSerialize())
                    }
                }
            }
            is Effect.Time -> {
                scope.launch {
                    try {
                        handleTimeEffect(request.id, effect.value)
                    } catch (e: CancellationException) { throw e }
                    catch (e: Exception) { Log.e(TAG, "Time effect error", e) }
                }
            }
            is Effect.Platform -> {
                handlePlatformEffect(request.id)
            }
            is Effect.KeyValue -> {
                val result = keyValueClient.perform(effect.value)
                resolveAndHandleEffects(request.id, result.bincodeSerialize())
            }
        }
    }

    private suspend fun resolveAndHandleEffects(requestId: UInt, data: ByteArray) {
        val effects = coreFfi.resolve(requestId, data)
        handleEffects(effects)
    }

    private fun getViewModel(): ViewModel =
        ViewModel.bincodeDeserialize(coreFfi.view())
}
```

### Core with HTTP Capability

Add the `is Effect.Http` case. Delegate to an `HttpClient` class.

```kotlin
is Effect.Http -> {
    val result = httpClient.request(effect.value)
    resolveAndHandleEffects(request.id, result.bincodeSerialize())
}
```

### Core with Time Capability

The Time capability has multiple request types. Handle each variant:

- `TimeRequest.Now` -- respond with current time
- `TimeRequest.NotifyAfter(id, duration)` -- delay, then respond with `DurationElapsed(id)`
- `TimeRequest.NotifyAt(id, instant)` -- delay until instant, then respond with `InstantArrived(id)`
- `TimeRequest.Clear(id)` -- cancel a pending timer's coroutine job, then respond with `Cleared(id)`

**CRITICAL**: `Duration` has a single `nanos: ULong` field (total nanoseconds), NOT separate `secs`/`nanos`. `TimeResponse` variants use `DurationElapsed`, `InstantArrived`, and `Cleared` -- not `DURATIONREACHED` or `NOTIFYREACHED`.

**CRITICAL**: `NotifyAfter` and `NotifyAt` must store their coroutine `Job` in `timerJobs` keyed by `TimerId`. `Clear` must cancel and remove the stored job before responding. Without this, cleared timers continue to fire and deliver stale `DurationElapsed` or `InstantArrived` events to the core. The map is safe to access without synchronization because all coroutines run on `Dispatchers.Main.immediate`.

```kotlin
import com.example.app.Instant
import com.example.app.TimerId
import com.example.app.TimeRequest
import com.example.app.TimeResponse
import java.time.ZoneOffset
import java.time.ZonedDateTime

private suspend fun handleTimeEffect(requestId: UInt, timeRequest: TimeRequest) {
    when (timeRequest) {
        is TimeRequest.Now -> {
            val now = ZonedDateTime.now(ZoneOffset.UTC)
            val response = TimeResponse.Now(
                Instant(now.toEpochSecond().toULong(), now.nano.toUInt())
            )
            resolveAndHandleEffects(requestId, response.bincodeSerialize())
        }
        is TimeRequest.NotifyAfter -> {
            val timerId = timeRequest.value.id
            timerJobs[timerId] = scope.launch {
                try {
                    val millis = timeRequest.value.duration.nanos / 1_000_000UL
                    delay(millis.toLong())
                    timerJobs.remove(timerId)
                    val response = TimeResponse.DurationElapsed(timerId)
                    resolveAndHandleEffects(requestId, response.bincodeSerialize())
                } catch (e: CancellationException) {
                    timerJobs.remove(timerId)
                    throw e
                } catch (e: Exception) {
                    timerJobs.remove(timerId)
                    Log.e(TAG, "Timer NotifyAfter error", e)
                    resolveAndHandleEffects(requestId, TimeResponse.DurationElapsed(timerId).bincodeSerialize())
                }
            }
        }
        is TimeRequest.NotifyAt -> {
            val timerId = timeRequest.value.id
            timerJobs[timerId] = scope.launch {
                try {
                    val now = java.time.Instant.now()
                    val target = java.time.Instant.ofEpochSecond(
                        timeRequest.value.instant.seconds.toLong(),
                        timeRequest.value.instant.nanos.toLong()
                    )
                    val delayMs = java.time.Duration.between(now, target).toMillis()
                    if (delayMs > 0) delay(delayMs)
                    timerJobs.remove(timerId)
                    val response = TimeResponse.InstantArrived(timerId)
                    resolveAndHandleEffects(requestId, response.bincodeSerialize())
                } catch (e: CancellationException) {
                    timerJobs.remove(timerId)
                    throw e
                } catch (e: Exception) {
                    timerJobs.remove(timerId)
                    Log.e(TAG, "Timer NotifyAt error", e)
                    resolveAndHandleEffects(requestId, TimeResponse.InstantArrived(timerId).bincodeSerialize())
                }
            }
        }
        is TimeRequest.Clear -> {
            timerJobs.remove(timeRequest.value)?.cancel()
            val response = TimeResponse.Cleared(timeRequest.value)
            resolveAndHandleEffects(requestId, response.bincodeSerialize())
        }
    }
}
```

### Core with Platform Capability

Use `android.os.Build` for platform info:

```kotlin
import android.os.Build

private suspend fun handlePlatformEffect(requestId: UInt) {
    val response = PlatformResponse(Build.BRAND + " " + Build.VERSION.RELEASE)
    resolveAndHandleEffects(requestId, response.bincodeSerialize())
}
```

### Core with SSE Capability

SSE produces a stream of values. Each value is resolved against the same request ID, producing a new batch of effects each time. MUST be wrapped in `scope.launch` with `try/catch`:

```kotlin
is Effect.ServerSentEvents -> {
    scope.launch {
        try {
            sseClient.request(effect.value) { response ->
                resolveAndHandleEffects(request.id, response.bincodeSerialize())
            }
        } catch (e: CancellationException) { throw e }
        catch (e: Exception) {
            Log.e(TAG, "SSE error: ${e.message}", e)
            resolveAndHandleEffects(request.id, SseResponse.Done.bincodeSerialize())
        }
    }
}
```

### Core with Key-Value Capability

Use Android SharedPreferences for key-value storage:

```kotlin
is Effect.KeyValue -> {
    val result = keyValueClient.perform(effect.value)
    resolveAndHandleEffects(request.id, result.bincodeSerialize())
}
```

## HttpClient.kt

Full HTTP client implementation using Ktor + OkHttp:

```kotlin
package com.vectis.myapp.core

import com.example.app.HttpError
import com.example.app.HttpHeader
import com.example.app.HttpRequest
import com.example.app.HttpResponse
import com.example.app.HttpResult
import com.novi.serde.Bytes
import io.ktor.client.call.body
import io.ktor.client.engine.okhttp.OkHttp
import io.ktor.client.plugins.*
import io.ktor.client.plugins.logging.*
import io.ktor.client.request.*
import io.ktor.http.HttpMethod
import io.ktor.util.flattenEntries
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import io.ktor.client.HttpClient as KtorHttpClient

class HttpClient {
    private val ktorHttpClient = KtorHttpClient(OkHttp) {
        install(Logging) {
            logger = Logger.DEFAULT
            level = LogLevel.ALL
        }
        install(HttpTimeout) {
            requestTimeoutMillis = 30_000
            connectTimeoutMillis = 15_000
            socketTimeoutMillis = 15_000
        }
    }

    suspend fun request(request: HttpRequest): HttpResult =
        withContext(Dispatchers.Default) {
            try {
                val response = requestResponse(request)
                HttpResult.Ok(response)
            } catch (ce: CancellationException) {
                throw ce
            } catch (error: Throwable) {
                HttpResult.Err(toHttpError(error))
            }
        }

    private suspend fun requestResponse(
        request: HttpRequest
    ): HttpResponse {
        val response = ktorHttpClient.request(request.url) {
            this.method = HttpMethod.parse(request.method)
            this.headers {
                for (header in request.headers) {
                    append(header.name, header.value)
                }
            }
            if (request.body.content.isNotEmpty()) {
                setBody(request.body.content)
            }
        }
        val bytes: ByteArray = response.body()
        val headers = response.headers.flattenEntries().map {
            HttpHeader(it.first, it.second)
        }
        return HttpResponse(
            response.status.value.toUShort(),
            headers,
            Bytes(bytes)
        )
    }

    private fun toHttpError(error: Throwable): HttpError = when (error) {
        is HttpRequestTimeoutException -> HttpError.Timeout
        is IllegalArgumentException -> HttpError.Url(
            error.message ?: "Invalid URL"
        )
        else -> HttpError.Io(error.message ?: "HTTP request failed")
    }
}
```

## SseClient.kt

SSE streaming client using Ktor. Note the `@OptIn(ExperimentalUnsignedTypes::class)` annotation -- required because `toUByteArray()` is experimental.

```kotlin
package com.vectis.myapp.core

import com.example.app.SseRequest
import com.example.app.SseResponse
import io.ktor.client.engine.okhttp.OkHttp
import io.ktor.client.plugins.HttpTimeout
import io.ktor.client.request.prepareGet
import io.ktor.client.statement.bodyAsChannel
import io.ktor.utils.io.readLine
import io.ktor.client.HttpClient

@OptIn(ExperimentalUnsignedTypes::class)
class SseClient {
    private val httpClient = HttpClient(OkHttp) {
        install(HttpTimeout) {
            requestTimeoutMillis = Long.MAX_VALUE
            socketTimeoutMillis = Long.MAX_VALUE
        }
    }

    suspend fun request(
        request: SseRequest,
        callback: suspend (SseResponse) -> Unit
    ) {
        httpClient.prepareGet(request.url).execute { response ->
            val channel = response.bodyAsChannel()
            while (!channel.isClosedForRead) {
                var chunk = channel.readLine() ?: break
                chunk += "\n\n"
                callback(
                    SseResponse.Chunk(
                        chunk.toByteArray().toUByteArray().toList()
                    )
                )
            }
            callback(SseResponse.Done)
        }
    }
}
```

## KeyValueClient.kt

Key-Value storage using SharedPreferences.

**CRITICAL type notes:**
- `Value.Bytes` takes `List<UByte>` -- convert with `.toUByteArray().toList()`
- `KeyValueOperation.Set.value` is `List<UByte>` -- convert back with `.map { it.toByte() }.toByteArray()`
- `KeyValueResponse.ListKeys` second param is `ULong` (cursor), NOT `String`
- `KeyValueError` is a sealed interface -- use `KeyValueError.Other(msg)`, NOT `KeyValueError(msg)`

```kotlin
package com.vectis.myapp.core

import android.content.Context
import android.content.SharedPreferences
import com.example.app.KeyValueError
import com.example.app.KeyValueOperation
import com.example.app.KeyValueResponse
import com.example.app.KeyValueResult
import com.example.app.Value

@OptIn(ExperimentalUnsignedTypes::class)
class KeyValueClient(context: Context) {
    private val prefs: SharedPreferences =
        context.getSharedPreferences("crux_kv", Context.MODE_PRIVATE)

    fun perform(op: KeyValueOperation): KeyValueResult {
        return try {
            val response = when (op) {
                is KeyValueOperation.Get -> {
                    val value = prefs.getString(op.key, null)
                    if (value != null) {
                        KeyValueResponse.Get(
                            Value.Bytes(value.toByteArray().toUByteArray().toList())
                        )
                    } else {
                        KeyValueResponse.Get(Value.None)
                    }
                }
                is KeyValueOperation.Set -> {
                    val previous = prefs.getString(op.key, null)
                    val bytes = op.value.map { it.toByte() }.toByteArray()
                    prefs.edit().putString(op.key, String(bytes)).apply()
                    val prev = if (previous != null) {
                        Value.Bytes(previous.toByteArray().toUByteArray().toList())
                    } else {
                        Value.None
                    }
                    KeyValueResponse.Set(prev)
                }
                is KeyValueOperation.Delete -> {
                    val previous = prefs.getString(op.key, null)
                    prefs.edit().remove(op.key).apply()
                    val prev = if (previous != null) {
                        Value.Bytes(previous.toByteArray().toUByteArray().toList())
                    } else {
                        Value.None
                    }
                    KeyValueResponse.Delete(prev)
                }
                is KeyValueOperation.Exists -> {
                    KeyValueResponse.Exists(prefs.contains(op.key))
                }
                is KeyValueOperation.ListKeys -> {
                    val allKeys = prefs.all.keys
                        .filter { it.startsWith(op.prefix) }
                        .sorted()
                    KeyValueResponse.ListKeys(allKeys, 0UL)
                }
            }
            KeyValueResult.Ok(response)
        } catch (e: Exception) {
            KeyValueResult.Err(
                KeyValueError.Other(e.message ?: "KeyValue operation failed")
            )
        }
    }
}
```

## Serialization Protocol

All data crossing the FFI boundary uses **Bincode** serialization via the generated `bincodeSerialize()` and `bincodeDeserialize()` methods on the shared types.

| Direction | Data | Serialization |
|-----------|------|---------------|
| Shell → Core | `Event` | `event.bincodeSerialize()` → `coreFfi.update(data)` |
| Core → Shell | Effect requests | `coreFfi.update(data)` → `Requests.bincodeDeserialize(effects)` |
| Shell → Core | Effect response | `response.bincodeSerialize()` → `coreFfi.resolve(id, data)` |
| Core → Shell | `ViewModel` | `coreFfi.view()` → `ViewModel.bincodeDeserialize(bytes)` |

## Effect Loop

The effect processing loop is recursive: resolving one effect may produce additional effects. The loop runs until no more effects are returned.

```
User taps button
    → coreFfi.update(Event.ButtonTapped.bincodeSerialize())
    → [Request(id: 1, effect: Effect.Http(...))]
    → perform HTTP request
    → coreFfi.resolve(1, httpResponse.bincodeSerialize())
    → [Request(id: 2, effect: Effect.Render)]
    → update StateFlow with new ViewModel
    → Compose recomposes
```

## Initialization

Send an initialization event when the app starts. This triggers the core to load persisted state or fetch initial data. The exact event name depends on what the Crux core defines -- check the generated types in `generated/`.

```kotlin
// In Activity.onCreate or after Core creation
// Use the actual event variant defined in app.rs, e.g.:
core.update(Event.StartWatch)    // if core defines Event::StartWatch
core.update(Event.Navigate(Route.MAIN))  // if using route-based init
```

Note: `Event` is typically a sealed interface (mixed enum) so unit variants use PascalCase as `data object` (e.g., `Event.StartWatch`). Only simple enums where *every* variant is a unit use `UPPER_CASE` as `enum class` values (e.g., `Filter.ALL`). See the "Enum mapping" section below for details.

## Thread Safety

- Simple Core pattern: `mutableStateOf` is thread-safe for Compose reads.
- Full Core pattern: `StateFlow` is thread-safe; all mutations happen on `Dispatchers.Main.immediate`.
- `CoreFfi` is thread-safe internally (Rust `Bridge` uses interior mutability).
- Async effect handlers run in coroutines scoped to `SupervisorJob`, so one failure doesn't cancel other in-flight effects.

## Application Class (Required)

Every Android shell MUST have an Application class that sets the UniFFI library override in `onCreate()`, BEFORE any UniFFI class is loaded. Without it, JNA tries to load `libuniffi_shared.so` which doesn't exist -- Cargo produces `libshared.so` -- causing an `UnsatisfiedLinkError` crash on launch.

### Minimal Application class (no Koin)

```kotlin
package com.vectis.myapp

import android.app.Application

class MyAppApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        System.setProperty("uniffi.component.shared.libraryOverride", "shared")
    }
}
```

### Application class with Koin

When using the full Core pattern with DI, add `startKoin` after the library override:

```kotlin
package com.vectis.myapp

import android.app.Application
import com.vectis.myapp.di.appModule
import org.koin.android.ext.koin.androidContext
import org.koin.core.context.startKoin

class MyAppApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        System.setProperty("uniffi.component.shared.libraryOverride", "shared")
        startKoin {
            androidContext(this@MyAppApplication)
            modules(appModule)
        }
    }
}
```

## Crash Recovery Handler

Compose has no equivalent of React's `ErrorBoundary` -- layout-phase exceptions propagate as unhandled crashes on the main thread and cannot be caught at the composable level. A global uncaught exception handler in the Application class converts unrecoverable crashes into graceful Activity restarts. This is especially effective for Crux apps because the core manages state independently of the shell -- restarting the Activity re-creates the `Core`, which re-renders the current ViewModel. If KV persistence is used, the core recovers its state from storage, so the user sees at most a brief flash rather than the app disappearing.

This is a last-resort safety net, not a substitute for fixing the underlying bug. The handler catches all uncaught exceptions, not just Compose layout crashes.

### Application Class with Crash Recovery (minimal)

```kotlin
package com.vectis.myapp

import android.app.AlarmManager
import android.app.Application
import android.app.PendingIntent
import android.content.Intent
import android.util.Log

private const val CRASH_LOOP_WINDOW_MS = 10_000L

class MyAppApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        System.setProperty("uniffi.component.shared.libraryOverride", "shared")
        installCrashRecoveryHandler()
    }

    private fun installCrashRecoveryHandler() {
        val defaultHandler = Thread.getDefaultUncaughtExceptionHandler()

        Thread.setDefaultUncaughtExceptionHandler { thread, throwable ->
            try {
                Log.e("CrashRecovery", "Uncaught exception on ${thread.name}", throwable)

                val prefs = getSharedPreferences("crux_crash_recovery", MODE_PRIVATE)
                val lastCrash = prefs.getLong("last_crash_ms", 0)
                val now = System.currentTimeMillis()

                prefs.edit()
                    .putBoolean("crashed", true)
                    .putString("crash_summary", throwable.message ?: throwable::class.simpleName)
                    .putLong("last_crash_ms", now)
                    .commit()

                if (now - lastCrash > CRASH_LOOP_WINDOW_MS) {
                    val intent = packageManager.getLaunchIntentForPackage(packageName)
                    if (intent != null) {
                        intent.addFlags(
                            Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
                        )
                        val pending = PendingIntent.getActivity(
                            this@MyAppApplication, 0, intent,
                            PendingIntent.FLAG_ONE_SHOT or PendingIntent.FLAG_IMMUTABLE
                        )
                        val alarm = getSystemService(ALARM_SERVICE) as AlarmManager
                        alarm.set(AlarmManager.RTC, now + 200, pending)
                    }
                }
            } catch (e: Exception) {
                Log.e("CrashRecovery", "Failed to schedule restart", e)
            }

            defaultHandler?.uncaughtException(thread, throwable)
        }
    }
}
```

### Application Class with Crash Recovery (Koin)

```kotlin
package com.vectis.myapp

import android.app.AlarmManager
import android.app.Application
import android.app.PendingIntent
import android.content.Intent
import android.util.Log
import com.vectis.myapp.di.appModule
import org.koin.android.ext.koin.androidContext
import org.koin.core.context.startKoin

private const val CRASH_LOOP_WINDOW_MS = 10_000L

class MyAppApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        System.setProperty("uniffi.component.shared.libraryOverride", "shared")
        installCrashRecoveryHandler()
        startKoin {
            androidContext(this@MyAppApplication)
            modules(appModule)
        }
    }

    private fun installCrashRecoveryHandler() {
        val defaultHandler = Thread.getDefaultUncaughtExceptionHandler()

        Thread.setDefaultUncaughtExceptionHandler { thread, throwable ->
            try {
                Log.e("CrashRecovery", "Uncaught exception on ${thread.name}", throwable)

                val prefs = getSharedPreferences("crux_crash_recovery", MODE_PRIVATE)
                val lastCrash = prefs.getLong("last_crash_ms", 0)
                val now = System.currentTimeMillis()

                prefs.edit()
                    .putBoolean("crashed", true)
                    .putString("crash_summary", throwable.message ?: throwable::class.simpleName)
                    .putLong("last_crash_ms", now)
                    .commit()

                if (now - lastCrash > CRASH_LOOP_WINDOW_MS) {
                    val intent = packageManager.getLaunchIntentForPackage(packageName)
                    if (intent != null) {
                        intent.addFlags(
                            Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
                        )
                        val pending = PendingIntent.getActivity(
                            this@MyAppApplication, 0, intent,
                            PendingIntent.FLAG_ONE_SHOT or PendingIntent.FLAG_IMMUTABLE
                        )
                        val alarm = getSystemService(ALARM_SERVICE) as AlarmManager
                        alarm.set(AlarmManager.RTC, now + 200, pending)
                    }
                }
            } catch (e: Exception) {
                Log.e("CrashRecovery", "Failed to schedule restart", e)
            }

            defaultHandler?.uncaughtException(thread, throwable)
        }
    }
}
```

### Activity Restart Detection

In `MainActivity.onCreate`, after Core initialization, read the crash flag from SharedPreferences and clear it if set. Then always call `setContent` and conditionally show a recovery snackbar:

```kotlin
override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)

    // ... Core initialization ...

    val crashPrefs = getSharedPreferences("crux_crash_recovery", MODE_PRIVATE)
    val crashSummary = if (crashPrefs.getBoolean("crashed", false)) {
        val summary = crashPrefs.getString("crash_summary", "an unexpected error")
        crashPrefs.edit().clear().apply()
        summary
    } else {
        null
    }

    setContent {
        AppTheme {
            val snackbarHostState = remember { SnackbarHostState() }
            if (crashSummary != null) {
                LaunchedEffect(Unit) {
                    snackbarHostState.showSnackbar(
                        message = "The app recovered from $crashSummary",
                        duration = SnackbarDuration.Long
                    )
                }
            }
            Scaffold(snackbarHost = { SnackbarHost(snackbarHostState) }) { padding ->
                Box(modifier = Modifier.padding(padding)) {
                    // ... normal app content
                }
            }
        }
    }
}
```

## Dependency Injection (Koin)

When using the full Core pattern, set up Koin for DI:

### AppModule.kt

```kotlin
package com.vectis.myapp.di

import com.vectis.myapp.core.Core
import com.vectis.myapp.core.HttpClient
import com.vectis.myapp.core.SseClient
import org.koin.core.module.dsl.singleOf
import org.koin.dsl.module

val appModule = module {
    singleOf(::HttpClient)
    singleOf(::SseClient)
    singleOf(::Core)
}
```

## Type Mapping: Rust → Kotlin

The `codegen` binary produces Kotlin equivalents of all Crux types that cross the FFI boundary.

### Primitive types

| Rust Type | Kotlin Type | Notes |
|-----------|-------------|-------|
| `String` | `String` | |
| `Vec<T>` | `List<T>` | |
| `Option<T>` | `T?` | |
| `bool` | `Boolean` | |
| `isize` / `i64` | `Long` | |
| `i32` | `Int` | |
| `usize` / `u64` | `ULong` | Cast to `Long` for Compose text display |
| `u32` | `UInt` | Effect request IDs are `UInt` |
| `u16` | `UShort` | HTTP status codes |
| `Vec<u8>` | `List<UByte>` | Use `.toUByteArray().toList()` to convert from `ByteArray` |

### Enum mapping (TWO patterns)

**Simple enums (no payloads)** are generated as Kotlin `enum class` with `UPPER_CASE` values:

```
// Rust: enum Filter { All, Active, Completed }
// Kotlin: enum class Filter { ALL, ACTIVE, COMPLETED }
```

Match with `==` equality, NOT `is`:

```kotlin
when (filter) {
    Filter.ALL -> ...       // CORRECT
    // is Filter.All -> ... // WRONG -- enum values are not types
}
```

**Enums with payloads** are generated as Kotlin `sealed interface` with nested `data class` or `data object` variants:

```
// Rust: enum Event { Navigate(Route), ClearCompleted }
// Kotlin:
// sealed interface Event {
//     data class Navigate(val value: Route) : Event
//     data object ClearCompleted : Event
// }
```

Match with `is` for data classes, direct ref for data objects:

```kotlin
when (event) {
    is Event.Navigate -> event.value        // data class
    Event.ClearCompleted -> ...             // data object (no `is`)
}
```

### Struct mapping

| Rust Type | Kotlin Type |
|-----------|-------------|
| `struct MainView { pub items: Vec<ItemView> }` | `data class MainView(val items: List<ItemView>)` |

### Generated type packages

All generated types live in `com.example.app.*`, NOT in the app's package. All hand-written Kotlin files MUST import them explicitly:

```kotlin
import com.example.app.Event
import com.example.app.ViewModel
import uniffi.shared.CoreFfi
```
