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
    namespace = "__ANDROID_PACKAGE__.shared"
    compileSdk { version = release(36) }

    ndkVersion = "__ANDROID_NDK_VERSION__"

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
