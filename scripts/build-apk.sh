#!/usr/bin/env bash
# Builds the unsigned debug APK into Debug/ (arm64-v8a only).
# Needs: rust target aarch64-linux-android, cargo-ndk, ANDROID_NDK_HOME, ANDROID_SDK_ROOT
# (platforms;android-34 and build-tools;34.0.0). NativeActivity only, so no Java/Gradle step.
set -euo pipefail
cd "$(dirname "$0")/.."
: "${ANDROID_NDK_HOME:?set ANDROID_NDK_HOME}"
: "${ANDROID_SDK_ROOT:?set ANDROID_SDK_ROOT}"
BT="$ANDROID_SDK_ROOT/build-tools/34.0.0"
JAR="$ANDROID_SDK_ROOT/platforms/android-34/android.jar"
OUT=Debug/lightcraft-debug-unsigned.apk
WORK=target/apk
cargo ndk -t arm64-v8a --platform 29 -o jniLibs build --release
rm -rf "$WORK" && mkdir -p "$WORK/lib/arm64-v8a"
cp jniLibs/arm64-v8a/liblightcraft_android.so "$WORK/lib/arm64-v8a/"
"$BT/aapt2" compile --dir android/res -o "$WORK/res.zip"
"$BT/aapt2" link -R "$WORK/res.zip" --manifest android/AndroidManifest.xml -I "$JAR" --min-sdk-version 29 --target-sdk-version 34 -o "$WORK/base.apk"
(cd "$WORK" && zip -q -0 -r base.apk lib)   # .so stored uncompressed: page-aligned, mmap-able
mkdir -p Debug
rm -f "$OUT"
"$BT/zipalign" -p -f 4 "$WORK/base.apk" "$OUT"
"$BT/zipalign" -c -p 4 "$OUT" && echo "OK: $OUT ($(du -h "$OUT" | cut -f1), unsigned)"

# Installable copy, signed with the throwaway debug key (Debug/debug.keystore, password "android").
"$BT/apksigner" sign --ks Debug/debug.keystore --ks-pass pass:android --key-pass pass:android \
  --ks-key-alias androiddebugkey --out Debug/lightcraft-debug.apk "$OUT"
rm -f Debug/lightcraft-debug.apk.idsig
echo "OK: Debug/lightcraft-debug.apk (debug-signed, installable)"
