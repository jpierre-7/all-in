//! The pure combat model: one duel, no Bevy. Vocabulary follows `CONTEXT.md`.

use crate::run::{Card, CombatOutcome, Enemy, Perk, Tell};

pub const DRAW_SIZE: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayError {
    NoPlaysLeft,
    NoSuchCard,
    /// An All In card was played without naming a card to sacrifice.
    AllInNeedsSacrifice,
    /// A sacrifice was named for a card that isn't All In.
    NotAllIn,
    /// The Hand has been shown and the Push Your Luck prompt is up.
    HandIsFinal,
}

/// Where the turn is. Playing covers steps 1-4; the Hand is only final once
/// it has been shown and cleared House Edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Playing,
    PushYourLuck,
}

/// How a Push went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Push {
    Won,
    Lost,
}

/// The Push Your Luck coin. House-favoured by default, so Slotz Option 1 is
/// worth taking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coin {
    /// The player's chance of taking one flip, in percent.
    pub player_pct: u32,
    /// How many flips are tossed, best-of. The majority takes it.
    pub best_of: u8,
}

impl Coin {
    /// 45% player / 55% House, one flip.
    pub const BASE: Coin = Coin { player_pct: 45, best_of: 1 };
    /// Slotz Option 1: best 2-of-3 at 49/51, which is 3p2 - 2p3 =~ 48.5%.
    pub const SLOTZ: Coin = Coin { player_pct: 49, best_of: 3 };

    pub fn for_perks(perks: &[Perk]) -> Coin {
        if perks.contains(&Perk::PylBestTwoOfThree) { Coin::SLOTZ } else { Coin::BASE }
    }

