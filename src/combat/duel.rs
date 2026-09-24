//! The pure combat model: one duel, no Bevy. Vocabulary follows `CONTEXT.md`.
//!
//! A turn is two rows facing each other. The enemy lays its Opposing Cards
//! down first, some face up and some face down; the player fills the row
//! across from them out of the Draw, one card per slot, and confirms. Both
//! rows turn over, each resolves its Tells by position, and the side that
//! comes up short loses the difference off its own Chips.

use crate::run::{Card, CombatOutcome, Enemy, HoleCard, LOADED_DICE_BONUS, Perk, Tell, xorshift64};

pub const DRAW_SIZE: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayError {
    /// Every slot the player can cover is already covered.
    RowIsFull,
    NoSuchCard,
    /// An All In card was played without naming a card to sacrifice.
    AllInNeedsSacrifice,
    /// A sacrifice was named for a card that isn't All In.
    NotAllIn,
    /// The rows are face up and the Push Your Luck prompt is up.
    HandIsFinal,
}

/// Where the turn is. Playing is the player building their row against the
/// Opposing Cards; the Hand is only final once the rows have been compared.
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
    pub const BASE: Coin = Coin {
        player_pct: 45,
        best_of: 1,
    };
    /// Slotz Option 1: best 2-of-3 at 49/51, which is 3p2 - 2p3 =~ 48.5%.
    pub const SLOTZ: Coin = Coin {
        player_pct: 49,
        best_of: 3,
    };

    pub fn for_perks(perks: &[Perk]) -> Coin {
        if perks.contains(&Perk::PylBestTwoOfThree) {
            Coin::SLOTZ
        } else {
            Coin::BASE
        }
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
    /// The Hand beat the Opposing Cards by this much; dealt to the enemy's
    /// Chips. Zero when the two rows tied and nobody pays.
    Payout(u32),
    /// The Hand fell short by this much; dealt to the player's Chips.
    Whiff(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnResult {
    pub hand: u32,
    /// What the Opposing Cards came to: the House Edge this Hand was compared
    /// against.
    pub house_edge: u32,
    pub kind: Outcome,
    /// The flip, if the player Pushed. `None` means Hold, or no prompt.
    pub pyl: Option<Push>,
    /// Whether Rising Blinds ticked at the end of this turn.
    pub blinds_rose: bool,
}

/// A card in a row and what it resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Played {
    pub card: Card,
    /// What the card added to its row, after its Tell.
    pub value: u32,
}

/// A card the player has put in the row, with whatever it burned to get there.
/// The burned card only reaches the discard when the turn resolves, so lifting
/// the All In back out of the row hands its sacrifice back too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placed {
    pub card: Card,
    /// The All In's sacrifice, waiting out the turn.
    pub sacrifice: Option<Card>,
}

impl Placed {
    /// A card with nothing burned for it: every Opposing Card, and every
    /// player card that isn't an All In.
    fn plain(card: Card) -> Self {
        Placed {
            card,
            sacrifice: None,
        }
    }
}

/// One of the enemy's cards, and whether the player can see it yet. The first
/// Opposing Card of a row is always face up; the rest are a coin toss at the
/// odds the enemy's `Deal` sets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opposing {
    pub card: Card,
    pub face_up: bool,
}

/// Both rows as they turned over, and what they added up to. Kept so the
/// screen can go on showing the comparison after the turn has resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Showdown {
    /// The player's row, resolved left to right.
    pub row: Vec<Played>,
    /// The Opposing Cards, resolved left to right.
    pub opposing: Vec<Played>,
    /// The Hand, Loaded Dice included. A lost Push zeroes it.
    pub hand: u32,
    /// The House Edge: what the Opposing Cards came to.
    pub house_edge: u32,
}

/// Resolve a row left to right against the row across from it, and say what
/// each card is worth.
///
/// Every Tell reads by position — Streak one slot left, Copycat one slot
/// right, Flop straight across — and every one of them reads a *printed*
/// Face Value. That is what keeps this a single pass with no ordering to argue
/// about: no card's value depends on another card's value, so the two rows
/// facing each other can be worked out in either order and a Flop on each
/// side of the table never chases the other one round in a circle.
fn resolve_row(row: &[Placed], across: &[Option<Card>]) -> Vec<Played> {
    row.iter()
        .enumerate()
        .map(|(slot, placed)| {
            let printed = placed.card.face_value;
            let value = match placed.card.tell {
                Some(Tell::Streak) if slot > 0 && row[slot - 1].card.tell.is_some() => printed * 2,
                Some(Tell::AllIn) => {
                    printed + placed.sacrifice.as_ref().map_or(0, |c| c.face_value)
                }
                Some(Tell::Copycat) => row
                    .get(slot + 1)
                    .map_or(printed, |next| next.card.face_value),
                Some(Tell::Flop) => across
                    .get(slot)
                    .and_then(Option::as_ref)
                    .map_or(printed, |card| card.face_value),
                _ => printed,
            };
            Played {
                card: placed.card.clone(),
                value,
            }
        })
        .collect()
}

pub struct Duel {
    deck: Vec<Card>,
    draw: Vec<Card>,
    discard: Vec<Card>,
    /// The enemy's row this turn, laid down before the player builds theirs.
    opposing: Vec<Opposing>,
    /// The player's row. Slot `i` sits across from Opposing Card `i`.
    row: Vec<Placed>,
    /// The most cards the player may put in the row.
    plays: u8,
    /// How many Opposing Cards the enemy lays down. Rising Blinds grow it.
    row_size: u8,
    player_chips: u32,
    enemy: Enemy,
    turn: u32,
    phase: Phase,
    coin: Coin,
    /// Hands still carrying the Loaded Dice bonus, carried in from the run.
    dice_left: u8,
    /// The House's live margin (#5). `None` for an ordinary enemy.
    hole_card: Option<HoleCard>,
    /// The Arcade's fixed row (#40), dealt again every turn. `None` in a real
    /// duel, where the enemy rolls its own.
    fixed: Option<Vec<(Card, bool)>>,
    /// Set when the rows turn over; taken when the turn resolves.
    showdown: Option<Showdown>,
    /// The last showdown, for the screen to keep showing into the next turn.
    last: Option<Showdown>,
    rng: u64,
}

impl Duel {
    /// `deck` is taken in draw order (last element drawn first); shuffling is
    /// the caller's job so the model stays deterministic. The enemy's first
    /// row is dealt here and re-dealt by [`Duel::with_seed`].
    pub fn new(deck: Vec<Card>, player_chips: u32, plays: u8, enemy: Enemy) -> Self {
        let mut duel = Duel {
            deck,
            draw: Vec::new(),
            discard: Vec::new(),
            opposing: Vec::new(),
            row: Vec::new(),
            plays,
            row_size: enemy.deal.row,
            player_chips,
            hole_card: enemy.hole_card,
            enemy,
            turn: 1,
            phase: Phase::Playing,
            coin: Coin::BASE,
            dice_left: 0,
            fixed: None,
            showdown: None,
            last: None,
            rng: 0x5eed_cafe_f00d_d1ce,
        };
        duel.refill();
        duel.deal_opposing();
        duel
    }

