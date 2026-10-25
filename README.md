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
- **Library**: a photo grid with a top bar. Tap a photo to open it. **Select** starts the selection mode: a tap
  ticks a photo, and the bar at the bottom offers Select all and Delete (to Recently Deleted).
- **Edit screen**: the photo on top and a tool bar at the bottom (Presets, Crop, Edit, Masking, Remove).
  The Edit tool has the tabs Light, Color, Effects, Detail and Optics. On wide screens (tablet, landscape)
  the tool sheet moves to a panel on the right.
  - **Light**: an Auto button at the top, which adapts Exposure, Highlights, Whites, Shadows, Blacks, Contrast
    and Vibrance to the picture (clipping headroom, share of deep shadow, tonal range, colourfulness), plus
    Reset and a parametric tone curve.
  - **Color**: a white balance preset list, an automatic white balance button, a white balance selector
    (drag a ring onto something white or grey and tap the check mark), B&W, temperature, tint, vibrance and
    saturation.
  - **Detail**: sharpening and noise reduction. Noise reduction is a wavelet filter that measures the noise
    of the picture itself and keeps edges and texture.
  - **Optics**: a lens profile drop-down at the top (None, Automatic, or any lens of the loaded database with
    a filter) and a button that loads lensfun-format XML profile files; below it the lens correction, manual
    distortion, vignetting, defringe and chromatic aberration sliders.
- **Crop sheet**: an aspect ratio drop-down list (free, original, common print and screen ratios, and the picture
  formats of popular social networks: Instagram, TikTok, Facebook, X, YouTube, LinkedIn, Pinterest, Snapchat), rotate,
  flip, straighten, and a Geometry tab with Upright and the manual transform sliders.
- **Histogram**: a button in the edit top bar shows a histogram card over the photo. Its chips switch between
  luminosity, RGB (the three channels over each other, to judge the colour balance), each channel alone and a
  parade, with clipping markers and the average level of each channel.
- **Masking**: the colour range mask is picked with the same kind of ring as the white balance; a +/1 chip adds more
  samples (up to 5) instead of a Shift-click.
- **Menu and Settings**: the menu button next to the app name opens Settings: the interface language (English,
  Spanish, German, French, Croatian, Romanian, Hungarian, Slovak, Polish, Norwegian, Ukrainian, Russian, Italian;
  Automatic follows the system language and falls back to English), the export settings (format, quality, size,
  colour space, metadata, destination folder, default `DCIM/LightCraft`) and the copyright information that is
  written into exported files.
- **Export**: the export button at the top right of the edit screen saves the photo with those settings; in the
  selection mode it exports the selected photos, all photos or only the edited ones. Edited photos carry a badge in
  the bottom left corner of their thumbnail.
- **Touch behaviour**: a slider moves only when its thumb is dragged (a tap on the track or a scroll that
  starts on it changes nothing; tap the number to type an exact value). Menus stay open until a tap outside
  them, long menus scroll, and submenus unfold on a tap.
- **Add photos**: a built-in file browser for the device storage with thumbnails (a grid or a list; a raw
  file shows its embedded preview), so the photos can be seen before they are imported. Tick photos or import
  a whole folder. The files can be sorted by date (newest or oldest first), name or size.
  Files are referenced where they are; nothing is copied or moved.
- **Safe areas**: the bars keep clear of the status bar, the navigation bar and display cut-outs.
- **Formats** (all decoded by the upstream engine): JPEG, PNG, TIFF, WebP, JPEG XL, GIF, BMP, PSD, HEIC/HEIF
  and raw files from Canon (CR2, CR3, CRW), Nikon, Sony, Fujifilm, Olympus, Panasonic, Pentax, Samsung,
  Phase One, Hasselblad, Leica/Adobe DNG and others. AVIF cannot be opened (the upstream project has no AV1
  decoder). Some raw variants open as the camera's embedded preview; the editor says so.

