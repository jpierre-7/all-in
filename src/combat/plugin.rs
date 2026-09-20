//! The Bevy side of combat. Honours the seam in ADR-0001: consumes the
//! `Encounter`, drives a `Duel`, writes `RunState.stack` back, inserts
//! `CombatOutcome`, and makes the one transition combat may make.

use bevy::prelude::*;

use super::duel::{Coin, Duel, Phase, PlayError, TurnResult};
use super::ui::{self, Zone};
use crate::overworld::narrative;
use crate::run::{Card, CombatOutcome, Encounter, EncounterId, RunState, xorshift64};
use crate::state::AppState;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((super::info::InfoPlugin, super::hits::HitsPlugin))
            .add_systems(Startup, ui::load_art)
            .init_resource::<HandoverDelay>()
            .init_resource::<super::peek::Peek>()
            .add_systems(OnEnter(AppState::Combat), start_duel)
            .add_systems(
                Update,
                (
                    super::peek::walk.run_if(not(resource_exists::<Leaving>)),
                    take_input.run_if(not(resource_exists::<Leaving>)),
                    leave_when_ready.run_if(resource_exists::<Leaving>),
                    ui::redraw.run_if(resource_exists_and_changed::<ActiveDuel>),
                    ui::hover_cards,
                    super::peek::show,
                )
                    .chain()
                    .run_if(in_state(AppState::Combat)),
            );
    }
}

/// The duel in progress plus what the screen needs to say about it.
#[derive(Resource)]
pub struct ActiveDuel {
    pub duel: Duel,
    /// Who is across the table, for the art the screen picks by encounter.
    pub id: EncounterId,
    pub enemy_name: &'static str,
    /// Set after playing an All In: the next digit names the sacrifice.
    pub awaiting_sacrifice: Option<usize>,
    /// What the last turn did, for the feedback line.
    pub last_turn: Option<TurnResult>,
    /// What the last keypress did, for the feedback line.
    pub notice: Option<String>,
    /// The Arcade's script (#40). `None` in a real duel.
    pub guide: Option<Guide>,
    /// The keyboard's place in the Draw, for the Peek (#106). `Left` and
    /// `Right` move it; the screen brackets the slot.
    pub pointer: Option<usize>,
}

/// Walks the player through the scripted first turn, one expected key at a
/// time, then lets go.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Guide {
    step: usize,
}

impl Guide {
    const SCRIPT: [KeyCode; 8] = [
        KeyCode::Digit1,
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit1,
        KeyCode::Digit1,
        KeyCode::Digit1,
        KeyCode::Enter,
        KeyCode::KeyP,
    ];

    pub fn is_free_play(&self) -> bool {
        self.step >= Self::SCRIPT.len()
    }

    /// What the screen tells the player to do next.
    pub fn prompt(&self) -> &'static str {
        narrative::TUTORIAL_STEPS
            .get(self.step)
            .copied()
            .unwrap_or(narrative::TUTORIAL_HINT)
    }

    /// Whether this input is the one the script is waiting for; if so the
    /// script advances. A card arrives as `digit` whether it came from a
    /// number key or a click on the card; everything else as a key. Free
    /// play accepts everything.
    fn accepts(&mut self, keys: &ButtonInput<KeyCode>, digit: Option<u8>) -> bool {
        let Some(expected) = Self::SCRIPT.get(self.step) else {
            return true;
        };
        let matched = match expected {
            KeyCode::Digit1 => digit == Some(1),
            KeyCode::Digit2 => digit == Some(2),
            KeyCode::Enter => {
                keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter)
            }
            other => keys.just_pressed(*other),
        };
        if matched {
            self.step += 1;
        }
        matched
    }
}

/// How long the table stays up after the last blow, so the final hit
/// marker (#67) plays before the hand-over. Tests set it to zero.
#[derive(Resource, Debug, Clone, Copy)]
pub struct HandoverDelay(pub f32);

impl Default for HandoverDelay {
    fn default() -> Self {
        Self(0.9)
    }
}

/// The duel is decided and the table is holding for `HandoverDelay` before
/// combat makes its one transition. Input is ignored meanwhile.
#[derive(Resource, Debug)]
struct Leaving {
    outcome: CombatOutcome,
    age: f32,
}

fn leave_when_ready(
    mut commands: Commands,
    time: Res<Time>,
    delay: Res<HandoverDelay>,
    mut leaving: ResMut<Leaving>,
    mut next: ResMut<NextState<AppState>>,
) {
    leaving.age += time.delta_secs();
    if leaving.age >= delay.0 {
        let outcome = leaving.outcome;
        commands.remove_resource::<Leaving>();
        hand_over(&mut commands, outcome, &mut next);
    }
}

/// A pinned seed for every duel this session, set by the dev entry point
/// (`--seed`, #33). Absent in a normal game, where the clock shuffles.
///
/// It advances per duel rather than handing out the same number each time, so
/// a pinned session replays as a whole run — the same cards in the same fights
/// in the same order — instead of dealing every fight identically.
#[derive(Resource, Debug)]
pub struct DuelSeed(u64);

impl DuelSeed {
    /// Zero is the one number xorshift64 cannot start from - it is a fixed
    /// point - and `--seed 0` is the first thing anyone types. Anything else
    /// is taken as given: rounding seeds off (an `| 1`, say) would quietly
    /// hand two different numbers the same deal.
    const INSTEAD_OF_ZERO: u64 = 0x9e37_79b9_7f4a_7c15;

    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    pub fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            Self::INSTEAD_OF_ZERO
        } else {
            seed
        })
    }

    /// xorshift64 never returns zero from a non-zero state, so the stream
    /// keeps itself alive.
    fn next(&mut self) -> u64 {
        xorshift64(&mut self.0)
    }
}

fn start_duel(
    mut commands: Commands,
    encounter: Res<Encounter>,
    run: Res<RunState>,
    time: Res<Time>,
    pinned: Option<ResMut<DuelSeed>>,
) {
    let seed = match pinned {
        Some(mut pinned) => pinned.next(),
        None => time.elapsed_secs_f64().to_bits() | 1,
    };
    // Everything the run has picked up lands here, in one place: the deck the
    // rewards built, the Plays and Blinds the perks bought, the coin Slotz
    // rigged, and whatever is left of the Loaded Dice.
    let tutorial = encounter.id == EncounterId::Tutorial;
    let duel = if tutorial {
        // The Arcade: a fixed deal on both sides of the table, a fresh Stack,
        // a coin that can't lose, and nothing the run has picked up (#40).
        Duel::new(
            crate::run::tutorial_deal(),
            crate::run::STARTING_STACK,
            5,
            encounter.enemy.clone(),
        )
        .with_opposing(crate::run::tutorial_opposing())
        .with_coin(Coin {
            player_pct: 100,
            best_of: 1,
        })
    } else {
        let mut enemy = encounter.enemy.clone();
        enemy.blinds = run.blinds(enemy.blinds);
        // The Hole Card rule rides in on the enemy itself (#5), so there is
        // nothing to switch on here: The House is the only one carrying one.
        Duel::new(
            shuffled(run.deck.clone(), seed),
            run.stack,
            run.plays(),
            enemy,
        )
        .with_seed(seed.rotate_left(17))
        .with_coin(Coin::for_perks(&run.perks))
        .with_loaded_dice(run.loaded_dice())
    };

    commands.insert_resource(ActiveDuel {
        duel,
        id: encounter.id,
        enemy_name: encounter.enemy.name,
        awaiting_sacrifice: None,
        last_turn: None,
        notice: None,
        guide: tutorial.then(Guide::default),
        pointer: None,
    });
}

