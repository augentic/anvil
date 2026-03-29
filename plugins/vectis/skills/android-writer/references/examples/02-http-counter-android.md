# Example: HTTP Counter Android Shell

An Android shell for a Crux counter app that persists count to a server via
HTTP and streams updates via SSE. Demonstrates async HTTP effect handling,
SSE streaming, Koin dependency injection, and Ktor HTTP client.

This shell pairs with the core-writer example `02-http-counter.md`. The
shared crate defines:

- `ViewModel { text: String, confirmed: bool }`
- Shell-facing events: `Event::Get`, `Event::Increment`, `Event::Decrement`, `Event::StartWatch`
- Internal events: `Event::Set(Result<...>)`, `Event::Update(Count)` (serde/facet skipped)
- `Effect::Render(RenderOperation)`, `Effect::Http(HttpRequest)`, `Effect::ServerSentEvents(SseRequest)`

## Capabilities Handled

- **Render** -- update the ViewModel StateFlow
- **HTTP** -- perform HTTP requests via Ktor + OkHttp
- **ServerSentEvents** -- stream SSE via Ktor

## Directory Structure

```
examples/http-counter/
    shared/
    Android/
        build.gradle.kts
        settings.gradle.kts
        gradle.properties
        local.properties        # gitignored -- sdk.dir
        gradlew                 # Gradle wrapper (MUST exist)
        gradlew.bat
        Makefile
        .gitignore
        gradle/
            wrapper/
                gradle-wrapper.jar
                gradle-wrapper.properties
            libs.versions.toml
        app/
            build.gradle.kts
            src/main/
                AndroidManifest.xml
                res/
                    values/
                        themes.xml          # REQUIRED
                    xml/
                        network_security_config.xml
                java/com/vectis/counter/
                    CounterApplication.kt
                    MainActivity.kt
                    core/
                        Core.kt
                        HttpClient.kt
                        SseClient.kt
                    di/
                        AppModule.kt
                    ui/
                        screens/
                            CounterScreen.kt
                        theme/
                            Color.kt
                            Theme.kt
                            Type.kt
        shared/
            build.gradle.kts
        generated/
```

## `Android/gradle/libs.versions.toml`

```toml
[versions]
agp = "8.13.2"
kotlin = "2.3.0"
coreKtx = "1.17.0"
junit = "4.13.2"
junitVersion = "1.3.0"
espressoCore = "3.7.0"
lifecycleRuntimeKtx = "2.10.0"
activityCompose = "1.12.3"
composeBom = "2026.01.01"
appcompat = "1.7.1"
material = "1.13.0"
jna = "5.18.1"
lifecycle = "2.10.0"
rustAndroid = "0.9.6"
ktor = "3.4.0"
koin-bom = "4.1.1"

[libraries]
androidx-core-ktx = { group = "androidx.core", name = "core-ktx", version.ref = "coreKtx" }
junit = { group = "junit", name = "junit", version.ref = "junit" }
androidx-junit = { group = "androidx.test.ext", name = "junit", version.ref = "junitVersion" }
androidx-espresso-core = { group = "androidx.test.espresso", name = "espresso-core", version.ref = "espressoCore" }
androidx-lifecycle-runtime-ktx = { group = "androidx.lifecycle", name = "lifecycle-runtime-ktx", version.ref = "lifecycleRuntimeKtx" }
androidx-activity-compose = { group = "androidx.activity", name = "activity-compose", version.ref = "activityCompose" }
androidx-compose-bom = { group = "androidx.compose", name = "compose-bom", version.ref = "composeBom" }
androidx-ui = { group = "androidx.compose.ui", name = "ui" }
androidx-ui-graphics = { group = "androidx.compose.ui", name = "ui-graphics" }
androidx-ui-tooling = { group = "androidx.compose.ui", name = "ui-tooling" }
androidx-ui-tooling-preview = { group = "androidx.compose.ui", name = "ui-tooling-preview" }
androidx-ui-test-manifest = { group = "androidx.compose.ui", name = "ui-test-manifest" }
androidx-ui-test-junit4 = { group = "androidx.compose.ui", name = "ui-test-junit4" }
androidx-material3 = { group = "androidx.compose.material3", name = "material3" }
jna = { module = "net.java.dev.jna:jna", version.ref = "jna" }
lifecycle-viewmodel-compose = { module = "androidx.lifecycle:lifecycle-viewmodel-compose", version.ref = "lifecycle" }
androidx-appcompat = { group = "androidx.appcompat", name = "appcompat", version.ref = "appcompat" }
material = { group = "com.google.android.material", name = "material", version.ref = "material" }
koin-bom = { module = "io.insert-koin:koin-bom", version.ref = "koin-bom" }
koin-core = { module = "io.insert-koin:koin-core" }
koin-android = { module = "io.insert-koin:koin-android" }
koin-androidx-compose = { module = "io.insert-koin:koin-androidx-compose" }
ktor-client-core = { group = "io.ktor", name = "ktor-client-core", version.ref = "ktor" }
ktor-client-okhttp = { group = "io.ktor", name = "ktor-client-okhttp", version.ref = "ktor" }
ktor-client-logging = { group = "io.ktor", name = "ktor-client-logging", version.ref = "ktor" }

[plugins]
android-application = { id = "com.android.application", version.ref = "agp" }
kotlin-android = { id = "org.jetbrains.kotlin.android", version.ref = "kotlin" }
kotlin-compose = { id = "org.jetbrains.kotlin.plugin.compose", version.ref = "kotlin" }
android-library = { id = "com.android.library", version.ref = "agp" }
rust-android = { id = "org.mozilla.rust-android-gradle.rust-android", version.ref = "rustAndroid" }
```

