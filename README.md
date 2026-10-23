# LightCraft for Android

An Android port of [LightCraft](https://github.com/storytold/lightcraft), an open-source, non-destructive
photo library and raw developer written in pure Rust. The desktop and web versions use an egui interface
built for a mouse and a large window. This port keeps the upstream engine and rendering code and adds a
phone and tablet interface modelled on Lightroom for Android.

> Status: early alpha. It builds and links for `arm64-v8a`, and the layout, import and raw decoding were
> checked on a desktop with real CR2, CR3 and DNG files. It has not been tested on a physical device by the
> author of this repository yet.

## What you get

- **Welcome screen** while the library is empty: a generated nature picture (mountains, a lake, a lavender
  field, clouds) split by a diagonal line, black and white on one side and colour on the other, with a large
  button that opens the file browser. A new library starts without sample photos.
- **Library**: a photo grid with a top bar. Tap a photo to open it.
- **Edit screen**: the photo on top and a tool bar at the bottom (Auto, Presets, Crop, Edit, Masking,
  Remove). The Edit tool has the tabs Light, Color, Effects, Detail and Optics with touch-sized sliders.
  Every slider drag is one undo step. On wide screens (tablet, landscape) the tool sheet moves to a panel
  on the right.
- **Add photos**: a built-in file browser for the device storage. Tick photos or import a whole folder.
  Files are referenced where they are; nothing is copied or moved.
- **Safe areas**: the bars keep clear of the status bar, the navigation bar and display cut-outs.
- **Formats** (all decoded by the upstream engine): JPEG, PNG, TIFF, WebP, JPEG XL, GIF, BMP, PSD, HEIC/HEIF
  and raw files from Canon (CR2, CR3, CRW), Nikon, Sony, Fujifilm, Olympus, Panasonic, Pentax, Samsung,
  Phase One, Hasselblad, Leica/Adobe DNG and others. AVIF cannot be opened (the upstream project has no AV1
  decoder). Some raw variants open as the camera's embedded preview; the editor says so.

## Install the test build

`Debug/lightcraft-debug.apk` is a debuggable build for 64-bit ARM phones and tablets (Android 10 or newer),
signed with a throwaway debug key (`Debug/debug.keystore`, password `android`). It is not a release key.

1. Copy the APK to the device (or use `adb install Debug/lightcraft-debug.apk`).
2. Allow installing apps from your file manager or browser when Android asks.
3. On the first use of Add photos, allow file access. On Android 11 and newer the app opens the
   "All files access" settings page; switch it on and come back to the app.

`Debug/lightcraft-debug-unsigned.apk` is the same package without a signature. Android refuses to install
it; it is kept for people who want to sign it with their own key (`apksigner sign --ks my.keystore ...`).

If you upgrade from an older test build that showed sample photos, clear the app's storage (or uninstall
first) to get the empty library and the welcome screen.

## How it is built

```
app/src/lib.rs        crate root
app/src/mobile.rs     the phone / tablet layout: library, edit screen, tool bar, file browser
app/src/welcome.rs    the welcome screen and its generated picture
app/src/android.rs    the NativeActivity entry point and the JNI calls (insets, file access)
app/examples/         desktop tools: preview (UI snapshots), cr3check and cr3probe (raw file checks)
patches/              the small patch applied to the upstream sources
scripts/              fetch-upstream.sh and build-apk.sh
android/              AndroidManifest.xml and the launcher icon resources
Debug/                the test APKs and the debug key
third_party/          the patched upstream checkout (created by the scripts, not committed)
```

The upstream crates are used as they are, except for a small patch (`patches/0001-mobile-layout-hook.patch`)
that lets a host replace the desktop layout with its own. The phone layout in `app/src/mobile.rs` reuses
upstream widgets: the photo grid, the loupe, the slider row and the preset, crop, masking and remove panels.

### Build requirements

- Rust with the target `aarch64-linux-android` (`rustup target add aarch64-linux-android`)
- `cargo install cargo-ndk`
- Android NDK r27 (`ANDROID_NDK_HOME`)
- An Android SDK with `platforms;android-34` and `build-tools;34.0.0` (`ANDROID_SDK_ROOT`)
- `git`, `zip` and `python3`

```sh
export ANDROID_NDK_HOME=/path/to/android-ndk-r27c
export ANDROID_SDK_ROOT=/path/to/android-sdk
./scripts/build-apk.sh        # writes Debug/lightcraft-debug-unsigned.apk and Debug/lightcraft-debug.apk
```

The first build compiles the whole engine and takes several minutes.

### Desktop tools

These run on a normal computer and need no device:

```sh
# Render the welcome screen, library, file browser and edit tabs for a phone and a tablet into PNG files
cargo run --release --example preview -- out-dir

# Import a folder of raw files through the app's import path and report how each file opened
cargo run --release --example cr3check -- folder-with-photos snapshot.png

# Probe and decode one raw file and print sensor statistics
cargo run --release --example cr3probe -- photo.CR3
```

Run `./scripts/fetch-upstream.sh` once before the first desktop build.

## Known limits

- Not yet tried on a physical device; memory use with 45 to 60 megapixel raw files is unknown.
- No pinch-to-zoom or swipe-between-photos tuning for touch; the upstream loupe handles zoom and pan.
- The crop, masking, remove and presets panels are the upstream desktop panels shown in a sheet.
- Export and sharing are not wired to the Android share sheet yet.
- The interface is English only.

## Licence

The code in this repository is released under CC0 1.0 (see `LICENSE`). It builds on LightCraft, which is
licensed MIT OR Apache-2.0, and on third-party crates with their own licences; see the upstream repository.