/// Fisher-Yates, same as the duel's own reshuffle.
fn shuffled(mut deck: Vec<Card>, mut rng: u64) -> Vec<Card> {
    for i in (1..deck.len()).rev() {
        let j = (xorshift64(&mut rng) % (i as u64 + 1)) as usize;
        deck.swap(i, j);
    }
    deck
}

fn take_input(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    active: Option<ResMut<ActiveDuel>>,
    info: Option<Res<super::info::InfoOpen>>,
    clicked: Query<(&Interaction, &ui::CardSlot), Changed<Interaction>>,
    mut run: ResMut<RunState>,
    mut next: ResMut<NextState<AppState>>,
) {
    let Some(mut active) = active else { return };
    // The glossary is up, or is about to be: the table hears nothing.
    if info.is_some() || super::info::toggled(&keys) {
        return;
    }
    // A click anywhere on the table: a card in the Draw picks the slot it
    // goes into, a card already in the row comes back out.
    let click = card_click(&clicked);
    // A card out of the Draw, from either the number keys or a click on it
    // (#58). From here on the two are the same thing.
    let digit = card_key(&keys).or_else(|| match click {
        Some(slot) if slot.zone == Zone::Draw => Some(slot.index as u8 + 1),
        _ => None,
    });
    // A card out of the row: clicked, or Backspace for the last one placed.
    let lifting = match click {
        Some(slot) if slot.zone == Zone::Row => Some(slot.index),
        _ => keys
            .just_pressed(KeyCode::Backspace)
            .then(|| active.duel.row().len().checked_sub(1))
            .flatten(),
    };

    if active.guide.is_some() {
        // Esc leaves the Arcade from anywhere; the overworld routes it home.
        if keys.just_pressed(KeyCode::Escape) && active.awaiting_sacrifice.is_none() {
            hand_over(&mut commands, CombatOutcome::Lost, &mut next);
            return;
        }
        // While the script runs, only the key it names does anything; every
        // other key just leaves the prompt up.
        let pressed_something = keys.get_just_pressed().next().is_some() || digit.is_some();
        let accepted = active
            .guide
            .as_mut()
            .is_none_or(|g| g.accepts(&keys, digit));
        if pressed_something && !accepted {
            active.notice = None;
            return;
        }
    }

    if let Some(digit) = digit {
        let index = usize::from(digit - 1);
        let result = match active.awaiting_sacrifice.take() {
            Some(all_in) => active.duel.place(all_in, Some(index)),
            None => match active.duel.place(index, None) {
                Err(PlayError::AllInNeedsSacrifice) => {
                    active.awaiting_sacrifice = Some(index);
                    active.notice = Some("All In. Which card do you burn?".into());
                    return;
                }
                other => other,
            },
        };
        let notice = match result {
            Ok(slot) => placed_line(&active.duel, slot),
            Err(PlayError::RowIsFull) => {
                "Every slot you can cover is covered. Enter to confirm, or click a card in your row to take it back.".into()
            }
            Err(PlayError::NoSuchCard) => "No card there.".into(),
            Err(PlayError::AllInNeedsSacrifice) => "All In needs a sacrifice.".into(),
            Err(PlayError::NotAllIn) => "Only All In burns a card.".into(),
            Err(PlayError::HandIsFinal) => "The rows are down. Push (P) or Hold (H).".into(),
        };
        active.notice = Some(notice);
        return;
    }

    // Taking a card back out of the row. Everything to its right slides left,
    // so the Tells that read a neighbour read a new one.
    if let Some(slot) = lifting
        && active.awaiting_sacrifice.is_none()
    {
        let notice = match active.duel.lift(slot) {
            Ok(card) => format!(
                "{} back in the Draw. Your row reads {}.",
                card.name,
                active.duel.hand()
            ),
            Err(PlayError::HandIsFinal) => "The rows are down. Push (P) or Hold (H).".into(),
            Err(_) => "Nothing in that slot.".into(),
        };
        active.notice = Some(notice);
        return;
    }

    // Step 5: the prompt is up and only Push or Hold answers it.
    if active.duel.phase() == Phase::PushYourLuck {
        let pushed = keys.just_pressed(KeyCode::KeyP);
        let held = keys.just_pressed(KeyCode::KeyH);
        if !pushed && !held {
            return;
        }
        let result = if pushed {
            active.duel.push()
        } else {
            active.duel.hold()
        };
        let Some(result) = result else { return };
        active.last_turn = Some(result);
        active.notice = None;
        finish_if_over(&mut commands, &mut active, &mut run, &mut next);
        return;
    }

    if keys.just_pressed(KeyCode::Escape) && active.awaiting_sacrifice.is_some() {
        // Backing out of an All In before naming the sacrifice.
        active.awaiting_sacrifice = None;
        active.notice = Some("Never mind.".into());
        return;
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter) {
        if active.awaiting_sacrifice.is_some() {
            active.notice = Some("Name the sacrifice first (1-7), or Esc.".into());
            return;
        }
        // Both rows turn over. A Hand that beats the Opposing Cards puts the
        // Push Your Luck prompt up instead of resolving; a Whiff, or a tie
        // that pays nobody, resolves here.
        let Some(result) = active.duel.confirm() else {
            active.notice = None;
            return;
        };
        active.last_turn = Some(result);
        active.notice = None;
        finish_if_over(&mut commands, &mut active, &mut run, &mut next);
    }
}

/// What the table says when a card lands in the row. Nothing resolves until
/// the rows turn over, so this is what the row reads so far and what is left
/// to cover — not what the card was "worth", which nothing yet knows.
fn placed_line(duel: &Duel, slot: usize) -> String {
    let left = duel.plays_left();
    let mut line = format!("Slot {}. Your row reads {}.", slot + 1, duel.hand());
    if left == 0 {
        line.push_str("  Enter to confirm.");
    } else {
        let cards = if left == 1 { "card" } else { "cards" };
        line.push_str(&format!("  {left} more {cards} to cover the row."));
    }
    line
}

/// Write the Stack back and hand over at `PostCombat` once one Stack is out.
/// The only transition combat ever makes (ADR-0001).
fn finish_if_over(
    commands: &mut Commands,
    active: &mut ActiveDuel,
    run: &mut RunState,
    next: &mut NextState<AppState>,
) {
    let Some(outcome) = active.duel.outcome() else {
        return;
    };
    if active.guide.is_none() {
        // The Arcade never touches the run.
        run.stack = active.duel.player_stack();
        run.set_loaded_dice(active.duel.dice_left());
    }
    // Hold the table for the last hit marker, then go.
    let _ = next;
    commands.insert_resource(Leaving { outcome, age: 0.0 });
}

fn hand_over(commands: &mut Commands, outcome: CombatOutcome, next: &mut NextState<AppState>) {
    commands.remove_resource::<ActiveDuel>();
    commands.remove_resource::<Encounter>();
    commands.insert_resource(outcome);
    next.set(AppState::PostCombat);
}

/// The card just clicked, wherever on the table it was.
fn card_click(
    clicked: &Query<(&Interaction, &ui::CardSlot), Changed<Interaction>>,
) -> Option<ui::CardSlot> {
    clicked
        .iter()
        .find(|(interaction, _)| **interaction == Interaction::Pressed)
        .map(|(_, slot)| *slot)
}

