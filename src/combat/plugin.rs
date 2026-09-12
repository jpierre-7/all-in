//! The Bevy side of combat. Honours the seam in ADR-0001: consumes the
//! `Encounter`, drives a `Duel`, writes `RunState.stack` back, inserts
//! `CombatOutcome`, and makes the one transition combat may make.

use bevy::prelude::*;

use super::duel::{Coin, Duel, Phase, PlayError, TurnResult};
use super::ui;
use crate::run::{Card, Encounter, RunState};
use crate::state::AppState;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, ui::load_art)
            .add_systems(OnEnter(AppState::Combat), start_duel)
            .add_systems(
                Update,
                (take_input, ui::redraw.run_if(resource_exists_and_changed::<ActiveDuel>))
                    .chain()
                    .run_if(in_state(AppState::Combat)),
            );
    }
}

/// The duel in progress plus what the screen needs to say about it.
#[derive(Resource)]
pub struct ActiveDuel {
    pub duel: Duel,
    pub enemy_name: &'static str,
    /// Set after playing an All In: the next digit names the sacrifice.
    pub awaiting_sacrifice: Option<usize>,
    /// What the last turn did, for the feedback line.
    pub last_turn: Option<TurnResult>,
    /// What the last keypress did, for the feedback line.
    pub notice: Option<String>,
}

fn start_duel(mut commands: Commands, encounter: Res<Encounter>, run: Res<RunState>, time: Res<Time>) {
    let deck = if run.deck.is_empty() {
        // The overworld still builds RunState from its own placeholder; once
        // it calls `RunState::new` this branch is dead.
        crate::run::starter_deck()
    } else {
        run.deck.clone()
    };
    let seed = time.elapsed_secs_f64().to_bits() | 1;
    let duel = Duel::new(shuffled(deck, seed), run.stack, run.plays(), encounter.enemy.clone())
        .with_seed(seed.rotate_left(17))
        .with_coin(Coin::for_perks(&run.perks));

    commands.insert_resource(ActiveDuel {
        duel,
        enemy_name: encounter.enemy.name,
        awaiting_sacrifice: None,
        last_turn: None,
        notice: None,
    });
}

/// Fisher-Yates on a xorshift, same as the duel's own reshuffle.
fn shuffled(mut deck: Vec<Card>, mut rng: u64) -> Vec<Card> {
    for i in (1..deck.len()).rev() {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        deck.swap(i, (rng % (i as u64 + 1)) as usize);
    }
    deck
}

fn take_input(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    active: Option<ResMut<ActiveDuel>>,
    mut run: ResMut<RunState>,
    mut next: ResMut<NextState<AppState>>,
) {
    let Some(mut active) = active else { return };

    if let Some(digit) = card_key(&keys) {
        let index = usize::from(digit - 1);
        let result = match active.awaiting_sacrifice.take() {
            Some(all_in) => active.duel.play(all_in, Some(index)),
            None => match active.duel.play(index, None) {
                Err(PlayError::AllInNeedsSacrifice) => {
                    active.awaiting_sacrifice = Some(index);
                    active.notice = Some("All In. Which card do you burn?".into());
                    return;
                }
                other => other,
            },
        };
        active.notice = Some(match result {
            Ok(value) => format!("+{value} to The Hand."),
            Err(PlayError::NoPlaysLeft) => "No Plays left. Enter to show your Hand.".into(),
            Err(PlayError::NoSuchCard) => "No card there.".into(),
            Err(PlayError::AllInNeedsSacrifice) => "All In needs a sacrifice.".into(),
            Err(PlayError::NotAllIn) => "Only All In burns a card.".into(),
            Err(PlayError::HandIsFinal) => "The Hand is final. Push (P) or Hold (H).".into(),
        });
        return;
    }

    // Step 5: the prompt is up and only Push or Hold answers it.
    if active.duel.phase() == Phase::PushYourLuck {
        let pushed = keys.just_pressed(KeyCode::KeyP);
        let held = keys.just_pressed(KeyCode::KeyH);
        if !pushed && !held {
            return;
        }
        let result = if pushed { active.duel.push() } else { active.duel.hold() };
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
        // A clearing Hand puts the Push Your Luck prompt up instead of
        // resolving; a Whiff resolves here.
        let Some(result) = active.duel.show_hand() else {
            active.notice = None;
            return;
        };
        active.last_turn = Some(result);
        active.notice = None;
        finish_if_over(&mut commands, &mut active, &mut run, &mut next);
    }
}