## Install the test build

Download the APK from the **[Releases page](https://github.com/galandras12/LightCraft-Android/releases)**.
It is a debuggable build for 64-bit ARM phones and tablets (Android 10 or newer), signed with a throwaway
debug key (`Debug/debug.keystore`, password `android`). It is not a release key.

1. Download the APK on the device (or use `adb install lightcraft-debug.apk`).
2. Allow installing apps from your browser or file manager when Android asks.
3. On the first use of Add photos, allow file access. On Android 11 and newer the app opens the
   "All files access" settings page; switch it on and come back to the app.

The installable file is `lightcraft-debug.apk`. `lightcraft-debug-unsigned.apk` is the same package without a
signature: Android refuses to install it, it is there for people who want to sign it with their own key
(`apksigner sign --ks my.keystore ...`). The build script also writes both files to the `Debug/` folder of
this repository.

If you upgrade from an older test build that showed sample photos, clear the app's storage (or uninstall
first) to get the empty library and the welcome screen.

What changed in each version is listed in the [Changelog](CHANGELOG.md).

## How it is built

```
app/src/lib.rs        crate root
app/src/mobile.rs     the phone / tablet layout: library, edit screen, tool bar, file browser, lens profile chooser
app/src/settings.rs   the Settings screen, the preferences and the export
app/src/i18n.rs       translation of the strings of this crate (app/locales/host.json)
app/src/crop.rs       the Crop sheet
app/src/histogram.rs  the histogram card
app/src/welcome.rs    the welcome screen and its generated picture
app/src/android.rs    the NativeActivity entry point and the JNI calls (insets, file access)
app/examples/         desktop tools: preview (UI snapshots), cr3check and cr3probe (raw file checks)
patches/              the patches applied to the upstream sources (see below)
scripts/              fetch-upstream.sh and build-apk.sh
android/              AndroidManifest.xml and the launcher icon resources
Debug/                the test APKs and the debug key
third_party/          the patched upstream checkout (created by the scripts, not committed)
```

The upstream crates are used as they are, except for the patches in `patches/`:

| Patch | What it does |
|---|---|
| `0001-mobile-layout-hook` | lets a host replace the desktop layout with its own; makes two upstream helpers public |
| `0002-touch-friendly-widgets` | scrolling context menus that close on a tap outside, tap-to-unfold submenus, sliders that only the thumb moves |
| `0003-light-and-noise-reduction` | adaptive Auto light and the wavelet noise reduction (GPU render falls back to the CPU filter) |
| `0004-lens-profiles` | a reloadable lens database, the lens profile commands, the chosen profile in the render |
| `0007-touch-edited-badge` | the edited badge is always drawn on a thumbnail (a touch screen has no hover) |
| `0006-more-languages` | Croatian, Romanian, Hungarian, Slovak and Norwegian catalogs for the upstream interface |
| `0005-touch-selectors` | a draggable ring with check and cancel buttons for the white balance selector and the colour range mask (with an add-sample switch instead of Shift-click) |

The phone layout in `app/src/mobile.rs` reuses upstream widgets: the photo grid, the loupe, the slider row and
the preset, masking and remove panels. After editing files in `third_party/lightcraft`, run
`./scripts/save-patches.sh` to write the changes back into the patches (the build resets that folder).

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
- The masking, remove and presets panels are the upstream desktop panels shown in a sheet.
- The app ships no lens profiles; load a lensfun database (CC-BY-SA, <https://github.com/lensfun/lensfun>, folder `data/db`).
- Export and sharing are not wired to the Android share sheet yet.
- The interface is translated into 13 languages (machine-assisted; corrections are welcome). Strings that have no translation stay English.

## Licence

The code in this repository is released under CC0 1.0 (see `LICENSE`). It builds on LightCraft, which is
licensed MIT OR Apache-2.0, and on third-party crates with their own licences; see the upstream repository.
