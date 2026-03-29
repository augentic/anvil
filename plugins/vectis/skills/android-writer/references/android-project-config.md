# Android Project Configuration

Build infrastructure for a Crux Android shell using Gradle (Kotlin DSL),
Mozilla's rust-android-gradle plugin, and the facet-based codegen binary
from the shared crate. Targets Crux 0.17.0+.

## Directory Layout

```
{app-dir}/
    Cargo.toml              # Rust workspace (already exists from core-writer)
    shared/                 # Rust shared crate (already exists)
    Android/
        build.gradle.kts        # Root build file
        settings.gradle.kts     # Module includes
        gradle.properties       # JVM and Android properties
        local.properties        # gitignored -- sdk.dir path
        gradlew                 # Gradle wrapper script (MUST exist)
        gradlew.bat
        Makefile                # Build automation
        .gitignore
        gradle/
            wrapper/
                gradle-wrapper.jar
                gradle-wrapper.properties
            libs.versions.toml  # Version catalog
        app/
            build.gradle.kts    # App module
            src/main/
                AndroidManifest.xml
                res/
                    values/
                        themes.xml          # REQUIRED for theme reference
                    xml/
                        network_security_config.xml  # if HTTP/SSE effects
                java/com/vectis/{appname}/
                    MainActivity.kt
                    {AppName}Application.kt # if Koin is used
                    core/
                        Core.kt
                        HttpClient.kt      # if HTTP effect
                        SseClient.kt       # if SSE effect
                        KeyValueClient.kt  # if KV effect
                    di/
                        AppModule.kt       # if Koin is used
                    ui/
                        screens/
                            LoadingScreen.kt
                            MainScreen.kt
                            ErrorScreen.kt
                        theme/
                            Color.kt
                            Theme.kt
                            Type.kt
        shared/
            build.gradle.kts    # Shared module (Rust cross-compile)
        generated/              # gitignored -- codegen output
```

## Makefile

Build automation for type generation. Run from the `Android/` directory.

**CRITICAL**: `cargo` commands must run from the Rust workspace root (one level
up from `Android/`). The `WORKSPACE` variable and `cd $(WORKSPACE) &&` prefix
ensure correct path resolution.

### Template

```makefile
.PHONY: all build clean lib typegen android-lib apk install launch run

WORKSPACE := ..

all: build

# Full build pipeline: compile Rust lib then generate Kotlin types
build: typegen

# Generate Kotlin types and UniFFI bindings
typegen: lib
	@echo "Generating Kotlin types..."
	@cd $(WORKSPACE) && RUST_LOG=info cargo run \
		--package shared \
		--bin codegen \
		--features codegen,facet_typegen \
		-- \
		--language kotlin \
		--output-dir Android/generated

# Build the Rust shared library (host target, for codegen)
lib:
	@echo "Building Rust shared library..."
	@cd $(WORKSPACE) && cargo build --features uniffi

# Cross-compile for Android (delegates to Gradle)
android-lib:
	@echo "Cross-compiling Rust for Android..."
	@./gradlew :shared:cargoBuild

# Build debug APK
apk: typegen
	@echo "Building debug APK..."
	@./gradlew :app:assembleDebug

# Install debug APK on connected device/emulator
install: apk
	@./gradlew :app:installDebug

# Launch the app on connected device/emulator
launch:
	@adb shell am start -n com.vectis.{appname}/.MainActivity

# Full build + install + launch
run: install launch

# Clean all build artifacts
clean:
	@./gradlew clean
	@rm -rf generated/
```

## Root `build.gradle.kts`

```kotlin
plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.kotlin.android) apply false
    alias(libs.plugins.kotlin.compose) apply false
    alias(libs.plugins.android.library) apply false
    alias(libs.plugins.rust.android) apply false
}
```

## `settings.gradle.kts`

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

rootProject.name = "{AppName}"
include(":app")
include(":shared")
```

## `gradle.properties`

**CRITICAL**: Pin `org.gradle.java.home` to Java 21 LTS. Java 25+ version
strings cause `IllegalArgumentException` in Gradle's Kotlin compiler. Detect
the path with `/usr/libexec/java_home -v 21` on macOS.

```properties
org.gradle.jvmargs=-Xmx2048m -Dfile.encoding=UTF-8
android.useAndroidX=true
kotlin.code.style=official
android.nonTransitiveRClass=true
org.gradle.java.home=/Library/Java/JavaVirtualMachines/jdk-21.jdk/Contents/Home
```

## `gradle/libs.versions.toml`

### Minimal (Render only)

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
androidx-material-icons-extended = { group = "androidx.compose.material", name = "material-icons-extended" }

[plugins]
android-application = { id = "com.android.application", version.ref = "agp" }
kotlin-android = { id = "org.jetbrains.kotlin.android", version.ref = "kotlin" }
kotlin-compose = { id = "org.jetbrains.kotlin.plugin.compose", version.ref = "kotlin" }
android-library = { id = "com.android.library", version.ref = "agp" }
rust-android = { id = "org.mozilla.rust-android-gradle.rust-android", version.ref = "rustAndroid" }
```

### Additional libraries for HTTP/SSE (add when needed)

