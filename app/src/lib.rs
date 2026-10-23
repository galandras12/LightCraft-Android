//! LightCraft on Android: the upstream egui UI with a phone / tablet layout, hosted in a
//! NativeActivity. The layout (`mobile`) also builds on the desktop, for snapshots
//! (`cargo run --example preview`).

pub mod mobile;

#[cfg(target_os = "android")]
mod android;
