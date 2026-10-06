//! The game as a library, so the binaries can share it: `main.rs` builds the
//! app, and the balance sim in `tools/sim` plays the real `Duel` headless.

pub mod boss;
pub mod combat;
#[cfg(debug_assertions)]
pub mod devstart;
pub mod modifier;
pub mod music;
pub mod overworld;
pub mod run;
pub mod save;
pub mod state;
pub mod theme;