/// 1-7, top row or numpad: the Draw slot to play or to sacrifice.
fn card_key(keys: &ButtonInput<KeyCode>) -> Option<u8> {
    keys.get_just_pressed().find_map(|key| match key {
        KeyCode::Digit1 | KeyCode::Numpad1 => Some(1),
        KeyCode::Digit2 | KeyCode::Numpad2 => Some(2),
        KeyCode::Digit3 | KeyCode::Numpad3 => Some(3),
        KeyCode::Digit4 | KeyCode::Numpad4 => Some(4),
        KeyCode::Digit5 | KeyCode::Numpad5 => Some(5),
        KeyCode::Digit6 | KeyCode::Numpad6 => Some(6),
        KeyCode::Digit7 | KeyCode::Numpad7 => Some(7),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use bevy::input::ButtonInput;
    use bevy::prelude::*;
    use bevy::state::app::StatesPlugin;

    use super::super::duel::Coin;
    use super::super::ui;
    use super::{ActiveDuel, CombatPlugin, DuelSeed};
    use crate::run::{
        Card, CombatOutcome, Deal, Encounter, EncounterId, Enemy, Perk, RisingBlinds, RunState,
    };
    use crate::state::AppState;

    /// An enemy that deals five Opposing Cards worth nothing at all, so a
    /// test that hasn't laid its own row out is facing an Edge of zero.
    pub(super) fn shill(stack: u32) -> Enemy {
        Enemy {
            name: "shill",
            stack,
            deal: Deal {
                row: 5,
                low: 0,
                high: 0,
                tell_pct: 0,
                tells: &[],
                hidden_pct: 0,
            },
            blinds: RisingBlinds {
                every_turns: 2,
                cards: 1,
            },
            hole_card: None,
        }
    }

    /// Combat on its own: no window, no overworld. The test plays the
    /// overworld's part by inserting the Encounter and entering the state,
    /// then lays a known row across the table worth `edge`.
    pub(super) fn table(player_stack: u32, enemy_stack: u32, edge: u32) -> App {
        table_with(player_stack, enemy_stack, edge, Vec::new())
    }

    pub(super) fn table_with(
        player_stack: u32,
        enemy_stack: u32,
        edge: u32,
        perks: Vec<Perk>,
    ) -> App {
        table_for_run(
            RunState {
                stack: player_stack,
                perks,
                ..RunState::new()
            },
            enemy_stack,
            edge,
        )
    }

    /// A table set for a run that has already picked things up.
    pub(super) fn table_for_run(run: RunState, enemy_stack: u32, edge: u32) -> App {
        let mut app = dealt_table(run, shill(enemy_stack));
        lay_out(&mut app, edge);
        // Laying the row out changed the duel, so the screen is a frame
        // behind it. The mouse tests click on nodes; let them be the right
        // ones before anyone reaches for them.
        app.update();
        app
    }

    /// The same table with whatever `enemy` deals itself left alone, for the
    /// tests that are about the deal rather than about a known Edge.
    pub(super) fn dealt_table(run: RunState, enemy: Enemy) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(StatesPlugin)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_state::<AppState>()
            .insert_resource(run)
            .insert_resource(super::HandoverDelay(0.0))
            .insert_resource(Encounter {
                id: EncounterId::FloorMinion,
                enemy,
            })
            .add_plugins(CombatPlugin);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Combat);
        app.update();
        app
    }

    /// Five slots across the table, the first card carrying the whole Edge
    /// and the other four carrying nothing. Fixed, so it comes back the same
    /// every turn the way a flat House Edge used to.
    pub(super) fn lay_out(app: &mut App, edge: u32) {
        let mut row = vec![Card {
            name: "the edge",
            stack: edge,
            tell: None,
        }];
        row.extend((1..5).map(|_| Card {
            name: "nothing",
            stack: 0,
            tell: None,
        }));
        app.world_mut()
            .resource_mut::<ActiveDuel>()
            .duel
            .lay_out(row);
    }

    /// The card node for a slot in one of the three rows.
    pub(super) fn node_at(app: &mut App, zone: ui::Zone, index: usize) -> Entity {
        let wanted = ui::CardSlot { zone, index };
        app.world_mut()
            .query::<(Entity, &ui::CardSlot)>()
            .iter(app.world())
            .find(|(_, slot)| **slot == wanted)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("a card node for {zone:?} slot {index}"))
    }

    /// A mouse click on a card, as the UI focus system would report it: the
    /// node's Interaction goes to Pressed for a frame.
    pub(super) fn click_at(app: &mut App, zone: ui::Zone, index: usize) {
        let entity = node_at(app, zone, index);
        *app.world_mut().get_mut::<Interaction>(entity).unwrap() = Interaction::Pressed;
        app.update();
        if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(entity) {
            *interaction = Interaction::None;
        }
        app.update();
    }

    /// A click on a card in the Draw, which is most of them.
    pub(super) fn click(app: &mut App, index: usize) {
        click_at(app, ui::Zone::Draw, index);
    }

    pub(super) fn press(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.update();
    }

    pub(super) fn state(app: &App) -> AppState {
        *app.world().resource::<State<AppState>>().get()
    }

    pub(super) fn rig(app: &mut App, coin: Coin) {
        app.world_mut()
            .resource_mut::<ActiveDuel>()
            .duel
            .set_coin(coin);
    }

    /// A table whose duels roll off a pinned seed, the way `--seed` sets them.
    fn seeded_table(seed: u64) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(StatesPlugin)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_state::<AppState>()
            .insert_resource(RunState::new())
            .insert_resource(DuelSeed::new(seed))
            .insert_resource(Encounter {
                id: EncounterId::PitBoss,
                enemy: Enemy::for_encounter(EncounterId::PitBoss),
            })
            .add_plugins(CombatPlugin);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Combat);
        app.update();
        app
    }

    fn draw(app: &App) -> Vec<crate::run::Card> {
        app.world().resource::<ActiveDuel>().duel.draw().to_vec()
    }

    #[test]
    fn a_pinned_seed_deals_the_same_draw_twice_running() {
        assert_eq!(draw(&seeded_table(42)), draw(&seeded_table(42)));
    }

    #[test]
    fn a_different_seed_deals_a_different_draw() {
        assert_ne!(draw(&seeded_table(42)), draw(&seeded_table(43)));
        // Zero is special-cased; it still has to deal like a seed.
        assert_ne!(draw(&seeded_table(0)), draw(&seeded_table(1)));
    }

    #[test]
    fn a_pinned_session_deals_each_fight_differently() {
        let mut app = seeded_table(42);
        let first = draw(&app);

        // Back out to the Lobby and into Combat again: the next duel of the
        // same pinned session, not a replay of the first.
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Lobby);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Combat);
        app.update();

        assert_ne!(first, draw(&app));
    }

    #[test]
    fn the_big_shots_table_plays_under_the_hole_card_rule() {
        let app = dealt_table(
            RunState::new(),
            Enemy::for_encounter(EncounterId::TheHouse),
        );

        let duel = &app.world().resource::<ActiveDuel>().duel;
        assert_eq!(duel.margin(), Some(1));
        // The last Opposing Card is face down and worth nothing yet.
        let last = duel.opposing().last().expect("a row");
        assert!(!last.revealed);
        assert_eq!(last.card.stack, 0);
        assert_eq!(duel.showing().1, 1, "one card kept back");
    }

    #[test]
    fn the_enemy_lays_its_row_down_before_the_player_touches_a_card() {
        let app = dealt_table(
            RunState::new(),
            Enemy::for_encounter(EncounterId::PitBoss),
        );

        let duel = &app.world().resource::<ActiveDuel>().duel;
        assert_eq!(duel.slots(), 4, "the Pit Boss deals four");
        assert!(duel.row().is_empty());
        assert!(duel.opposing()[0].revealed, "the first is always face up");
        assert!(duel.opposing().iter().all(|o| o.card.stack >= 4));
    }

    #[test]
    fn a_card_goes_into_the_row_and_can_be_taken_back_out_again() {
        let mut app = table(40, 999, 10);
        let held = app.world().resource::<ActiveDuel>().duel.draw()[0].clone();
        if held.tell == Some(crate::run::Tell::AllIn) {
            return; // this deal wants a sacrifice; the All In tests cover it
        }

        press(&mut app, KeyCode::Digit1);
        let duel = &app.world().resource::<ActiveDuel>().duel;
        assert_eq!(duel.row().len(), 1);
        assert_eq!(duel.draw().len(), 6);
        assert_eq!(duel.hand(), held.stack);

        press(&mut app, KeyCode::Backspace);
        let duel = &app.world().resource::<ActiveDuel>().duel;
        assert!(duel.row().is_empty());
        assert_eq!(duel.draw().len(), 7);
        assert_eq!(duel.hand(), 0);
    }

    #[test]
    fn the_duel_remembers_which_encounter_it_is() {
        // The screen picks the portrait off this, so it has to survive the
        // handoff from the overworld.
        let mut app = table(50, 35, 1);
        assert_eq!(
            app.world().resource::<ActiveDuel>().id,
            EncounterId::FloorMinion
        );

        app.world_mut().resource_mut::<Encounter>().id = EncounterId::PitBoss;
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Lobby);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Combat);
        app.update();

        assert_eq!(
            app.world().resource::<ActiveDuel>().id,
            EncounterId::PitBoss
        );
    }

    #[test]
    fn entering_combat_deals_a_draw_from_the_encounter() {
        let app = table(40, 30, 20);

        let active = app.world().resource::<ActiveDuel>();
        assert_eq!(active.duel.draw().len(), 7);
        assert_eq!(active.duel.enemy_stack(), 30);
        assert_eq!(active.duel.player_stack(), 40);
        assert_eq!(state(&app), AppState::Combat);
    }

    #[test]
    fn a_lost_duel_writes_the_stack_back_and_hands_over_at_post_combat() {
        // Nothing played, House Edge 50 against a 10 Stack: one Whiff ends it.
        let mut app = table(10, 999, 50);

        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::KeyH);

        assert_eq!(state(&app), AppState::PostCombat);
        assert_eq!(
            *app.world().resource::<CombatOutcome>(),
            CombatOutcome::Lost
        );
        assert_eq!(app.world().resource::<RunState>().stack, 0);
        assert!(app.world().get_resource::<Encounter>().is_none());
        assert!(app.world().get_resource::<ActiveDuel>().is_none());
    }

    #[test]
    fn a_won_duel_reports_won_and_keeps_your_remaining_stack() {
        // House Edge 0 and a 1-chip enemy: any card wins.
        let mut app = table(40, 1, 0);

        // Slot 1 might be an All In; if so the next key names the sacrifice.
        press(&mut app, KeyCode::Digit1);
        if app
            .world()
            .resource::<ActiveDuel>()
            .awaiting_sacrifice
            .is_some()
        {
            press(&mut app, KeyCode::Digit2); // All In burns the next card
        }
        press(&mut app, KeyCode::Enter); // shows the Hand: the prompt goes up
        press(&mut app, KeyCode::KeyH); // Hold

        assert_eq!(state(&app), AppState::PostCombat);
        assert_eq!(*app.world().resource::<CombatOutcome>(), CombatOutcome::Won);
        assert_eq!(app.world().resource::<RunState>().stack, 40);
    }
}

