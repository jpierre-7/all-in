//! The Save Slot (#166): one player's whole between-runs record, kept in one
//! file per slot. Vocabulary follows `GLOSSARY.md`.
//!
//! Only what carries across runs lives here: Golden Chips, The Wheel,
//! whether The House has been beaten, and the Legacy Perk waiting for the
//! next run. Quitting mid-run loses the run. Only slot 1 is used until the
//! slot picker ships.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::legacy::LegacyPerk;
use crate::run::{Card, Tell};
use crate::state::AppState;
use crate::wheel::{GOLDEN_CHIPS_PER_WIN, Square};

/// The slot every game plays in until there's a picker.
pub const SLOT: u8 = 1;

/// One player's between-runs record.
///
/// Every field falls back to its default when the file doesn't have it, so a
/// later section (the run, for mid-run saves) can be added without a
/// migration: an older file simply reads as having none.
#[derive(Resource, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(default)]
pub struct SaveSlot {
    /// Golden Chips in hand, not counting the ones placed on The Wheel.
    pub golden_chips: u32,
    /// The rank of each square: how many Golden Chips sit on it. An empty
    /// square isn't listed.
    pub wheel: BTreeMap<Square, u8>,
    /// Beating The House once opens The Wheel.
    pub house_beaten: bool,
    /// Picked after a House win, for the next run only. The next run takes
    /// it out as it starts.
    pub legacy_perk: Option<LegacyPerk>,
}

/// A [`Card`] as the file holds it: the same three things, with the name
/// owned, since a card read off disk has no `'static` name to point at.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SavedCard {
    pub name: String,
    pub face_value: u32,
    pub tell: Option<Tell>,
}

impl From<&Card> for SavedCard {
    fn from(card: &Card) -> Self {
        SavedCard {
            name: card.name.to_owned(),
            face_value: card.face_value,
            tell: card.tell,
        }
    }
}

impl SavedCard {
    /// Back into a playable [`Card`]. The name is leaked to get its
    /// `'static`: one deck's worth of short strings, once per run at most.
    pub fn card(&self) -> Card {
        Card {
            name: self.name.clone().leak(),
            face_value: self.face_value,
            tell: self.tell,
        }
    }
}

/// Where slot `slot` lives inside `dir`.
pub fn slot_file(dir: &Path, slot: u8) -> PathBuf {
    dir.join(format!("slot{slot}.ron"))
}

/// The per-OS data directory, or `None` on a machine without one, where the
/// game plays on unsaved.
pub fn data_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "All In").map(|dirs| dirs.data_dir().to_path_buf())
}

/// Read the slot at `file`. A missing file is a fresh slot. So is one that
/// can't be read or parsed, which is set aside as `<file>.bad` first, so the
/// next write doesn't destroy what might still be recovered. Never panics.
pub fn load(file: &Path) -> SaveSlot {
    let text = match std::fs::read(file) {
        Ok(bytes) => String::from_utf8(bytes).map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return SaveSlot::default(),
        Err(e) => Err(e.to_string()),
    };
    match text.and_then(|text| ron::from_str(&text).map_err(|e| e.to_string())) {
        Ok(slot) => slot,
        Err(e) => {
            let bad = file.with_extension("ron.bad");
            warn!(
                "{} is unreadable ({e}); moved to {}, starting a fresh slot",
                file.display(),
                bad.display()
            );
            if let Err(e) = std::fs::rename(file, &bad) {
                warn!("couldn't move {} aside: {e}", file.display());
            }
            SaveSlot::default()
        }
    }
}

/// Write `slot` to `file`, through a temporary file and a rename, so a crash
/// mid-write leaves the old save rather than half a new one.
pub fn store(slot: &SaveSlot, file: &Path) -> std::io::Result<()> {
    let text = ron::ser::to_string_pretty(slot, ron::ser::PrettyConfig::default())
        .map_err(std::io::Error::other)?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = file.with_extension("ron.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, file)
}

/// Loads the slot at startup and writes it back whenever it changes, which
/// covers both a run ending and The Wheel being rearranged.
pub struct SavePlugin {
    /// `None` plays unsaved.
    pub file: Option<PathBuf>,
}

impl Default for SavePlugin {
    fn default() -> Self {
        SavePlugin {
            file: data_dir().map(|dir| slot_file(&dir, SLOT)),
        }
    }
}

/// Where the loaded slot is written back to.
#[derive(Resource)]
struct SaveFile(Option<PathBuf>);

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        let slot = self.file.as_deref().map(load).unwrap_or_default();
        app.insert_resource(slot)
            .insert_resource(SaveFile(self.file.clone()))
            .add_systems(OnEnter(AppState::Ending), beat_the_house)
            .add_systems(Last, write_on_change);
    }
}

/// The Ending is only reached by beating The House, and each time pays
/// Golden Chips.
fn beat_the_house(mut slot: ResMut<SaveSlot>) {
    slot.house_beaten = true;
    slot.golden_chips += GOLDEN_CHIPS_PER_WIN;
}

