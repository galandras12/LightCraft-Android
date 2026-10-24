# Changelog

All notable changes to LightCraft for Android, newest date first. Each date lists what was added, changed or
fixed on that day. Entries are written in English.

## 2026-10-24

### Added

- **White balance selector** (Color tab, **Pick**): a ring with a small circle appears on the photo. Drag it onto
  something that should be white or grey (a tap moves it there) and tap the check mark to set the white balance
  from that spot; the cross cancels. It replaces the click-on-the-photo selector of the desktop interface, which
  a finger cannot aim.
- **Sorting in the Add photos browser**: a Sort button offers date (newest or oldest first), name (A to Z or
  Z to A) and size (largest or smallest first). Newest first is the default and the choice is kept between visits.
  Folders always come first.

### Changed

- The Add photos browser has a second row under the top bar for Sort, Grid or List and Select all.
- The README install section points to the Releases page, and a changelog was added.

## 2026-10-23

First working version of the Android port, built up over the day from a bare `arm64-v8a` build to a phone and
tablet interface.

### Added

- **Android host**: the upstream LightCraft engine and egui interface run in a NativeActivity (eframe + wgpu),
  built with `cargo-ndk` for `arm64-v8a`.
- **Debug APKs**: `scripts/build-apk.sh` writes an unsigned and a debug-signed APK (the signed one installs),
  with a manifest and the LightCraft lynx as the launcher icon.
- **Phone and tablet layout** modelled on Lightroom for Android: a library grid, an edit screen with the photo on
  top, a bottom tool bar (Presets, Crop, Edit, Masking, Remove), the edit tabs (Light, Color, Effects, Detail,
  Optics) and a sliders sheet, which becomes a side panel on wide screens.
- **Safe areas**: the bars keep clear of the status bar, the navigation bar and display cut-outs.
- **Add photos**: a built-in file browser for the device storage, with the file access permission flow
  (All files access on Android 11 and newer), multi-select, folder import, and thumbnails (grid or list; a raw
  file shows its embedded preview) before importing.
- **Formats**: HEIC and HEIF decoding is switched on, next to the JPEG, PNG, TIFF, WebP, JPEG XL and raw
  formats the engine already reads (checked with Canon CR2 and CR3 and phone DNG files).
- **Welcome screen** for an empty library: a generated nature picture (mountains, a lake, a lavender field,
  clouds) split by a diagonal line into black and white and colour, with a large "Import images before use..."
  button.
- **Selection mode** in the library (Select): a tap ticks photos, with Select all and Delete at the bottom.
  Delete moves photos to Recently Deleted.
- **Light tab**: an Auto button at the top, Reset and a parametric tone curve. Auto adapts Highlights, Whites,
  Shadows, Blacks, Contrast and Vibrance to the picture instead of using fixed values.
- **Color tab**: a white balance preset list, an Auto white balance button and B&W.
- **Crop sheet**: an aspect ratio drop-down list (Free, Original, common ratios and the picture formats of
  Instagram, TikTok, Facebook, X, YouTube, LinkedIn, Pinterest and Snapchat), swap orientation, rotate, flip,
  straighten, and a Geometry tab with Upright and the manual transform sliders.
- **Histogram card** over the photo (button in the edit top bar): luminosity, RGB, each channel alone and a
  parade, with clipping markers and the average level of each channel.
- **Noise reduction**: a wavelet filter that measures the noise of the picture itself and keeps edges and
  texture; colour noise is cleaned separately. The GPU render falls back to this CPU filter.
- **Optics**: a lens profile drop-down (None, Automatic, or any loaded lens, with a filter) and a button that
  loads lensfun-format XML profile files. The chosen profile is applied to the render.
- **Desktop tools** for checking without a device: `preview` (UI snapshots), `cr3check` and `cr3probe` (raw file
  import and decoding checks).
- **Patch system**: the upstream sources are fetched at a pinned revision and patched (`scripts/fetch-upstream.sh`,
  `scripts/save-patches.sh`, `patches/`).
- English comments on every function, constant and interface block, and a README.

### Changed

- New libraries start empty (no sample photos).
- Menus close only on a tap outside them; long menus scroll; submenus unfold on a tap.
- Sliders move only when their thumb is dragged (relative to where the touch began). A tap on the track or a
  scroll that starts on it changes nothing; tapping the number still types an exact value.
- The Auto button moved from the tool bar to the top of the Light tab.

### Fixed

- The "Settings file problem" dialog no longer appears (the library lock warning is only logged).
- The long photo context menu no longer runs off the screen, so Delete and Export can be reached.
- A catalogue lens profile now wins over the lens the decoder found, and is part of the render key, so choosing
  a profile changes the picture.