#[cfg(test)]
mod push_your_luck_tests {
    use bevy::prelude::*;

    use super::ActiveDuel;
    use super::tests::{press, rig, state, table, table_with};
    use crate::combat::duel::{Coin, Outcome, Phase, Push};
    use crate::run::{CombatOutcome, Perk, RunState};
    use crate::state::AppState;

    /// Play the card in slot 1, burning slot 2 if it turns out to be All In.
    fn play_one(app: &mut App) {
        press(app, KeyCode::Digit1);
        if app
            .world()
            .resource::<ActiveDuel>()
            .awaiting_sacrifice
            .is_some()
        {
            press(app, KeyCode::Digit2);
        }
    }

    /// Spend every Play, so The Hand is at least 2 per Play.
    fn play_them_all(app: &mut App) {
        while app.world().resource::<ActiveDuel>().duel.plays_left() > 0 {
            play_one(app);
        }
    }

    #[test]
    fn showing_a_clearing_hand_puts_the_prompt_up_without_resolving() {
        let mut app = table(40, 999, 0);
        play_one(&mut app);

        press(&mut app, KeyCode::Enter);

        let active = app.world().resource::<ActiveDuel>();
        assert_eq!(active.duel.phase(), Phase::PushYourLuck);
        assert_eq!(active.last_turn, None);
        assert_eq!(active.duel.enemy_stack(), 999);
        assert_eq!(state(&app), AppState::Combat);
    }

    #[test]
    fn a_whiff_is_offered_the_prompt_and_hold_takes_it() {
        let mut app = table(40, 999, 30);

        press(&mut app, KeyCode::Enter);
        assert_eq!(
            app.world().resource::<ActiveDuel>().duel.phase(),
            Phase::PushYourLuck
        );

        press(&mut app, KeyCode::KeyH);

        let active = app.world().resource::<ActiveDuel>();
        assert_eq!(active.duel.phase(), Phase::Playing);
        assert_eq!(active.last_turn.unwrap().kind, Outcome::Whiff(30));
        assert_eq!(active.duel.player_stack(), 10);
    }

    #[test]
    fn pushing_a_whiff_and_winning_forgives_it() {
        let mut app = table(40, 999, 30);
        rig(&mut app, Coin::SURE_THING);
        press(&mut app, KeyCode::Enter);

        press(&mut app, KeyCode::KeyP);

        let active = app.world().resource::<ActiveDuel>();
        let turn = active.last_turn.unwrap();
        assert_eq!(turn.pyl, Some(Push::Won));
        assert_eq!(turn.kind, Outcome::Whiff(0));
        assert_eq!(active.duel.player_stack(), 40);
    }

    #[test]
    fn pushing_a_whiff_and_losing_doubles_it() {
        let mut app = table(100, 999, 30);
        rig(&mut app, Coin::RIGGED);
        press(&mut app, KeyCode::Enter);

        press(&mut app, KeyCode::KeyP);

        let active = app.world().resource::<ActiveDuel>();
        let turn = active.last_turn.unwrap();
        assert_eq!(turn.pyl, Some(Push::Lost));
        assert_eq!(turn.kind, Outcome::Whiff(60));
        assert_eq!(active.duel.player_stack(), 40);
    }

    #[test]
    fn holding_resolves_the_turn_as_normal() {
        let mut app = table(40, 999, 0);
        play_one(&mut app);
        press(&mut app, KeyCode::Enter);

        press(&mut app, KeyCode::KeyH);

        let active = app.world().resource::<ActiveDuel>();
        let turn = active.last_turn.expect("the turn resolved");
        assert_eq!(turn.pyl, None);
        assert_eq!(active.duel.phase(), Phase::Playing);
        assert_eq!(active.duel.enemy_stack(), 999 - payout(turn.kind));
    }

    #[test]
    fn pushing_and_winning_doubles_the_payout() {
        let mut app = table(40, 999, 0);
        play_one(&mut app);
        press(&mut app, KeyCode::Enter);
        rig(&mut app, Coin::SURE_THING);

        press(&mut app, KeyCode::KeyP);

        let active = app.world().resource::<ActiveDuel>();
        let turn = active.last_turn.expect("the turn resolved");
        assert_eq!(turn.pyl, Some(Push::Won));
        assert_eq!(payout(turn.kind), turn.hand * 2);
    }

    #[test]
    fn a_lost_push_whiffs_and_can_end_the_duel() {
        // Stack 10 against a House Edge of 10: a lost Push is fatal.
        let mut app = table(10, 999, 10);
        play_them_all(&mut app);
        press(&mut app, KeyCode::Enter);
        rig(&mut app, Coin::RIGGED);

        press(&mut app, KeyCode::KeyP);

        assert_eq!(state(&app), AppState::PostCombat);
        assert_eq!(
            *app.world().resource::<CombatOutcome>(),
            CombatOutcome::Lost
        );
        assert_eq!(app.world().resource::<RunState>().stack, 0);
    }