## `Android/app/build.gradle.kts`

```kotlin
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "com.vectis.counter"
    compileSdk { version = release(36) }

    defaultConfig {
        applicationId = "com.vectis.counter"
        minSdk = 34
        targetSdk = 36
        versionCode = 1
        versionName = "1.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }

    kotlin {
        compilerOptions {
            jvmTarget = JvmTarget.JVM_11
        }
    }

    buildFeatures {
        compose = true
    }
}

dependencies {
    implementation(project(":shared"))
    implementation(libs.lifecycle.viewmodel.compose)

    // Koin DI
    implementation(platform(libs.koin.bom))
    implementation(libs.koin.core)
    implementation(libs.koin.android)
    implementation(libs.koin.androidx.compose)

    // Ktor HTTP client
    implementation(libs.ktor.client.core)
    implementation(libs.ktor.client.okhttp)
    implementation(libs.ktor.client.logging)

    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.lifecycle.runtime.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.ui)
    implementation(libs.androidx.ui.graphics)
    implementation(libs.androidx.ui.tooling.preview)
    implementation(libs.androidx.material3)

    debugImplementation(libs.androidx.ui.tooling)
    debugImplementation(libs.androidx.ui.test.manifest)
}
```

## `Android/app/src/main/java/com/vectis/counter/core/Core.kt`

```kotlin
package com.vectis.counter.core

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
                    catch (e: Exception) { Log.e(TAG, "SSE error: ${e.message}", e) }
                }
            }
            is Effect.Render -> {
                _viewModel.value = getViewModel()
            }
        }
    }

    private suspend fun resolveAndHandleEffects(
        requestId: UInt,
        data: ByteArray
    ) {
        val effects = coreFfi.resolve(requestId, data)
        handleEffects(effects)
    }

    private fun getViewModel(): ViewModel =
        ViewModel.bincodeDeserialize(coreFfi.view())
}
```

## `Android/app/src/main/java/com/vectis/counter/core/HttpClient.kt`

```kotlin
package com.vectis.counter.core

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

## `Android/app/src/main/java/com/vectis/counter/core/SseClient.kt`

```kotlin
package com.vectis.counter.core

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

## `Android/app/src/main/java/com/vectis/counter/di/AppModule.kt`

```kotlin
package com.vectis.counter.di

import com.vectis.counter.core.Core
import com.vectis.counter.core.HttpClient
import com.vectis.counter.core.SseClient
import org.koin.core.module.dsl.singleOf
import org.koin.dsl.module

val appModule = module {
    singleOf(::HttpClient)
    singleOf(::SseClient)
    singleOf(::Core)
}
```

## `Android/app/src/main/java/com/vectis/counter/CounterApplication.kt`

```kotlin
package com.vectis.counter

import android.app.Application
import com.vectis.counter.di.appModule
import org.koin.android.ext.koin.androidContext
import org.koin.core.context.startKoin

class CounterApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        System.setProperty("uniffi.component.shared.libraryOverride", "shared")
        startKoin {
            androidContext(this@CounterApplication)
            modules(appModule)
        }
    }
}
```

## `Android/app/src/main/java/com/vectis/counter/MainActivity.kt`

