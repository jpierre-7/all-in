//! The Wheel's two screens (#167): arranging Golden Chips off the Lobby, and
//! the spin that starts a run, with the waiting Legacy Perk (#168) and the
//! choices Trim and Pocket Change put.
//! What each square does is `crate::wheel`'s; this only shows it and takes
//! the keys.

use bevy::prelude::*;

use super::narrative;
use super::progression::Progress;
use super::screens::{Screen, any_key, confirm, digit_pressed};
use crate::combat::plugin::{DuelSeed, roll_seed};
use crate::item::Item;
use crate::legacy::{self, LegacyPerk};
use crate::run::{Reward, RunState};
use crate::save::SaveSlot;
use crate::state::AppState;
use crate::wheel::{self, MAX_RANK, Refused, Spun, Square};

pub(super) fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(AppState::Wheel), open_wheel)
        .add_systems(OnEnter(AppState::Spin), spin_the_wheel)
        .add_systems(
            Update,
            (
                arrange.run_if(in_state(AppState::Wheel)),
                start_the_run.run_if(in_state(AppState::Spin)),
            ),
        );
}

/// Whichever screen of this module is on the felt, so a key that changes it
/// can take it down and put the new one up.
#[derive(Component)]
struct WheelScreen;

fn redraw(commands: &mut Commands, shown: &Query<Entity, With<WheelScreen>>) {
    for screen in shown {
        commands.entity(screen).despawn();
    }
}

fn up(keys: &ButtonInput<KeyCode>) -> bool {
    keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW)
}

fn down(keys: &ButtonInput<KeyCode>) -> bool {
    keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS)
}

/// A square's rank as pips: one filled per Golden Chip.
fn pips(rank: u8) -> String {
    (0..MAX_RANK)
        .map(|i| if i < rank { 'o' } else { '.' })
        .collect()
}

// ---------------------------------------------------------------------------
// The Wheel, between runs
// ---------------------------------------------------------------------------

/// The square the cursor is on, and what the last key did.
#[derive(Resource, Default)]
struct Arranging {
    at: usize,
    notice: Option<&'static str>,
}

fn open_wheel(mut commands: Commands, slot: Res<SaveSlot>) {
    let arranging = Arranging::default();
    spawn_wheel(&mut commands, &slot, &arranging);
    commands.insert_resource(arranging);
}

fn spawn_wheel(commands: &mut Commands, slot: &SaveSlot, arranging: &Arranging) {
    let board: Vec<String> = Square::ALL
        .iter()
        .enumerate()
        .map(|(i, &square)| {
            let cursor = if i == arranging.at { ">" } else { " " };
            let rank = slot.wheel.get(&square).copied().unwrap_or(0);
            let does = if !square.ready() {
                "coming soon".to_string()
            } else if rank == 0 {
                format!("at rank 1: {}", square.describe(1))
            } else {
                square.describe(rank)
            };
            format!("{cursor} {:<14} {}  {does}", square.name(), pips(rank))
        })
        .collect();
    let mut screen = Screen::new()
        .title("The Wheel")
        .prose(narrative::WHEEL)
        .prose(board.join("\n"))
        .prose(format!("Golden Chips in hand: {}", slot.golden_chips));
    if let Some(notice) = arranging.notice {
        screen = screen.note(notice);
    }
    let root = screen
        .footer("Up / Down picks a square. Right puts a Golden Chip on it, Left takes one off. Enter or Esc: back to the Lobby.")
        .spawn(commands, AppState::Wheel);
    commands.entity(root).insert(WheelScreen);
}

fn arrange(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut slot: ResMut<SaveSlot>,
    mut arranging: ResMut<Arranging>,
    shown: Query<Entity, With<WheelScreen>>,
    mut next: ResMut<NextState<AppState>>,
) {
    if confirm(&keys) || keys.just_pressed(KeyCode::Escape) {
        commands.remove_resource::<Arranging>();
        next.set(AppState::Lobby);
        return;
    }
    let squares = Square::ALL.len();
    let square = Square::ALL[arranging.at];
    let moved = if up(&keys) {
        arranging.at = (arranging.at + squares - 1) % squares;
        None
    } else if down(&keys) {
        arranging.at = (arranging.at + 1) % squares;
        None
    } else if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD) {
        Some(wheel::place(&mut slot, square))
    } else if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA) {
        Some(wheel::take_off(&mut slot, square))
    } else {
        return;
    };
    arranging.notice = match moved {
        Some(Err(Refused::ComingSoon)) => Some("That square doesn't do anything yet."),
        Some(Err(Refused::NoGoldenChips)) => {
            Some("No Golden Chips in hand. Take one off another square.")
        }
        Some(Err(Refused::Full)) => {
            Some("Three is as high as a square goes. Only the spin takes it higher.")
        }
        Some(Err(Refused::Empty)) => Some("Nothing on that square."),
        Some(Err(Refused::Closed)) => Some("The Wheel opens once The House is beaten."),
        Some(Ok(())) | None => None,
    };
    redraw(&mut commands, &shown);
    spawn_wheel(&mut commands, &slot, &arranging);
}

// ---------------------------------------------------------------------------
// The spin, at the start of a run
// ---------------------------------------------------------------------------