    #[test]
    fn cards_cannot_be_played_while_the_prompt_is_up() {
        let mut app = table(40, 999, 0);
        play_one(&mut app);
        press(&mut app, KeyCode::Enter);
        let hand = app.world().resource::<ActiveDuel>().duel.hand();

        press(&mut app, KeyCode::Digit2);

        let active = app.world().resource::<ActiveDuel>();
        assert_eq!(active.duel.hand(), hand);
        assert_eq!(active.duel.phase(), Phase::PushYourLuck);
        assert!(
            active.notice.is_some(),
            "the screen says why nothing happened"
        );
    }

    #[test]
    fn push_and_hold_do_nothing_before_the_hand_is_shown() {
        let mut app = table(40, 999, 0);
        play_one(&mut app);

        press(&mut app, KeyCode::KeyP);
        press(&mut app, KeyCode::KeyH);

        let active = app.world().resource::<ActiveDuel>();
        assert_eq!(active.last_turn, None);
        assert_eq!(active.duel.enemy_stack(), 999);
    }

    #[test]
    fn the_slotz_perk_arms_the_best_of_three_coin() {
        let plain = table(40, 999, 20);
        assert_eq!(
            plain.world().resource::<ActiveDuel>().duel.coin(),
            Coin::BASE
        );

        let slotz = table_with(40, 999, 20, vec![Perk::PylBestTwoOfThree]);
        assert_eq!(
            slotz.world().resource::<ActiveDuel>().duel.coin(),
            Coin::SLOTZ
        );
    }

    fn payout(kind: Outcome) -> u32 {
        match kind {
            Outcome::Payout(n) => n,
            Outcome::Whiff(n) => panic!("expected a Payout, got a Whiff of {n}"),
        }
    }
}

#[cfg(test)]
mod run_modifier_tests {
    use bevy::prelude::*;

    use super::ActiveDuel;
    use super::tests::{dealt_table, press, shill, state, table_for_run};
    use crate::run::{CombatOutcome, Perk, Reward, RunState};
    use crate::state::AppState;

    /// A six-card row, so there is a sixth slot for the perk's sixth Play to
    /// go into and a sixth card to go uncovered without it.
    fn wide_table(perks: Vec<Perk>) -> App {
        let mut enemy = shill(999);
        enemy.deal.row = 6;
        dealt_table(
            RunState {
                stack: 400,
                perks,
                ..RunState::new()
            },
            enemy,
        )
    }

    #[test]
    fn the_pit_boss_perk_deals_a_sixth_play() {
        // Six Opposing Cards. Five Plays covers five of them and leaves the
        // sixth sitting there counting for the enemy.
        let plain = wide_table(Vec::new());
        let duel = &plain.world().resource::<ActiveDuel>().duel;
        assert_eq!(duel.slots(), 6);
        assert_eq!(duel.plays_left(), 5);

        let six = wide_table(vec![Perk::SixPlaysSteepBlinds]);

        assert_eq!(six.world().resource::<ActiveDuel>().duel.plays_left(), 6);
    }

    #[test]
    fn the_pit_boss_perk_lays_another_opposing_card_down_every_turn() {
        // Base blinds are every 2 turns at this table, so after one turn the
        // plain row has not grown and the perk's already has.
        let mut plain = wide_table(Vec::new());
        let mut steep = wide_table(vec![Perk::SixPlaysSteepBlinds]);

        for app in [&mut plain, &mut steep] {
            press(app, KeyCode::Enter); // nothing covered: the turn resolves
        }

        assert_eq!(plain.world().resource::<ActiveDuel>().duel.slots(), 6);
        assert_eq!(steep.world().resource::<ActiveDuel>().duel.slots(), 7);
    }

    #[test]
    fn loaded_dice_come_to_the_table_and_what_is_left_goes_home() {
        let mut run = RunState {
            stack: 40,
            ..RunState::new()
        };
        run.apply(Reward::LoadedDice, 1);
        let mut app = table_for_run(run, 1, 0);

        assert_eq!(app.world().resource::<ActiveDuel>().duel.dice_left(), 2);

        // One Hand of nothing at all still clears an Edge of 0 on the dice.
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::KeyH);

        assert_eq!(state(&app), AppState::PostCombat);
        assert_eq!(*app.world().resource::<CombatOutcome>(), CombatOutcome::Won);
        assert_eq!(app.world().resource::<RunState>().loaded_dice(), 1);
    }

    #[test]
    fn the_deck_the_rewards_built_is_the_deck_that_is_dealt() {
        let mut run = RunState {
            stack: 40,
            ..RunState::new()
        };
        run.deck.clear();
        run.apply(Reward::SlotzStreakCards, 1);
        let app = table_for_run(run, 999, 20);

        let draw = app.world().resource::<ActiveDuel>().duel.draw();
        assert_eq!(draw.len(), 3, "a three-card deck deals three cards");
        assert!(draw.iter().all(|c| c.name == "Loose Slot"
            || c.name == "Second Cherry"
            || c.name == "Jackpot Bell"));
    }
}

#[cfg(test)]
mod tutorial_tests {
    use bevy::input::ButtonInput;
    use bevy::prelude::*;
    use bevy::state::app::StatesPlugin;

    use super::{ActiveDuel, CombatPlugin};
    use crate::run::{CombatOutcome, Encounter, EncounterId, Enemy, RunState, STARTING_STACK};
    use crate::state::AppState;

