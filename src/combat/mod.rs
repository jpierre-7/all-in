//! Combat. Owned by Dev 1. The pure duel model lives in `duel`; the Bevy
//! plugin and UI (#11) wrap it and are the only things that touch the seam
//! in `run.rs`.

#[allow(dead_code)] // the plugin and UI that call this arrive with #11
pub mod duel;