/// Where the start of the run is: the spin on show, then the choices.
#[derive(Resource)]
enum Starting {
    Spun,
    /// Trim: the starting card under the cursor and the ones marked to go.
    Trim {
        at: usize,
        marked: Vec<usize>,
    },
    /// Pocket Change's pick of three.
    Pick(Vec<&'static Item>),
}

/// The board locks and The Wheel spins: the run starts under it and under
/// the Legacy Perk waiting in the slot, which this run takes and spends. The
/// choices still to put are kept for after the spin is shown.
fn spin_the_wheel(
    mut commands: Commands,
    mut slot: ResMut<SaveSlot>,
    time: Res<Time>,
    pinned: Option<ResMut<DuelSeed>>,
    mut run: ResMut<RunState>,
) {
    let seed = roll_seed(pinned, &time);
    let spun = Spun::new(&slot.wheel, wheel::spin(seed));
    let perk = slot.legacy_perk.take();
    let mut fresh = RunState::new();
    if let Some(perk) = &perk {
        legacy::apply(&mut fresh, perk);
    }
    *run = wheel::start_run(fresh, spun, seed.rotate_left(23));
    spawn_spin(&mut commands, &run.wheel, perk.as_ref());
    commands.insert_resource(Starting::Spun);
}

fn spawn_spin(commands: &mut Commands, spun: &Spun, perk: Option<&LegacyPerk>) {
    let mut screen = Screen::new().title("The Wheel");
    if let Some(perk) = perk {
        screen = screen.prose(format!("Your Legacy Perk tonight. {}", perk.describe()));
    }
    screen = screen.prose(narrative::SPIN);
    screen = match spun.hot() {
        Some(hot) => screen.prose(format!(
            "It drops on {}. That square is hot tonight.",
            hot.name()
        )),
        None => screen.prose(narrative::SPIN_ZERO),
    };
    let working = spun.working();
    if working.is_empty() {
        screen = screen.prose("Nothing on the board.");
    }
    for (square, rank) in working {
        let hot = if spun.hot() == Some(square) {
            "  (hot)"
        } else {
            ""
        };
        screen = screen.prose(format!(
            "{}, rank {rank}{hot}: {}",
            square.name(),
            square.describe(rank)
        ));
    }
    let root = screen
        .footer(narrative::ANY_KEY)
        .spawn(commands, AppState::Spin);
    commands.entity(root).insert(WheelScreen);
}

fn spawn_trim(commands: &mut Commands, run: &RunState, at: usize, marked: &[usize]) {
    let deck: Vec<String> = run
        .deck
        .iter()
        .enumerate()
        .map(|(i, card)| {
            let cursor = if i == at { ">" } else { " " };
            let mark = if marked.contains(&i) { "OUT " } else { "    " };
            format!("{cursor} {mark}{}", wheel::card_line(card))
        })
        .collect();
    let root = Screen::new()
        .title("Trim")
        .prose(narrative::TRIM)
        .prose(deck.join("\n"))
        .footer(format!(
            "Up / Down picks a card, Space marks it to go. Enter takes them out. ({} of up to {})",
            marked.len(),
            run.wheel.trims()
        ))
        .spawn(commands, AppState::Spin);
    commands.entity(root).insert(WheelScreen);
}

fn spawn_pick(commands: &mut Commands, items: &[&'static Item]) {
    let mut screen = Screen::new()
        .title("Pocket Change")
        .prose(narrative::POCKET_CHANGE);
    for (i, &item) in items.iter().enumerate() {
        screen = screen.option(i as u8 + 1, &Reward::Item(item).label());
    }
    let root = screen
        .footer(format!("Press 1 to {}.", items.len()))
        .spawn(commands, AppState::Spin);
    commands.entity(root).insert(WheelScreen);
}

/// Put the next choice up, or walk onto the Floor once there are none left.
fn next_step(
    commands: &mut Commands,
    after: &Starting,
    run: &RunState,
    seed: u64,
    progress: &Progress,
    next: &mut NextState<AppState>,
) {
    if matches!(after, Starting::Spun) && run.wheel.trims() > 0 {
        spawn_trim(commands, run, 0, &[]);
        commands.insert_resource(Starting::Trim {
            at: 0,
            marked: Vec::new(),
        });
        return;
    }
    if !matches!(after, Starting::Pick(_)) {
        let items = run.wheel.pocket_pick(seed);
        if !items.is_empty() {
            spawn_pick(commands, &items);
            commands.insert_resource(Starting::Pick(items));
            return;
        }
    }
    commands.remove_resource::<Starting>();
    next.set(progress.arrival());
}

#[allow(clippy::too_many_arguments)]
fn start_the_run(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    starting: Option<ResMut<Starting>>,
    shown: Query<Entity, With<WheelScreen>>,
    mut run: ResMut<RunState>,
    progress: Res<Progress>,
    mut next: ResMut<NextState<AppState>>,
) {
    let Some(mut starting) = starting else { return };
    let seed = time.elapsed_secs_f64().to_bits() | 1;
    match &mut *starting {
        Starting::Spun => {
            if !any_key(&keys) {
                return;
            }
        }
        Starting::Trim { at, marked } => {
            let deck = run.deck.len();
            if up(&keys) {
                *at = (*at + deck - 1) % deck;
            } else if down(&keys) {
                *at = (*at + 1) % deck;
            } else if keys.just_pressed(KeyCode::Space) {
                if let Some(i) = marked.iter().position(|m| m == at) {
                    marked.remove(i);
                } else if marked.len() < run.wheel.trims() {
                    marked.push(*at);
                }
            } else if confirm(&keys) {
                let marked = std::mem::take(marked);
                wheel::trim(&mut run, &marked);
            } else {
                return;
            }
            if let Starting::Trim { at, marked } = &*starting
                && !confirm(&keys)
            {
                redraw(&mut commands, &shown);
                spawn_trim(&mut commands, &run, *at, marked);
                return;
            }
        }
        Starting::Pick(items) => {
            let Some(&item) = digit_pressed(&keys).and_then(|n| items.get(usize::from(n) - 1))
            else {
                return;
            };
            run.apply(Reward::Item(item), seed);
        }
    }
    redraw(&mut commands, &shown);
    next_step(&mut commands, &starting, &run, seed, &progress, &mut next);
}