```kotlin
package com.vectis.counter

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.example.app.Event
import com.example.app.ViewModel
import com.vectis.counter.core.Core
import com.vectis.counter.ui.screens.CounterScreen
import com.vectis.counter.ui.theme.CounterTheme
import org.koin.android.ext.android.inject

class MainActivity : ComponentActivity() {
    private val core by inject<Core>()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        core.update(Event.StartWatch)

        setContent {
            CounterTheme {
                val state by core.viewModel.collectAsState()
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    CounterScreen(
                        viewModel = state,
                        onEvent = { core.update(it) }
                    )
                }
            }
        }
    }
}
```

## `Android/app/src/main/java/com/vectis/counter/ui/screens/CounterScreen.kt`

```kotlin
package com.vectis.counter.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.example.app.Event
import com.example.app.ViewModel
import com.vectis.counter.ui.theme.CounterTheme

@Composable
fun CounterScreen(
    viewModel: ViewModel,
    onEvent: (Event) -> Unit
) {
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
        modifier = Modifier
            .fillMaxSize()
            .padding(10.dp)
    ) {
        Text(
            text = "Crux Counter Example",
            fontSize = 30.sp,
            modifier = Modifier.padding(bottom = 16.dp)
        )
        Text(
            text = viewModel.text,
            fontSize = 24.sp,
            color = if (viewModel.confirmed) Color.Black else Color.Gray,
            modifier = Modifier.padding(10.dp)
        )
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Button(
                onClick = { onEvent(Event.DECREMENT) },
                colors = ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.secondary
                )
            ) {
                Text(text = "Decrement", color = Color.White)
            }
            Button(
                onClick = { onEvent(Event.INCREMENT) },
                colors = ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.primary
                )
            ) {
                Text(text = "Increment", color = Color.White)
            }
        }
    }
}

@Preview(showBackground = true)
@Composable
fun CounterScreenPreview() {
    CounterTheme {
        CounterScreen(
            viewModel = ViewModel("42 (2026-01-01T00:00:00Z)", true),
            onEvent = { }
        )
    }
}
```

## `Android/app/src/main/AndroidManifest.xml`

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <uses-permission android:name="android.permission.INTERNET" />
    <application
        android:name=".CounterApplication"
        android:allowBackup="true"
        android:label="Counter"
        android:supportsRtl="true"
        android:theme="@style/Theme.Counter"
        android:networkSecurityConfig="@xml/network_security_config">
        <activity
            android:name=".MainActivity"
            android:exported="true"
            android:theme="@style/Theme.Counter">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
```

## `Android/app/src/main/res/values/themes.xml`

```xml
<?xml version="1.0" encoding="utf-8"?>
<resources>
    <style name="Theme.Counter" parent="android:Theme.Material.Light.NoActionBar" />
</resources>
```

## `Android/app/src/main/res/xml/network_security_config.xml`

```xml
<?xml version="1.0" encoding="utf-8"?>
<network-security-config>
    <domain-config cleartextTrafficPermitted="true">
        <domain includeSubdomains="true">localhost</domain>
        <domain includeSubdomains="true">10.0.2.2</domain>
        <domain includeSubdomains="true">10.0.3.2</domain>
        <domain includeSubdomains="true">127.0.0.1</domain>
    </domain-config>
</network-security-config>
```

## Key Patterns Demonstrated

1. **HTTP effect handling** -- `HttpClient` uses Ktor + OkHttp engine with
   proper timeout configuration and error mapping.
2. **SSE streaming** -- `SseClient` reads a Ktor channel line-by-line and
   invokes a callback for each chunk. Uses `@OptIn(ExperimentalUnsignedTypes::class)`.
3. **Defensive error handling** -- SSE effect is wrapped in `scope.launch`
   with `try/catch` to prevent crashes. `CancellationException` is rethrown.
4. **Koin DI** -- `Core`, `HttpClient`, and `SseClient` are singletons
   injected via Koin. The `CounterApplication` bootstraps Koin.
5. **UniFFI library override** -- `System.setProperty("uniffi.component.shared.libraryOverride", "shared")`
   set in `CounterApplication.onCreate()` before Koin initialization.
6. **Generated type imports** -- all `.kt` files explicitly import types
   from `com.example.app.*` and `uniffi.shared.CoreFfi`.
7. **Network security config** -- cleartext HTTP allowed for localhost/`10.0.2.2`
   to support development servers.
8. **themes.xml** -- Android theme resource required by `AndroidManifest.xml`.
9. **StateFlow observation** -- `core.viewModel.collectAsState()` in Compose
   triggers recomposition on ViewModel changes.
10. **Effect loop** -- `update -> handleEffects -> processRequest -> resolve
    -> handleEffects` forms the recursive effect processing loop.
11. **INTERNET permission** -- Required in AndroidManifest for HTTP/SSE.
