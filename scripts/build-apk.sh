#!/usr/bin/env bash
# Builds the debug APKs into Debug/ (arm64-v8a only):
#   Debug/lightcraft-debug-unsigned.apk  the package without a signature (Android will not install it)
#   Debug/lightcraft-debug.apk           the same package signed with the throwaway debug key (installable)
#
# Needs: the Rust target aarch64-linux-android, cargo-ndk, and the environment variables
#   ANDROID_NDK_HOME  the Android NDK (r27)
#   ANDROID_SDK_ROOT  an SDK with "platforms;android-34" and "build-tools;34.0.0"
# The app is a NativeActivity, so there is no Java or Gradle step.
set -euo pipefail
cd "$(dirname "$0")/.."
: "${ANDROID_NDK_HOME:?set ANDROID_NDK_HOME}"
: "${ANDROID_SDK_ROOT:?set ANDROID_SDK_ROOT}"

BT="$ANDROID_SDK_ROOT/build-tools/34.0.0"                  # aapt2, zipalign, apksigner
JAR="$ANDROID_SDK_ROOT/platforms/android-34/android.jar"   # the Android API the manifest is checked against
OUT=Debug/lightcraft-debug-unsigned.apk
WORK=target/apk                                            # scratch folder for the package contents

# 1. Get the upstream LightCraft sources and apply our patches to them.
./scripts/fetch-upstream.sh

# 2. Compile the Rust code into liblightcraft_android.so.
cargo ndk -t arm64-v8a --platform 29 -o jniLibs build --release

# 3. Assemble the package: compiled resources (the icon), the manifest, then the library.
rm -rf "$WORK" && mkdir -p "$WORK/lib/arm64-v8a"
cp jniLibs/arm64-v8a/liblightcraft_android.so "$WORK/lib/arm64-v8a/"
"$BT/aapt2" compile --dir android/res -o "$WORK/res.zip"
"$BT/aapt2" link -R "$WORK/res.zip" --manifest android/AndroidManifest.xml -I "$JAR" \
  --min-sdk-version 29 --target-sdk-version 34 -o "$WORK/base.apk"
# the library is stored uncompressed, so that Android can map it straight from the APK
(cd "$WORK" && zip -q -0 -r base.apk lib)

# 4. Align the package (page-aligned libraries) and check the result.
mkdir -p Debug
rm -f "$OUT"
"$BT/zipalign" -p -f 4 "$WORK/base.apk" "$OUT"
"$BT/zipalign" -c -p 4 "$OUT" && echo "OK: $OUT ($(du -h "$OUT" | cut -f1), unsigned)"

# 5. The installable copy, signed with the throwaway debug key (Debug/debug.keystore, password "android").
"$BT/apksigner" sign --ks Debug/debug.keystore --ks-pass pass:android --key-pass pass:android \
  --ks-key-alias androiddebugkey --out Debug/lightcraft-debug.apk "$OUT"
rm -f Debug/lightcraft-debug.apk.idsig
echo "OK: Debug/lightcraft-debug.apk (debug-signed, installable)"