/// Write the Stack back and hand over at `PostCombat` once one Stack is out.
/// The only transition combat ever makes (ADR-0001).
fn finish_if_over(
    commands: &mut Commands,
    active: &mut ActiveDuel,
    run: &mut RunState,
    next: &mut NextState<AppState>,
) {
    let Some(outcome) = active.duel.outcome() else { return };
    run.stack = active.duel.player_stack();
    commands.remove_resource::<ActiveDuel>();
    commands.remove_resource::<Encounter>();
    commands.insert_resource(outcome);
    next.set(AppState::PostCombat);
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
    use super::{ActiveDuel, CombatPlugin};
    use crate::run::{CombatOutcome, Encounter, EncounterId, Enemy, Perk, RisingBlinds, RunState};
    use crate::state::AppState;

    /// Combat on its own: no window, no overworld. The test plays the
    /// overworld's part by inserting the Encounter and entering the state.
    pub(super) fn table(player_stack: u32, enemy_stack: u32, house_edge: u32) -> App {
        table_with(player_stack, enemy_stack, house_edge, Vec::new())
    }

    pub(super) fn table_with(player_stack: u32, enemy_stack: u32, house_edge: u32, perks: Vec<Perk>) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(StatesPlugin)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_state::<AppState>()
            .insert_resource(RunState { stack: player_stack, deck: Vec::new(), perks, items: Vec::new() })
            .insert_resource(Encounter {
                id: EncounterId::FloorMinion,
                enemy: Enemy { name: "shill", stack: enemy_stack, house_edge, blinds: RisingBlinds { every_turns: 2, increase: 2 } },
            })
            .add_plugins(CombatPlugin);
        app.update();
        app.world_mut().resource_mut::<NextState<AppState>>().set(AppState::Combat);
        app.update();
        app
    }

    pub(super) fn press(app: &mut App, key: KeyCode) {
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(key);
        app.update();
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset_all();
        app.update();
    }

    pub(super) fn state(app: &App) -> AppState {
        *app.world().resource::<State<AppState>>().get()
    }

    pub(super) fn rig(app: &mut App, coin: Coin) {
        app.world_mut().resource_mut::<ActiveDuel>().duel.set_coin(coin);
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

        assert_eq!(state(&app), AppState::PostCombat);
        assert_eq!(*app.world().resource::<CombatOutcome>(), CombatOutcome::Lost);
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
        if app.world().resource::<ActiveDuel>().awaiting_sacrifice.is_some() {
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

    use super::tests::{press, rig, state, table, table_with};
    use super::ActiveDuel;
    use crate::combat::duel::{Coin, Outcome, Phase, Push};
    use crate::run::{CombatOutcome, Perk, RunState};
    use crate::state::AppState;

    /// Play the card in slot 1, burning slot 2 if it turns out to be All In.
    fn play_one(app: &mut App) {
        press(app, KeyCode::Digit1);
        if app.world().resource::<ActiveDuel>().awaiting_sacrifice.is_some() {
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
    fn a_whiff_needs_no_prompt_and_resolves_on_enter() {
        let mut app = table(40, 999, 30);

        press(&mut app, KeyCode::Enter);

        let active = app.world().resource::<ActiveDuel>();
        assert_eq!(active.duel.phase(), Phase::Playing);
        assert_eq!(active.last_turn.unwrap().kind, Outcome::Whiff(30));
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
        assert_eq!(*app.world().resource::<CombatOutcome>(), CombatOutcome::Lost);
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
        assert!(active.notice.is_some(), "the screen says why nothing happened");
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
        assert_eq!(plain.world().resource::<ActiveDuel>().duel.coin(), Coin::BASE);

        let slotz = table_with(40, 999, 20, vec![Perk::PylBestTwoOfThree]);
        assert_eq!(slotz.world().resource::<ActiveDuel>().duel.coin(), Coin::SLOTZ);
    }

    fn payout(kind: Outcome) -> u32 {
        match kind {
            Outcome::Payout(n) => n,
            Outcome::Whiff(n) => panic!("expected a Payout, got a Whiff of {n}"),
        }
    }
}