    /// The Arcade: the overworld inserts the Tutorial encounter and enters
    /// Combat, same as any floor would.
    pub(super) fn arcade() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(StatesPlugin)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_state::<AppState>()
            .insert_resource(RunState::new())
            .insert_resource(super::HandoverDelay(0.0))
            .insert_resource(Encounter {
                id: EncounterId::Tutorial,
                enemy: Enemy::for_encounter(EncounterId::Tutorial),
            })
            .add_plugins(CombatPlugin);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Combat);
        app.update();
        app
    }

    fn press(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.update();
    }

    fn active(app: &App) -> &ActiveDuel {
        app.world().resource::<ActiveDuel>()
    }

    fn state(app: &App) -> AppState {
        *app.world().resource::<State<AppState>>().get()
    }

    #[test]
    fn the_arcade_deals_the_scripted_hand_in_order() {
        let app = arcade();
        let names: Vec<&str> = active(&app).duel.draw().iter().map(|c| c.name).collect();
        assert_eq!(
            names,
            [
                "Pawned Ring",
                "Last Dollar",
                "Two of Clubs",
                "Hot Streak",
                "Dealer Blinks",
                "Four of Hearts",
                "Cheap Seat"
            ]
        );
        assert_eq!(active(&app).duel.enemy_stack(), 30);
        assert_eq!(active(&app).duel.house_edge(), 20);
        assert!(active(&app).guide.is_some());
    }

    #[test]
    fn the_wrong_key_plays_nothing_and_keeps_the_prompt() {
        let mut app = arcade();
        let prompt = active(&app).guide.as_ref().unwrap().prompt();

        press(&mut app, KeyCode::Digit3);

        assert_eq!(active(&app).duel.hand(), 0);
        assert_eq!(active(&app).duel.draw().len(), 7);
        assert_eq!(active(&app).guide.as_ref().unwrap().prompt(), prompt);
    }

    #[test]
    fn following_the_script_makes_thirty_two_and_a_doubled_payout() {
        let mut app = arcade();

        press(&mut app, KeyCode::Digit1); // Pawned Ring 8
        press(&mut app, KeyCode::Digit1); // Last Dollar: All In, waits for the burn
        press(&mut app, KeyCode::Digit2); // burn Two of Clubs: +4
        press(&mut app, KeyCode::Digit1); // Hot Streak, doubled: 6
        press(&mut app, KeyCode::Digit1); // Dealer Blinks, doubled: 10
        press(&mut app, KeyCode::Digit1); // Four of Hearts: 4
        assert_eq!(active(&app).duel.hand(), 32);

        press(&mut app, KeyCode::Enter); // show: 32 vs 20, PYL prompt
        press(&mut app, KeyCode::KeyP); // rigged coin: Payout 24

        assert_eq!(active(&app).duel.enemy_stack(), 6);
        assert_eq!(active(&app).duel.player_stack(), STARTING_STACK);
        // The script is done; free play from here.
        assert!(active(&app).guide.as_ref().unwrap().is_free_play());
        assert_eq!(state(&app), AppState::Combat);
    }

    #[test]
    fn escape_leaves_the_arcade_without_touching_the_run() {
        let mut app = arcade();
        press(&mut app, KeyCode::Digit1);
        app.world_mut().resource_mut::<RunState>().stack = 7;

        press(&mut app, KeyCode::Escape);

        assert_eq!(state(&app), AppState::PostCombat);
        assert_eq!(
            *app.world().resource::<CombatOutcome>(),
            CombatOutcome::Lost
        );
        assert_eq!(app.world().resource::<RunState>().stack, 7);
        assert!(app.world().get_resource::<Encounter>().is_none());
    }

    #[test]
    fn winning_the_arcade_never_writes_the_run_stack() {
        let mut app = arcade();
        for key in [
            KeyCode::Digit1,
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit1,
            KeyCode::Digit1,
            KeyCode::Digit1,
            KeyCode::Enter,
            KeyCode::KeyP,
        ] {
            press(&mut app, key);
        }
        app.world_mut().resource_mut::<RunState>().stack = 7;
        // Free play: the dealer has 6 left; any Hand of 26 clears it. Show whatever is dealt.
        for _ in 0..5 {
            let index = 0;
            let _ = index;
            press(&mut app, KeyCode::Digit1);
            if active(&app).awaiting_sacrifice.is_some() {
                press(&mut app, KeyCode::Digit2);
            }
        }
        press(&mut app, KeyCode::Enter);
        if app.world().get_resource::<ActiveDuel>().is_some() && state(&app) == AppState::Combat {
            press(&mut app, KeyCode::KeyH);
        }

        assert_eq!(app.world().resource::<RunState>().stack, 7);
    }
}

#[cfg(test)]
mod info_tests {
    use bevy::prelude::*;

    use super::super::info::{InfoOpen, InfoOverlay};
    use super::ActiveDuel;
    use super::tests::{press, state, table};
    use crate::state::AppState;

    fn open(app: &App) -> bool {
        app.world().get_resource::<InfoOpen>().is_some()
    }
    fn overlays(app: &mut App) -> usize {
        app.world_mut()
            .query::<&InfoOverlay>()
            .iter(app.world())
            .count()
    }

    #[test]
    fn i_opens_the_glossary_over_the_table_and_again_closes_it() {
        let mut app = table(40, 30, 20);
        assert!(!open(&app));

        press(&mut app, KeyCode::KeyI);
        assert!(open(&app));
        assert_eq!(overlays(&mut app), 1);

        press(&mut app, KeyCode::KeyI);
        assert!(!open(&app));
        assert_eq!(overlays(&mut app), 0);
        assert_eq!(state(&app), AppState::Combat);
    }

    #[test]
    fn nothing_underneath_fires_while_the_glossary_is_up() {
        let mut app = table(40, 30, 20);
        press(&mut app, KeyCode::KeyI);

        press(&mut app, KeyCode::Digit1);
        press(&mut app, KeyCode::Enter);

        let active = app.world().resource::<ActiveDuel>();
        assert_eq!(active.duel.hand(), 0);
        assert_eq!(active.duel.draw().len(), 7);
        assert!(open(&app));
    }

    #[test]
    fn escape_closes_the_glossary_and_nothing_else() {
        let mut app = table(40, 30, 20);
        press(&mut app, KeyCode::KeyI);

        press(&mut app, KeyCode::Escape);

        assert!(!open(&app));
        assert_eq!(state(&app), AppState::Combat);
        assert!(app.world().get_resource::<ActiveDuel>().is_some());
    }
}

#[cfg(test)]
mod mouse_tests {
    use bevy::prelude::*;

    use super::super::info::InfoOpen;
    use super::super::ui::Zone;
    use super::ActiveDuel;
    use super::tests::{click, click_at, press, table};

    fn active(app: &App) -> &ActiveDuel {
        app.world().resource::<ActiveDuel>()
    }

    #[test]
    fn clicking_a_card_in_the_draw_covers_a_slot_like_its_number_key() {
        let mut app = table(40, 30, 20);
        // Any card but an All In, which would wait for its burn instead.
        let slot = active(&app)
            .duel
            .draw()
            .iter()
            .position(|c| c.tell != Some(crate::run::Tell::AllIn))
            .expect("a deal with a playable card");
        let stack = active(&app).duel.draw()[slot].stack;

        click(&mut app, slot);

        assert_eq!(active(&app).duel.hand(), stack);
        assert_eq!(active(&app).duel.draw().len(), 6);
        assert_eq!(active(&app).duel.row().len(), 1);
    }

    #[test]
    fn clicking_a_card_in_your_own_row_takes_it_back_out() {
        let mut app = table(40, 30, 20);
        let slot = active(&app)
            .duel
            .draw()
            .iter()
            .position(|c| c.tell != Some(crate::run::Tell::AllIn))
            .expect("a deal with a playable card");
        click(&mut app, slot);
        assert_eq!(active(&app).duel.row().len(), 1);

        click_at(&mut app, Zone::Row, 0);

        assert!(active(&app).duel.row().is_empty());
        assert_eq!(active(&app).duel.draw().len(), 7);
        assert_eq!(active(&app).duel.hand(), 0);
    }

    #[test]
    fn a_click_on_the_enemys_row_does_nothing_at_all() {
        let mut app = table(40, 30, 20);

        click_at(&mut app, Zone::Opposing, 0);

        assert!(active(&app).duel.row().is_empty());
        assert_eq!(active(&app).duel.draw().len(), 7);
    }

    #[test]
    fn an_empty_slot_in_your_row_is_not_a_card_to_take_back() {
        let mut app = table(40, 30, 20);

        // Slot 3 has nothing in it; clicking it must not disturb the row.
        click_at(&mut app, Zone::Row, 3);

        assert!(active(&app).duel.row().is_empty());
        assert_eq!(active(&app).duel.draw().len(), 7);
    }

    #[test]
    fn a_click_names_the_burn_when_an_all_in_is_waiting() {
        let mut app = table(40, 30, 20);
        // Find an All In in the Draw, play it by key, then click the burn.
        let all_in = active(&app)
            .duel
            .draw()
            .iter()
            .position(|c| c.tell == Some(crate::run::Tell::AllIn));
        let Some(all_in) = all_in else { return }; // this seed dealt none; nothing to check
        let key = [
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::Digit4,
            KeyCode::Digit5,
            KeyCode::Digit6,
            KeyCode::Digit7,
        ][all_in];
        press(&mut app, key);
        assert_eq!(active(&app).awaiting_sacrifice, Some(all_in));
        let burn = if all_in == 0 { 1 } else { 0 };

        click(&mut app, burn);

        assert_eq!(active(&app).awaiting_sacrifice, None);
        assert_eq!(active(&app).duel.draw().len(), 5);
    }

    #[test]
    fn clicks_go_nowhere_while_the_glossary_is_up() {
        let mut app = table(40, 30, 20);
        press(&mut app, KeyCode::KeyI);
        assert!(app.world().get_resource::<InfoOpen>().is_some());

        click(&mut app, 0);

        assert_eq!(active(&app).duel.hand(), 0);
    }
}

