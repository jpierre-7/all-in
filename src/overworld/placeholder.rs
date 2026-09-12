//! Temporary stand-ins for the `run.rs` constructors that are still `todo!()`.
//!
//! `RunState::new` waits on the starter deck (#3) and the starting Stack (#8);
//! `Enemy::for_encounter` waits on the combat numbers (#8). Both live in Dev
//! 1's file, so the shell builds its own throwaway versions here rather than
//! editing across the seam. **Delete this file** once #3 and #8 land and call
//! the real constructors from `mod.rs`.

use crate::run::{EncounterId, Enemy, RisingBlinds, RunState};

pub fn new_run_state() -> RunState {
    RunState {
        stack: 100,
        deck: Vec::new(),
        perks: Vec::new(),
        items: Vec::new(),
    }
}

pub fn enemy_for(id: EncounterId) -> Enemy {
    let (name, stack, house_edge) = match id {
        EncounterId::FloorMinion => ("A shill in a rented tux", 40, 10),
        EncounterId::Slotz => ("SLOTZ", 80, 14),
        EncounterId::PitMinion => ("A dealer with a scar", 100, 18),
        EncounterId::PitBoss => ("THE PIT BOSS", 150, 22),
        EncounterId::TheHouse => ("THE HOUSE", 220, 26),
    };

    Enemy {
        name,
        stack,
        house_edge,
        blinds: RisingBlinds {
            every_turns: 2,
            increase: 2,
        },
    }
}
