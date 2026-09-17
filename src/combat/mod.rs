//! Combat. Owned by Dev 1. The pure duel model lives in `duel`; `plugin`
//! wraps it in Bevy and is the only thing that touches the seam in `run.rs`;
//! `ui` draws it.

pub mod duel;
pub mod hits;
pub mod info;
pub mod peek;
pub mod plugin;
pub mod ui;

pub use plugin::CombatPlugin;
/// Only the dev entry point names this; `start_duel` reads it either way.
#[cfg(debug_assertions)]
pub use plugin::DuelSeed;