```toml
[versions]
# ... add to existing versions
ktor = "3.4.0"
koin-bom = "4.1.1"

[libraries]
# ... add to existing libraries
koin-bom = { module = "io.insert-koin:koin-bom", version.ref = "koin-bom" }
koin-core = { module = "io.insert-koin:koin-core" }
koin-android = { module = "io.insert-koin:koin-android" }
koin-androidx-compose = { module = "io.insert-koin:koin-androidx-compose" }
ktor-client-core = { group = "io.ktor", name = "ktor-client-core", version.ref = "ktor" }
ktor-client-okhttp = { group = "io.ktor", name = "ktor-client-okhttp", version.ref = "ktor" }
ktor-client-logging = { group = "io.ktor", name = "ktor-client-logging", version.ref = "ktor" }
```

## App `build.gradle.kts`

### Minimal (Render only)

```kotlin
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "com.vectis.{appname}"
    compileSdk { version = release(36) }

    defaultConfig {
        applicationId = "com.vectis.{appname}"
        minSdk = 34
        targetSdk = 36
        versionCode = 1
        versionName = "1.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
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
    implementation(libs.androidx.material.icons.extended)

    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.junit)
    androidTestImplementation(libs.androidx.espresso.core)
    androidTestImplementation(platform(libs.androidx.compose.bom))
    androidTestImplementation(libs.androidx.ui.test.junit4)
    debugImplementation(libs.androidx.ui.tooling)
    debugImplementation(libs.androidx.ui.test.manifest)
}
```

### Additional dependencies for HTTP/SSE/DI (add when needed)

```kotlin
dependencies {
    // ... existing dependencies

    // Koin DI
    implementation(platform(libs.koin.bom))
    implementation(libs.koin.core)
    implementation(libs.koin.android)
    implementation(libs.koin.androidx.compose)

    // Ktor HTTP client
    implementation(libs.ktor.client.core)
    implementation(libs.ktor.client.okhttp)
    implementation(libs.ktor.client.logging)
}
```

## Shared Module `build.gradle.kts`

This is the critical module that cross-compiles the Rust crate for Android.

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
    // MUST differ from app module namespace to avoid collision warning
    namespace = "com.vectis.{appname}.shared"
    compileSdk { version = release(36) }

    ndkVersion = "29.0.14206865"

    defaultConfig {
        minSdk = 34
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        consumerProguardFiles("consumer-rules.pro")
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
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

    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.junit)
    androidTestImplementation(libs.androidx.espresso.core)
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

### Adapting the Shared Module

- Replace `{appname}` with the lowercase app name in the namespace.
- The `module = "../.."` points to the Rust workspace root (two levels up
  from `Android/shared/`). Adjust if the workspace layout differs.
- The `ndkVersion` must match an installed NDK. Check with
  `sdkmanager --list | grep ndk` or in Android Studio SDK Manager.
- `features { defaultAnd(arrayOf("uniffi")) }` enables the `uniffi` feature
  flag during the Android build.
- The `sourceSets` directive adds `../generated` so the shared module can
  compile against the codegen output.

## `local.properties`

**Gitignored.** Provides the Android SDK location to Gradle. Create this file
after generating the project:

```properties
sdk.dir=/Users/{user}/Library/Android/sdk
```

Detect the path from the `ANDROID_HOME` environment variable.

## Gradle Wrapper (CRITICAL)

The `gradlew` script MUST exist before any `./gradlew` command works. Generate
it after writing all Gradle configuration files:

```bash
cd {project-dir}
echo "sdk.dir=$ANDROID_HOME" > local.properties
gradle wrapper --gradle-version 8.13
```

The Gradle wrapper version MUST match or exceed the minimum required by AGP:
- AGP 8.13.x requires Gradle 8.13+
- AGP 8.12.x requires Gradle 8.12+

If `gradle` is not installed globally, download and bootstrap from:
`https://services.gradle.org/distributions/gradle-8.13-bin.zip`

## `.gitignore`

```
# Gradle
.gradle/
build/
local.properties

# Generated types and bindings
generated/

# IDE
.idea/
*.iml

# Android
*.apk
*.aab
```

## Command-Line Build Workflow

The entire build can be driven from the terminal without opening Android Studio:

```bash
# 1. Generate Kotlin types from Rust core
make build

# 2. Cross-compile Rust for Android (4 ABIs)
./gradlew :shared:cargoBuild

# 3. Build debug APK
./gradlew :app:assembleDebug

# 4. List available emulators
emulator -list-avds

# 5. Start emulator (background)
emulator -avd Pixel_API_34 &

# 6. Install and run
./gradlew :app:installDebug
adb shell am start -n com.vectis.{appname}/.MainActivity
```

### First-time emulator setup (if no AVD exists)

```bash
sdkmanager "system-images;android-34;google_apis;x86_64"
avdmanager create avd \
    -n "Pixel_API_34" \
    -k "system-images;android-34;google_apis;x86_64" \
    --device "pixel_8"
```

## Android Studio Integration

While builds work from the command line, Android Studio provides:
- SDK/NDK management (Tools > SDK Manager)
- Visual layout preview for Compose
- Logcat for runtime debugging
- Profiler for performance

Open the `Android/` directory as a project in Android Studio. The Gradle
sync will automatically configure the project.
