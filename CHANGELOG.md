# Changelog

All notable changes to LightCraft for Android, newest date first. Each date lists what was added, changed or
fixed on that day. Entries are written in English.

## 2026-10-24

### Added

- **Menu and Settings**: the library's top bar has a menu button (three lines) next to the app name; it opens a list
  with **Settings**. Settings holds the interface language, the export settings and the copyright information.
- **Interface languages**: English, Spanish, German, French, Croatian, Romanian, Hungarian, Slovak, Polish,
  Norwegian, Ukrainian, Russian and Italian. **Automatic** follows the system language; a system language outside
  that list shows English. The language can also be chosen by hand. Croatian, Romanian, Hungarian, Slovak and
  Norwegian catalogs were added to the upstream interface (patch 0006); the others come from upstream.
- **Export settings**: file format (JPEG, PNG, TIFF, WebP), quality, size, colour space, which metadata the file
  carries, removing the GPS position, and the destination folder (default `DCIM/LightCraft`).
- **Copyright information**: creator and copyright notice, written into exported files (and optionally added to
  photos when they are imported).
- **Export button** at the top right of the edit screen: saves the photo with the export settings. In the
  library's selection mode the same button asks which photos to export: the selected ones, all, or only the edited.
- **Edited badge**: every thumbnail of an edited photo shows a small badge in its bottom left corner (the stars
  and the flag follow it).
- **Enable lens corrections** switch in the Optics tab, which turns the embedded lens data of the file (DNG,
  and the Sony, Nikon, Fujifilm and Panasonic conversions of the engine) or the chosen database profile on or off.
- **Automatic chromatic aberration removal** switch at the top of the Optics tab (below the lens profile). The
  engine measures the colour fringes of the picture itself; the Red/Cyan and Blue/Yellow sliders fine-tune it.
- **Close and Cancel in the Add photos browser**: a cross in the top bar and a Cancel button next to Import leave
  the browser at once without importing anything.
- **Colour range selector for masks**: adding a Colour Range mask (or tapping **Pick**) shows the same draggable ring
  as the white balance selector. The check mark samples the colour under the ring; a **+ / 1** chip chooses whether
  the sample is added to the range (up to 5) or replaces it, so the desktop Shift-click is no longer needed. The
  Masking panel texts that mentioned Shift-click were reworded.
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