    /// `rolls` yields flips; the player takes one when `roll % 100` is under
    /// `player_pct`. Stops as soon as one side has the majority, so a decided
    /// best-of-three never tosses its third coin.
    pub fn resolve(&self, rolls: impl Iterator<Item = u32>) -> Push {
        let best_of = u32::from(self.best_of.max(1));
        let needed = best_of / 2 + 1;
        let (mut won, mut lost) = (0, 0);
        for roll in rolls.take(best_of as usize) {
            if roll % 100 < self.player_pct {
                won += 1;
            } else {
                lost += 1;
            }
            if won >= needed || lost >= needed {
                break;
            }
        }
        if won >= needed { Push::Won } else { Push::Lost }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The Hand cleared House Edge by this much; dealt to the enemy's Stack.
    Payout(u32),
    /// The Hand fell short by this much; dealt to the player's Stack.
    Whiff(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnResult {
    pub hand: u32,
    /// The House Edge this Hand was compared against.
    pub house_edge: u32,
    pub kind: Outcome,
    /// The flip, if the player Pushed. `None` means Hold, or no prompt.
    pub pyl: Option<Push>,
    /// Whether Rising Blinds ticked at the end of this turn.
    pub blinds_rose: bool,
}

pub struct Duel {
    deck: Vec<Card>,
    draw: Vec<Card>,
    discard: Vec<Card>,
    /// The Hand so far this turn.
    hand: u32,
    plays: u8,
    plays_left: u8,
    /// Whether the last card played this turn carried a Tell (for Streak).
    last_had_tell: bool,
    player_stack: u32,
    enemy: Enemy,
    house_edge: u32,
    turn: u32,
    phase: Phase,
    coin: Coin,
    rng: u64,
}

impl Duel {
    /// `deck` is taken in draw order (last element drawn first); shuffling is
    /// the caller's job so the model stays deterministic.
    pub fn new(deck: Vec<Card>, player_stack: u32, plays: u8, enemy: Enemy) -> Self {
        let mut duel = Duel {
            deck,
            draw: Vec::new(),
            discard: Vec::new(),
            hand: 0,
            plays,
            plays_left: plays,
            last_had_tell: false,
            player_stack,
            house_edge: enemy.house_edge,
            enemy,
            turn: 1,
            phase: Phase::Playing,
            coin: Coin::BASE,
            rng: 0x5eed_cafe_f00d_d1ce,
        };
        duel.refill();
        duel
    }

    /// Seeds the reshuffle of the discard pile. The initial deck order is
    /// still the caller's.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng = seed | 1;
        self
    }

    /// The coin Push Your Luck is flipped with; Slotz Option 1 swaps it.
    pub fn with_coin(mut self, coin: Coin) -> Self {
        self.coin = coin;
        self
    }

    pub fn coin(&self) -> Coin {
        self.coin
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// The Payout The Hand would deal right now: its excess over House Edge,
    /// doubled by a won Push. 0 on a Whiff.
    pub fn payout(&self, pyl: Option<Push>) -> u32 {
        self.hand.saturating_sub(self.house_edge) * if pyl == Some(Push::Won) { 2 } else { 1 }
    }

    pub fn house_edge(&self) -> u32 {
        self.house_edge
    }

    pub fn player_stack(&self) -> u32 {
        self.player_stack
    }

    pub fn enemy_stack(&self) -> u32 {
        self.enemy.stack
    }

    pub fn outcome(&self) -> Option<CombatOutcome> {
        if self.enemy.stack == 0 {
            Some(CombatOutcome::Won)
        } else if self.player_stack == 0 {
            Some(CombatOutcome::Lost)
        } else {
            None
        }
    }

    /// Step 5: The Hand is final. A Hand that clears House Edge puts the
    /// Push Your Luck prompt up and resolves nothing yet (`None`); a Whiff is
    /// never offered the flip and resolves on the spot.
    pub fn show_hand(&mut self) -> Option<TurnResult> {
        if self.phase == Phase::PushYourLuck {
            return None;
        }
        if self.hand >= self.house_edge {
            self.phase = Phase::PushYourLuck;
            return None;
        }
        Some(self.resolve(None))
    }

    /// Answer the prompt with Hold: the turn resolves as normal. `None` when
    /// no prompt is up.
    pub fn hold(&mut self) -> Option<TurnResult> {
        (self.phase == Phase::PushYourLuck).then(|| self.resolve(None))
    }

    /// Answer the prompt with Push: flip the coin. Win and the Payout
    /// doubles; lose and The Hand becomes 0, a full Whiff for the whole House
    /// Edge. `None` when no prompt is up.
    pub fn push(&mut self) -> Option<TurnResult> {
        if self.phase != Phase::PushYourLuck {
            return None;
        }
        let coin = self.coin;
        let flip = coin.resolve(std::iter::repeat_with(|| (self.next_rng() % 100) as u32));
        if flip == Push::Lost {
            self.hand = 0;
        }
        Some(self.resolve(Some(flip)))
    }

    /// Step 6 onwards: compare The Hand to House Edge, deal the Payout or
    /// Whiff, tick Rising Blinds, and refill the Draw for the next turn. The
    /// enemy does nothing on its turn.
    fn resolve(&mut self, pyl: Option<Push>) -> TurnResult {
        let hand = self.hand;
        let house_edge = self.house_edge;
        let kind = if hand >= house_edge {
            let payout = self.payout(pyl);
            self.enemy.stack = self.enemy.stack.saturating_sub(payout);
            Outcome::Payout(payout)
        } else {
            let whiff = house_edge - hand;
            self.player_stack = self.player_stack.saturating_sub(whiff);
            Outcome::Whiff(whiff)
        };

        let every = u32::from(self.enemy.blinds.every_turns.max(1));
        let blinds_rose = self.turn.is_multiple_of(every);
        if blinds_rose {
            self.house_edge += self.enemy.blinds.increase;
        }

        self.turn += 1;
        self.hand = 0;
        self.plays_left = self.plays;
        self.last_had_tell = false;
        self.phase = Phase::Playing;
        self.refill();

        TurnResult { hand, house_edge, kind, pyl, blinds_rose }
    }

    pub fn draw(&self) -> &[Card] {
        &self.draw
    }

    pub fn hand(&self) -> u32 {
        self.hand
    }

    pub fn plays_left(&self) -> u8 {
        self.plays_left
    }

    /// Play the card at `card` in the Draw. It resolves immediately into The
    /// Hand and the contribution is returned. All In must name a `sacrifice`
    /// index (also into the Draw); no other card may.
    pub fn play(&mut self, card: usize, sacrifice: Option<usize>) -> Result<u32, PlayError> {
        if self.phase == Phase::PushYourLuck {
            return Err(PlayError::HandIsFinal);
        }
        if self.plays_left == 0 {
            return Err(PlayError::NoPlaysLeft);
        }
        let played = self.draw.get(card).ok_or(PlayError::NoSuchCard)?.clone();
        let sacrificed = match (played.tell, sacrifice) {
            (Some(Tell::AllIn), None) => return Err(PlayError::AllInNeedsSacrifice),
            (Some(Tell::AllIn), Some(i)) if i == card => return Err(PlayError::NoSuchCard),
            (Some(Tell::AllIn), Some(i)) => Some(self.draw.get(i).ok_or(PlayError::NoSuchCard)?.clone()),
            (_, Some(_)) => return Err(PlayError::NotAllIn),
            (_, None) => None,
        };

        let mut value = played.stack;
        match played.tell {
            Some(Tell::Streak) if self.last_had_tell => value *= 2,
            Some(Tell::AllIn) => value += sacrificed.as_ref().map_or(0, |c| c.stack),
            _ => {}
        }

        // Remove the higher index first so the lower one stays valid.
        let mut gone: Vec<usize> = sacrifice.into_iter().chain([card]).collect();
        gone.sort_unstable_by(|a, b| b.cmp(a));
        for i in gone {
            let c = self.draw.remove(i);
            self.discard.push(c);
        }

        self.hand += value;
        self.plays_left -= 1;
        self.last_had_tell = played.tell.is_some();
        Ok(value)
    }

    fn refill(&mut self) {
        while self.draw.len() < DRAW_SIZE {
            if self.deck.is_empty() {
                if self.discard.is_empty() {
                    break;
                }
                self.deck = std::mem::take(&mut self.discard);
                self.shuffle_deck();
            }
            let card = self.deck.pop().expect("deck was just checked non-empty");
            self.draw.push(card);
        }
    }

    /// Fisher-Yates over the duel's xorshift64; enough randomness for a card
    /// game, and the same stream the coin is flipped from.
    fn shuffle_deck(&mut self) {
        for i in (1..self.deck.len()).rev() {
            let j = (self.next_rng() % (i as u64 + 1)) as usize;
            self.deck.swap(i, j);
        }
    }

    fn next_rng(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }
}

/// Coins with no suspense in them, so a test can say what a Push does
/// without saying what the odds are.
#[cfg(test)]
impl Coin {
    pub const SURE_THING: Coin = Coin { player_pct: 100, best_of: 1 };
    pub const RIGGED: Coin = Coin { player_pct: 0, best_of: 1 };
}

/// Conveniences for tests that don't care about the prompt.
#[cfg(test)]
impl Duel {
    pub fn set_coin(&mut self, coin: Coin) {
        self.coin = coin;
    }

    /// Show the Hand and Hold.
    pub fn end_turn(&mut self) -> TurnResult {
        match self.show_hand() {
            Some(result) => result,
            None => self.hold().expect("showing a clearing Hand puts the prompt up"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::{RisingBlinds, Tell};

    pub(super) fn card(stack: u32) -> Card {
        Card { name: "card", stack, tell: None }
    }
    fn streak(stack: u32) -> Card {
        Card { name: "streak", stack, tell: Some(Tell::Streak) }
    }
    fn all_in(stack: u32) -> Card {
        Card { name: "all in", stack, tell: Some(Tell::AllIn) }
    }
    pub(super) fn enemy(stack: u32, house_edge: u32) -> Enemy {
        Enemy { name: "shill", stack, house_edge, blinds: RisingBlinds { every_turns: 2, increase: 2 } }
    }
    /// A deck of vanilla cards 1..=n, so card n is drawn first.
    fn vanilla_deck(n: u32) -> Vec<Card> {
        (1..=n).map(card).collect()
    }

    #[test]
    fn a_new_duel_deals_seven_cards_into_the_draw() {
        let duel = Duel::new(vanilla_deck(18), 40, 5, enemy(30, 20));

        assert_eq!(duel.draw().len(), 7);
        let stacks: Vec<u32> = duel.draw().iter().map(|c| c.stack).collect();
        assert_eq!(stacks, vec![18, 17, 16, 15, 14, 13, 12]);
    }

    #[test]
    fn playing_a_vanilla_card_adds_its_stack_to_the_hand() {
        let mut duel = Duel::new(vanilla_deck(18), 40, 5, enemy(30, 20));

        let contributed = duel.play(0, None).unwrap();

        assert_eq!(contributed, 18);
        assert_eq!(duel.hand(), 18);
        assert_eq!(duel.plays_left(), 4);
        assert_eq!(duel.draw().len(), 6);
        assert!(duel.draw().iter().all(|c| c.stack != 18));
    }

    #[test]
    fn streak_doubles_when_the_previous_card_had_any_tell() {
        // Drawn first: all_in(2), streak(5), card(3), ...
        let deck = vec![card(1), card(1), card(1), card(1), card(3), streak(5), all_in(2)];
        let mut duel = Duel::new(deck, 40, 5, enemy(30, 20));

        duel.play(0, Some(2)).unwrap(); // All In 2, sacrificing card(3): 5
        let doubled = duel.play(0, None).unwrap(); // Streak 5 after a Tell

        assert_eq!(doubled, 10);
        assert_eq!(duel.hand(), 15);
    }

    #[test]
    fn streak_does_not_double_after_a_vanilla_card_or_as_the_first_play() {
        let deck = vec![card(1), card(1), card(1), card(1), streak(4), card(3), streak(5)];
        let mut duel = Duel::new(deck, 40, 5, enemy(30, 20));

        assert_eq!(duel.play(0, None).unwrap(), 5); // first play
        assert_eq!(duel.play(0, None).unwrap(), 3); // vanilla
        assert_eq!(duel.play(0, None).unwrap(), 4); // Streak after vanilla
        assert_eq!(duel.hand(), 12);
    }

    #[test]
    fn all_in_burns_the_sacrifice_and_adds_its_stack() {
        let deck = vec![card(1), card(1), card(1), card(1), card(3), card(8), all_in(2)];
        let mut duel = Duel::new(deck, 40, 5, enemy(30, 20));

        let contributed = duel.play(0, Some(1)).unwrap(); // sacrifice card(8)

        assert_eq!(contributed, 10);
        assert_eq!(duel.hand(), 10);
        assert_eq!(duel.plays_left(), 4);
        // Both the All In and the sacrifice have left the Draw.
        assert_eq!(duel.draw().len(), 5);
        assert!(duel.draw().iter().all(|c| c.stack != 8 && c.tell.is_none()));
    }

    #[test]
    fn illegal_plays_are_refused_and_change_nothing() {
        let deck = vec![card(1), card(1), card(1), card(1), card(3), card(8), all_in(2)];
        let mut duel = Duel::new(deck, 40, 5, enemy(30, 20));

        assert_eq!(duel.play(9, None), Err(PlayError::NoSuchCard));
        assert_eq!(duel.play(0, None), Err(PlayError::AllInNeedsSacrifice));
        assert_eq!(duel.play(0, Some(0)), Err(PlayError::NoSuchCard));
        assert_eq!(duel.play(0, Some(9)), Err(PlayError::NoSuchCard));
        assert_eq!(duel.play(1, Some(2)), Err(PlayError::NotAllIn));
        assert_eq!(duel.hand(), 0);
        assert_eq!(duel.draw().len(), 7);

        for _ in 0..5 {
            duel.play(1, None).unwrap();
        }
        assert_eq!(duel.play(0, Some(1)), Err(PlayError::NoPlaysLeft));
    }

    #[test]
    fn clearing_the_house_edge_pays_the_excess_out_of_the_enemy_stack() {
        // Draw: 18,17,16,15,14,13,12. Play the top five: 80.
        let mut duel = Duel::new(vanilla_deck(18), 40, 5, enemy(100, 20));
        for _ in 0..5 {
            duel.play(0, None).unwrap();
        }

        let result = duel.end_turn();

        assert_eq!(result, TurnResult { hand: 80, house_edge: 20, kind: Outcome::Payout(60), pyl: None, blinds_rose: false });
        assert_eq!(duel.enemy_stack(), 40);
        assert_eq!(duel.player_stack(), 40);
        assert_eq!(duel.outcome(), None);
    }

    #[test]
    fn falling_short_is_a_whiff_that_comes_out_of_your_own_stack() {
        let mut duel = Duel::new(vanilla_deck(18), 40, 5, enemy(100, 30));
        duel.play(0, None).unwrap(); // 18

        let result = duel.end_turn();

        assert_eq!(result.kind, Outcome::Whiff(12));
        assert_eq!(duel.player_stack(), 28);
        assert_eq!(duel.enemy_stack(), 100);
    }

    #[test]
    fn ending_the_turn_resets_the_hand_and_refills_the_draw() {
        let mut duel = Duel::new(vanilla_deck(18), 40, 5, enemy(100, 20));
        for _ in 0..5 {
            duel.play(0, None).unwrap();
        }
        duel.end_turn();

        assert_eq!(duel.hand(), 0);
        assert_eq!(duel.plays_left(), 5);
        assert_eq!(duel.draw().len(), 7);
        // The two unplayed cards carried over, then five fresh ones.
        let stacks: Vec<u32> = duel.draw().iter().map(|c| c.stack).collect();
        assert_eq!(stacks, vec![13, 12, 11, 10, 9, 8, 7]);
    }

    #[test]
    fn rising_blinds_raise_the_house_edge_every_two_turns() {
        let mut duel = Duel::new(vanilla_deck(40), 999, 5, enemy(9999, 20));

        assert_eq!(duel.house_edge(), 20);
        let t1 = duel.end_turn();
        assert!(!t1.blinds_rose);
        assert_eq!(duel.house_edge(), 20);
        let t2 = duel.end_turn();
        assert!(t2.blinds_rose);
        assert_eq!(duel.house_edge(), 22);
        duel.end_turn();
        duel.end_turn();
        assert_eq!(duel.house_edge(), 24);
    }

    #[test]
    fn the_duel_is_won_when_the_enemy_stack_hits_zero() {
        let mut duel = Duel::new(vanilla_deck(18), 40, 5, enemy(50, 20));
        for _ in 0..5 {
            duel.play(0, None).unwrap();
        }

        duel.end_turn(); // 80 vs 20: Payout 60 against a 50 Stack

        assert_eq!(duel.enemy_stack(), 0);
        assert_eq!(duel.outcome(), Some(CombatOutcome::Won));
    }

    #[test]
    fn the_duel_is_lost_when_your_stack_hits_zero() {
        let mut duel = Duel::new(vanilla_deck(18), 10, 5, enemy(50, 30));

        duel.end_turn(); // nothing played: Whiff 30 against a 10 Stack

        assert_eq!(duel.player_stack(), 0);
        assert_eq!(duel.outcome(), Some(CombatOutcome::Lost));
    }

    #[test]
    fn the_discard_is_reshuffled_into_the_deck_when_it_runs_dry() {
        // 9 cards: 7 dealt, 2 in the deck. After a turn of 5 plays there are
        // 2 left in the Draw, 2 come from the deck, and 3 must come back
        // from the discard.
        let mut duel = Duel::new(vanilla_deck(9), 40, 5, enemy(999, 1));
        for _ in 0..5 {
            duel.play(0, None).unwrap();
        }
        duel.end_turn();

        assert_eq!(duel.draw().len(), 7);
        let mut stacks: Vec<u32> = duel.draw().iter().map(|c| c.stack).collect();
        stacks.sort_unstable();
        // 4,3 carried over; 2,1 from the deck; three of {9,8,7,6,5} recycled.
        assert!(stacks.contains(&4) && stacks.contains(&3) && stacks.contains(&2) && stacks.contains(&1));
        assert_eq!(stacks.iter().filter(|&&v| v >= 5).count(), 3);
    }
}

#[cfg(test)]
mod push_your_luck_tests {
    use super::tests::{card, enemy};
    use super::*;
    use crate::run::Perk;

    /// Play the whole Draw down to a Hand of `hand` against `house_edge`.
    fn hand_of(hand: u32, house_edge: u32) -> Duel {
        // One vanilla card worth `hand`, then filler the test never plays.
        let mut deck = vec![card(1); 6];
        deck.push(card(hand));
        let mut duel = Duel::new(deck, 40, 5, enemy(999, house_edge)).with_coin(Coin::SURE_THING);
        duel.play(0, None).unwrap();
        duel
    }

    #[test]
    fn showing_a_clearing_hand_offers_push_your_luck_instead_of_resolving() {
        let mut duel = hand_of(30, 20);

        assert_eq!(duel.show_hand(), None);
        assert_eq!(duel.phase(), Phase::PushYourLuck);
        // Nothing has been dealt yet.
        assert_eq!(duel.enemy_stack(), 999);
        assert_eq!(duel.hand(), 30);
    }

    #[test]
    fn a_whiff_is_never_offered_the_flip_and_resolves_on_the_spot() {
        let mut duel = hand_of(10, 20);

        let result = duel.show_hand().expect("a Whiff resolves without a prompt");

        assert_eq!(result.kind, Outcome::Whiff(10));
        assert_eq!(result.pyl, None);
        assert_eq!(duel.phase(), Phase::Playing);
        assert_eq!(duel.player_stack(), 30);
    }

    #[test]
    fn holding_resolves_the_turn_as_normal() {
        let mut duel = hand_of(30, 20);
        duel.show_hand();

        let result = duel.hold().expect("the prompt is up");

        assert_eq!(result, TurnResult { hand: 30, house_edge: 20, kind: Outcome::Payout(10), pyl: None, blinds_rose: false });
        assert_eq!(duel.enemy_stack(), 989);
        assert_eq!(duel.phase(), Phase::Playing);
    }

    #[test]
    fn pushing_and_winning_doubles_the_payout_and_not_the_hand() {
        let mut duel = hand_of(30, 20);
        duel.show_hand();

        let result = duel.push().expect("the prompt is up");

        assert_eq!(result.pyl, Some(Push::Won));
        assert_eq!(result.hand, 30);
        assert_eq!(result.kind, Outcome::Payout(20));
        assert_eq!(duel.enemy_stack(), 979);
    }

    #[test]
    fn pushing_and_losing_zeroes_the_hand_into_a_full_whiff() {
        let mut duel = hand_of(30, 20);
        duel.set_coin(Coin::RIGGED);
        duel.show_hand();

        let result = duel.push().expect("the prompt is up");

        assert_eq!(result.pyl, Some(Push::Lost));
        assert_eq!(result.hand, 0);
        assert_eq!(result.kind, Outcome::Whiff(20));
        assert_eq!(duel.player_stack(), 20);
        assert_eq!(duel.enemy_stack(), 999);
    }

    #[test]
    fn a_lost_push_can_end_the_duel() {
        let mut deck = vec![card(1); 6];
        deck.push(card(30));
        let mut duel = Duel::new(deck, 15, 5, enemy(999, 20)).with_coin(Coin::RIGGED);
        duel.play(0, None).unwrap();
        duel.show_hand();

        duel.push().unwrap();

        assert_eq!(duel.player_stack(), 0);
        assert_eq!(duel.outcome(), Some(CombatOutcome::Lost));
    }

    #[test]
    fn the_hand_is_final_once_the_prompt_is_up() {
        let mut duel = hand_of(30, 20);
        duel.show_hand();

        assert_eq!(duel.play(0, None), Err(PlayError::HandIsFinal));
        assert_eq!(duel.show_hand(), None);
        assert_eq!(duel.hand(), 30);
    }

    #[test]
    fn push_and_hold_do_nothing_when_no_prompt_is_up() {
        let mut duel = hand_of(30, 20);

        assert_eq!(duel.push(), None);
        assert_eq!(duel.hold(), None);
        assert_eq!(duel.enemy_stack(), 999);
    }

    #[test]
    fn end_turn_shows_the_hand_and_holds() {
        let mut duel = hand_of(30, 20);

        let result = duel.end_turn();

        assert_eq!(result.kind, Outcome::Payout(10));
        assert_eq!(result.pyl, None);
        assert_eq!(duel.phase(), Phase::Playing);
    }

    #[test]
    fn the_base_coin_is_one_flip_the_player_takes_45_times_in_100() {
        assert_eq!(Coin::BASE, Coin { player_pct: 45, best_of: 1 });
        assert_eq!(Coin::BASE.resolve([44].into_iter()), Push::Won);
        assert_eq!(Coin::BASE.resolve([45].into_iter()), Push::Lost);
        assert_eq!(wins_over_every_roll(Coin::BASE), 450_000);
    }

    /// Every flip the coin can be handed, counted: 100 rolls per flip.
    fn wins_over_every_roll(coin: Coin) -> u32 {
        let mut wins = 0;
        for a in 0..100 {
            for b in 0..100 {
                for c in 0..100 {
                    if coin.resolve([a, b, c].into_iter()) == Push::Won {
                        wins += 1;
                    }
                }
            }
        }
        wins
    }

    #[test]
    fn slotz_option_one_is_best_two_of_three_at_49_51() {
        assert_eq!(Coin::SLOTZ, Coin { player_pct: 49, best_of: 3 });
        // Two wins take it, whichever way the third would have gone.
        assert_eq!(Coin::SLOTZ.resolve([10, 90, 10].into_iter()), Push::Won);
        assert_eq!(Coin::SLOTZ.resolve([90, 10, 10].into_iter()), Push::Won);
        assert_eq!(Coin::SLOTZ.resolve([90, 10, 90].into_iter()), Push::Lost);
        // 49 is the House's; 48 is the player's.
        assert_eq!(Coin::SLOTZ.resolve([48, 48].into_iter()), Push::Won);
        assert_eq!(Coin::SLOTZ.resolve([49, 49].into_iter()), Push::Lost);
    }

    #[test]
    fn a_decided_best_of_three_stops_flipping() {
        let mut rolls = [10, 10, 10].into_iter();
        assert_eq!(Coin::SLOTZ.resolve(rolls.by_ref()), Push::Won);
        assert_eq!(rolls.count(), 1, "the third coin is never tossed");
    }

    #[test]
    fn the_slotz_perk_is_what_swaps_the_coin() {
        assert_eq!(Coin::for_perks(&[]), Coin::BASE);
        assert_eq!(Coin::for_perks(&[Perk::SixPlaysSteepBlinds]), Coin::BASE);
        assert_eq!(Coin::for_perks(&[Perk::PylBestTwoOfThree]), Coin::SLOTZ);
    }

    #[test]
    fn slotz_option_one_is_worth_about_48_point_5_percent() {
        // 3p^2 - 2p^3 at p = 49/100, over the million flips it can be handed.
        assert_eq!(wins_over_every_roll(Coin::SLOTZ), 485_002);
        assert!(wins_over_every_roll(Coin::SLOTZ) > wins_over_every_roll(Coin::BASE), "the perk is worth taking");
    }

    #[test]
    fn the_duels_own_coin_lands_on_both_sides() {
        let mut seen = (false, false);
        for seed in 1..200u64 {
            let mut duel = hand_of(30, 20).with_seed(seed * 2 + 1);
            duel.set_coin(Coin::BASE);
            duel.show_hand();
            match duel.push().unwrap().pyl {
                Some(Push::Won) => seen.0 = true,
                Some(Push::Lost) => seen.1 = true,
                None => panic!("a Push always flips"),
            }
        }
        assert_eq!(seen, (true, true), "the duel's own rolls reach both sides of the coin");
    }
}