#[cfg(test)]
mod mouse_tutorial_tests {
    use super::ActiveDuel;
    use super::tests::click;
    use super::tutorial_tests::arcade;

    #[test]
    fn the_script_treats_a_click_on_card_one_as_pressing_one() {
        let mut app = arcade();

        click(&mut app, 3); // wrong card: the script wants card 1
        assert_eq!(app.world().resource::<ActiveDuel>().duel.hand(), 0);

        click(&mut app, 0); // Pawned Ring
        assert_eq!(app.world().resource::<ActiveDuel>().duel.hand(), 8);
    }
}

#[cfg(test)]
mod hit_marker_tests {
    use bevy::prelude::*;

    use super::super::hits::{HitMarker, Side};
    use super::tests::{press, table};

    /// Every marker on screen, with the text its child carries.
    fn markers(app: &mut App) -> Vec<(Side, String)> {
        let found: Vec<(Side, Vec<Entity>)> = app
            .world_mut()
            .query::<(&HitMarker, &Children)>()
            .iter(app.world())
            .map(|(m, c)| (m.side, c.iter().collect()))
            .collect();
        found
            .into_iter()
            .map(|(side, children)| {
                let text = children
                    .into_iter()
                    .find_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
                    .unwrap_or_default();
                (side, text)
            })
            .collect()
    }

    #[test]
    fn a_whiff_floats_the_loss_at_the_players_stack() {
        // Nothing played, Edge 30: Whiff 30.
        let mut app = table(50, 999, 30);

        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::KeyH);

        assert_eq!(markers(&mut app), vec![(Side::Player, "-30".to_string())]);
    }

    #[test]
    fn a_payout_floats_the_hit_at_the_enemys_stack() {
        // Edge 1: whatever the shuffle dealt, one card clears it.
        let mut app = table(50, 999, 1);
        press(&mut app, KeyCode::Digit1);
        if app
            .world()
            .resource::<super::ActiveDuel>()
            .awaiting_sacrifice
            .is_some()
        {
            press(&mut app, KeyCode::Digit2);
        }
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::KeyH);

        let found = markers(&mut app);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, Side::Enemy);
        assert!(found[0].1.starts_with('-'));
    }

    #[test]
    fn the_marker_is_gone_once_its_time_is_up() {
        let mut app = table(50, 999, 30);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::KeyH);
        assert_eq!(markers(&mut app).len(), 1);

        // Run its clock out by hand: the test harness has no real frame time.
        let entity = app
            .world_mut()
            .query_filtered::<Entity, With<HitMarker>>()
            .single(app.world())
            .unwrap();
        app.world_mut().get_mut::<HitMarker>(entity).unwrap().age = 10.0;
        app.update();

        assert!(markers(&mut app).is_empty());
    }

    #[test]
    fn one_marker_per_turn_even_when_the_screen_redraws() {
        let mut app = table(50, 999, 30);
        press(&mut app, KeyCode::Enter);
        // Any input redraws the table; the marker must not multiply.
        press(&mut app, KeyCode::Digit9);
        press(&mut app, KeyCode::KeyH);

        assert_eq!(markers(&mut app).len(), 1);
    }
}

#[cfg(test)]
mod peek_tests {
    use bevy::prelude::*;

    use super::super::peek::{Peek, Tag};
    use super::super::ui::{CardSlot, Zone};
    use super::ActiveDuel;
    use super::tests::{node_at, press, table, table_for_run};
    use crate::run::{Card, RunState, Tell};

    /// A table dealt from a deck where every card is the same, so the test
    /// never has to go looking for a Tell.
    fn table_of(tell: Option<Tell>) -> App {
        let card = Card {
            name: "test card",
            stack: 4,
            tell,
        };
        table_for_run(
            RunState {
                deck: vec![card; 18],
                ..RunState::new()
            },
            30,
            20,
        )
    }

    fn pointer(app: &App) -> Option<usize> {
        app.world().resource::<ActiveDuel>().pointer
    }

    /// The tag on the table, if there is one, and the slot it hangs off.
    fn tag(app: &mut App) -> Option<(Tag, usize)> {
        let (tag, parent) = app
            .world_mut()
            .query::<(&Tag, &ChildOf)>()
            .iter(app.world())
            .map(|(tag, child_of)| (*tag, child_of.parent()))
            .next()?;
        let slot = app
            .world()
            .get::<CardSlot>(parent)
            .expect("a tag hangs off a card");
        Some((tag, slot.index))
    }

    /// A card in the Draw, as the Peek's `Tag` names it.
    fn held(index: usize) -> CardSlot {
        CardSlot {
            zone: Zone::Draw,
            index,
        }
    }

    /// The cursor over a card, as the focus system would report it.
    fn hover_at(app: &mut App, zone: Zone, index: usize) {
        let entity = node_at(app, zone, index);
        *app.world_mut().get_mut::<Interaction>(entity).unwrap() = Interaction::Hovered;
        app.update();
    }

    /// The cursor over the card in `slot` of the Draw.
    fn hover(app: &mut App, slot: usize) {
        hover_at(app, Zone::Draw, slot);
    }

    #[test]
    fn right_walks_onto_the_first_card_and_left_wraps_to_the_last() {
        let mut app = table(40, 30, 20);
        assert_eq!(pointer(&app), None);

        press(&mut app, KeyCode::ArrowRight);
        assert_eq!(pointer(&app), Some(0));

        press(&mut app, KeyCode::ArrowLeft);
        assert_eq!(pointer(&app), Some(6));
    }

    #[test]
    fn the_pointer_hangs_a_tag_off_its_card() {
        let mut app = table_of(None);
        assert_eq!(tag(&mut app), None);

        press(&mut app, KeyCode::ArrowRight);
        press(&mut app, KeyCode::ArrowRight);

        let (tag, slot) = tag(&mut app).expect("a tag");
        assert_eq!(slot, 1);
        assert_eq!(
            tag,
            Tag {
                card: held(1),
                term: None,
                burn: None
            }
        );
    }

    #[test]
    fn the_peek_reads_a_face_up_opposing_card_and_not_a_face_down_one() {
        let mut app = table(40, 30, 20);
        // `lay_out` puts everything across the table face up.
        hover_at(&mut app, Zone::Opposing, 0);
        let (showing, _) = tag(&mut app).expect("a tag on the enemy's card");
        assert_eq!(showing.card.zone, Zone::Opposing);

        // Turn that card face down and the Peek has nothing to say.
        app.world_mut()
            .resource_mut::<ActiveDuel>()
            .duel
            .hide_opposing(0);
        app.update();
        hover_at(&mut app, Zone::Opposing, 0);
        assert_eq!(tag(&mut app), None);
    }

    #[test]
    fn the_peek_follows_a_card_into_your_own_row() {
        let mut app = table_of(Some(Tell::Streak));
        press(&mut app, KeyCode::Digit1);

        hover_at(&mut app, Zone::Row, 0);

        let (showing, _) = tag(&mut app).expect("a tag on the card you placed");
        assert_eq!(showing.card.zone, Zone::Row);
        assert_eq!(showing.burn, None, "a card in the row is not a burn");
    }

    #[test]
    fn the_mouse_over_a_card_wins_over_the_pointer() {
        let mut app = table_of(None);
        press(&mut app, KeyCode::ArrowRight);

        hover(&mut app, 4);

        assert_eq!(tag(&mut app).map(|(_, slot)| slot), Some(4));
    }

    /// The cursor off the card again, onto the tag or the felt.
    fn unhover(app: &mut App, slot: usize) {
        let entity = node_at(app, Zone::Draw, slot);
        *app.world_mut().get_mut::<Interaction>(entity).unwrap() = Interaction::None;
        app.update();
    }