/// A failed write is logged and the game plays on: the slot in memory is
/// still right, and the next change tries again.
fn write_on_change(slot: Res<SaveSlot>, file: Res<SaveFile>) {
    if !slot.is_changed() || slot.is_added() {
        return;
    }
    if let Some(path) = &file.0
        && let Err(e) = store(&slot, path)
    {
        warn!("couldn't save to {}: {e}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    /// A fresh, empty directory of the test's own under the system temp dir.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("all-in-save-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn played_slot() -> SaveSlot {
        SaveSlot {
            golden_chips: 2,
            wheel: BTreeMap::from([(Square::Bankroll, 3), (Square::EarlyRead, 1)]),
            house_beaten: true,
            legacy_perk: Some(LegacyPerk::VictoryLap {
                deck: vec![
                    SavedCard {
                        name: "Hot Streak".into(),
                        face_value: 3,
                        tell: Some(Tell::Streak),
                    },
                    SavedCard {
                        name: "Pawned Ring".into(),
                        face_value: 8,
                        tell: None,
                    },
                ],
            }),
        }
    }

    #[test]
    fn a_slot_round_trips_through_its_file() {
        let file = slot_file(&scratch("round-trip"), SLOT);
        let slot = played_slot();
        store(&slot, &file).unwrap();
        assert_eq!(load(&file), slot);
    }

    #[test]
    fn storing_twice_overwrites() {
        let file = slot_file(&scratch("overwrite"), SLOT);
        store(&played_slot(), &file).unwrap();
        let fresh = SaveSlot::default();
        store(&fresh, &file).unwrap();
        assert_eq!(load(&file), fresh);
    }

    #[test]
    fn a_missing_file_is_a_fresh_slot() {
        let file = slot_file(&scratch("missing"), SLOT);
        assert_eq!(load(&file), SaveSlot::default());
    }

    #[test]
    fn a_corrupt_file_is_a_fresh_slot_and_is_kept_aside() {
        let file = slot_file(&scratch("corrupt"), SLOT);
        std::fs::write(&file, "(golden_chips: lots").unwrap();
        assert_eq!(load(&file), SaveSlot::default());
        assert!(!file.exists());
        let bad = file.with_extension("ron.bad");
        assert_eq!(std::fs::read_to_string(bad).unwrap(), "(golden_chips: lots");
    }

    #[test]
    fn a_file_that_isnt_text_is_a_fresh_slot() {
        let file = slot_file(&scratch("binary"), SLOT);
        std::fs::write(&file, [0xff, 0xfe, 0x00, 0x9f]).unwrap();
        assert_eq!(load(&file), SaveSlot::default());
    }

    #[test]
    fn a_file_missing_a_section_reads_it_as_empty() {
        // What an older file looks like once a run section is added: the
        // fields it never wrote come back as their defaults.
        let file = slot_file(&scratch("partial"), SLOT);
        std::fs::write(&file, "(golden_chips: 3)").unwrap();
        assert_eq!(
            load(&file),
            SaveSlot {
                golden_chips: 3,
                ..default()
            }
        );
    }

    #[test]
    fn a_file_with_a_section_this_build_doesnt_know_still_loads() {
        // What a newer file looks like to this build.
        let file = slot_file(&scratch("newer"), SLOT);
        std::fs::write(&file, "(house_beaten: true, run: (floor: 2))").unwrap();
        assert!(load(&file).house_beaten);
    }

    #[test]
    fn a_card_survives_being_saved() {
        let card = Card {
            name: "Deed to the House",
            face_value: 4,
            tell: Some(Tell::AllIn),
        };
        assert_eq!(SavedCard::from(&card).card(), card);
    }

    fn app_saving_to(file: &Path) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(StatesPlugin)
            .init_state::<AppState>()
            .add_plugins(SavePlugin {
                file: Some(file.to_path_buf()),
            });
        app.update();
        app
    }

    #[test]
    fn the_slot_on_disk_is_loaded_at_startup() {
        let file = slot_file(&scratch("startup"), SLOT);
        store(&played_slot(), &file).unwrap();
        let app = app_saving_to(&file);
        assert_eq!(*app.world().resource::<SaveSlot>(), played_slot());
    }

    #[test]
    fn starting_up_writes_nothing() {
        let file = slot_file(&scratch("quiet"), SLOT);
        let mut app = app_saving_to(&file);
        app.update();
        assert!(!file.exists());
    }

    fn beat_the_house_in(app: &mut App) {
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Ending);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Lobby);
        app.update();
    }

    #[test]
    fn beating_the_house_is_saved() {
        let file = slot_file(&scratch("house"), SLOT);
        let mut app = app_saving_to(&file);
        beat_the_house_in(&mut app);
        assert!(load(&file).house_beaten);
    }

    #[test]
    fn every_win_over_the_house_pays_three_golden_chips() {
        let file = slot_file(&scratch("golden"), SLOT);
        let mut app = app_saving_to(&file);
        beat_the_house_in(&mut app);
        beat_the_house_in(&mut app);
        assert_eq!(load(&file).golden_chips, 6);
    }

    #[test]
    fn a_change_to_the_slot_is_written() {
        let file = slot_file(&scratch("change"), SLOT);
        let mut app = app_saving_to(&file);
        app.world_mut()
            .resource_mut::<SaveSlot>()
            .wheel
            .insert(Square::Trim, 2);
        app.update();
        assert_eq!(load(&file).wheel[&Square::Trim], 2);
    }

    #[test]
    fn with_nowhere_to_save_the_game_plays_on() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(StatesPlugin)
            .init_state::<AppState>()
            .add_plugins(SavePlugin { file: None });
        app.update();
        app.world_mut().resource_mut::<SaveSlot>().golden_chips = 3;
        app.update();
        assert_eq!(app.world().resource::<SaveSlot>().golden_chips, 3);
    }
}
