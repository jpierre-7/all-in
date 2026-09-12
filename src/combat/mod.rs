//! Combat. Owned by Dev 1. The pure duel model lives in `duel`; `plugin`
//! wraps it in Bevy and is the only thing that touches the seam in `run.rs`;
//! `ui` draws it.

pub mod duel;
pub mod plugin;
pub mod ui;

pub use plugin::CombatPlugin;