    /// Seeds the reshuffle of the discard pile and the enemy's deal. The
    /// initial deck order is still the caller's, but the Opposing Cards are
    /// dealt again off the real seed: they were rolled in `new` before the
    /// caller got a chance to hand one over.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng = seed | 1;
        self.deal_opposing();
        self
    }

    /// The Arcade's fixed Opposing Cards (#40): the same row, the same face
    /// up and face down, every turn. Nothing about a practice hand is rolled.
    pub fn with_opposing(mut self, row: Vec<(Card, bool)>) -> Self {
        self.fixed = Some(row);
        self.deal_opposing();
        self
    }

    /// The coin Push Your Luck is flipped with; Slotz Option 1 swaps it.
    pub fn with_coin(mut self, coin: Coin) -> Self {
        self.coin = coin;
        self
    }

    /// Loaded Dice carried in from the run: +5 on each of the next `hands`
    /// Hands. Combat writes back whatever is left when the duel ends.
    pub fn with_loaded_dice(mut self, hands: u8) -> Self {
        self.dice_left = hands;
        self
    }

    /// The House's current margin; `None` for an ordinary enemy.
    pub fn margin(&self) -> Option<u32> {
        self.hole_card.map(|h| h.margin)
    }

    pub fn coin(&self) -> Coin {
        self.coin
    }

    /// Hands still to come with the dice on them.
    pub fn dice_left(&self) -> u8 {
        self.dice_left
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// The Payout The Hand would deal right now: its excess over the House
    /// Edge, doubled by a won Push. 0 on a Whiff, and 0 on a tie.
    pub fn payout(&self, pyl: Option<Push>) -> u32 {
        self.hand().saturating_sub(self.house_edge()) * if pyl == Some(Push::Won) { 2 } else { 1 }
    }

    /// The Whiff The Hand would take right now: its shortfall under House
    /// Edge, forgiven by a won Push and doubled by a lost one. 0 on a
    /// clearing Hand.
    pub fn whiff(&self, pyl: Option<Push>) -> u32 {
        let short = self.house_edge().saturating_sub(self.hand());
        match pyl {
            Some(Push::Won) => 0,
            Some(Push::Lost) => short * 2,
            None => short,
        }
    }

    /// The turn about to be played, counting from 1.
    pub fn turn(&self) -> u32 {
        self.turn
    }

    pub fn player_chips(&self) -> u32 {
        self.player_chips
    }

    pub fn enemy_chips(&self) -> u32 {
        self.enemy.chips
    }

    pub fn outcome(&self) -> Option<CombatOutcome> {
        if self.enemy.chips == 0 {
            Some(CombatOutcome::Won)
        } else if self.player_chips == 0 {
            Some(CombatOutcome::Lost)
        } else {
            None
        }
    }

    pub fn draw(&self) -> &[Card] {
        &self.draw
    }

    /// The Opposing Cards this turn, left to right.
    pub fn opposing(&self) -> &[Opposing] {
        &self.opposing
    }

    /// The player's row so far, left to right.
    pub fn row(&self) -> &[Placed] {
        &self.row
    }

    /// The rows as they last turned over, for the screen. `None` until the
    /// first turn resolves.
    pub fn last_showdown(&self) -> Option<&Showdown> {
        self.last.as_ref()
    }

    /// The rows on the table right now, resolved — the moment between the
    /// confirm and the answer to Push Your Luck, and the only time the screen
    /// can put a number under every card. `None` at every other moment.
    pub fn showdown(&self) -> Option<&Showdown> {
        self.showdown.as_ref()
    }

    /// The Hand: the player's row resolved against the Opposing Cards.
    /// Once the rows have turned over this is the settled number, Loaded Dice
    /// and a lost Push included.
    pub fn hand(&self) -> u32 {
        match &self.showdown {
            Some(showdown) => showdown.hand,
            None => row_value(&resolve_row(&self.row, &self.opposing_face_up_cards())),
        }
    }

    /// The House Edge, what the Opposing Cards add up to, resolved against the
    /// row the player has built so far.
    ///
    /// The whole truth, face-down cards included, so before the showdown this
    /// is more than the player is entitled to know. The screen shows
    /// [`Duel::showing`] instead until the rows turn over.
    pub fn house_edge(&self) -> u32 {
        match &self.showdown {
            Some(showdown) => showdown.house_edge,
            None => row_value(&resolve_row(&self.opposing_row(), &self.row_cards())),
        }
    }

    /// What the player can add up for themselves: the value of the
    /// Opposing Cards that are face up, and how many are still face down.
    ///
    /// A face-up card whose Tell reads a face-down one counts for its Face
    /// Value alone. The row never says more about a hidden card than the fact
    /// of its being hidden already does, so this only ever under-reads.
    pub fn showing(&self) -> (u32, usize) {
        let across = self.row_cards();
        let face_up = |slot: usize| {
            self.opposing
                .get(slot)
                .filter(|o| o.face_up)
                .map(|o| &o.card)
        };
        let mut total = 0;
        let mut hidden = 0;
        for (slot, opposing) in self.opposing.iter().enumerate() {
            if !opposing.face_up {
                hidden += 1;
                continue;
            }
            let printed = opposing.card.face_value;
            total += match opposing.card.tell {
                Some(Tell::Streak) => match slot.checked_sub(1).and_then(face_up) {
                    Some(left) if left.tell.is_some() => printed * 2,
                    _ => printed,
                },
                Some(Tell::Copycat) => face_up(slot + 1).map_or(printed, |next| next.face_value),
                Some(Tell::Flop) => across
                    .get(slot)
                    .and_then(Option::as_ref)
                    .map_or(printed, |card| card.face_value),
                _ => printed,
            };
        }
        (total, hidden)
    }

    /// How many slots there are to cover: one per Opposing Card.
    pub fn slots(&self) -> usize {
        self.opposing.len()
    }

    /// How many more cards the player may put in the row. An enemy row longer
    /// than the player's Plays can't be covered slot for slot, and whatever
    /// is left uncovered is free chips for the enemy — which is what the
    /// Rising Blinds now buy.
    pub fn plays_left(&self) -> u8 {
        let cover = usize::from(self.plays).min(self.slots());
        cover.saturating_sub(self.row.len()) as u8
    }

    /// Put the card at `card` in the Draw into the next empty slot of the
    /// row, and say which slot it landed in. Nothing resolves yet: a row is
    /// only worth anything once it is finished and every Tell knows its
    /// neighbours. All In must name a `sacrifice` index (also into the Draw);
    /// no other card may.
    pub fn place(&mut self, card: usize, sacrifice: Option<usize>) -> Result<usize, PlayError> {
        if self.phase == Phase::PushYourLuck {
            return Err(PlayError::HandIsFinal);
        }
        if self.plays_left() == 0 {
            return Err(PlayError::RowIsFull);
        }
        let played = self.draw.get(card).ok_or(PlayError::NoSuchCard)?.clone();
        let sacrificed = match (played.tell, sacrifice) {
            (Some(Tell::AllIn), None) => return Err(PlayError::AllInNeedsSacrifice),
            (Some(Tell::AllIn), Some(i)) if i == card => return Err(PlayError::NoSuchCard),
            (Some(Tell::AllIn), Some(i)) => {
                Some(self.draw.get(i).ok_or(PlayError::NoSuchCard)?.clone())
            }
            (_, Some(_)) => return Err(PlayError::NotAllIn),
            (_, None) => None,
        };

        // Remove the higher index first so the lower one stays valid.
        let mut gone: Vec<usize> = sacrifice.into_iter().chain([card]).collect();
        gone.sort_unstable_by(|a, b| b.cmp(a));
        for i in gone {
            self.draw.remove(i);
        }

        self.row.push(Placed {
            card: played,
            sacrifice: sacrificed,
        });
        Ok(self.row.len() - 1)
    }

    /// Take the card in `slot` back out of the row and into the Draw, with
    /// whatever it burned to get there.
    ///
    /// Everything to its right shifts one slot left, so the row stays a row
    /// and the Tells re-read whatever their new neighbours are. Rearranging a
    /// row is lifting cards off the right-hand end and putting them back in
    /// another order.
    pub fn lift(&mut self, slot: usize) -> Result<Card, PlayError> {
        if self.phase == Phase::PushYourLuck {
            return Err(PlayError::HandIsFinal);
        }
        if slot >= self.row.len() {
            return Err(PlayError::NoSuchCard);
        }
        let placed = self.row.remove(slot);
        if let Some(burned) = placed.sacrifice {
            self.draw.push(burned);
        }
        self.draw.push(placed.card.clone());
        Ok(placed.card)
    }

    /// Confirm the row. Both sides turn over, each resolves its Tells, and
    /// The Hand meets the House Edge. A Hand that beats the Opposing Cards puts
    /// the Push Your Luck prompt up and resolves nothing yet (`None`); a
    /// Whiff, or a tie where nobody pays, resolves on the spot.
    pub fn confirm(&mut self) -> Option<TurnResult> {
        if self.phase == Phase::PushYourLuck {
            return None;
        }
        // The House fills its last Opposing Card now, on everything it can
        // see, which is the player's row but their last card (#5).
        self.house_sets_its_hole_card();
        for opposing in &mut self.opposing {
            opposing.face_up = true;
        }

        let row = resolve_row(&self.row, &self.opposing_cards());
        let opposing = resolve_row(&self.opposing_row(), &self.row_cards());
        let mut hand = row_value(&row);
        let house_edge = row_value(&opposing);
        // The items that modify The Hand land here, after every card is down
        // and after The House has set the card it set on what it could see.
        if self.dice_left > 0 {
            self.dice_left -= 1;
            hand += LOADED_DICE_BONUS;
        }
        self.last = self.showdown.clone();

        self.showdown = Some(Showdown {
            row,
            opposing,
            hand,
            house_edge,
        });

        if hand as i32 - house_edge as i32 != 0 {
            self.phase = Phase::PushYourLuck;
            return None;
        }

        Some(self.resolve(None))
    }

    /// Answer the prompt with Hold: the turn resolves as normal. `None` when
    /// no prompt is up.
    pub fn hold(&mut self) -> Option<TurnResult> {
        if self.phase != Phase::PushYourLuck {
            return None;
        }
        Some(self.resolve(None))
    }

    /// Answer the prompt with Push: flip the coin. On a clearing Hand, win
    /// and the Payout doubles; lose and The Hand becomes 0, a full Whiff for
    /// the whole House Edge. On a Whiff, win and it is forgiven; lose and it
    /// doubles. `None` when no prompt is up.
    pub fn push(&mut self) -> Option<TurnResult> {
        if self.phase != Phase::PushYourLuck {
            return None;
        }
        let coin = self.coin;
        let flip = coin.resolve(std::iter::repeat_with(|| (self.next_rng() % 100) as u32));

        Some(self.resolve(Some(flip)))
    }

    /// Deal the Payout or the Whiff, tick Rising Blinds, clear both rows, and
    /// deal the next turn's Opposing Cards. The enemy does nothing else on
    /// its turn: laying the row down *was* its turn.
    fn resolve(&mut self, pyl: Option<Push>) -> TurnResult {
        let hand = self.hand();
        let house_edge = self.house_edge();
        let kind = match (hand >= house_edge, pyl) {
            // A lost Push on a clearing Hand zeroes it: a full Whiff.
            (true, Some(Push::Lost)) => Outcome::Whiff(house_edge),
            (true, _) => Outcome::Payout(self.payout(pyl)),
            (false, _) => Outcome::Whiff(self.whiff(pyl)),
        };
        match kind {
            Outcome::Payout(n) => self.enemy.chips = self.enemy.chips.saturating_sub(n),
            Outcome::Whiff(n) => self.player_chips = self.player_chips.saturating_sub(n),
        }

        let every = u32::from(self.enemy.blinds.every_turns.max(1));
        let blinds_rose = self.turn.is_multiple_of(every);
        if blinds_rose {
            match self.hole_card.as_mut() {
                // The House's Blinds raise the margin, not the row: it wins
                // by outlasting the deck, not by laying more cards down.
                Some(hole) => hole.margin += hole.step,
                None => self.row_size = self.row_size.saturating_add(self.enemy.blinds.cards),
            }
        }

        self.turn += 1;
        self.phase = Phase::Playing;
        // The row and everything it burned go to the discard. The Opposing
        // Cards came off no deck and go nowhere.
        for placed in std::mem::take(&mut self.row) {
            if let Some(burned) = placed.sacrifice {
                self.discard.push(burned);
            }
            self.discard.push(placed.card);
        }
        self.last = self.showdown.take();
        self.refill();
        self.deal_opposing();

        TurnResult {
            hand,
            house_edge,
            kind,
            pyl,
            blinds_rose,
        }
    }

    /// The House sets the card it kept back so that its row reads the
    /// player's row — everything but the player's last card — plus the
    /// margin. The player's last card is the Hole Card, and the Payout is
    /// whatever it is worth over the margin.
    ///
    /// The cards it already dealt itself are part of that total, not on top
    /// of it, so the Hole Card is the remainder. When they have already made
    /// more than the margin asks for, the Hole Card is worth nothing and the
    /// row is simply what The House dealt — it cannot un-deal a card to come
    /// back down. That only happens against a row with almost nothing in it,
    /// which is why The House deals itself scraps.
    fn house_sets_its_hole_card(&mut self) {
        let Some(hole) = self.hole_card else { return };
        let Some(slot) = self.opposing.len().checked_sub(1) else {
            return;
        };
        // Everything the player put down but their last card.
        let seen = self.row.len().saturating_sub(1).min(slot);
        let across: Vec<Option<Card>> = self.opposing[..seen]
            .iter()
            .map(|o| Some(o.card.clone()))
            .collect();
        let read = row_value(&resolve_row(&self.row[..seen], &across));
        // What its own cards in front of the blank already make. They can't
        // move once the blank is filled: The House deals itself no Copycat,
        // which is the one Tell that would have to read the card it hasn't
        // decided on yet.
        let mine: Vec<Placed> = self.opposing[..slot]
            .iter()
            .map(|o| Placed::plain(o.card.clone()))
            .collect();
        let opposing = row_value(&resolve_row(&mine, &self.row_cards()));

        self.opposing[slot].card = Card {
            name: "The House's Hole Card",
            face_value: (read + hole.margin).saturating_sub(opposing),
            tell: None,
        };
    }

    /// The Opposing Cards as a row that [`resolve_row`] can read. Nothing
    /// across the table ever burns a card, so none of them carry a sacrifice.
    fn opposing_row(&self) -> Vec<Placed> {
        self.opposing
            .iter()
            .map(|o| Placed::plain(o.card.clone()))
            .collect()
    }

    /// The printed Opposing Cards, for the player's Flops to read across at.
    fn opposing_cards(&self) -> Vec<Option<Card>> {
        self.opposing.iter().map(|o| Some(o.card.clone())).collect()
    }

    /// The Opposing Cards the player can see. A Flop across a face-down card
    /// reads nothing across, its own print, until the rows turn over.
    fn opposing_face_up_cards(&self) -> Vec<Option<Card>> {
        self.opposing
            .iter()
            .map(|o| o.face_up.then(|| o.card.clone()))
            .collect()
    }

    /// The printed cards in the player's row, for the enemy's Flops.
    fn row_cards(&self) -> Vec<Option<Card>> {
        self.row.iter().map(|p| Some(p.card.clone())).collect()
    }

    /// Lay the enemy's row down for the turn ahead.
    fn deal_opposing(&mut self) {
        self.opposing.clear();
        match self.fixed.clone() {
            Some(row) => {
                self.opposing.extend(
                    row.into_iter()
                        .map(|(card, face_up)| Opposing { card, face_up }),
                );
            }
            None => {
                let deal = self.enemy.deal;
                for slot in 0..usize::from(self.row_size) {
                    let card = deal.card(&mut self.rng);
                    let hidden = self.next_rng() % 100 < u64::from(deal.hidden_pct);
                    self.opposing.push(Opposing {
                        card,
                        // The first Opposing Card is always face up: the
                        // player is owed one thing to read the rest against.
                        face_up: slot == 0 || !hidden,
                    });
                }
            }
        }
        // The House deals its last card blank and face down. It isn't decided
        // until the showdown, and the screen says so rather than lying about
        // a zero (#5).
        if self.hole_card.is_some()
            && let Some(last) = self.opposing.last_mut()
        {
            last.card = Card {
                name: "The House's Hole Card",
                face_value: 0,
                tell: None,
            };
            last.face_up = false;
        }
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

    /// Fisher-Yates over the run's xorshift64; the same stream the coin is
    /// flipped from and the enemy deals off.
    fn shuffle_deck(&mut self) {
        for i in (1..self.deck.len()).rev() {
            let j = (self.next_rng() % (i as u64 + 1)) as usize;
            self.deck.swap(i, j);
        }
    }

    fn next_rng(&mut self) -> u64 {
        xorshift64(&mut self.rng)
    }
}

/// What a resolved row is worth.
fn row_value(row: &[Played]) -> u32 {
    row.iter().map(|p| p.value).sum()
}

/// Coins with no suspense in them, so a test can say what a Push does
/// without saying what the odds are.
#[cfg(test)]
impl Coin {
    pub const SURE_THING: Coin = Coin {
        player_pct: 100,
        best_of: 1,
    };
    pub const RIGGED: Coin = Coin {
        player_pct: 0,
        best_of: 1,
    };
}

/// Conveniences for tests that don't care about the prompt.
#[cfg(test)]
impl Duel {
    pub fn set_coin(&mut self, coin: Coin) {
        self.coin = coin;
    }

    /// Confirm the row and Hold.
    pub fn end_turn(&mut self) -> TurnResult {
        match self.confirm() {
            Some(result) => result,
            None => self
                .hold()
                .expect("a Hand that beats the row puts the prompt up"),
        }
    }

    /// Lay a known row down across the table, all of it face up, so a test
    /// can say what the player is up against without rolling for it. Fixed,
    /// so the same row comes back every turn.
    pub fn facing(mut self, cards: Vec<Card>) -> Self {
        self.lay_out(cards);
        self
    }

    /// [`Duel::facing`] for a duel that is already running, which is how the
    /// Bevy-side tests get a known Edge across the table.
    pub fn lay_out(&mut self, cards: Vec<Card>) {
        self.fixed = Some(cards.into_iter().map(|c| (c, true)).collect());
        self.deal_opposing();
    }

    /// Turn one Opposing Card back over, for the tests that are about what
    /// the player is and isn't allowed to see.
    pub fn hide_opposing(&mut self, slot: usize) {
        if let Some(opposing) = self.opposing.get_mut(slot) {
            opposing.face_up = false;
        }
    }
}

#[cfg(test)]
pub(super) mod cards {
    use crate::run::{Card, Deal, Enemy, HoleCard, RisingBlinds, Tell};

    pub fn card(face_value: u32) -> Card {
        Card {
            name: "card",
            face_value,
            tell: None,
        }
    }
    pub fn streak(face_value: u32) -> Card {
        Card {
            name: "streak",
            face_value,
            tell: Some(Tell::Streak),
        }
    }
    pub fn all_in(face_value: u32) -> Card {
        Card {
            name: "all in",
            face_value,
            tell: Some(Tell::AllIn),
        }
    }
    pub fn copycat(face_value: u32) -> Card {
        Card {
            name: "copycat",
            face_value,
            tell: Some(Tell::Copycat),
        }
    }
    pub fn flop(face_value: u32) -> Card {
        Card {
            name: "flop",
            face_value,
            tell: Some(Tell::Flop),
        }
    }

    /// An enemy that deals a row of `row` cards, every one of them a 6 with
    /// no Tell and face up, so a test that isn't about the deal can ignore
    /// it. Most tests lay their own row down with `Duel::facing`.
    pub fn enemy(chips: u32, row: u8) -> Enemy {
        Enemy {
            name: "shill",
            chips,
            deal: Deal {
                row,
                low: 6,
                high: 6,
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

    /// The House: it keeps its last card back and the Blinds raise its
    /// margin. Its own dealt cards are 1s, so the Hole Card carries the row.
    pub fn the_house(chips: u32, margin: u32) -> Enemy {
        Enemy {
            hole_card: Some(HoleCard { margin, step: 2 }),
            deal: Deal {
                low: 1,
                high: 1,
                ..enemy(chips, 5).deal
            },
            ..enemy(chips, 5)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::cards::*;
    use super::*;

    /// A deck of vanilla cards 1..=n, so card n is drawn first.
    fn vanilla_deck(n: u32) -> Vec<Card> {
        (1..=n).map(card).collect()
    }

    /// A duel facing a row of `n` sixes: the plain case, worth 6n.
    fn duel_facing_sixes(n: u8) -> Duel {
        Duel::new(vanilla_deck(18), 40, 5, enemy(999, n))
    }

    #[test]
    fn a_new_duel_deals_seven_cards_into_the_draw() {
        let duel = duel_facing_sixes(3);

        assert_eq!(duel.draw().len(), 7);
        let face_values: Vec<u32> = duel.draw().iter().map(|c| c.face_value).collect();
        assert_eq!(face_values, vec![18, 17, 16, 15, 14, 13, 12]);
    }

    #[test]
    fn the_enemy_lays_its_row_down_before_the_player_plays_a_card() {
        let duel = duel_facing_sixes(4);

        assert_eq!(duel.opposing().len(), 4);
        assert_eq!(duel.slots(), 4);
        assert!(duel.row().is_empty());
        assert_eq!(duel.house_edge(), 24);
    }

    #[test]
    fn the_first_opposing_card_is_always_face_up() {
        // Every card after the first is dealt face down, so only the rule
        // that the first one isn't can put a card face up.
        let mut hidden = enemy(999, 5);
        hidden.deal.hidden_pct = 100;

        for seed in 1..40u64 {
            let duel = Duel::new(vanilla_deck(18), 40, 5, hidden.clone()).with_seed(seed * 2 + 1);
            let face_up: Vec<bool> = duel.opposing().iter().map(|o| o.face_up).collect();
            assert_eq!(face_up, vec![true, false, false, false, false]);
        }
    }

    #[test]
    fn a_middling_deal_turns_some_cards_over_and_leaves_others_down() {
        let mut coin_toss = enemy(999, 6);
        coin_toss.deal.hidden_pct = 50;
        let mut seen = (false, false);

        for seed in 1..60u64 {
            let duel =
                Duel::new(vanilla_deck(18), 40, 5, coin_toss.clone()).with_seed(seed * 2 + 1);
            // Past the first card, which is never a toss.
            for opposing in &duel.opposing()[1..] {
                if opposing.face_up {
                    seen.0 = true;
                } else {
                    seen.1 = true;
                }
            }
        }

        assert_eq!(seen, (true, true), "the deal lands on both faces");
    }

    #[test]
    fn placing_a_card_puts_it_in_the_next_empty_slot() {
        let mut duel = duel_facing_sixes(3);

        assert_eq!(duel.place(0, None), Ok(0));
        assert_eq!(duel.place(0, None), Ok(1));

        let row: Vec<u32> = duel.row().iter().map(|p| p.card.face_value).collect();
        assert_eq!(row, vec![18, 17]);
        assert_eq!(duel.hand(), 35);
        assert_eq!(duel.plays_left(), 1);
        assert_eq!(duel.draw().len(), 5);
    }

    #[test]
    fn the_row_is_as_long_as_the_enemys_and_no_longer() {
        let mut duel = duel_facing_sixes(2);
        assert_eq!(duel.plays_left(), 2, "two Opposing Cards, two slots");

        duel.place(0, None).unwrap();
        duel.place(0, None).unwrap();

        assert_eq!(duel.plays_left(), 0);
        assert_eq!(duel.place(0, None), Err(PlayError::RowIsFull));
    }

    #[test]
    fn a_row_longer_than_your_plays_leaves_slots_you_cannot_cover() {
        // Seven Opposing Cards against five Plays: two slots go uncovered,
        // and what sits in them is free chips for the enemy.
        let mut duel = Duel::new(vanilla_deck(18), 40, 5, enemy(999, 7));
        assert_eq!(duel.slots(), 7);
        assert_eq!(duel.plays_left(), 5);

        for _ in 0..5 {
            duel.place(0, None).unwrap();
        }

        assert_eq!(duel.plays_left(), 0);
        assert_eq!(duel.row().len(), 5);
        assert_eq!(duel.house_edge(), 42, "all seven still count for them");
    }

    #[test]
    fn lifting_a_card_hands_it_back_and_closes_the_gap() {
        let mut duel = duel_facing_sixes(4);
        duel.place(0, None).unwrap(); // 18
        duel.place(0, None).unwrap(); // 17
        duel.place(0, None).unwrap(); // 16
        assert_eq!(duel.hand(), 51);

        let lifted = duel.lift(1).expect("a card in slot 1");

        assert_eq!(lifted.face_value, 17);
        let row: Vec<u32> = duel.row().iter().map(|p| p.card.face_value).collect();
        assert_eq!(row, vec![18, 16], "the 16 slid left into slot 1");
        assert_eq!(duel.hand(), 34);
        assert_eq!(duel.plays_left(), 2);
        assert!(
            duel.draw().iter().any(|c| c.face_value == 17),
            "back in the Draw"
        );
    }

    #[test]
    fn lifting_an_all_in_hands_back_what_it_burned_too() {
        // Drawn first: all_in(2), card(8), ...
        let deck = vec![
            card(1),
            card(1),
            card(1),
            card(1),
            card(1),
            card(8),
            all_in(2),
        ];
        let mut duel = Duel::new(deck, 40, 5, enemy(999, 3));
        duel.place(0, Some(0)).unwrap_err(); // can't burn itself
        duel.place(0, Some(1)).unwrap(); // All In 2 burning the 8
        assert_eq!(duel.hand(), 10);
        assert_eq!(duel.draw().len(), 5);

        duel.lift(0).unwrap();

        assert_eq!(duel.hand(), 0);
        assert_eq!(duel.draw().len(), 7, "both cards came back");
        assert!(duel.draw().iter().any(|c| c.face_value == 8));
    }

    #[test]
    fn lifting_a_slot_nobody_filled_is_refused() {
        let mut duel = duel_facing_sixes(3);

        assert_eq!(duel.lift(0), Err(PlayError::NoSuchCard));
    }

    #[test]
    fn illegal_placements_are_refused_and_change_nothing() {
        let deck = vec![
            card(1),
            card(1),
            card(1),
            card(1),
            card(3),
            card(8),
            all_in(2),
        ];
        let mut duel = Duel::new(deck, 40, 5, enemy(999, 5));

        assert_eq!(duel.place(9, None), Err(PlayError::NoSuchCard));
        assert_eq!(duel.place(0, None), Err(PlayError::AllInNeedsSacrifice));
        assert_eq!(duel.place(0, Some(0)), Err(PlayError::NoSuchCard));
        assert_eq!(duel.place(0, Some(9)), Err(PlayError::NoSuchCard));
        assert_eq!(duel.place(1, Some(2)), Err(PlayError::NotAllIn));
        assert_eq!(duel.hand(), 0);
        assert_eq!(duel.draw().len(), 7);
    }

    #[test]
    fn the_higher_row_takes_the_difference_off_the_other_sides_chips() {
        // Three 6s across: 18. The player covers them with 18, 17 and 16.
        let mut duel = Duel::new(vanilla_deck(18), 40, 5, enemy(100, 3));
        for _ in 0..3 {
            duel.place(0, None).unwrap();
        }

        let result = duel.end_turn();

        assert_eq!(
            result,
            TurnResult {
                hand: 51,
                house_edge: 18,
                kind: Outcome::Payout(33),
                pyl: None,
                blinds_rose: false,
            }
        );
        assert_eq!(duel.enemy_chips(), 67);
        assert_eq!(duel.player_chips(), 40);
    }

    #[test]
    fn falling_short_takes_the_difference_out_of_your_own_chips() {
        // Five 6s across: 30, against one card of 18.
        let mut duel = Duel::new(vanilla_deck(18), 40, 5, enemy(100, 5));
        duel.place(0, None).unwrap();

        let result = duel.end_turn();

        assert_eq!(result.kind, Outcome::Whiff(12));
        assert_eq!(duel.player_chips(), 28);
        assert_eq!(duel.enemy_chips(), 100);
    }

    #[test]
    fn two_rows_that_tie_pay_nobody() {
        // Three 6s across: 18. Cover it with exactly 18.
        let deck = vec![
            card(1),
            card(1),
            card(1),
            card(1),
            card(4),
            card(6),
            card(8),
        ];
        let mut duel = Duel::new(deck, 40, 5, enemy(100, 3));
        for _ in 0..3 {
            duel.place(0, None).unwrap();
        }
        assert_eq!(duel.hand(), 18);

        let result = duel.end_turn();

        assert_eq!(result.kind, Outcome::Payout(0));
        assert_eq!(duel.player_chips(), 40);
        assert_eq!(duel.enemy_chips(), 100);
        assert_eq!(
            duel.phase(),
            Phase::Playing,
            "a tie is never offered the coin"
        );
    }

    #[test]
    fn confirming_clears_both_rows_and_deals_the_next_ones() {
        let mut duel = Duel::new(vanilla_deck(18), 40, 5, enemy(999, 3));
        for _ in 0..3 {
            duel.place(0, None).unwrap();
        }
        duel.end_turn();

        assert!(duel.row().is_empty());
        assert_eq!(duel.opposing().len(), 3, "a fresh row across the table");
        assert_eq!(duel.plays_left(), 3);
        assert_eq!(duel.draw().len(), 7);
        // The four unplayed cards carried over, then three fresh ones.
        let face_values: Vec<u32> = duel.draw().iter().map(|c| c.face_value).collect();
        assert_eq!(face_values, vec![15, 14, 13, 12, 11, 10, 9]);
    }

    #[test]
    fn the_showdown_is_kept_for_the_screen_after_the_turn_is_over() {
        let mut duel = Duel::new(vanilla_deck(18), 40, 5, enemy(999, 2));
        duel.place(0, None).unwrap();
        duel.place(0, None).unwrap();

        assert!(duel.last_showdown().is_none());
        duel.end_turn();

        let showdown = duel
            .last_showdown()
            .expect("the rows that just turned over");
        assert_eq!(showdown.hand, 35);
        assert_eq!(showdown.house_edge, 12);
        assert_eq!(showdown.row.len(), 2);
        assert_eq!(showdown.opposing.len(), 2);
    }

    #[test]
    fn rising_blinds_lay_another_opposing_card_down_every_two_turns() {
        let mut duel = Duel::new(vanilla_deck(40), 999, 5, enemy(9999, 3));

        assert_eq!(duel.slots(), 3);
        let t1 = duel.end_turn();
        assert!(!t1.blinds_rose);
        assert_eq!(duel.slots(), 3);
        let t2 = duel.end_turn();
        assert!(t2.blinds_rose);
        assert_eq!(duel.slots(), 4, "the enemy lays one more");
        duel.end_turn();
        duel.end_turn();
        assert_eq!(duel.slots(), 5);
    }

    #[test]
    fn the_duel_is_won_when_the_enemy_chips_hit_zero() {
        let mut duel = Duel::new(vanilla_deck(18), 40, 5, enemy(30, 3));
        for _ in 0..3 {
            duel.place(0, None).unwrap();
        }

        duel.end_turn(); // 51 against 18: a Payout of 33

        assert_eq!(duel.enemy_chips(), 0);
        assert_eq!(duel.outcome(), Some(CombatOutcome::Won));
    }

    #[test]
    fn the_duel_is_lost_when_your_chips_hit_zero() {
        // Five 6s across and nothing covering them: a Whiff of 30.
        let mut duel = Duel::new(vanilla_deck(18), 10, 5, enemy(50, 5));

        duel.end_turn();

        assert_eq!(duel.player_chips(), 0);
        assert_eq!(duel.outcome(), Some(CombatOutcome::Lost));
    }

    #[test]
    fn the_discard_is_reshuffled_into_the_deck_when_it_runs_dry() {
        // 9 cards: 7 dealt, 2 in the deck. After a turn of 5 cards there are
        // 2 left in the Draw, 2 come from the deck, and 3 must come back
        // from the discard.
        let mut duel = Duel::new(vanilla_deck(9), 40, 5, enemy(999, 5));
        for _ in 0..5 {
            duel.place(0, None).unwrap();
        }
        duel.end_turn();

        assert_eq!(duel.draw().len(), 7);
        let mut face_values: Vec<u32> = duel.draw().iter().map(|c| c.face_value).collect();
        face_values.sort_unstable();
        assert!(face_values.contains(&4) && face_values.contains(&3));
        assert_eq!(face_values.iter().filter(|&&v| v >= 5).count(), 3);
    }
}

#[cfg(test)]
mod tell_tests {
    use super::cards::*;
    use super::*;

    /// What the player's row resolved to, slot by slot.
    fn resolved(duel: &Duel) -> Vec<u32> {
        resolve_row(&duel.row, &duel.opposing_cards())
            .iter()
            .map(|p| p.value)
            .collect()
    }

    /// A duel holding exactly `draw` in the Draw, facing exactly `across`.
    fn table(draw: Vec<Card>, across: Vec<Card>) -> Duel {
        let plays = draw.len() as u8;
        // `Duel` draws off the end, so the first card of `draw` goes last.
        let deck: Vec<Card> = draw.into_iter().rev().collect();
        Duel::new(deck, 40, plays, enemy(999, 1)).facing(across)
    }

    #[test]
    fn streak_doubles_when_the_card_in_the_slot_to_its_left_has_a_tell() {
        let mut duel = table(
            vec![all_in(2), streak(5), card(3)],
            vec![card(1), card(1), card(1)],
        );

        duel.place(0, Some(2)).unwrap(); // All In 2 burning card(3): 5
        duel.place(0, None).unwrap(); // Streak 5, after a Tell

        assert_eq!(resolved(&duel), vec![5, 10]);
        assert_eq!(duel.hand(), 15);
    }

    #[test]
    fn streak_does_not_double_after_a_plain_card_or_in_the_first_slot() {
        let mut duel = table(
            vec![streak(5), card(3), streak(4)],
            vec![card(1), card(1), card(1)],
        );

        for _ in 0..3 {
            duel.place(0, None).unwrap();
        }

        assert_eq!(resolved(&duel), vec![5, 3, 4]);
        assert_eq!(duel.hand(), 12);
    }

    #[test]
    fn all_in_burns_the_sacrifice_and_adds_its_face_value_wherever_it_sits() {
        let mut duel = table(
            vec![all_in(2), card(8), card(3)],
            vec![card(1), card(1), card(1)],
        );

        duel.place(0, Some(1)).unwrap(); // burn the 8

        assert_eq!(resolved(&duel), vec![10]);
        // Both the All In and what it burned have left the Draw.
        assert_eq!(duel.draw().len(), 1);
    }

    #[test]
    fn copycat_takes_the_face_value_of_the_slot_to_its_right() {
        let mut duel = table(
            vec![copycat(3), card(6), card(2)],
            vec![card(1), card(1), card(1)],
        );

        duel.place(0, None).unwrap();
        // On its own at the end of the row, a Copycat is worth its own print.
        assert_eq!(resolved(&duel), vec![3]);

        duel.place(0, None).unwrap();
        assert_eq!(resolved(&duel), vec![6, 6], "the 6 filled it in");
        assert_eq!(duel.hand(), 12);
    }

    #[test]
    fn copycat_takes_the_print_and_not_what_the_card_resolved_to() {
        let mut duel = table(
            vec![copycat(3), streak(5), card(4)],
            vec![card(1), card(1), card(1)],
        );

        for _ in 0..3 {
            duel.place(0, None).unwrap();
        }

        // The Streak doubled off the Copycat's Tell; the Copycat still only
        // took the 5 that is printed on it.
        assert_eq!(resolved(&duel), vec![5, 10, 4]);
    }

    #[test]
    fn flop_takes_the_face_value_of_the_opposing_card_across_from_it() {
        let mut duel = table(
            vec![flop(2), flop(2), flop(2)],
            vec![card(9), card(3), card(7)],
        );

        for _ in 0..3 {
            duel.place(0, None).unwrap();
        }

        assert_eq!(resolved(&duel), vec![9, 3, 7], "each one ties its own slot");
        assert_eq!(duel.hand(), duel.house_edge());
    }

    #[test]
    fn a_flop_moves_with_its_slot_when_the_row_is_rearranged() {
        let mut duel = table(vec![card(4), flop(2)], vec![card(9), card(3)]);

        duel.place(0, None).unwrap(); // the 4, in slot 0
        duel.place(0, None).unwrap(); // the Flop, in slot 1, across the 3
        assert_eq!(resolved(&duel), vec![4, 3]);

        // Lift the 4 and the Flop slides left, across the 9 instead.
        duel.lift(0).unwrap();
        assert_eq!(resolved(&duel), vec![9]);
    }

    #[test]
    fn a_flop_with_nothing_across_it_is_worth_its_own_print() {
        // Two slots, but the player has three Plays and only two can land;
        // the third slot doesn't exist, so a Flop there would read nothing.
        let mut duel = table(vec![flop(7)], vec![]);
        // No Opposing Cards at all: no slots to fill.
        assert_eq!(duel.slots(), 0);
        assert_eq!(duel.place(0, None), Err(PlayError::RowIsFull));

        // The same card against a row it does reach.
        let mut duel = table(vec![flop(7), card(1)], vec![card(2), card(2)]);
        duel.place(0, None).unwrap();
        assert_eq!(resolved(&duel), vec![2]);
    }

    #[test]
    fn the_enemys_tells_resolve_by_the_same_rules() {
        // A Streak in slot 1 doubling off the Copycat in slot 0, and a Flop
        // in slot 2 reading the player's card across from it.
        let mut duel = table(
            vec![card(4), card(5), card(9)],
            vec![copycat(3), streak(6), flop(1)],
        );

        for _ in 0..3 {
            duel.place(0, None).unwrap();
        }

        // Copycat takes the Streak's print (6), the Streak doubles after a
        // Tell (12), the Flop takes the 9 across from it.
        assert_eq!(duel.house_edge(), 6 + 12 + 9);
    }

    #[test]
    fn a_flop_on_each_side_of_the_table_reads_the_print_and_not_the_other_flop() {
        // Both rows are Flops facing each other. Face Values throughout,
        // so each takes the other's print and neither chases the other.
        let mut duel = table(vec![flop(4)], vec![flop(6)]);

        duel.place(0, None).unwrap();

        assert_eq!(duel.hand(), 6, "yours took their print");
        assert_eq!(duel.house_edge(), 4, "theirs took yours");
    }

    #[test]
    fn an_all_in_across_the_table_is_worth_its_print_and_nothing_else() {
        // No enemy deals itself an All In, but nothing in the model breaks if
        // a reward or a future enemy ever puts one there.
        let mut duel = table(vec![card(4)], vec![all_in(5)]);
        duel.place(0, None).unwrap();

        assert_eq!(duel.house_edge(), 5);
    }
}

#[cfg(test)]
mod reveal_tests {
    use super::cards::*;
    use super::*;

    /// A duel facing `across`, with the slots named in `hidden` face down.
    fn facing(across: Vec<Card>, hidden: &[usize]) -> Duel {
        let row: Vec<(Card, bool)> = across
            .into_iter()
            .enumerate()
            .map(|(i, card)| (card, !hidden.contains(&i)))
            .collect();
        Duel::new(vec![card(4); 18], 40, 5, enemy(999, 3)).with_opposing(row)
    }

    #[test]
    fn the_player_only_adds_up_what_is_face_up() {
        let duel = facing(vec![card(7), card(5), card(6)], &[1]);

        assert_eq!(duel.showing(), (13, 1), "the 5 is face down");
        assert_eq!(duel.house_edge(), 18, "the model knows the whole of it");
    }

    #[test]
    fn a_face_up_streak_beside_a_face_down_card_never_says_what_is_under_it() {
        // The hidden card has a Tell, so the Streak really will double. The
        // player has no way to know that until the showdown.
        let duel = facing(vec![copycat(4), streak(5)], &[0]);

        assert_eq!(duel.showing(), (5, 1), "the Streak counts as its print");
        assert_eq!(duel.house_edge(), 5 + 10);
    }

    #[test]
    fn a_flop_across_a_face_down_card_reads_its_own_print_until_the_rows_turn_over() {
        let mut deck = vec![card(1); 17];
        deck.push(flop(3)); // drawn first
        let mut duel = Duel::new(deck, 40, 5, enemy(999, 3)).with_opposing(vec![
            (card(9), false),
            (card(2), true),
            (card(2), true),
        ]);

        duel.place(0, None).unwrap();
        assert_eq!(duel.hand(), 3, "nothing it can see across, so its own 3");

        duel.confirm();
        assert_eq!(duel.hand(), 9, "turned over, the 9 was across");
    }

    #[test]
    fn a_face_up_copycat_reading_a_face_down_card_counts_as_its_print() {
        let duel = facing(vec![copycat(4), card(9)], &[1]);

        assert_eq!(duel.showing(), (4, 1));
        assert_eq!(duel.house_edge(), 9 + 9);
    }

    #[test]
    fn a_face_up_flop_reads_your_own_card_so_it_hides_nothing() {
        let mut duel = facing(vec![flop(2), card(3)], &[]);
        assert_eq!(duel.showing(), (2 + 3, 0), "nothing across it yet");

        duel.place(0, None).unwrap(); // a 4 in slot 0

        assert_eq!(duel.showing(), (4 + 3, 0));
        assert_eq!(duel.house_edge(), 7);
    }

    #[test]
    fn everything_turns_over_when_the_row_is_confirmed() {
        // A row worth 4, so two of the Draw's 4s beat it and the rows stay
        // on the table with the coin on offer rather than resolving away.
        let mut duel = facing(vec![card(2), card(1), card(1)], &[1, 2]);
        assert_eq!(duel.showing(), (2, 2));

        duel.place(0, None).unwrap();
        duel.place(0, None).unwrap();

        duel.confirm();

        assert!(duel.opposing().iter().all(|o| o.face_up));
        assert_eq!(duel.showing(), (4, 0));
        assert_eq!(duel.house_edge(), 4);
    }

    #[test]
    fn the_next_turns_cards_come_down_face_down_again() {
        // The reveal belongs to the turn that was confirmed, not to the duel.
        let mut duel = facing(vec![card(7), card(5), card(6)], &[1, 2]);

        duel.place(0, None).unwrap();
        duel.end_turn(); // a Whiff: it resolves and deals the next row

        assert_eq!(duel.showing().1, 2, "two face down again");
    }
}

#[cfg(test)]
mod push_your_luck_tests {
    use super::cards::*;
    use super::*;
    use crate::run::Perk;

    /// One card of `hand` against a row that adds up to `edge`.
    fn hand_of(hand: u32, edge: u32) -> Duel {
        let mut deck = vec![card(1); 6];
        deck.push(card(hand));
        let mut duel = Duel::new(deck, 40, 5, enemy(999, 1))
            .facing(vec![card(edge)])
            .with_coin(Coin::SURE_THING);
        duel.place(0, None).unwrap();
        duel
    }

    #[test]
    fn beating_the_opposing_cards_offers_push_your_luck_instead_of_resolving() {
        let mut duel = hand_of(30, 20);

        duel.confirm();

        assert_eq!(duel.phase(), Phase::PushYourLuck);
        assert_eq!(duel.enemy_chips(), 999, "nothing dealt yet");
        assert_eq!(duel.hand(), 30);
    }

    #[test]
    fn a_whiff_is_offered_the_flip_too() {
        let mut duel = hand_of(10, 20);

        duel.confirm();

        assert_eq!(duel.phase(), Phase::PushYourLuck);
        // Nothing has been dealt yet.
        assert_eq!(duel.player_chips(), 40);
        assert_eq!(duel.hand(), 10);
    }

    #[test]
    fn the_prompt_can_quote_what_a_whiff_would_cost() {
        let mut duel = hand_of(10, 20);
        duel.confirm();

        assert_eq!(duel.whiff(None), 10);
        assert_eq!(duel.whiff(Some(Push::Won)), 0);
        assert_eq!(duel.whiff(Some(Push::Lost)), 20);
        assert_eq!(duel.payout(None), 0);
    }

    #[test]
    fn holding_a_whiff_takes_it_as_normal() {
        let mut duel = hand_of(10, 20);
        duel.confirm();

        let result = duel.hold().expect("the prompt is up");

        assert_eq!(result.kind, Outcome::Whiff(10));
        assert_eq!(result.pyl, None);
        assert_eq!(duel.phase(), Phase::Playing);
        assert_eq!(duel.player_chips(), 30);
    }

    #[test]
    fn pushing_a_whiff_and_winning_forgives_it() {
        let mut duel = hand_of(10, 20);
        duel.confirm();

        let result: TurnResult = duel.push().expect("the prompt is up");

        assert_eq!(result.pyl, Some(Push::Won));
        assert_eq!(result.hand, 10);
        assert_eq!(result.kind, Outcome::Whiff(0));
        assert_eq!(duel.player_chips(), 40);
        assert_eq!(duel.enemy_chips(), 999);
        assert_eq!(duel.phase(), Phase::Playing);
    }

    #[test]
    fn pushing_a_whiff_and_losing_doubles_it() {
        let mut duel = hand_of(10, 20);
        duel.set_coin(Coin::RIGGED);
        duel.confirm();

        let result = duel.push().expect("the prompt is up");

        assert_eq!(result.pyl, Some(Push::Lost));
        assert_eq!(result.hand, 10);
        assert_eq!(result.kind, Outcome::Whiff(20));
        assert_eq!(duel.player_chips(), 20);
    }

    #[test]
    fn a_doubled_whiff_can_end_the_duel() {
        let mut deck = vec![card(1); 6];
        deck.push(card(10));
        let mut duel = Duel::new(deck, 15, 5, enemy(999, 20)).with_coin(Coin::RIGGED);
        duel.place(0, None).unwrap();
        duel.confirm();

        duel.push().expect("the prompt is up");

        assert_eq!(duel.player_chips(), 0);
        assert_eq!(duel.outcome(), Some(CombatOutcome::Lost));
    }

    #[test]
    fn holding_resolves_the_turn_as_normal() {
        let mut duel = hand_of(30, 20);
        duel.confirm();

        let result = duel.hold().expect("the prompt is up");

        assert_eq!(
            result,
            TurnResult {
                hand: 30,
                house_edge: 20,
                kind: Outcome::Payout(10),
                pyl: None,
                blinds_rose: false,
            }
        );
        assert_eq!(duel.enemy_chips(), 989);
        assert_eq!(duel.phase(), Phase::Playing);
    }

    #[test]
    fn pushing_and_winning_doubles_the_payout_and_not_the_hand() {
        let mut duel = hand_of(30, 20);
        duel.confirm();

        let result = duel.push().expect("the prompt is up");

        assert_eq!(result.pyl, Some(Push::Won));
        assert_eq!(result.hand, 30);
        assert_eq!(result.kind, Outcome::Payout(20));
        assert_eq!(duel.enemy_chips(), 979);
    }

    #[test]
    fn pushing_and_losing_zeroes_the_hand_into_a_full_whiff() {
        let mut duel = hand_of(30, 20);
        duel.set_coin(Coin::RIGGED);
        duel.confirm();

        let result = duel.push().expect("the prompt is up");

        assert_eq!(result.pyl, Some(Push::Lost));
        assert_eq!(result.hand, 30); // as shown; the flip is what zeroed it
        assert_eq!(result.kind, Outcome::Whiff(20));
        assert_eq!(duel.player_chips(), 20);
        assert_eq!(duel.enemy_chips(), 999);
    }

    #[test]
    fn the_row_is_final_once_the_prompt_is_up() {
        let mut duel = hand_of(30, 20);
        duel.confirm();

        assert_eq!(duel.place(0, None), Err(PlayError::HandIsFinal));
        assert_eq!(duel.lift(0), Err(PlayError::HandIsFinal));
        assert_eq!(duel.hand(), 30);
    }

    #[test]
    fn push_and_hold_do_nothing_when_no_prompt_is_up() {
        let mut duel = hand_of(30, 20);

        assert_eq!(duel.push(), None);
        assert_eq!(duel.hold(), None);
        assert_eq!(duel.enemy_chips(), 999);
    }

    #[test]
    fn the_base_coin_is_one_flip_the_player_takes_45_times_in_100() {
        assert_eq!(
            Coin::BASE,
            Coin {
                player_pct: 45,
                best_of: 1
            }
        );
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
        assert_eq!(
            Coin::SLOTZ,
            Coin {
                player_pct: 49,
                best_of: 3
            }
        );
        assert_eq!(Coin::SLOTZ.resolve([10, 90, 10].into_iter()), Push::Won);
        assert_eq!(Coin::SLOTZ.resolve([90, 10, 10].into_iter()), Push::Won);
        assert_eq!(Coin::SLOTZ.resolve([90, 10, 90].into_iter()), Push::Lost);
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
        assert_eq!(wins_over_every_roll(Coin::SLOTZ), 485_002);
        assert!(
            wins_over_every_roll(Coin::SLOTZ) > wins_over_every_roll(Coin::BASE),
            "the perk is worth taking"
        );
    }

    #[test]
    fn the_duels_own_coin_lands_on_both_sides() {
        let mut seen = (false, false);
        for seed in 1..200u64 {
            let mut duel = hand_of(30, 20).with_seed(seed * 2 + 1);
            // `with_seed` re-deals the row, and this one is fixed, so the
            // card the test placed is still sitting across a 20.
            duel.set_coin(Coin::BASE);
            duel.confirm();
            match duel.push().unwrap().pyl {
                Some(Push::Won) => seen.0 = true,
                Some(Push::Lost) => seen.1 = true,
                None => panic!("a Push always flips"),
            }
        }
        assert_eq!(seen, (true, true));
    }
}

#[cfg(test)]
mod loaded_dice_tests {
    use super::cards::*;
    use super::*;
    use crate::run::LOADED_DICE_BONUS;

    fn deck() -> Vec<Card> {
        (1..=18).map(card).collect()
    }

    /// A duel facing a single card worth `edge`.
    fn facing(edge: u32) -> Duel {
        Duel::new(deck(), 40, 5, enemy(999, 1)).facing(vec![card(edge)])
    }

    #[test]
    fn a_duel_with_no_dice_adds_nothing() {
        let mut duel = facing(0);

        let turn = duel.end_turn();

        assert_eq!(turn.hand, 0);
        assert_eq!(duel.dice_left(), 0);
    }

    #[test]
    fn the_dice_land_on_the_hand_as_the_rows_turn_over() {
        let mut duel = facing(0).with_loaded_dice(2);
        duel.place(0, None).unwrap(); // 18

        // Nothing on The Hand until the showdown: The House reads what it can
        // see, and the dice land after that (the Hole Card rule).
        assert_eq!(duel.hand(), 18);
        let turn = duel.end_turn();

        assert_eq!(turn.hand, 18 + LOADED_DICE_BONUS);
        assert_eq!(duel.dice_left(), 1);
    }

    #[test]
    fn the_dice_run_out_after_two_hands() {
        let mut duel = facing(0).with_loaded_dice(2);

        let hands: Vec<u32> = (0..3).map(|_| duel.end_turn().hand).collect();

        assert_eq!(hands, vec![LOADED_DICE_BONUS, LOADED_DICE_BONUS, 0]);
        assert_eq!(duel.dice_left(), 0);
    }

    #[test]
    fn the_dice_soften_a_whiff_too() {
        let mut duel = facing(30).with_loaded_dice(1);

        let turn = duel.end_turn();

        assert_eq!(turn.kind, Outcome::Whiff(30 - LOADED_DICE_BONUS));
        assert_eq!(duel.dice_left(), 0);
    }

    #[test]
    fn confirming_twice_only_spends_one_pair() {
        let mut duel = facing(0).with_loaded_dice(2);

        duel.confirm(); // the prompt goes up
        duel.confirm(); // and stays up, spending nothing more

        assert_eq!(duel.hand(), LOADED_DICE_BONUS);
        assert_eq!(duel.dice_left(), 1);
    }
}

#[cfg(test)]
mod the_house_tests {
    use super::cards::*;
    use super::*;

    fn vanilla_deck(n: u32) -> Vec<Card> {
        (1..=n).map(card).collect()
    }

    /// The House, dealing itself 1s in every slot but the one it keeps back.
    fn table(chips: u32, margin: u32) -> Duel {
        Duel::new(vanilla_deck(18), 50, 5, the_house(chips, margin))
    }

    #[test]
    fn the_house_deals_its_last_card_blank_and_face_down() {
        let duel = table(35, 1);

        assert_eq!(duel.slots(), 5);
        let last = duel.opposing().last().expect("a row");
        assert!(!last.face_up);
        assert_eq!(last.card.face_value, 0, "not decided yet");
        // Four 1s showing, and the one it kept back.
        assert_eq!(duel.showing(), (4, 1));
        assert_eq!(duel.margin(), Some(1));
    }

    #[test]
    fn the_house_sets_its_hole_card_to_the_row_it_read_plus_the_margin() {
        let mut duel = table(35, 1);
        // Draw: 18,17,16,15,14. The first four are all The House sees: 66.
        for _ in 0..5 {
            duel.place(0, None).unwrap();
        }

        let result = duel.end_turn();

        assert_eq!(result.house_edge, 67, "the read row, plus the margin");
        assert_eq!(result.hand, 80);
        // The Hole Card was the 14, and it pays its value over the margin.
        assert_eq!(result.kind, Outcome::Payout(13));
        assert_eq!(duel.enemy_chips(), 22);
    }

    #[test]
    fn a_short_row_still_keeps_only_its_last_card_from_the_house() {
        let mut duel = table(35, 1);
        duel.place(0, None).unwrap();
        duel.place(0, None).unwrap(); // 18 and 17, three slots left empty

        let result = duel.end_turn();

        // It read the 18 alone; the 17 was the card it couldn't see.
        assert_eq!(result.house_edge, 19);
        assert_eq!(result.hand, 35);
        assert_eq!(result.kind, Outcome::Payout(16));
    }

    #[test]
    fn a_row_of_nothing_at_all_loses_to_what_the_house_dealt_itself() {
        let mut duel = table(35, 1);

        let result = duel.end_turn();

        // Nothing to read, so the Hole Card is worth nothing and the row is
        // just the four 1s The House dealt itself. It can't un-deal them to
        // come down to the margin.
        assert_eq!(result.house_edge, 4);
        assert_eq!(result.kind, Outcome::Whiff(4));
        assert_eq!(duel.player_chips(), 46);
    }

    #[test]
    fn the_hole_card_is_worth_nothing_rather_than_less_than_nothing() {
        // The House deals itself 9s here, so its own cards are already past
        // anything a one-card row plus the margin could ask for.
        let mut rich = the_house(35, 1);
        rich.deal.low = 9;
        rich.deal.high = 9;
        let mut duel = Duel::new(vanilla_deck(18), 50, 5, rich);

        duel.place(0, None).unwrap(); // an 18, and it is the Hole Card
        let result = duel.end_turn();

        assert_eq!(result.house_edge, 36, "four 9s, and a Hole Card of 0");
        assert_eq!(result.kind, Outcome::Whiff(18));
    }

    #[test]
    fn the_blinds_raise_the_margin_and_not_the_row() {
        let mut duel = Duel::new(vanilla_deck(40), 999, 5, the_house(9999, 1));

        assert_eq!(duel.margin(), Some(1));
        assert_eq!(duel.slots(), 5);
        duel.end_turn(); // turn 1
        duel.end_turn(); // turn 2: the Blinds tick
        assert_eq!(duel.margin(), Some(3));
        assert_eq!(duel.slots(), 5, "the row stayed where it was");
    }

    #[test]
    fn the_loaded_dice_land_after_the_house_has_read_the_row() {
        let mut duel = table(35, 1).with_loaded_dice(1);
        for _ in 0..5 {
            duel.place(0, None).unwrap();
        }

        let result = duel.end_turn();

        // The Edge is the same 67 it would have been without the dice, and
        // the whole +5 falls on the player's side of the comparison.
        assert_eq!(result.house_edge, 67);
        assert_eq!(result.hand, 85);
        assert_eq!(result.kind, Outcome::Payout(18));
    }

    #[test]
    fn a_copycat_hole_card_lands_its_own_print_after_the_lock() {
        // Drawn first: 10,10,10,10, copycat(3).
        let deck = vec![
            card(1),
            card(1),
            copycat(3),
            card(10),
            card(10),
            card(10),
            card(10),
        ];
        let mut duel = Duel::new(deck, 50, 5, the_house(100, 1));
        for _ in 0..5 {
            duel.place(0, None).unwrap();
        }

        let result = duel.end_turn();

        assert_eq!(result.house_edge, 41, "the four 10s, plus the margin");
        assert_eq!(result.hand, 43, "the Copycat at the end took its own 3");
        assert_eq!(result.kind, Outcome::Payout(2));
    }

    #[test]
    fn ordinary_enemies_keep_no_hole_card() {
        let duel = Duel::new(vanilla_deck(18), 40, 5, enemy(30, 3));

        assert_eq!(duel.margin(), None);
        assert_eq!(duel.showing(), (18, 0), "nothing kept back");
    }
}
