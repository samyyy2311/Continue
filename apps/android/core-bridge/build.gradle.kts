plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
    id("org.jlleitschuh.gradle.ktlint")
    id("io.gitlab.arturbosch.detekt")
}

val uniffiBindingsDir = layout.buildDirectory.dir("generated/uniffi")
val rustLibsDir = layout.buildDirectory.dir("rustJniLibs")

android {
    namespace = "org.continueapp.bridge"
    compileSdk = 35

    defaultConfig {
        minSdk = 26
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        consumerProguardFiles("consumer-rules.pro")
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
        }
        debug {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    lint {
        abortOnError = true
        checkReleaseBuilds = false
        warningsAsErrors = false
    }

    sourceSets["main"].java.srcDir(uniffiBindingsDir)
    sourceSets["main"].jniLibs.srcDir(rustLibsDir)
}

// The Rust core lives at the repository root and is built for each Android ABI with cargo-ndk.
// Cargo skips work that is already up to date, so these always run and stay cheap.
val repoRoot = rootDir.resolve("../..")

val buildRustCore by tasks.registering(Exec::class) {
    description = "Builds the Rust core library for each Android ABI."
    workingDir = repoRoot
    commandLine(
        "cargo",
        "ndk",
        "-t",
        "arm64-v8a",
        "-t",
        "armeabi-v7a",
        "-t",
        "x86_64",
        "--platform",
        android.defaultConfig.minSdk.toString(),
        "-o",
        rustLibsDir.get().asFile.absolutePath,
        "build",
        "--release",
        "-p",
        "ffi",
    )
}

val generateUniffiBindings by tasks.registering(Exec::class) {
    description = "Generates the Kotlin bindings for the Rust core."
    workingDir = repoRoot
    commandLine(
        "cargo",
        "run",
        "-p",
        "ffi",
        "--features",
        "bindgen",
        "--bin",
        "uniffi-bindgen",
        "--",
        "generate",
        "crates/ffi/src/continue.udl",
        "--language",
        "kotlin",
        "--config",
        "crates/ffi/uniffi.toml",
        "--no-format",
        "--out-dir",
        uniffiBindingsDir.get().asFile.absolutePath,
    )
}

tasks.named("preBuild") {
    dependsOn(buildRustCore, generateUniffiBindings)
}

ktlint {
    version.set("1.2.1")
    android.set(true)
    outputToConsole.set(true)
    ignoreFailures.set(false)
    filter {
        exclude { it.file.path.contains("generated") }
    }
}

detekt {
    buildUponDefaultConfig = true
    allRules = false
    config.setFrom(files("$rootDir/detekt.yml"))
}

dependencies {
    implementation("net.java.dev.jna:jna:5.19.1@aar")
    implementation("androidx.core:core-ktx:1.19.1")
    implementation("androidx.annotation:annotation:1.11.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.11.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.11.0")

    testImplementation("junit:junit:4.13.2")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.11.0")

    androidTestImplementation("androidx.test.ext:junit:1.3.0")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.7.0")
    androidTestImplementation("androidx.test:runner:1.7.0")
    androidTestImplementation("androidx.test:rules:1.7.0")
}
