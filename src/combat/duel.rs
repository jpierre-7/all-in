//! The pure combat model: one duel, no Bevy. Vocabulary follows `CONTEXT.md`.

use crate::run::{Card, CombatOutcome, Enemy, Tell};

pub const DRAW_SIZE: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayError {
    NoPlaysLeft,
    NoSuchCard,
    /// An All In card was played without naming a card to sacrifice.
    AllInNeedsSacrifice,
    /// A sacrifice was named for a card that isn't All In.
    NotAllIn,
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

    /// Compare The Hand to House Edge, deal the Payout or Whiff, tick Rising
    /// Blinds, and refill the Draw for the next turn. The enemy does nothing
    /// on its turn.
    pub fn end_turn(&mut self) -> TurnResult {
        let hand = self.hand;
        let house_edge = self.house_edge;
        let kind = if hand >= house_edge {
            let payout = hand - house_edge;
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
        self.refill();

        TurnResult { hand, house_edge, kind, blinds_rose }
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

    /// Fisher-Yates over a xorshift64; enough randomness for a card game.
    fn shuffle_deck(&mut self) {
        for i in (1..self.deck.len()).rev() {
            self.rng ^= self.rng << 13;
            self.rng ^= self.rng >> 7;
            self.rng ^= self.rng << 17;
            let j = (self.rng % (i as u64 + 1)) as usize;
            self.deck.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::{RisingBlinds, Tell};

    fn card(stack: u32) -> Card {
        Card { name: "card", stack, tell: None }
    }
    fn streak(stack: u32) -> Card {
        Card { name: "streak", stack, tell: Some(Tell::Streak) }
    }
    fn all_in(stack: u32) -> Card {
        Card { name: "all in", stack, tell: Some(Tell::AllIn) }
    }
    fn enemy(stack: u32, house_edge: u32) -> Enemy {
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

        assert_eq!(result, TurnResult { hand: 80, house_edge: 20, kind: Outcome::Payout(60), blinds_rose: false });
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
