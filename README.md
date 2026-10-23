# LightCraft-Android

Android port of [LightCraft](https://github.com/storytold/lightcraft) (open-source, pure-Rust photo library and raw developer).
The `app/` crate hosts the upstream egui UI in a NativeActivity (eframe + wgpu); the upstream crates are pulled in by git rev.

## Status
Compiles and links for `arm64-v8a`. Not yet run on a device. Still missing: Storage Access Framework import,
touch / responsive layout, memory tuning for large raw files.

## Test build
`Debug/lightcraft-debug-unsigned.apk` — debuggable, **unsigned** (arm64-v8a, Android 10+). An unsigned APK
does not install as-is: sign it first, e.g.

    apksigner sign --ks my.keystore Debug/lightcraft-debug-unsigned.apk

(or use a throwaway key from `keytool -genkeypair`).

## Rebuild
`ANDROID_NDK_HOME=… ANDROID_SDK_ROOT=… ./scripts/build-apk.sh`
(needs `rustup target add aarch64-linux-android`, `cargo install cargo-ndk`, NDK r27, SDK platform 34 + build-tools 34.0.0).
