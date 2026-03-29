# Example: Simple Counter Android Shell (Render Only)

A minimal Android shell for a Crux counter app with local state and no external
side-effects. Demonstrates Core.kt, MainActivity.kt, screen composables,
Gradle configuration, and Makefile.

This shell pairs with the core-writer example `01-simple-counter.md`. The
shared crate defines:

- `ViewModel` as a flat struct: `ViewModel { count: String }`
- `Event::Increment`, `Event::Decrement`, `Event::Reset`
- `Effect::Render(RenderOperation)`

## Capabilities Handled

- **Render** -- update the ViewModel state

## Directory Structure

```
examples/counter/
    shared/             # Already exists from core-writer
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
                        themes.xml  # REQUIRED
                java/com/vectis/counter/
                    MainActivity.kt
                    core/
                        Core.kt
                    ui/
                        screens/
                            CounterScreen.kt
                        theme/
                            Color.kt
                            Theme.kt
                            Type.kt
        shared/
            build.gradle.kts
        generated/          # gitignored
```

## `Android/Makefile`

```makefile
.PHONY: all build clean lib typegen

WORKSPACE := ..

all: build

build: typegen

typegen: lib
	@echo "Generating Kotlin types..."
	@cd $(WORKSPACE) && RUST_LOG=info cargo run \
		--package shared \
		--bin codegen \
		--features codegen,facet_typegen \
		-- \
		--language kotlin \
		--output-dir Android/generated

lib:
	@echo "Building Rust shared library..."
	@cd $(WORKSPACE) && cargo build --features uniffi

clean:
	@rm -rf generated/
```

## `Android/build.gradle.kts`

```kotlin
plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.kotlin.android) apply false
    alias(libs.plugins.kotlin.compose) apply false
    alias(libs.plugins.android.library) apply false
    alias(libs.plugins.rust.android) apply false
}
```

## `Android/settings.gradle.kts`

```kotlin
pluginManagement {
    repositories {
        google {
            content {
                includeGroupByRegex("com\\.android.*")
                includeGroupByRegex("com\\.google.*")
                includeGroupByRegex("androidx.*")
            }
        }
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "Counter"
include(":app")
include(":shared")
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

## `Android/shared/build.gradle.kts`

```kotlin
import org.jetbrains.kotlin.gradle.dsl.JvmTarget
import com.android.build.gradle.tasks.MergeSourceSetFolders
import com.nishtahir.CargoBuildTask
import com.nishtahir.CargoExtension

plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.rust.android)
}

android {
    namespace = "com.vectis.counter.shared"
    compileSdk { version = release(36) }
    ndkVersion = "29.0.14206865"

    defaultConfig {
        minSdk = 34
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

    sourceSets {
        getByName("main") {
            kotlin.srcDirs("../generated")
        }
    }
}

dependencies {
    implementation(libs.jna) {
        artifact {
            type = "aar"
        }
    }
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.appcompat)
    implementation(libs.material)
}

apply(plugin = "org.mozilla.rust-android-gradle.rust-android")

extensions.configure<CargoExtension>("cargo") {
    module = "../.."
    extraCargoBuildArguments = listOf("--package", "shared")
    libname = "shared"
    profile = "debug"
    targets = listOf("arm", "arm64", "x86", "x86_64")
    features {
        defaultAnd(arrayOf("uniffi"))
    }
    cargoCommand = System.getProperty("user.home") + "/.cargo/bin/cargo"
    rustcCommand = System.getProperty("user.home") + "/.cargo/bin/rustc"
    pythonCommand = "python3"
}

afterEvaluate {
    android.libraryVariants.configureEach {
        val productFlavor = productFlavors.joinToString("") {
            it.name.replaceFirstChar(Char::titlecase)
        }
        val buildType = this.buildType.name.replaceFirstChar(Char::titlecase)

        tasks.named("generate${productFlavor}${buildType}Assets") {
            dependsOn(tasks.named("cargoBuild"))
        }

        tasks.withType<CargoBuildTask>().forEach { buildTask ->
            tasks.withType<MergeSourceSetFolders>().configureEach {
                inputs.dir(
                    File(File(layout.buildDirectory.asFile.get(), "rustJniLibs"), buildTask.toolchain!!.folder)
                )
                dependsOn(buildTask)
            }
        }
    }
}

tasks.matching { it.name.matches(Regex("merge.*JniLibFolders")) }.configureEach {
    inputs.dir(File(layout.buildDirectory.asFile.get(), "rustJniLibs/android"))
    dependsOn("cargoBuild")
}
```

## `Android/app/src/main/java/com/vectis/counter/core/Core.kt`

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

## `Android/app/src/main/java/com/vectis/counter/MainActivity.kt`

```kotlin
package com.vectis.counter

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.example.app.ViewModel
import com.vectis.counter.core.Core
import com.vectis.counter.ui.screens.CounterScreen
import com.vectis.counter.ui.theme.CounterTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            CounterTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    AppView()
                }
            }
        }
    }
}

@Composable
fun AppView(core: Core = viewModel()) {
    CounterScreen(
        viewModel = core.view,
        onEvent = { core.update(it) }
    )
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
            text = viewModel.count,
            fontSize = 32.sp,
            modifier = Modifier.padding(10.dp)
        )
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Button(
                onClick = { onEvent(Event.RESET) },
                colors = ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.error
                )
            ) {
                Text(text = "Reset", color = Color.White)
            }
            Button(
                onClick = { onEvent(Event.INCREMENT) },
                colors = ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.primary
                )
            ) {
                Text(text = "Increment", color = Color.White)
            }
            Button(
                onClick = { onEvent(Event.DECREMENT) },
                colors = ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.secondary
                )
            ) {
                Text(text = "Decrement", color = Color.White)
            }
        }
    }
}

@Preview(showBackground = true)
@Composable
fun CounterScreenPreview() {
    CounterTheme {
        CounterScreen(
            viewModel = ViewModel("Count is: 42"),
            onEvent = { }
        )
    }
}
```

## `Android/app/src/main/AndroidManifest.xml`

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <application
        android:allowBackup="true"
        android:label="Counter"
        android:supportsRtl="true"
        android:theme="@style/Theme.Counter">
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

## Key Patterns Demonstrated

1. **Simple Core pattern** -- `Core` extends `ViewModel`, uses `mutableStateOf`.
2. **Event callback pattern** -- screens receive `(Event) -> Unit`, not the `Core`.
3. **Material 3 theming** -- colors from `MaterialTheme.colorScheme`.
4. **Preview support** -- every screen has a `@Preview` with sample data.
5. **Render-only Core.kt** -- the simplest possible effect handler.
6. **No DI needed** -- simple apps use `viewModel()` directly.
7. **Command-line build** -- `make build && ./gradlew :app:assembleDebug`.
