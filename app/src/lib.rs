//! LightCraft on Android: the upstream egui UI with a phone / tablet layout, hosted in a
//! NativeActivity. The layout (`mobile`) also builds on the desktop, for snapshots
//! (`cargo run --example preview`).

/// The phone / tablet layout (library, edit screen, file browser).
pub mod crop;
pub mod mobile;
/// The welcome screen of an empty library.
pub mod welcome;

#[cfg(target_os = "android")]
mod android;

/// Whether this build opens HEIC / HEIF photos.
pub fn heic_supported() -> bool {
    lightcraft_codecs::Format::Heif.can_decode()
}