    #[test]
    fn the_tag_stays_on_the_mouses_card_after_the_cursor_leaves_it() {
        // Crossing from the card up to its tag passes over nothing; the tag
        // must not snap back to the keyboard's card on the way.
        let mut app = table_of(None);
        press(&mut app, KeyCode::ArrowRight);
        hover(&mut app, 4);
        unhover(&mut app, 4);

        assert_eq!(tag(&mut app).map(|(_, slot)| slot), Some(4));

        // The keyboard moving takes the tag back.
        press(&mut app, KeyCode::ArrowRight);
        assert_eq!(tag(&mut app).map(|(_, slot)| slot), Some(1));
    }

    #[test]
    fn a_play_lands_the_pointer_on_the_nearest_slot() {
        let mut app = table_of(None);
        press(&mut app, KeyCode::ArrowLeft); // the last card
        assert_eq!(pointer(&app), Some(6));

        press(&mut app, KeyCode::Digit1);

        assert_eq!(app.world().resource::<ActiveDuel>().duel.draw().len(), 6);
        assert_eq!(pointer(&app), Some(5));
        assert_eq!(tag(&mut app).map(|(_, slot)| slot), Some(5));
    }

    #[test]
    fn t_waves_the_peek_off_and_calls_it_back() {
        let mut app = table_of(None);
        press(&mut app, KeyCode::ArrowRight);
        assert!(tag(&mut app).is_some());

        press(&mut app, KeyCode::KeyT);
        assert!(!app.world().resource::<Peek>().on);
        assert_eq!(tag(&mut app), None);

        press(&mut app, KeyCode::KeyT);
        assert!(tag(&mut app).is_some());
    }

    #[test]
    fn esc_lets_go_of_the_pointer() {
        let mut app = table_of(None);
        press(&mut app, KeyCode::ArrowRight);

        press(&mut app, KeyCode::Escape);

        assert_eq!(pointer(&app), None);
        assert_eq!(tag(&mut app), None);
    }

    #[test]
    fn up_opens_the_tells_terms_in_order_and_down_closes_them() {
        let mut app = table_of(Some(Tell::Streak));
        press(&mut app, KeyCode::ArrowRight);
        assert_eq!(tag(&mut app).unwrap().0.term, None);

        press(&mut app, KeyCode::ArrowUp);
        assert_eq!(tag(&mut app).unwrap().0.term, Some("Stack"));
        press(&mut app, KeyCode::ArrowUp);
        assert_eq!(tag(&mut app).unwrap().0.term, Some("Tell"));
        press(&mut app, KeyCode::ArrowUp);
        assert_eq!(tag(&mut app).unwrap().0.term, Some("Stack"), "wraps");

        press(&mut app, KeyCode::ArrowDown);
        assert_eq!(tag(&mut app).unwrap().0.term, None);
    }

    #[test]
    fn a_waiting_all_in_says_what_the_burn_adds() {
        let mut app = table_of(Some(Tell::AllIn));
        press(&mut app, KeyCode::Digit1);
        assert_eq!(
            app.world().resource::<ActiveDuel>().awaiting_sacrifice,
            Some(0)
        );

        hover(&mut app, 3);
        assert_eq!(tag(&mut app).unwrap().0.burn, Some(4));

        // The All In itself is not a burn.
        hover(&mut app, 0);
        assert_eq!(tag(&mut app).unwrap().0.burn, None);
    }

    #[test]
    fn esc_out_of_a_sacrifice_keeps_the_pointer() {
        let mut app = table_of(Some(Tell::AllIn));
        press(&mut app, KeyCode::ArrowRight);
        press(&mut app, KeyCode::Digit1);

        press(&mut app, KeyCode::Escape);

        assert_eq!(
            app.world().resource::<ActiveDuel>().awaiting_sacrifice,
            None
        );
        assert_eq!(pointer(&app), Some(0));
    }

    #[test]
    fn the_glossary_hides_the_tag_and_takes_the_keys() {
        let mut app = table_of(None);
        press(&mut app, KeyCode::ArrowRight);

        press(&mut app, KeyCode::KeyI);
        assert_eq!(tag(&mut app), None);
        press(&mut app, KeyCode::ArrowRight);
        assert_eq!(
            pointer(&app),
            Some(0),
            "the pointer did not move under the glossary"
        );

        press(&mut app, KeyCode::KeyI);
        assert!(tag(&mut app).is_some());
    }
}

#[cfg(test)]
mod row_feedback_tests {
    use bevy::prelude::*;

    use super::ActiveDuel;
    use super::tests::{press, table_for_run};
    use crate::run::{Card, RunState, Tell};

    const KEYS: [KeyCode; 7] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
    ];

    /// A table dealt from Copycats and sixes, half and half.
    fn table() -> App {
        let copycat = Card {
            name: "copycat",
            stack: 3,
            tell: Some(Tell::Copycat),
        };
        let six = Card {
            name: "six",
            stack: 6,
            tell: None,
        };
        let mut deck = vec![copycat; 9];
        deck.extend(vec![six; 9]);
        table_for_run(
            RunState {
                deck,
                ..RunState::new()
            },
            30,
            20,
        )
    }

    /// The key for the first card in the Draw with, or without, a Tell.
    fn key_of(app: &App, tell: Option<Tell>) -> KeyCode {
        let slot = app
            .world()
            .resource::<ActiveDuel>()
            .duel
            .draw()
            .iter()
            .position(|c| c.tell == tell)
            .expect("the Draw holds both kinds");
        KEYS[slot]
    }

    fn notice(app: &App) -> String {
        app.world()
            .resource::<ActiveDuel>()
            .notice
            .clone()
            .unwrap_or_default()
    }

    /// What the row reads, which is the only number the table can honestly
    /// give a card as it lands: nothing resolves until the rows turn over,
    /// and the card to the right of a Copycat hasn't been chosen yet.
    #[test]
    fn the_table_says_where_the_card_landed_and_what_the_row_now_reads() {
        let mut app = table();

        let key = key_of(&app, Some(Tell::Copycat));
        press(&mut app, key);

        // A Copycat alone at the end of the row is worth its own printed 3.
        assert_eq!(app.world().resource::<ActiveDuel>().duel.hand(), 3);
        assert_eq!(
            notice(&app),
            "Slot 1. Your row reads 3.  4 more cards to cover the row."
        );

        let key = key_of(&app, None);
        press(&mut app, key);

        // Now a six follows it, so the Copycat takes the six's print too.
        assert_eq!(app.world().resource::<ActiveDuel>().duel.hand(), 12);
        assert_eq!(
            notice(&app),
            "Slot 2. Your row reads 12.  3 more cards to cover the row."
        );
    }

    #[test]
    fn the_table_says_what_came_back_when_a_card_is_taken_out_of_the_row() {
        let mut app = table();
        let key = key_of(&app, None);
        press(&mut app, key);
        assert_eq!(app.world().resource::<ActiveDuel>().duel.hand(), 6);

        press(&mut app, KeyCode::Backspace);

        assert_eq!(app.world().resource::<ActiveDuel>().duel.hand(), 0);
        assert_eq!(notice(&app), "six back in the Draw. Your row reads 0.");
    }

    #[test]
    fn the_last_slot_says_the_row_is_ready_to_confirm() {
        let mut app = table();
        // Neither kind in this deck wants a sacrifice, so slot 1 of the Draw
        // five times over fills the row whatever the shuffle dealt.
        for _ in 0..5 {
            press(&mut app, KeyCode::Digit1);
        }

        let line = notice(&app);
        assert!(line.starts_with("Slot 5."), "{line}");
        assert!(line.ends_with("Enter to confirm."), "{line}");
        assert_eq!(app.world().resource::<ActiveDuel>().duel.plays_left(), 0);
    }
}
