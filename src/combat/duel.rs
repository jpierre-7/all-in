//! The pure combat model: one duel, no Bevy. Vocabulary follows `GLOSSARY.md`.
//!
//! Both sides play the same turn (ADR-0003). Each draws up to 7 from its own
//! Deck; the enemy commits up to its Blind face down, by its strategy; the
//! player fills the row across from it out of the Draw, up to their own
//! Blind, and confirms. Both rows turn over, each resolves its Tells by slot,
//! and the side that comes up short loses the difference off its own Chips.
//!
//! Anything that bends the duel (a boss's Table Rule, a Perk, an Item) does
//! it through the hooks in [`crate::modifier`].

use std::fmt;

use crate::item::{Held, Item, Lasts, When};
use crate::modifier::{Modifier, Side};
use crate::run::{Card, CombatOutcome, Encounter, Enemy, RunState, Tell, xorshift64};

pub const DRAW_SIZE: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayError {
    /// The player's row already holds their Blind.
    RowIsFull,
    NoSuchCard,
    /// An All In card was played without naming a card to sacrifice.
    AllInNeedsSacrifice,
    /// A sacrifice was named for a card that isn't All In.
    NotAllIn,
    /// The rows are face up and the Push Your Luck prompt is up.
    HandIsFinal,
}

/// Why an Item couldn't be spent or taken back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemError {
    NoSuchItem,
    /// Every use is gone.
    NoUsesLeft,
    /// One use of each Item a Hand, and an Item lasting the encounter is
    /// already working.
    AlreadySpent,
    /// Spent while building the row, and the rows are down; or spent at the
    /// prompt, and the prompt isn't up.
    NotNow,
    /// Not spent this Hand.
    NotSpent,
    /// It has already shown or drawn something, or the rows are down.
    CannotTakeBack,
    /// Taking it back would leave more cards in the row than the Blind.
    RowTooLong,
}

/// Where one held Item stands this Hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spent {
    /// In the pocket.
    No,
    /// Spent this Hand, and not paid for until Confirm, so it can still be
    /// taken back.
    Pending,
    /// Spent and paid for this Hand: it has shown or drawn something, or
    /// the rows are down.
    Final,
    /// Working for the rest of the encounter.
    Lasting,
}

/// An Item the player brought to the table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pocketed {
    pub item: &'static Item,
    /// Uses left, not counting one still pending.
    pub uses: u8,
    pub spent: Spent,
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

/// The Push Your Luck coin. House-favoured by default, so a Perk that
/// bends it is worth taking.
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
}

/// A card in a row and what it resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Played {
    pub card: Card,
    /// What the card added to its row, after its Tell. Nothing if mucked.
    pub value: u32,
    /// A Lowball across from it took it off the table before any Tell
    /// resolved.
    pub mucked: bool,
}

/// A card put in a row, with whatever it burned to get there. The burned
/// card only reaches the discard when the turn resolves, so lifting the All
/// In back out of the row hands its sacrifice back too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placed {
    pub card: Card,
    /// The All In's sacrifice, waiting out the turn.
    pub sacrifice: Option<Card>,
}

/// One of the enemy's cards, and whether the player can see it yet. Every
/// one is committed face down; only a reveal, or Confirm, turns it over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opposing {
    pub card: Card,
    /// What an All In burned to get here.
    pub sacrifice: Option<Card>,
    pub face_up: bool,
}

/// A Bluff going off at the Showdown: the slot it sat in, whose Chips took
/// the hit, and how many. Never more than that side had left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BluffHit {
    pub slot: usize,
    pub loser: Side,
    pub amount: u32,
}

/// Both rows as they turned over, and what they added up to. Kept so the
/// screen can go on showing the comparison after the turn has resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Showdown {
    /// The player's row, resolved left to right.
    pub row: Vec<Played>,
    /// The Opposing Cards, resolved left to right.
    pub opposing: Vec<Played>,
    /// The Hand, the Items that bend it included. A lost Push zeroes it.
    pub hand: u32,
    /// The House Edge: what the Opposing Cards came to.
    pub house_edge: u32,
    /// The Bluffs that went off, left to right, before the Payout.
    pub bluffs: Vec<BluffHit>,
    /// The turn these rows were played on.
    pub turn: u32,
}

/// What a Flop in the player's next slot would take, as far as the player
/// can tell: the revealed Opposing Cards it would read, and how many it
/// would read that are still face down. An empty slot adds nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlopPeek {
    pub seen: u32,
    pub hidden: usize,
}

/// `+N`, then `+ ?` for each card it can't see yet.
impl fmt::Display for FlopPeek {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "+{}", self.seen)?;
        for _ in 0..self.hidden {
            write!(f, " + ?")?;
        }
        Ok(())
    }
}

/// What a Lowball would do in the player's next slot, as far as the player
/// can tell. Only said when the card across is face up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LowballPeek {
    /// The card across is higher: this Face Value comes off the table.
    Mucks(u32),
    TooHigh,
}

impl fmt::Display for LowballPeek {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LowballPeek::Mucks(n) => write!(f, "Mucks the {n}"),
            LowballPeek::TooHigh => write!(f, "Too high"),
        }
    }
}

/// One card an enemy commits: its place in the Draw, and for an All In the
/// place of the card it burns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pick {
    pub card: usize,
    pub sacrifice: Option<usize>,
}

/// The default strategy (ADR-0003): the row that resolves highest this turn.
///
/// It tries every ordered pick of up to `blind` cards from `draw`, every All
/// In with every sacrifice, and keeps the best; a tie goes to the pick that
/// comes first in Draw order. It can't see the player's row, so a Flop is
/// scored as `player_average` (the average Face Value of the player's whole
/// Deck) for each slot it reads that the player could fill under
/// `player_blind`. That estimate only steers the choice: at the Showdown a
/// Flop still reads the real cards across from it.
pub fn greedy(draw: &[Card], blind: u8, player_average: u32, player_blind: u8) -> Vec<Pick> {
    let imagined: Vec<Option<Card>> = (0..usize::from(player_blind))
        .map(|_| {
            Some(Card {
                name: "an average card",
                face_value: player_average,
                tell: None,
            })
        })
        .collect();
    let mut search = Greedy {
        draw,
        blind: usize::from(blind),
        imagined,
        used: vec![false; draw.len()],
        picks: Vec::new(),
        row: Vec::new(),
        best: (0, Vec::new()),
    };
    search.walk();
    search.best.1
}

/// The depth-first walk behind [`greedy`], in Draw order, so the first row
/// to reach a score is the one that keeps it.
struct Greedy<'a> {
    draw: &'a [Card],
    blind: usize,
    imagined: Vec<Option<Card>>,
    used: Vec<bool>,
    picks: Vec<Pick>,
    row: Vec<Placed>,
    best: (u32, Vec<Pick>),
}

impl Greedy<'_> {
    fn walk(&mut self) {
        // A Lowball that would muck one of the player's imagined cards is
        // worth that card too: it comes off The Hand instead of going on
        // the House Edge, which is the same gap.
        let own: Vec<Option<Card>> = self.row.iter().map(|p| Some(p.card.clone())).collect();
        let mucked: u32 = mucks(&own, &self.imagined)
            .iter()
            .zip(&self.imagined)
            .filter(|(gone, _)| **gone)
            .filter_map(|(_, card)| card.as_ref().map(|c| c.face_value))
            .sum();
        let score = row_value(&resolve_row(&self.row, &self.imagined)) + mucked;
        if score > self.best.0 {
            self.best = (score, self.picks.clone());
        }
        if self.row.len() >= self.blind {
            return;
        }
        for card in 0..self.draw.len() {
            if self.used[card] {
                continue;
            }
            if self.draw[card].tell == Some(Tell::AllIn) {
                for burned in 0..self.draw.len() {
                    if burned != card && !self.used[burned] {
                        self.try_pick(card, Some(burned));
                    }
                }
            } else {
                self.try_pick(card, None);
            }
        }
    }

    fn try_pick(&mut self, card: usize, sacrifice: Option<usize>) {
        self.used[card] = true;
        if let Some(s) = sacrifice {
            self.used[s] = true;
        }
        self.picks.push(Pick { card, sacrifice });
        self.row.push(Placed {
            card: self.draw[card].clone(),
            sacrifice: sacrifice.map(|s| self.draw[s].clone()),
        });

        self.walk();

        self.row.pop();
        self.picks.pop();
        self.used[card] = false;
        if let Some(s) = sacrifice {
            self.used[s] = false;
        }
    }
}

/// What a Flop in `slot` is worth: the Face Values of the card across from it
/// and that card's two neighbours, never its own. A slot with nothing in it,
/// or a card it can't see yet, counts as nothing.
fn flop_value(across: &[Option<Card>], slot: usize) -> u32 {
    (slot.saturating_sub(1)..=slot + 1)
        .filter_map(|i| across.get(i)?.as_ref())
        .map(|card| card.face_value)
        .sum()
}

/// Which of `targets` the Lowballs in `lowballs` muck, slot by slot: a
/// Lowball mucks the card across from it when that card's Face Value is
/// higher than its own. Both rows are read on their print, so two Lowballs
/// facing each other never both muck.
fn mucks(lowballs: &[Option<Card>], targets: &[Option<Card>]) -> Vec<bool> {
    targets
        .iter()
        .enumerate()
        .map(|(slot, target)| {
            let lowball = lowballs.get(slot).and_then(Option::as_ref);
            match (lowball, target) {
                (Some(l), Some(t)) => l.tell == Some(Tell::Lowball) && l.face_value < t.face_value,
                _ => false,
            }
        })
        .collect()
}

/// Resolve a row left to right against the row across from it, and say what
/// each card is worth.
///
/// First the Lowballs on both sides muck, all at once, on printed Face
/// Values. A mucked card is worth nothing, its Tell doesn't fire, and every
/// Tell that reads its slot finds it empty.
///
/// Then every Tell reads by position — Streak one slot left, Copycat one slot
/// right, Flop across and to either side of that — and every one of them
/// reads a *printed* Face Value. That is what keeps this a single pass with no
/// ordering to argue about: no card's value depends on another card's value, so the two rows
/// facing each other can be worked out in either order and a Flop on each
/// side of the table never chases the other one round in a circle.
pub fn resolve_row(row: &[Placed], across: &[Option<Card>]) -> Vec<Played> {
    let own: Vec<Option<Card>> = row.iter().map(|p| Some(p.card.clone())).collect();
    let gone = mucks(across, &own);
    let across: Vec<Option<Card>> = mucks(&own, across)
        .into_iter()
        .zip(across)
        .map(|(mucked, card)| card.clone().filter(|_| !mucked))
        .collect();
    let standing = |slot: usize| row.get(slot).filter(|_| !gone[slot]).map(|p| &p.card);
    row.iter()
        .enumerate()
        .map(|(slot, placed)| {
            let printed = placed.card.face_value;
            let value = match placed.card.tell {
                _ if gone[slot] => 0,
                Some(Tell::Streak)
                    if slot
                        .checked_sub(1)
                        .and_then(standing)
                        .is_some_and(|left| left.tell.is_some()) =>
                {
                    printed * 2
                }
                Some(Tell::AllIn) => {
                    printed + placed.sacrifice.as_ref().map_or(0, |c| c.face_value)
                }
                Some(Tell::Copycat) => standing(slot + 1).map_or(printed, |next| next.face_value),
                Some(Tell::Flop) => flop_value(&across, slot),
                _ => printed,
            };
            Played {
                card: placed.card.clone(),
                value,
                mucked: gone[slot],
            }
        })
        .collect()
}

/// One side's cards: what it draws from, what it holds, and what it has
/// spent. The player and the enemy each have one.
#[derive(Debug, Clone, Default)]
struct Pile {
    /// Drawn off the end.
    deck: Vec<Card>,
    draw: Vec<Card>,
    discard: Vec<Card>,
}

pub struct Duel {
    player: Pile,
    enemy_cards: Pile,
    /// The enemy's row this turn, committed before the player builds theirs.
    opposing: Vec<Opposing>,
    /// The player's row. Slot `i` sits across from Opposing Card `i`.
    row: Vec<Placed>,
    /// The floor's Blind, before anything bends it.
    blind: u8,
    /// The Table Rule, then Perks in the order taken. The Items being spent
    /// follow them; see [`Duel::modifiers`].
    modifiers: Vec<&'static dyn Modifier>,
    /// The average Face Value of the player's whole Deck, which is all the
    /// enemy's strategy knows about the player's row.
    player_average: u32,
    player_chips: u32,
    enemy: Enemy,
    turn: u32,
    phase: Phase,
    coin: Coin,
    /// The Items carried in from the run, in the order taken.
    pocket: Vec<Pocketed>,
    /// The Arcade's fixed row (#40), laid again every turn instead of one the
    /// enemy chose. `None` in a real duel.
    fixed: Option<Vec<(Card, bool)>>,
    /// Set when the rows turn over; taken when the turn resolves.
    showdown: Option<Showdown>,
    /// The last showdown, for the screen to keep showing into the next turn.
    last: Option<Showdown>,
    rng: u64,
}

impl Duel {
    /// `deck` and `enemy.deck` are taken in draw order (last element drawn
    /// first); shuffling is the caller's job so the model stays
    /// deterministic. `blind` is the floor's. The enemy's Table Rule is the
    /// only modifier; see [`Duel::modified`] for more.
    pub fn new(deck: Vec<Card>, player_chips: u32, blind: u8, enemy: Enemy) -> Self {
        Duel::modified(deck, player_chips, blind, enemy, Vec::new())
    }

    /// [`Duel::new`] with `perks` applied after the enemy's Table Rule, in
    /// the order given. The enemy commits its first row here.
    pub fn modified(
        deck: Vec<Card>,
        player_chips: u32,
        blind: u8,
        mut enemy: Enemy,
        perks: Vec<&'static dyn Modifier>,
    ) -> Self {
        let modifiers: Vec<&'static dyn Modifier> =
            enemy.table_rule.into_iter().chain(perks).collect();
        let coin = modifiers.iter().fold(Coin::BASE, |coin, m| m.coin(coin));
        let player_average = average_face_value(&deck);
        let mut duel = Duel {
            player: Pile {
                deck,
                ..Pile::default()
            },
            enemy_cards: Pile {
                deck: std::mem::take(&mut enemy.deck),
                ..Pile::default()
            },
            opposing: Vec::new(),
            row: Vec::new(),
            blind,
            modifiers,
            player_average,
            player_chips,
            enemy,
            turn: 1,
            phase: Phase::Playing,
            coin,
            pocket: Vec::new(),
            fixed: None,
            showdown: None,
            last: None,
            rng: 0x5eed_cafe_f00d_d1ce,
        };
        duel.start_turn();
        duel
    }

    /// A duel against `encounter` with everything the run has picked up: the
    /// deck the rewards built and the enemy's, each shuffled off `seed`; the
    /// floor's Blind; the Perks taken, in order; the Items held. The game and the balance sim both start here, so the sim
    /// plays the duel the player does.
    pub fn for_run(run: &RunState, encounter: Encounter, seed: u64) -> Self {
        let mut enemy = encounter.enemy(run, seed.rotate_left(29));
        enemy.deck = shuffled(enemy.deck, seed.rotate_left(41));
        Duel::modified(
            shuffled(run.deck.clone(), seed),
            run.chips,
            encounter.blind(),
            enemy,
            run.perks.iter().map(|perk| perk.modifier).collect(),
        )
        .with_seed(seed.rotate_left(17))
        .with_items(&run.items)
    }

    /// Seeds the reshuffle of either discard pile and the coin.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng = seed | 1;
        self
    }

    /// The Arcade's fixed Opposing Cards (#40): the same row, the same face
    /// up and face down, every turn. Nothing about a practice hand is chosen.
    pub fn with_opposing(mut self, row: Vec<(Card, bool)>) -> Self {
        self.fixed = Some(row);
        self.take_back_opposing();
        self.lay_opposing();
        self
    }

    /// The coin Push Your Luck is flipped with, whatever the modifiers said.
    pub fn with_coin(mut self, coin: Coin) -> Self {
        self.coin = coin;
        self
    }

    /// The Items carried in from the run. Combat writes back what is left
    /// of them, [`Duel::items_left`], when the duel ends.
    pub fn with_items(mut self, held: &[Held]) -> Self {
        self.pocket = held
            .iter()
            .map(|h| Pocketed {
                item: h.item,
                uses: h.uses,
                spent: Spent::No,
            })
            .collect();
        self
    }

    /// The coin Push Your Luck will flip with, the Items spent at the prompt
    /// included.
    pub fn coin(&self) -> Coin {
        self.spending()
            .fold(self.coin, |coin, item| item.modifier.coin(coin))
    }

    /// Who is across the table.
    pub fn enemy_name(&self) -> &'static str {
        self.enemy.name
    }

    /// The Items held, in the order taken, and where each stands this Hand.
    pub fn items(&self) -> &[Pocketed] {
        &self.pocket
    }

    /// What the run keeps: every Item with a use left. One still working for
    /// the encounter has had its use.
    pub fn items_left(&self) -> Vec<Held> {
        self.pocket
            .iter()
            .filter(|p| p.uses > 0)
            .map(|p| Held {
                item: p.item,
                uses: p.uses,
            })
            .collect()
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// The most cards `side` may put in its row this turn: the floor's
    /// Blind, bent by every modifier in order, and never below 1.
    pub fn blind(&self, side: Side) -> u8 {
        self.modifiers()
            .fold(self.blind, |blind, m| m.blind(side, blind))
            .max(1)
    }

    /// How many cards `side`'s Draw refills to this turn.
    pub fn draw_size(&self, side: Side) -> usize {
        self.modifiers()
            .fold(DRAW_SIZE, |size, m| m.draw(side, size))
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
        &self.player.draw
    }

    /// The player's whole Deck this duel: what is left to draw, the Draw,
    /// the row, and the discard.
    pub fn deck_size(&self) -> usize {
        self.player.deck.len() + self.player.draw.len() + self.player.discard.len()
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

    /// The Hand: the player's row resolved against the Opposing Cards it can
    /// see. Once the rows have turned over this is the settled number, the
    /// Items that bend it and a lost Push included.
    pub fn hand(&self) -> u32 {
        match &self.showdown {
            Some(showdown) => showdown.hand,
            None => {
                let (row, opposing) = self.table();
                row_value(&resolve_row(&row, &self.face_up(&opposing)))
            }
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
            None => {
                let (row, opposing) = self.table();
                row_value(&resolve_row(&opposing, &printed(&row)))
            }
        }
    }

    /// What the player can add up for themselves: the value of the
    /// Opposing Cards that are face up, and how many are still face down.
    ///
    /// A face-up card whose Tell reads a face-down one counts for its Face
    /// Value alone. The row never says more about a hidden card than the fact
    /// of its being hidden already does, so this only ever under-reads.
    pub fn showing(&self) -> (u32, usize) {
        let (row, opposing) = self.table();
        let row = printed(&row);
        let seen = self.face_up(&opposing);
        // The player's Lowballs muck what they can see across from them, and
        // the face-up Lowballs across muck what they can see of the row.
        let gone = mucks(&row, &seen);
        let across: Vec<Option<Card>> = mucks(&seen, &row)
            .into_iter()
            .zip(row)
            .map(|(mucked, card)| card.filter(|_| !mucked))
            .collect();
        let face_up = |slot: usize| {
            seen.get(slot)
                .and_then(Option::as_ref)
                .filter(|_| !gone[slot])
        };
        let mut total = 0;
        let mut hidden = 0;
        for (slot, card) in seen.iter().enumerate() {
            let Some(card) = card else {
                hidden += 1;
                continue;
            };
            if gone[slot] {
                continue;
            }
            let printed = card.face_value;
            total += match card.tell {
                Some(Tell::Streak) => match slot.checked_sub(1).and_then(face_up) {
                    Some(left) if left.tell.is_some() => printed * 2,
                    _ => printed,
                },
                Some(Tell::Copycat) => face_up(slot + 1).map_or(printed, |next| next.face_value),
                Some(Tell::Flop) => flop_value(&across, slot),
                _ => printed,
            };
        }
        (total, hidden)
    }

    /// What a Flop would take if it went into the next empty slot right now,
    /// on the Opposing Cards the player can see. `None` when the row is
    /// full and it can't go in at all.
    pub fn flop_next(&self) -> Option<FlopPeek> {
        if self.plays_left() == 0 {
            return None;
        }
        let slot = self.row.len();
        let read = || (slot.saturating_sub(1)..=slot + 1).filter_map(|i| self.opposing.get(i));
        Some(FlopPeek {
            seen: read()
                .filter(|o| o.face_up)
                .map(|o| o.card.face_value)
                .sum(),
            hidden: read().filter(|o| !o.face_up).count(),
        })
    }

    /// What a Lowball printed `face_value` would do in the next empty slot,
    /// against the Opposing Card across from it. `None` when the row is
    /// full, or when that slot is empty or still face down: nothing about a
    /// hidden card is told.
    pub fn lowball_next(&self, face_value: u32) -> Option<LowballPeek> {
        if self.plays_left() == 0 {
            return None;
        }
        let across = self.opposing.get(self.row.len()).filter(|o| o.face_up)?;
        Some(if face_value < across.card.face_value {
            LowballPeek::Mucks(across.card.face_value)
        } else {
            LowballPeek::TooHigh
        })
    }

    /// The average Face Value of the enemy's whole Deck: all a player knows
    /// about a face-down Opposing Card, the way [`greedy`] knows the player's
    /// Deck. The balance sim reads it to score a Lowball across a card back.
    pub fn enemy_average(&self) -> u32 {
        let pile = &self.enemy_cards;
        let whole: Vec<Card> = pile
            .deck
            .iter()
            .chain(&pile.draw)
            .chain(&pile.discard)
            .chain(self.opposing.iter().map(|o| &o.card))
            .cloned()
            .collect();
        average_face_value(&whole)
    }

    /// How many slots the table has: as many as the larger Blind. A slot
    /// nobody filled is worth nothing to either side.
    pub fn slots(&self) -> usize {
        usize::from(self.blind(Side::Player).max(self.blind(Side::Enemy))).max(self.opposing.len())
    }

    /// How many more cards the player may put in the row this turn.
    pub fn plays_left(&self) -> u8 {
        self.blind(Side::Player)
            .saturating_sub(self.row.len() as u8)
    }

    /// Turn the Opposing Cards in `slots` face up for the rest of this turn.
    /// An empty slot has nothing to turn over.
    pub fn reveal(&mut self, slots: &[usize]) {
        for &slot in slots {
            if let Some(opposing) = self.opposing.get_mut(slot) {
                opposing.face_up = true;
            }
        }
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
        let draw = &mut self.player.draw;
        let played = draw.get(card).ok_or(PlayError::NoSuchCard)?.clone();
        let sacrificed = match (played.tell, sacrifice) {
            (Some(Tell::AllIn), None) => return Err(PlayError::AllInNeedsSacrifice),
            (Some(Tell::AllIn), Some(i)) if i == card => return Err(PlayError::NoSuchCard),
            (Some(Tell::AllIn), Some(i)) => Some(draw.get(i).ok_or(PlayError::NoSuchCard)?.clone()),
            (_, Some(_)) => return Err(PlayError::NotAllIn),
            (_, None) => None,
        };
        remove_both(draw, card, sacrifice);

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
            self.player.draw.push(burned);
        }
        self.player.draw.push(placed.card.clone());
        Ok(placed.card)
    }

    /// Confirm the row. Both sides turn over, each resolves its Tells, the
    /// Bluffs go off, and The Hand meets the House Edge. Anything but a tie
    /// puts the Push Your Luck prompt up and resolves nothing yet (`None`); a
    /// tie, where nobody pays, resolves on the spot, and so does a Bluff that
    /// ends the duel.
    pub fn confirm(&mut self) -> Option<TurnResult> {
        if self.phase == Phase::PushYourLuck {
            return None;
        }
        for opposing in &mut self.opposing {
            opposing.face_up = true;
        }
        let (mine, theirs) = self.table();
        let mut row = resolve_row(&mine, &printed(&theirs));
        let mut opposing = resolve_row(&theirs, &printed(&mine));
        // A card a modifier took off the end of a row is Mucked.
        row.extend(mucked(self.row.iter().skip(mine.len()).map(|p| &p.card)));
        opposing.extend(mucked(
            self.opposing.iter().skip(theirs.len()).map(|o| &o.card),
        ));
        let mut hand = row_value(&row);
        let house_edge = row_value(&opposing);
        // The Bluffs go off between the rows turning over and The Hand
        // meeting the House Edge, and one can end the duel right there.
        let bluffs = self.call_bluffs(&row, &opposing);
        let over = self.outcome().is_some();
        // What bends The Hand lands here, after every card is down, and the
        // Items spent on it are paid for. A Hand that will never be paid
        // spends nothing.
        if !over {
            hand = self
                .modifiers()
                .fold(hand, |hand, m| m.after_showdown(hand, house_edge, &row));
            for pocketed in &mut self.pocket {
                if pocketed.spent == Spent::Pending {
                    pocketed.uses -= 1;
                    pocketed.spent = Spent::Final;
                }
            }
        }

        self.showdown = Some(Showdown {
            row,
            opposing,
            hand,
            house_edge,
            bluffs,
            turn: self.turn,
        });

        if hand != house_edge && !over {
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
        let coin = self.coin();
        let flip = coin.resolve(std::iter::repeat_with(|| (self.next_rng() % 100) as u32));

        Some(self.resolve(Some(flip)))
    }

    /// Deal the Payout or the Whiff, send each side's row to its own discard,
    /// and start the next turn.
    fn resolve(&mut self, pyl: Option<Push>) -> TurnResult {
        let hand = self.hand();
        let house_edge = self.house_edge();
        let kind = match (hand >= house_edge, pyl) {
            // A Bluff ended the duel before The Hand met the House Edge, so
            // nobody is paid.
            _ if self.outcome().is_some() => Outcome::Payout(0),
            // A lost Push on a clearing Hand zeroes it: a full Whiff.
            (true, Some(Push::Lost)) => Outcome::Whiff(house_edge),
            (true, _) => Outcome::Payout(self.payout(pyl)),
            (false, _) => Outcome::Whiff(self.whiff(pyl)),
        };
        match kind {
            Outcome::Payout(n) => self.enemy.chips = self.enemy.chips.saturating_sub(n),
            Outcome::Whiff(n) => self.player_chips = self.player_chips.saturating_sub(n),
        }

        self.turn += 1;
        self.phase = Phase::Playing;
        for placed in std::mem::take(&mut self.row) {
            self.player.discard.extend(placed.sacrifice);
            self.player.discard.push(placed.card);
        }
        // A fixed row came off no deck and goes nowhere.
        let opposing = std::mem::take(&mut self.opposing);
        if self.fixed.is_none() {
            for o in opposing {
                self.enemy_cards.discard.extend(o.sacrifice);
                self.enemy_cards.discard.push(o.card);
            }
        }
        self.last = self.showdown.take();
        // A use lasting the Hand is done; one lasting the encounter goes on.
        for pocketed in &mut self.pocket {
            pocketed.spent = match (pocketed.spent, pocketed.item.lasts) {
                (Spent::Final, Lasts::Encounter) | (Spent::Lasting, _) => Spent::Lasting,
                _ => Spent::No,
            };
        }
        self.start_turn();

        TurnResult {
            hand,
            house_edge,
            kind,
            pyl,
        }
    }

    /// Both sides draw up to 7, or whatever their Draw size has been bent
    /// to, the enemy commits its row, and anything that reveals at the start
    /// of a turn does.
    fn start_turn(&mut self) {
        let sizes = (self.draw_size(Side::Player), self.draw_size(Side::Enemy));
        let mut rng = self.rng;
        refill(&mut self.player, sizes.0, &mut rng);
        refill(&mut self.enemy_cards, sizes.1, &mut rng);
        self.rng = rng;
        self.lay_opposing();
        let modifiers: Vec<_> = self.modifiers().collect();
        self.reveal_by(&modifiers);
    }

    /// Spend a use of the held Item at `index`. One spent while building the
    /// row is paid for at Confirm, and can be taken back until then unless it
    /// has already shown or drawn something; one spent at the prompt is paid
    /// for at once.
    pub fn spend(&mut self, index: usize) -> Result<(), ItemError> {
        let pocketed = *self.pocket.get(index).ok_or(ItemError::NoSuchItem)?;
        if pocketed.spent != Spent::No {
            return Err(ItemError::AlreadySpent);
        }
        if pocketed.uses == 0 {
            return Err(ItemError::NoUsesLeft);
        }
        let now = match self.phase {
            Phase::Playing => When::Row,
            Phase::PushYourLuck => When::Prompt,
        };
        if pocketed.item.when != now {
            return Err(ItemError::NotNow);
        }
        let size = self.draw_size(Side::Player);
        self.pocket[index].spent = Spent::Pending;
        let shown = self.reveal_by(&[pocketed.item.modifier]);
        let more = self.draw_size(Side::Player).saturating_sub(size);
        let mut rng = self.rng;
        let drawn = (0..more)
            .filter(|_| draw_one(&mut self.player, &mut rng))
            .count();
        self.rng = rng;
        if shown || drawn > 0 || now == When::Prompt {
            let pocketed = &mut self.pocket[index];
            pocketed.uses -= 1;
            pocketed.spent = Spent::Final;
        }
        Ok(())
    }

    /// Put back an Item spent on this Hand that hasn't shown or drawn
    /// anything yet. Refused if the row would then hold more than the Blind.
    pub fn take_back(&mut self, index: usize) -> Result<(), ItemError> {
        let pocketed = self.pocket.get(index).ok_or(ItemError::NoSuchItem)?;
        match pocketed.spent {
            Spent::Pending => {}
            Spent::No => return Err(ItemError::NotSpent),
            Spent::Final | Spent::Lasting => return Err(ItemError::CannotTakeBack),
        }
        self.pocket[index].spent = Spent::No;
        if self.row.len() > usize::from(self.blind(Side::Player)) {
            self.pocket[index].spent = Spent::Pending;
            return Err(ItemError::RowTooLong);
        }
        Ok(())
    }

    /// Every modifier bending the duel right now: the Table Rule, the Perks
    /// in the order taken, then the Items being spent, in the order taken.
    fn modifiers(&self) -> impl Iterator<Item = &'static dyn Modifier> + '_ {
        self.modifiers
            .iter()
            .copied()
            .chain(self.spending().map(|item| item.modifier))
    }

    /// The Items spent on this Hand, or still working for the encounter.
    fn spending(&self) -> impl Iterator<Item = &'static Item> + '_ {
        self.pocket
            .iter()
            .filter(|p| p.spent != Spent::No)
            .map(|p| p.item)
    }

    /// Turn face up whatever `modifiers` reveal, each seeing what the ones
    /// before it turned. Says whether anything turned over.
    fn reveal_by(&mut self, modifiers: &[&'static dyn Modifier]) -> bool {
        let mut shown = false;
        for modifier in modifiers {
            for slot in modifier.reveal(&self.opposing) {
                if let Some(opposing) = self.opposing.get_mut(slot)
                    && !opposing.face_up
                {
                    opposing.face_up = true;
                    shown = true;
                }
            }
        }
        shown
    }

    /// Both rows as they will turn over, once every modifier has bent them
    /// before the Tells resolve: the player's, then the enemy's.
    fn table(&self) -> (Vec<Placed>, Vec<Placed>) {
        let bend = |side: Side, row: Vec<Placed>| {
            self.modifiers()
                .fold(row, |row, m| m.before_showdown(side, row))
        };
        (
            bend(Side::Player, self.row.clone()),
            bend(Side::Enemy, self.opposing_row()),
        )
    }

    /// `opposing`, the enemy's row as [`Duel::table`] bent it, as far as the
    /// player can see it: a face-down card is nothing until the rows turn
    /// over.
    fn face_up(&self, opposing: &[Placed]) -> Vec<Option<Card>> {
        opposing
            .iter()
            .zip(&self.opposing)
            .map(|(bent, o)| o.face_up.then(|| bent.card.clone()))
            .collect()
    }

    /// The enemy commits its row face down: the Arcade's fixed one, or what
    /// its strategy picks out of its Draw.
    fn lay_opposing(&mut self) {
        if let Some(row) = &self.fixed {
            self.opposing = row
                .iter()
                .map(|(card, face_up)| Opposing {
                    card: card.clone(),
                    sacrifice: None,
                    face_up: *face_up,
                })
                .collect();
            return;
        }
        let picks = greedy(
            &self.enemy_cards.draw,
            self.blind(Side::Enemy),
            self.player_average,
            self.blind(Side::Player),
        );
        let draw = &self.enemy_cards.draw;
        self.opposing = picks
            .iter()
            .map(|pick| Opposing {
                card: draw[pick.card].clone(),
                sacrifice: pick.sacrifice.map(|s| draw[s].clone()),
                face_up: false,
            })
            .collect();
        // Out of the Draw, highest place first so the lower ones stay put.
        let mut gone: Vec<usize> = picks
            .iter()
            .flat_map(|p| std::iter::once(p.card).chain(p.sacrifice))
            .collect();
        gone.sort_unstable_by(|a, b| b.cmp(a));
        for i in gone {
            self.enemy_cards.draw.remove(i);
        }
    }

    /// Set off every Bluff on the table, left to right. In each slot where
    /// either card is a Bluff, the side with the lower Face Value loses the
    /// difference off its Chips: once, even with a Bluff on both sides. A
    /// Bluff across an empty slot has nothing to compare against. Stops as
    /// soon as either side's Chips run out, because the duel is over and
    /// nothing after that resolves.
    fn call_bluffs(&mut self, row: &[Played], opposing: &[Played]) -> Vec<BluffHit> {
        let mut hits = Vec::new();
        for (slot, (mine, theirs)) in row.iter().zip(opposing).enumerate() {
            if self.player_chips == 0 || self.enemy.chips == 0 {
                break;
            }
            let bluff = Some(Tell::Bluff);
            if mine.card.tell != bluff && theirs.card.tell != bluff {
                continue;
            }
            // The muck came first: a mucked Bluff doesn't hit, and a mucked
            // card isn't there to be hit.
            if mine.mucked || theirs.mucked {
                continue;
            }
            let (mine, theirs) = (mine.card.face_value, theirs.card.face_value);
            let (loser, chips) = match mine.cmp(&theirs) {
                std::cmp::Ordering::Less => (Side::Player, &mut self.player_chips),
                std::cmp::Ordering::Greater => (Side::Enemy, &mut self.enemy.chips),
                std::cmp::Ordering::Equal => continue,
            };
            let amount = mine.abs_diff(theirs).min(*chips);
            *chips -= amount;
            hits.push(BluffHit {
                slot,
                loser,
                amount,
            });
        }
        hits
    }

    /// Hand the committed row back to the enemy's Draw, for a row laid on top
    /// of it instead.
    fn take_back_opposing(&mut self) {
        for o in std::mem::take(&mut self.opposing) {
            self.enemy_cards.draw.extend(o.sacrifice);
            self.enemy_cards.draw.push(o.card);
        }
    }

    /// The Opposing Cards as a row that [`resolve_row`] can read.
    fn opposing_row(&self) -> Vec<Placed> {
        self.opposing
            .iter()
            .map(|o| Placed {
                card: o.card.clone(),
                sacrifice: o.sacrifice.clone(),
            })
            .collect()
    }

    fn next_rng(&mut self) -> u64 {
        xorshift64(&mut self.rng)
    }
}

/// Take a card and, if there is one, its sacrifice out of a Draw, the higher
/// place first so the lower one stays valid.
fn remove_both(draw: &mut Vec<Card>, card: usize, sacrifice: Option<usize>) {
    let mut gone: Vec<usize> = sacrifice.into_iter().chain([card]).collect();
    gone.sort_unstable_by(|a, b| b.cmp(a));
    for i in gone {
        draw.remove(i);
    }
}

/// Draw up to `size`. A side with nothing left to draw plays with what it
/// has.
fn refill(pile: &mut Pile, size: usize, rng: &mut u64) {
    while pile.draw.len() < size && draw_one(pile, rng) {}
}

/// Draw one card, shuffling the discard back in when the deck runs dry. Says
/// whether there was anything to draw.
fn draw_one(pile: &mut Pile, rng: &mut u64) -> bool {
    if pile.deck.is_empty() {
        if pile.discard.is_empty() {
            return false;
        }
        pile.deck = shuffled(std::mem::take(&mut pile.discard), xorshift64(rng));
    }
    let card = pile.deck.pop().expect("deck was just checked non-empty");
    pile.draw.push(card);
    true
}

/// The printed cards of a row, for the Tells across from it to read.
fn printed(row: &[Placed]) -> Vec<Option<Card>> {
    row.iter().map(|p| Some(p.card.clone())).collect()
}

/// Cards taken off the end of a row before the Showdown, as the Showdown
/// shows them: Mucked, and worth nothing.
fn mucked<'a>(cards: impl Iterator<Item = &'a Card>) -> impl Iterator<Item = Played> {
    cards.map(|card| Played {
        card: card.clone(),
        value: 0,
        mucked: true,
    })
}

/// The average Face Value of a deck, to the nearest whole number.
fn average_face_value(deck: &[Card]) -> u32 {
    let n = deck.len() as u32;
    if n == 0 {
        return 0;
    }
    (deck.iter().map(|c| c.face_value).sum::<u32>() + n / 2) / n
}

/// Fisher-Yates over the run's xorshift64.
fn shuffled(mut deck: Vec<Card>, mut rng: u64) -> Vec<Card> {
    for i in (1..deck.len()).rev() {
        let j = (xorshift64(&mut rng) % (i as u64 + 1)) as usize;
        deck.swap(i, j);
    }
    deck
}

/// What a resolved row is worth.
pub fn row_value(row: &[Played]) -> u32 {
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
            None => self.hold().expect("anything but a tie puts the prompt up"),
        }
    }

    /// Lay a known row down across the table, all of it face up, so a test
    /// can say what the player is up against without the enemy choosing it.
    /// Fixed, so the same row comes back every turn.
    pub fn facing(mut self, cards: Vec<Card>) -> Self {
        self.lay_out(cards);
        self
    }

    /// [`Duel::facing`] for a duel that is already running, which is how the
    /// Bevy-side tests get a known Edge across the table.
    pub fn lay_out(&mut self, cards: Vec<Card>) {
        self.fixed = Some(cards.into_iter().map(|c| (c, true)).collect());
        self.take_back_opposing();
        self.lay_opposing();
    }

    /// The floor's Blind, set outright, with the enemy's row committed again
    /// under it. The Bevy-side tests use it for room to play.
    pub fn set_blind(&mut self, blind: u8) {
        self.blind = blind;
        self.take_back_opposing();
        self.lay_opposing();
    }

    pub fn set_enemy_chips(&mut self, chips: u32) {
        self.enemy.chips = chips;
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
pub(crate) mod cards {
    use crate::modifier::{Modifier, Side};
    use crate::run::{Card, Enemy, Tell};

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
    pub fn bluff(face_value: u32) -> Card {
        Card {
            name: "bluff",
            face_value,
            tell: Some(Tell::Bluff),
        }
    }
    pub fn lowball(face_value: u32) -> Card {
        Card {
            name: "lowball",
            face_value,
            tell: Some(Tell::Lowball),
        }
    }

    /// An enemy whose Deck is nothing but plain 6s, so whatever it commits is
    /// worth 6 a card. Most tests lay their own row down with `Duel::facing`.
    pub fn enemy(chips: u32) -> Enemy {
        enemy_with(chips, vec![card(6); 30])
    }

    /// An enemy drawing from exactly `deck`, last card first.
    pub fn enemy_with(chips: u32, deck: Vec<Card>) -> Enemy {
        Enemy {
            name: "shill",
            chips,
            deck,
            table_rule: None,
        }
    }

    /// A modifier that sets one side's Blind outright, for tests that want
    /// the two Blinds apart.
    #[derive(Debug)]
    pub struct SetBlind(pub Side, pub u8);

    impl Modifier for SetBlind {
        fn blind(&self, side: Side, blind: u8) -> u8 {
            if side == self.0 { self.1 } else { blind }
        }
    }

    /// `modifier`, leaked for the `'static` the duel holds its modifiers by.
    pub fn leak(modifier: impl Modifier + 'static) -> &'static dyn Modifier {
        Box::leak(Box::new(modifier))
    }
}

#[cfg(test)]
mod tests {
    use super::cards::*;
    use super::*;
    use crate::modifier::Side;

    /// A deck of vanilla cards 1..=n, so card n is drawn first.
    fn vanilla_deck(n: u32) -> Vec<Card> {
        (1..=n).map(card).collect()
    }

    /// A duel at a Blind of 5 against an enemy that commits `n` sixes: the
    /// plain case, worth 6n.
    fn duel_facing_sixes(n: u8) -> Duel {
        sixes(vanilla_deck(18), 40, 999, n)
    }

    /// `deck` against `enemy_chips` worth of an enemy held to `n` sixes.
    fn sixes(deck: Vec<Card>, chips: u32, enemy_chips: u32, n: u8) -> Duel {
        Duel::modified(
            deck,
            chips,
            5,
            enemy(enemy_chips),
            vec![leak(SetBlind(Side::Enemy, n))],
        )
    }

    #[test]
    fn a_new_duel_deals_seven_cards_into_the_draw() {
        let duel = duel_facing_sixes(3);

        assert_eq!(duel.draw().len(), 7);
        let face_values: Vec<u32> = duel.draw().iter().map(|c| c.face_value).collect();
        assert_eq!(face_values, vec![18, 17, 16, 15, 14, 13, 12]);
    }

    #[test]
    fn the_enemy_commits_its_row_face_down_before_the_player_plays_a_card() {
        let duel = duel_facing_sixes(4);

        assert_eq!(duel.opposing().len(), 4);
        assert!(duel.opposing().iter().all(|o| !o.face_up));
        assert!(duel.row().is_empty());
        assert_eq!(duel.showing(), (0, 4), "nothing to read but four backs");
        assert_eq!(duel.house_edge(), 24);
    }

    #[test]
    fn the_enemy_keeps_what_it_did_not_commit_and_draws_back_up_to_seven() {
        // Drawn first: 9, 8, then five 1s; then two 7s wait in its Deck.
        let mut deck = vec![card(7), card(7)];
        deck.extend(vec![card(1); 5]);
        deck.extend([card(8), card(9)]);
        let mut duel = Duel::new(vanilla_deck(18), 40, 2, enemy_with(999, deck));
        assert_eq!(duel.house_edge(), 17, "the 9 and the 8");

        duel.end_turn();

        // Five 1s carried over, and the two 7s drawn on top of them.
        assert_eq!(duel.house_edge(), 14);
    }

    #[test]
    fn the_enemy_reshuffles_its_discard_when_its_deck_runs_dry() {
        // Exactly seven cards: all of them in the first Draw.
        let deck = vec![
            card(1),
            card(2),
            card(3),
            card(4),
            card(5),
            card(8),
            card(9),
        ];
        let mut duel = Duel::new(vanilla_deck(18), 40, 2, enemy_with(999, deck));
        assert_eq!(duel.house_edge(), 17);

        duel.end_turn();

        // Five left in the Draw, the Deck empty, so the 9 and the 8 come
        // back off the discard.
        assert_eq!(duel.house_edge(), 17);
    }

    #[test]
    fn an_enemy_with_nothing_left_plays_what_it_has() {
        let mut duel = Duel::new(vanilla_deck(18), 40, 3, enemy_with(999, vec![card(5)]));
        assert_eq!(duel.opposing().len(), 1, "one card, short of its Blind");
        duel.end_turn();

        // The 5 came back off the discard; running out never loses on its own.
        assert_eq!(duel.opposing().len(), 1);
        assert_eq!(duel.house_edge(), 5);

        let empty = Duel::new(vanilla_deck(18), 40, 3, enemy_with(999, Vec::new()));
        assert!(empty.opposing().is_empty());
        assert_eq!(empty.house_edge(), 0);
    }

    #[test]
    fn the_enemys_all_in_burns_a_card_from_its_own_draw() {
        let deck = vec![card(1), card(9), all_in(2)];
        let duel = Duel::new(vanilla_deck(18), 40, 1, enemy_with(999, deck));

        let committed = &duel.opposing()[0];
        assert_eq!(committed.card, all_in(2));
        assert_eq!(committed.sacrifice, Some(card(9)));
        assert_eq!(duel.house_edge(), 11);
    }

    #[test]
    fn placing_a_card_puts_it_in_the_next_empty_slot() {
        let mut duel = duel_facing_sixes(3);

        assert_eq!(duel.place(0, None), Ok(0));
        assert_eq!(duel.place(0, None), Ok(1));

        let row: Vec<u32> = duel.row().iter().map(|p| p.card.face_value).collect();
        assert_eq!(row, vec![18, 17]);
        assert_eq!(duel.hand(), 35);
        assert_eq!(duel.plays_left(), 3);
        assert_eq!(duel.draw().len(), 5);
    }

    #[test]
    fn the_player_plays_up_to_their_own_blind() {
        let mut duel = Duel::new(vanilla_deck(18), 40, 2, enemy(999));
        assert_eq!(duel.plays_left(), 2);

        duel.place(0, None).unwrap();
        duel.place(0, None).unwrap();

        assert_eq!(duel.plays_left(), 0);
        assert_eq!(duel.place(0, None), Err(PlayError::RowIsFull));
    }

    #[test]
    fn the_table_is_as_wide_as_the_larger_blind() {
        // The enemy's Blind is 4 and the player's 2: four slots, two of
        // them the player can't reach, and all four still count.
        let mut duel = Duel::modified(
            vanilla_deck(18),
            40,
            2,
            enemy(999),
            vec![leak(SetBlind(Side::Enemy, 4))],
        );
        assert_eq!(duel.slots(), 4);
        duel.place(0, None).unwrap();
        duel.place(0, None).unwrap();
        assert_eq!(duel.house_edge(), 24);

        // The other way round, the enemy's row is short and the slots past
        // it are empty, worth nothing.
        let duel = Duel::modified(
            vanilla_deck(18),
            40,
            4,
            enemy(999),
            vec![leak(SetBlind(Side::Enemy, 2))],
        );
        assert_eq!(duel.slots(), 4);
        assert_eq!(duel.opposing().len(), 2);
        assert_eq!(duel.plays_left(), 4);
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
        assert_eq!(duel.plays_left(), 3);
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
        let mut duel = Duel::new(deck, 40, 5, enemy(999));
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
        let mut duel = Duel::new(deck, 40, 5, enemy(999));

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
        let mut duel = sixes(vanilla_deck(18), 40, 100, 3);
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
            }
        );
        assert_eq!(duel.enemy_chips(), 67);
        assert_eq!(duel.player_chips(), 40);
    }

    #[test]
    fn falling_short_takes_the_difference_out_of_your_own_chips() {
        // Five 6s across: 30, against one card of 18.
        let mut duel = Duel::new(vanilla_deck(18), 40, 5, enemy(100));
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
        let mut duel = sixes(deck, 40, 100, 3);
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
        let mut duel = duel_facing_sixes(3);
        for _ in 0..3 {
            duel.place(0, None).unwrap();
        }
        duel.end_turn();

        assert!(duel.row().is_empty());
        assert_eq!(duel.opposing().len(), 3, "a fresh row across the table");
        assert!(duel.opposing().iter().all(|o| !o.face_up));
        assert_eq!(duel.plays_left(), 5);
        assert_eq!(duel.draw().len(), 7);
        // The four unplayed cards carried over, then three fresh ones.
        let face_values: Vec<u32> = duel.draw().iter().map(|c| c.face_value).collect();
        assert_eq!(face_values, vec![15, 14, 13, 12, 11, 10, 9]);
    }

    #[test]
    fn the_showdown_is_kept_for_the_screen_after_the_turn_is_over() {
        let mut duel = duel_facing_sixes(2);
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
    fn both_sides_start_at_the_floors_blind_and_it_stays_there() {
        let mut duel = Duel::new(vanilla_deck(40), 999, 3, enemy(9999));

        for _ in 0..4 {
            assert_eq!(duel.blind(Side::Player), 3);
            assert_eq!(duel.blind(Side::Enemy), 3);
            assert_eq!(duel.opposing().len(), 3);
            duel.end_turn();
        }
    }

    #[test]
    fn the_duel_is_won_when_the_enemy_chips_hit_zero() {
        let mut duel = sixes(vanilla_deck(18), 40, 30, 3);
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
        let mut duel = Duel::new(vanilla_deck(18), 10, 5, enemy(50));

        duel.end_turn();

        assert_eq!(duel.player_chips(), 0);
        assert_eq!(duel.outcome(), Some(CombatOutcome::Lost));
    }

    #[test]
    fn the_discard_is_reshuffled_into_the_deck_when_it_runs_dry() {
        // 9 cards: 7 dealt, 2 in the deck. After a turn of 5 cards there are
        // 2 left in the Draw, 2 come from the deck, and 3 must come back
        // from the discard.
        let mut duel = Duel::new(vanilla_deck(9), 40, 5, enemy(999));
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
        let across: Vec<Option<Card>> =
            duel.opposing.iter().map(|o| Some(o.card.clone())).collect();
        resolve_row(&duel.row, &across)
            .iter()
            .map(|p| p.value)
            .collect()
    }

    /// A duel holding exactly `draw` in the Draw, facing exactly `across`.
    fn table(draw: Vec<Card>, across: Vec<Card>) -> Duel {
        let plays = draw.len() as u8;
        // `Duel` draws off the end, so the first card of `draw` goes last.
        let deck: Vec<Card> = draw.into_iter().rev().collect();
        Duel::new(deck, 40, plays, enemy(999)).facing(across)
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
    fn flop_takes_the_opposing_card_across_from_it_and_that_cards_neighbours() {
        let mut duel = table(
            vec![flop(2), flop(2), flop(2)],
            vec![card(9), card(3), card(7)],
        );

        for _ in 0..3 {
            duel.place(0, None).unwrap();
        }

        // The ends of the row have one neighbour across; the middle has two.
        // The 2 printed on each Flop never counts.
        assert_eq!(resolved(&duel), vec![9 + 3, 9 + 3 + 7, 3 + 7]);
    }

    #[test]
    fn a_flop_moves_with_its_slot_when_the_row_is_rearranged() {
        let mut duel = table(vec![card(4), flop(2)], vec![card(9), card(3)]);

        duel.place(0, None).unwrap(); // the 4, in slot 0
        duel.place(0, None).unwrap(); // the Flop, in slot 1, across the 3
        assert_eq!(resolved(&duel), vec![4, 3 + 9]);

        // Lift the 4 and the Flop slides left, across the 9 instead.
        duel.lift(0).unwrap();
        assert_eq!(resolved(&duel), vec![9 + 3]);
    }

    #[test]
    fn a_flop_with_nothing_across_and_no_neighbours_is_worth_nothing() {
        // The enemy's Flop in slot 1 reads the player's slots 0 to 2. With
        // the player's row empty there is nothing there, and its own 5
        // doesn't count either.
        let mut duel = table(vec![card(4)], vec![card(1), flop(5)]);
        assert_eq!(duel.house_edge(), 1);

        // A 4 in slot 0 is a neighbour of the slot across from it.
        duel.place(0, None).unwrap();
        assert_eq!(duel.house_edge(), 1 + 4);
    }

    #[test]
    fn a_streak_to_the_right_of_a_flop_doubles() {
        let mut duel = table(vec![flop(1), streak(5)], vec![card(2), card(3)]);

        duel.place(0, None).unwrap();
        duel.place(0, None).unwrap();

        assert_eq!(resolved(&duel), vec![2 + 3, 10]);
    }

    #[test]
    fn the_enemys_tells_resolve_by_the_same_rules() {
        // A Streak in slot 1 doubling off the Copycat in slot 0, and a Flop
        // in slot 2 reading the player's card across from it and the one
        // beside that.
        let mut duel = table(
            vec![card(4), card(5), card(9)],
            vec![copycat(3), streak(6), flop(1)],
        );

        for _ in 0..3 {
            duel.place(0, None).unwrap();
        }

        // Copycat takes the Streak's print (6), the Streak doubles after a
        // Tell (12), the Flop takes the 9 across from it and the 5 beside it.
        assert_eq!(duel.house_edge(), 6 + 12 + 5 + 9);
        // And at the Showdown, which is where it counts.
        assert_eq!(duel.end_turn().house_edge, 6 + 12 + 5 + 9);
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

    /// Each card in a row: what it came to, and whether it was mucked.
    type Values = Vec<(u32, bool)>;

    /// Both rows once they turned over, player's first.
    fn turned_over(duel: &mut Duel) -> (Values, Values) {
        // A tie resolves on the spot, so its Showdown is already the last one.
        duel.confirm();
        let showdown = duel
            .showdown()
            .or(duel.last_showdown())
            .expect("the rows are turned over");
        let side = |row: &[Played]| row.iter().map(|p| (p.value, p.mucked)).collect();
        (side(&showdown.row), side(&showdown.opposing))
    }

    #[test]
    fn a_lowball_mucks_a_higher_card_across_from_it() {
        let mut duel = table(vec![lowball(1), card(4)], vec![card(6), card(2)]);
        duel.place(0, None).unwrap();
        duel.place(0, None).unwrap();

        let (row, opposing) = turned_over(&mut duel);
        assert_eq!(
            row,
            vec![(1, false), (4, false)],
            "the Lowball counts its own 1"
        );
        assert_eq!(opposing, vec![(0, true), (2, false)], "the 6 is gone");
        assert_eq!(duel.house_edge(), 2);
        assert_eq!(duel.hand(), 5);
    }

    #[test]
    fn a_lowball_across_an_equal_or_lower_card_does_nothing() {
        for across in [6, 5] {
            let mut duel = table(vec![lowball(6)], vec![card(across)]);
            duel.place(0, None).unwrap();

            let (row, opposing) = turned_over(&mut duel);
            assert_eq!(row, vec![(6, false)]);
            assert_eq!(opposing, vec![(across, false)]);
        }
    }

    #[test]
    fn a_lowball_across_an_empty_slot_does_nothing() {
        let mut duel = table(vec![card(3), lowball(1)], vec![card(2)]);
        duel.place(0, None).unwrap();
        duel.place(0, None).unwrap();

        let (row, opposing) = turned_over(&mut duel);
        assert_eq!(row, vec![(3, false), (1, false)]);
        assert_eq!(opposing, vec![(2, false)]);
    }

    #[test]
    fn the_enemys_lowball_mucks_the_players_card() {
        let mut duel = table(vec![card(8), card(3)], vec![lowball(2), card(1)]);
        duel.place(0, None).unwrap();
        duel.place(0, None).unwrap();

        let (row, opposing) = turned_over(&mut duel);
        assert_eq!(row, vec![(0, true), (3, false)]);
        assert_eq!(opposing, vec![(2, false), (1, false)]);
        // 3 against 3 ties, so the turn has already resolved.
        assert_eq!(duel.last_showdown().unwrap().hand, 3);
    }

    #[test]
    fn of_two_lowballs_facing_each_other_the_lower_one_mucks() {
        let mut duel = table(vec![lowball(2), lowball(4)], vec![lowball(5), lowball(4)]);
        duel.place(0, None).unwrap();
        duel.place(0, None).unwrap();

        let (row, opposing) = turned_over(&mut duel);
        assert_eq!(
            row,
            vec![(2, false), (4, false)],
            "4 against 4 is a standoff"
        );
        assert_eq!(opposing, vec![(0, true), (4, false)]);
    }

    #[test]
    fn a_mucked_cards_slot_reads_as_empty_to_its_neighbours() {
        // The enemy's Streak 9 is mucked, so the Streak 4 beside it has no
        // Tell to its left and stays a 4. The 8 is mucked too, so the
        // Copycat beside it has nothing to its right and keeps its own 1.
        let mut duel = table(
            vec![lowball(1), card(1), card(1), lowball(2)],
            vec![streak(9), streak(4), copycat(1), card(8)],
        );
        for _ in 0..4 {
            duel.place(0, None).unwrap();
        }
        let (_, opposing) = turned_over(&mut duel);
        assert_eq!(opposing, vec![(0, true), (4, false), (1, false), (0, true)]);
    }

    #[test]
    fn a_mucked_cards_tell_does_not_fire() {
        // The enemy's Copycat 7 would take the 9 beside it; mucked, it is
        // worth nothing at all.
        let mut duel = table(vec![lowball(2), card(1)], vec![copycat(7), card(9)]);
        duel.place(0, None).unwrap();
        duel.place(0, None).unwrap();

        let (_, opposing) = turned_over(&mut duel);
        assert_eq!(opposing, vec![(0, true), (9, false)]);
    }

    #[test]
    fn a_flop_counts_a_mucked_card_as_nothing() {
        // The enemy's Lowball mucks the player's 7, so the enemy's Flop
        // across the player's 3 reads only the 3.
        let mut duel = table(vec![card(7), card(3)], vec![lowball(2), flop(0)]);
        duel.place(0, None).unwrap();
        duel.place(0, None).unwrap();

        let (_, opposing) = turned_over(&mut duel);
        assert_eq!(opposing, vec![(2, false), (3, false)]);
    }

    #[test]
    fn a_streak_to_the_right_of_a_lowball_doubles() {
        let mut duel = table(vec![lowball(2), streak(5)], vec![card(1), card(1)]);
        duel.place(0, None).unwrap();
        duel.place(0, None).unwrap();

        assert_eq!(duel.hand(), 2 + 10);
    }

    #[test]
    fn a_mucked_card_goes_to_its_owners_discard() {
        let mut duel = table(vec![card(8)], vec![lowball(2)]);
        let whole = duel.deck_size();
        duel.place(0, None).unwrap();
        duel.end_turn();

        assert_eq!(duel.deck_size(), whole, "mucked, not gone from the Deck");
    }

    #[test]
    fn a_face_down_lowball_mucks_nothing_until_the_rows_turn_over() {
        let mut duel = table(vec![card(8)], vec![lowball(2)]);
        duel.hide_opposing(0);
        duel.place(0, None).unwrap();
        assert_eq!(duel.hand(), 8, "nothing about the face-down card shows");

        let (row, _) = turned_over(&mut duel);
        assert_eq!(row, vec![(0, true)]);
    }

    #[test]
    fn a_face_up_card_your_lowball_mucks_shows_as_nothing() {
        let mut duel = table(vec![lowball(1), card(1)], vec![card(6), card(2)]);
        duel.place(0, None).unwrap();
        assert_eq!(duel.showing(), (2, 0));
    }

    #[test]
    fn a_lowball_in_the_draw_says_whether_it_would_muck_the_card_across() {
        let mut deck = vec![card(1); 16];
        deck.extend([lowball(3), card(4)]); // the 4 drawn first, then the Lowball
        let mut duel = Duel::new(deck, 40, 3, enemy(999)).with_opposing(vec![
            (card(9), true),
            (card(2), true),
            (card(7), false),
        ]);

        let peek = |duel: &Duel| duel.lowball_next(3).map(|p| p.to_string());
        assert_eq!(peek(&duel), Some("Mucks the 9".into()));
        duel.place(0, None).unwrap();
        assert_eq!(peek(&duel), Some("Too high".into()), "a 2 across");
        duel.place(0, None).unwrap();
        assert_eq!(peek(&duel), None, "face down: nothing to say");
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
        Duel::new(vec![card(4); 18], 40, 5, enemy(999)).with_opposing(row)
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
    fn a_flop_counts_a_face_down_card_as_nothing_until_the_rows_turn_over() {
        let mut deck = vec![card(1); 17];
        deck.push(flop(3)); // drawn first
        let mut duel = Duel::new(deck, 40, 5, enemy(999)).with_opposing(vec![
            (card(9), true),
            (card(2), false),
            (card(2), false),
        ]);

        duel.place(0, None).unwrap();
        assert_eq!(
            duel.hand(),
            9,
            "the 9 across, and nothing it can see beside it"
        );

        duel.confirm();
        assert_eq!(duel.hand(), 9 + 2, "turned over, a 2 was beside it");
    }

    #[test]
    fn a_flop_in_the_draw_says_what_it_would_take_in_the_next_slot() {
        let mut deck = vec![card(1); 16];
        deck.extend([flop(3), card(4)]); // the 4 drawn first, then the Flop
        let mut duel = Duel::new(deck, 40, 2, enemy(999)).with_opposing(vec![
            (card(9), true),
            (card(5), true),
            (card(2), false),
        ]);

        let next = duel.flop_next().expect("a slot for it");
        assert_eq!(
            next,
            FlopPeek {
                seen: 9 + 5,
                hidden: 0
            }
        );
        assert_eq!(
            next.to_string(),
            "+14",
            "slot 0: the 9, and the 5 beside it"
        );

        duel.place(0, None).unwrap(); // the 4, in slot 0
        let next = duel.flop_next().expect("a slot for it");
        assert_eq!(
            next,
            FlopPeek {
                seen: 9 + 5,
                hidden: 1
            }
        );
        assert_eq!(next.to_string(), "+14 + ?", "slot 1: the 2 is face down");

        duel.place(0, None).unwrap(); // the Flop: the Blind is used up
        assert_eq!(duel.flop_next(), None, "no slot left for it");
    }

    #[test]
    fn a_flop_peek_adds_nothing_for_an_empty_slot() {
        // One Opposing Card on a table two wide: slot 1 reads slot 0 and
        // two empty slots.
        let mut duel =
            Duel::new(vec![card(4); 18], 40, 2, enemy(999)).with_opposing(vec![(card(9), false)]);
        duel.place(0, None).unwrap();

        assert_eq!(duel.flop_next(), Some(FlopPeek { seen: 0, hidden: 1 }));
        assert_eq!(duel.flop_next().unwrap().to_string(), "+0 + ?");
    }

    #[test]
    fn a_reveal_turns_chosen_opposing_cards_face_up_for_this_turn() {
        let enemy = enemy_with(999, vec![card(2), card(3), card(7)]);
        let mut duel = Duel::new(vec![card(4); 18], 40, 3, enemy);
        assert_eq!(duel.showing(), (0, 3));

        duel.reveal(&[0, 2, 7]); // slot 7 is off the table: nothing to turn

        let face_up: Vec<bool> = duel.opposing().iter().map(|o| o.face_up).collect();
        assert_eq!(face_up, vec![true, false, true]);
        assert_eq!(duel.showing(), (7 + 2, 1));
        assert_eq!(duel.flop_next(), Some(FlopPeek { seen: 7, hidden: 1 }));

        duel.end_turn();
        assert_eq!(duel.showing().1, 3, "the next row comes down face down");
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
        assert_eq!(duel.showing(), (3, 0), "nothing across it yet");

        duel.place(0, None).unwrap(); // a 4 in slot 0
        assert_eq!(duel.showing(), (4 + 3, 0));

        duel.place(0, None).unwrap(); // another beside it
        assert_eq!(duel.showing(), (4 + 4 + 3, 0));
        assert_eq!(duel.house_edge(), 11);
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
    use crate::boss::SLOTZ;
    use crate::run::Reward;

    /// The Slotz Perk's coin: best 2 of 3, at 49/51.
    const BEST_OF_THREE: Coin = Coin {
        player_pct: 49,
        best_of: 3,
    };

    /// One card of `hand` against a row that adds up to `edge`.
    fn hand_of(hand: u32, edge: u32) -> Duel {
        let mut deck = vec![card(1); 6];
        deck.push(card(hand));
        let mut duel = Duel::new(deck, 40, 5, enemy(999))
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
        let mut duel = Duel::new(deck, 15, 5, enemy(999)).with_coin(Coin::RIGGED);
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
    fn best_two_of_three_at_49_51_takes_the_majority() {
        assert_eq!(BEST_OF_THREE.resolve([10, 90, 10].into_iter()), Push::Won);
        assert_eq!(BEST_OF_THREE.resolve([90, 10, 10].into_iter()), Push::Won);
        assert_eq!(BEST_OF_THREE.resolve([90, 10, 90].into_iter()), Push::Lost);
        assert_eq!(BEST_OF_THREE.resolve([48, 48].into_iter()), Push::Won);
        assert_eq!(BEST_OF_THREE.resolve([49, 49].into_iter()), Push::Lost);
    }

    #[test]
    fn a_decided_best_of_three_stops_flipping() {
        let mut rolls = [10, 10, 10].into_iter();
        assert_eq!(BEST_OF_THREE.resolve(rolls.by_ref()), Push::Won);
        assert_eq!(rolls.count(), 1, "the third coin is never tossed");
    }

    #[test]
    fn the_slotz_perk_is_what_swaps_the_coin() {
        let Some([Reward::Perk(perk), _]) = SLOTZ.rewards else {
            panic!("Slotz offers its Perk first");
        };
        let plain = Duel::new(vec![card(4); 18], 40, 2, enemy(999));
        let slotz = Duel::modified(vec![card(4); 18], 40, 2, enemy(999), vec![perk.modifier]);

        assert_eq!(plain.coin(), Coin::BASE);
        assert_eq!(slotz.coin(), BEST_OF_THREE);
    }

    #[test]
    fn best_two_of_three_is_worth_about_48_point_5_percent() {
        assert_eq!(wins_over_every_roll(BEST_OF_THREE), 485_002);
        assert!(
            wins_over_every_roll(BEST_OF_THREE) > wins_over_every_roll(Coin::BASE),
            "the perk is worth taking"
        );
    }

    #[test]
    fn the_duels_own_coin_lands_on_both_sides() {
        let mut seen = (false, false);
        for seed in 1..200u64 {
            let mut duel = hand_of(30, 20).with_seed(seed * 2 + 1);
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
mod item_tests {
    use super::cards::*;
    use super::*;
    use crate::item::{self, Held, LOADED_DICE, LOADED_DICE_BONUS};

    fn deck() -> Vec<Card> {
        (1..=18).map(card).collect()
    }

    /// A duel facing a single card worth `edge`, holding `items`.
    fn facing(edge: u32, items: &[&'static Item]) -> Duel {
        holding(
            Duel::new(deck(), 40, 5, enemy(999)).facing(vec![card(edge)]),
            items,
        )
    }

    fn holding(duel: Duel, items: &[&'static Item]) -> Duel {
        let held: Vec<Held> = items.iter().map(|item| Held::new(item)).collect();
        duel.with_items(&held)
    }

    fn uses(duel: &Duel) -> Vec<u8> {
        duel.items().iter().map(|p| p.uses).collect()
    }

    #[test]
    fn an_item_held_but_not_spent_does_nothing() {
        let mut duel = facing(0, &[&LOADED_DICE]);

        let turn = duel.end_turn();

        assert_eq!(turn.hand, 0);
        assert_eq!(uses(&duel), vec![LOADED_DICE.uses]);
    }

    #[test]
    fn the_dice_land_on_the_hand_as_the_rows_turn_over() {
        let mut duel = facing(0, &[&LOADED_DICE]);
        duel.place(0, None).unwrap(); // 18
        duel.spend(0).unwrap();

        // Nothing on The Hand until the showdown.
        assert_eq!(duel.hand(), 18);
        let turn = duel.end_turn();

        assert_eq!(turn.hand, 18 + LOADED_DICE_BONUS);
        assert_eq!(uses(&duel), vec![LOADED_DICE.uses - 1]);
    }

    #[test]
    fn a_use_lasting_the_hand_is_gone_the_next_hand() {
        let mut duel = facing(0, &[&LOADED_DICE]);
        duel.spend(0).unwrap();
        duel.end_turn();

        assert_eq!(duel.items()[0].spent, Spent::No);
        assert_eq!(duel.end_turn().hand, 0);
    }

    #[test]
    fn an_item_spent_down_to_nothing_is_not_kept() {
        let mut duel = facing(0, &[&LOADED_DICE]);
        for _ in 0..LOADED_DICE.uses {
            duel.spend(0).unwrap();
            duel.end_turn();
        }

        assert_eq!(duel.spend(0), Err(ItemError::NoUsesLeft));
        assert!(duel.items_left().is_empty());
    }

    #[test]
    fn a_use_taken_back_before_confirm_costs_nothing() {
        let mut duel = facing(0, &[&LOADED_DICE]);
        duel.spend(0).unwrap();

        duel.take_back(0).unwrap();
        let turn = duel.end_turn();

        assert_eq!(turn.hand, 0);
        assert_eq!(uses(&duel), vec![LOADED_DICE.uses]);
    }

    #[test]
    fn one_use_of_each_item_a_hand() {
        let mut duel = facing(0, &[&LOADED_DICE, &item::INSURANCE]);

        duel.spend(0).unwrap();
        duel.spend(1).unwrap();

        assert_eq!(duel.spend(0), Err(ItemError::AlreadySpent));
        assert_eq!(duel.spend(9), Err(ItemError::NoSuchItem));
    }

    #[test]
    fn confirming_twice_only_spends_one_use() {
        let mut duel = facing(0, &[&LOADED_DICE]);
        duel.spend(0).unwrap();

        duel.confirm(); // the prompt goes up
        duel.confirm(); // and stays up, spending nothing more

        assert_eq!(duel.hand(), LOADED_DICE_BONUS);
        assert_eq!(uses(&duel), vec![LOADED_DICE.uses - 1]);
    }

    #[test]
    fn row_items_wait_for_the_row_and_the_coin_for_the_prompt() {
        let mut duel = facing(0, &[&LOADED_DICE, &item::WEIGHTED_COIN]);
        assert_eq!(duel.spend(1), Err(ItemError::NotNow));

        duel.place(0, None).unwrap();
        duel.confirm();

        assert_eq!(duel.spend(0), Err(ItemError::NotNow));
        assert_eq!(duel.take_back(0), Err(ItemError::NotSpent));
        duel.spend(1).unwrap();
        assert_eq!(duel.take_back(1), Err(ItemError::CannotTakeBack));
    }

    #[test]
    fn the_weighted_coin_flips_even() {
        let mut duel = facing(0, &[&item::WEIGHTED_COIN]);
        duel.place(0, None).unwrap();
        duel.confirm();
        assert_eq!(duel.coin(), Coin::BASE);

        duel.spend(0).unwrap();

        assert_eq!(duel.coin().player_pct, 50);
        assert_eq!(uses(&duel), vec![item::WEIGHTED_COIN.uses - 1]);
    }

    /// A duel whose Opposing Cards are 3, 4 and 5, all face down.
    fn three_face_down(items: &[&'static Item]) -> Duel {
        let mut duel = Duel::new(deck(), 40, 5, enemy(999)).facing(vec![card(3), card(4), card(5)]);
        for slot in 0..3 {
            duel.hide_opposing(slot);
        }
        holding(duel, items)
    }

    fn face_up(duel: &Duel) -> Vec<bool> {
        duel.opposing().iter().map(|o| o.face_up).collect()
    }

    #[test]
    fn the_sunglasses_turn_over_the_leftmost_face_down_card_and_stay_spent() {
        let mut duel = three_face_down(&[&item::SUNGLASSES]);
        duel.reveal(&[0]);

        duel.spend(0).unwrap();

        assert_eq!(face_up(&duel), vec![true, true, false]);
        assert_eq!(uses(&duel), vec![item::SUNGLASSES.uses - 1]);
        assert_eq!(duel.take_back(0), Err(ItemError::CannotTakeBack));
    }

    #[test]
    fn a_reveal_with_nothing_to_show_can_still_be_taken_back() {
        let mut duel = three_face_down(&[&item::SUNGLASSES]);
        duel.reveal(&[0, 1, 2]);

        duel.spend(0).unwrap();
        duel.take_back(0).unwrap();

        assert_eq!(uses(&duel), vec![item::SUNGLASSES.uses]);
    }

    #[test]
    fn the_two_way_mirror_turns_over_every_opposing_card() {
        let mut duel = three_face_down(&[&item::TWO_WAY_MIRROR]);

        duel.spend(0).unwrap();

        assert_eq!(face_up(&duel), vec![true, true, true]);
        assert_eq!(duel.showing(), (12, 0));
    }

    #[test]
    fn the_ace_up_the_sleeve_is_one_more_card_in_the_row() {
        let mut duel = facing(0, &[&item::ACE_UP_THE_SLEEVE]);
        duel.set_blind(1);
        duel.place(0, None).unwrap();
        assert_eq!(duel.plays_left(), 0);

        duel.spend(0).unwrap();
        assert_eq!(duel.plays_left(), 1);
        duel.place(0, None).unwrap();

        assert_eq!(duel.take_back(0), Err(ItemError::RowTooLong));
        duel.lift(1).unwrap();
        duel.take_back(0).unwrap();
        assert_eq!(duel.blind(Side::Player), 1);
    }

    #[test]
    fn the_shaved_card_is_what_every_tell_reads() {
        // The enemy's Flop across from it takes the shaved 7, not the 4.
        let mut duel = holding(
            Duel::new(vec![card(4)], 40, 5, enemy(999)).facing(vec![flop(1)]),
            &[&item::SHAVED_CARD],
        );
        duel.place(0, None).unwrap();
        duel.spend(0).unwrap();

        assert_eq!(duel.hand(), 7);
        assert_eq!(duel.house_edge(), 7);
        let turn = duel.end_turn();
        assert_eq!((turn.hand, turn.house_edge), (7, 7));
    }

    #[test]
    fn sleight_of_hand_mucks_the_rightmost_opposing_card() {
        let mut duel = three_face_down(&[&item::SLEIGHT_OF_HAND]);
        duel.spend(0).unwrap();

        duel.confirm();
        let showdown = duel.showdown().unwrap();

        assert_eq!(showdown.house_edge, 7);
        assert!(showdown.opposing[2].mucked);
        assert_eq!(showdown.opposing[2].value, 0);
        assert_eq!(showdown.opposing.len(), 3);
    }

    #[test]
    fn insurance_halves_a_whiff() {
        let mut duel = facing(25, &[&item::INSURANCE]);
        duel.place(0, None).unwrap(); // 18
        duel.spend(0).unwrap();

        let turn = duel.end_turn();

        // Short by 7: 3 of it is left, and the Edge still reads 25.
        assert_eq!(turn.kind, Outcome::Whiff(3));
        assert_eq!(turn.house_edge, 25);
    }

    #[test]
    fn the_shiny_card_sleeve_doubles_the_first_slot_for_the_encounter() {
        let mut duel = facing(0, &[&item::SHINY_CARD_SLEEVE]);
        duel.place(0, None).unwrap(); // 18
        duel.spend(0).unwrap();
        assert_eq!(duel.end_turn().hand, 36);
        assert_eq!(duel.items()[0].spent, Spent::Lasting);
        assert!(duel.items_left().is_empty(), "its one use is had");

        duel.lay_out(vec![card(0)]);
        duel.place(0, None).unwrap();
        let first = duel.row()[0].card.face_value;

        assert_eq!(duel.end_turn().hand, first * 2);
        assert_eq!(duel.spend(0), Err(ItemError::AlreadySpent));
    }

    #[test]
    fn deep_pockets_draws_one_now_and_eight_from_then_on() {
        let mut duel = facing(0, &[&item::DEEP_POCKETS]);
        assert_eq!(duel.draw().len(), 7);

        duel.spend(0).unwrap();

        assert_eq!(duel.draw().len(), 8);
        assert_eq!(duel.take_back(0), Err(ItemError::CannotTakeBack));
        duel.place(0, None).unwrap();
        duel.end_turn();
        assert_eq!(duel.draw().len(), 8);
        assert_eq!(duel.items()[0].spent, Spent::Lasting);
    }

    #[test]
    fn items_after_the_perks_each_see_the_last_answer() {
        // Dice first, then Insurance: 0+5 against 15 is short 10, halved to
        // 5. The other way round, 0 is short 15, halved to 7, then +5: 3.
        let mut duel = facing(15, &[&LOADED_DICE, &item::INSURANCE]);
        duel.spend(0).unwrap();
        duel.spend(1).unwrap();

        assert_eq!(duel.end_turn().kind, Outcome::Whiff(5));
    }
}

#[cfg(test)]
mod greedy_tests {
    use super::cards::*;
    use super::*;

    fn pick(card: usize) -> Pick {
        Pick {
            card,
            sacrifice: None,
        }
    }

    #[test]
    fn greedy_plays_its_biggest_cards_up_to_its_blind() {
        let draw = [card(3), card(9), card(1), card(7)];

        assert_eq!(greedy(&draw, 2, 4, 2), vec![pick(1), pick(3)]);
        assert_eq!(greedy(&draw, 1, 4, 2), vec![pick(1)]);
    }

    #[test]
    fn greedy_orders_its_row_for_the_tells() {
        // 4 then a doubled 5 is 14; 5 then a doubled 4 is only 13.
        let draw = [streak(5), streak(4)];

        assert_eq!(greedy(&draw, 2, 4, 2), vec![pick(1), pick(0)]);
    }

    #[test]
    fn greedy_burns_for_an_all_in_when_it_pays() {
        // The 9 alone is 9; the All In burning it is 11.
        let draw = [all_in(2), card(9), card(3)];

        assert_eq!(
            greedy(&draw, 1, 4, 2),
            vec![Pick {
                card: 0,
                sacrifice: Some(1)
            }]
        );
    }

    #[test]
    fn greedy_scores_a_flop_on_the_players_average_for_each_slot_they_can_fill() {
        let draw = [flop(1), card(5)];

        // In slot 0 a Flop reads slots 0 and 1. Both fillable: 4 + 4 beats 5.
        assert_eq!(greedy(&draw, 1, 4, 2), vec![pick(0)]);
        // Only slot 0 fillable: 4 loses to the 5.
        assert_eq!(greedy(&draw, 1, 4, 1), vec![pick(1)]);
    }

    #[test]
    fn greedy_scores_a_lowball_on_the_players_average_when_it_would_muck() {
        let draw = [lowball(1), card(3)];

        // Under the average of 6: 1 + 6 beats the 3.
        assert_eq!(greedy(&draw, 1, 6, 2), vec![pick(0)]);
        // Not under an average of 1: a 1 loses to the 3.
        assert_eq!(greedy(&draw, 1, 1, 2), vec![pick(1)]);
        // No slot the player can fill: nothing to muck.
        assert_eq!(greedy(&draw, 1, 6, 0), vec![pick(1)]);
    }

    #[test]
    fn a_tie_goes_to_the_earliest_pick_in_draw_order() {
        let draw = [card(5), card(2), card(5)];

        assert_eq!(greedy(&draw, 1, 4, 2), vec![pick(0)]);
    }

    #[test]
    fn greedy_commits_nothing_from_an_empty_draw() {
        assert_eq!(greedy(&[], 3, 4, 3), Vec::new());
    }
}

#[cfg(test)]
mod modifier_tests {
    use super::cards::*;
    use super::*;
    use crate::boss::{PIT_BOSS, SLOTZ};
    use crate::modifier::Side;
    use crate::run::{Encounter, Floor, Reward, RunState};

    /// A modifier that adds to one side's Blind.
    #[derive(Debug)]
    struct AddBlind(Side, u8);

    impl Modifier for AddBlind {
        fn blind(&self, side: Side, blind: u8) -> u8 {
            if side == self.0 {
                blind + self.1
            } else {
                blind
            }
        }
    }

    #[test]
    fn the_table_rule_bends_first_and_each_modifier_sees_the_last_answer() {
        let enemy = Enemy {
            table_rule: Some(leak(SetBlind(Side::Player, 4))),
            ..enemy(999)
        };
        // Set to 4 by the Table Rule, then +1 by the Perk: 5. Applied the
        // other way round it would be 4.
        let duel = Duel::modified(
            vec![card(4); 18],
            40,
            2,
            enemy,
            vec![leak(AddBlind(Side::Player, 1))],
        );

        assert_eq!(duel.blind(Side::Player), 5);
        assert_eq!(duel.blind(Side::Enemy), 2);
    }

    #[test]
    fn a_blind_never_drops_below_one() {
        let duel = Duel::modified(
            vec![card(4); 18],
            40,
            2,
            enemy(999),
            vec![
                leak(SetBlind(Side::Player, 0)),
                leak(SetBlind(Side::Enemy, 0)),
            ],
        );

        assert_eq!(duel.blind(Side::Player), 1);
        assert_eq!(duel.blind(Side::Enemy), 1);
        assert_eq!(duel.opposing().len(), 1);
    }

    #[test]
    fn a_run_duel_starts_at_its_floors_blind() {
        let run = RunState::new();
        let pit = Duel::for_run(
            &run,
            Encounter::Minion {
                floor: Floor::ThePit,
            },
            7,
        );

        assert_eq!(pit.blind(Side::Player), 3);
        assert_eq!(pit.blind(Side::Enemy), 3);
    }

    #[test]
    fn the_pit_boss_perk_raises_only_the_players_blind() {
        let Some([Reward::Perk(_), _]) = PIT_BOSS.rewards else {
            panic!("the Pit Boss offers its Perk first");
        };
        let mut run = RunState::new();
        run.apply(PIT_BOSS.rewards.unwrap()[0], 1);

        let duel = Duel::for_run(
            &run,
            Encounter::Minion {
                floor: Floor::TheFloor,
            },
            7,
        );

        assert_eq!(duel.blind(Side::Player), 3);
        assert_eq!(duel.blind(Side::Enemy), 2);
    }

    #[test]
    fn a_boss_plays_out_of_its_own_deck() {
        let mut duel = Duel::for_run(
            &RunState::new(),
            Encounter::Boss {
                boss: &SLOTZ,
                floor: Floor::TheFloor,
            },
            7,
        );

        for _ in 0..5 {
            assert_eq!(duel.opposing().len(), 2);
            assert!(duel.opposing().iter().all(|o| SLOTZ.deck.contains(&o.card)));
            duel.end_turn();
        }
    }
}

#[cfg(test)]
mod bluff_tests {
    use super::cards::*;
    use super::*;
    use crate::modifier::Side;

    /// The player holding exactly `draw` on `player` Chips, every card of it
    /// in the row, against an enemy on `enemy_chips` that laid `across`.
    fn played(draw: Vec<Card>, across: Vec<Card>, player: u32, enemy_chips: u32) -> Duel {
        let blind = draw.len().max(across.len()) as u8;
        let deck: Vec<Card> = draw.into_iter().rev().collect();
        let mut duel = Duel::new(deck, player, blind, enemy(enemy_chips)).facing(across);
        while !duel.draw().is_empty() && duel.plays_left() > 0 {
            duel.place(0, None).unwrap();
        }
        duel
    }

    fn hit(slot: usize, loser: Side, amount: u32) -> BluffHit {
        BluffHit {
            slot,
            loser,
            amount,
        }
    }

    #[test]
    fn a_bluff_under_the_card_across_costs_the_player_the_difference() {
        let mut duel = played(vec![bluff(3)], vec![card(7)], 40, 99);

        duel.confirm();

        assert_eq!((duel.player_chips(), duel.enemy_chips()), (36, 99));
        let showdown = duel.showdown().expect("a Whiff puts the prompt up");
        assert_eq!(showdown.bluffs, vec![hit(0, Side::Player, 4)]);
        assert_eq!(showdown.hand, 3, "the Bluff still counts its print");
    }

    #[test]
    fn a_bluff_over_the_card_across_costs_the_enemy_the_difference() {
        let mut duel = played(vec![bluff(9)], vec![card(4)], 40, 99);

        duel.confirm();

        assert_eq!((duel.player_chips(), duel.enemy_chips()), (40, 94));
        assert_eq!(duel.hand(), 9);
    }

    #[test]
    fn a_bluff_level_with_the_card_across_does_nothing() {
        let mut duel = played(vec![bluff(5)], vec![card(5)], 40, 99);

        let result = duel.confirm().expect("a tie resolves on the spot");

        assert_eq!(result.hand, 5);
        assert_eq!((duel.player_chips(), duel.enemy_chips()), (40, 99));
        assert!(duel.last_showdown().unwrap().bluffs.is_empty());
    }

    #[test]
    fn an_enemy_bluff_over_the_players_card_costs_the_player() {
        let mut duel = played(vec![card(2)], vec![bluff(8)], 40, 99);

        duel.confirm();

        assert_eq!((duel.player_chips(), duel.enemy_chips()), (34, 99));
        assert_eq!(duel.showdown().unwrap().house_edge, 8);
    }

    #[test]
    fn an_enemy_bluff_under_the_players_card_costs_the_enemy() {
        let mut duel = played(vec![card(8)], vec![bluff(2)], 40, 99);

        duel.confirm();

        assert_eq!((duel.player_chips(), duel.enemy_chips()), (40, 93));
    }

    #[test]
    fn two_bluffs_facing_each_other_hit_once() {
        let mut duel = played(vec![bluff(3)], vec![bluff(7)], 40, 99);

        duel.confirm();

        assert_eq!(duel.player_chips(), 36, "7 - 3, once");
        assert_eq!(
            duel.showdown().unwrap().bluffs,
            vec![hit(0, Side::Player, 4)]
        );
    }

    #[test]
    fn a_bluff_across_an_empty_slot_does_nothing() {
        // The player's Bluff in slot 1 has nothing across; the enemy's in
        // slot 1 of the other duel has nothing across either.
        let mut short_enemy = played(vec![card(1), bluff(9)], vec![card(1)], 40, 99);
        let mut short_player = played(vec![card(1)], vec![card(1), bluff(9)], 40, 99);

        short_enemy.confirm();
        short_player.confirm();

        for duel in [&short_enemy, &short_player] {
            assert_eq!((duel.player_chips(), duel.enemy_chips()), (40, 99));
            assert!(duel.showdown().unwrap().bluffs.is_empty());
        }
    }

    #[test]
    fn the_bluff_hit_lands_before_the_payout_and_both_are_paid() {
        let mut duel = played(vec![bluff(9), card(1)], vec![card(4), card(1)], 40, 99);

        let result = duel.end_turn();

        // Bluff: 9 - 4 = 5. Then a Hand of 10 against an Edge of 5 pays 5.
        assert_eq!(result.kind, Outcome::Payout(5));
        assert_eq!(duel.enemy_chips(), 99 - 5 - 5);
    }

    #[test]
    fn bluffs_hit_left_to_right_from_both_sides() {
        let mut duel = played(
            vec![bluff(6), card(2), bluff(1)],
            vec![card(3), bluff(5), card(2)],
            40,
            99,
        );

        duel.confirm();

        assert_eq!(
            duel.showdown().unwrap().bluffs,
            vec![
                hit(0, Side::Enemy, 3),
                hit(1, Side::Player, 3),
                hit(2, Side::Player, 1),
            ]
        );
    }

    #[test]
    fn a_bluff_that_ends_the_duel_stops_the_rest_and_nothing_is_paid() {
        // The first Bluff takes the enemy's last 3 Chips. The second would
        // have cost the player 8, and the Hand would have paid out; neither
        // happens.
        let mut duel = played(vec![bluff(6), bluff(1)], vec![card(3), card(9)], 40, 3);

        let result = duel.confirm().expect("the duel is over, no prompt");

        assert_eq!(duel.outcome(), Some(CombatOutcome::Won));
        assert_eq!(duel.player_chips(), 40, "the second Bluff never went off");
        assert_eq!(result.kind, Outcome::Payout(0), "and nothing was paid");
        assert_eq!(duel.phase(), Phase::Playing);
        let showdown = duel.last_showdown().expect("the table keeps the rows");
        assert_eq!(
            showdown.bluffs,
            vec![hit(0, Side::Enemy, 3)],
            "only what it had"
        );
    }

    #[test]
    fn an_enemy_bluff_can_take_the_player_out_before_a_winning_hand_pays() {
        let mut duel = played(vec![card(1), card(30)], vec![bluff(5), card(1)], 4, 99);

        duel.confirm().expect("the duel is over, no prompt");

        assert_eq!(duel.outcome(), Some(CombatOutcome::Lost));
        assert_eq!(duel.enemy_chips(), 99, "the Hand of 31 never paid");
    }

    #[test]
    fn items_are_not_spent_on_a_hand_that_never_paid() {
        let deck = vec![bluff(9)];
        let mut duel = Duel::new(deck, 40, 1, enemy(5))
            .facing(vec![card(1)])
            .with_items(&[crate::item::Held::new(&crate::item::LOADED_DICE)]);
        duel.place(0, None).unwrap();
        duel.spend(0).unwrap();

        duel.confirm();

        assert_eq!(duel.outcome(), Some(CombatOutcome::Won));
        assert_eq!(duel.items_left()[0].uses, crate::item::LOADED_DICE.uses);
    }

    #[test]
    fn a_bluff_mucked_by_the_lowball_across_does_not_hit() {
        // Unmucked, the 5 over the 2 would cost the enemy 3.
        let mut duel = played(vec![bluff(5)], vec![lowball(2)], 40, 99);

        duel.confirm();

        assert_eq!((duel.player_chips(), duel.enemy_chips()), (40, 99));
        let showdown = duel.showdown().expect("a Whiff puts the prompt up");
        assert!(showdown.row[0].mucked);
        assert_eq!(showdown.bluffs, vec![]);
    }

    #[test]
    fn an_enemy_bluff_mucked_by_your_lowball_hits_nobody() {
        // Unmucked, the enemy's 8 over the Lowball 2 would cost the player 6.
        let mut duel = played(vec![lowball(2)], vec![bluff(8)], 40, 99);

        duel.confirm();

        assert_eq!((duel.player_chips(), duel.enemy_chips()), (40, 99));
        let showdown = duel.showdown().expect("a clearing Hand puts the prompt up");
        assert!(showdown.opposing[0].mucked);
        assert_eq!(showdown.bluffs, vec![]);
    }

    #[test]
    fn a_bluff_beside_a_muck_still_hits() {
        // Slot 0 is mucked; the Bluff in slot 1 goes off as normal.
        let mut duel = played(vec![lowball(1), bluff(3)], vec![card(9), card(7)], 40, 99);

        duel.confirm();

        let showdown = duel.showdown().or(duel.last_showdown()).unwrap();
        assert!(showdown.opposing[0].mucked);
        assert_eq!(showdown.bluffs, vec![hit(1, Side::Player, 4)]);
        assert_eq!(duel.player_chips(), 36);
    }

    #[test]
    fn a_streak_to_the_right_of_a_bluff_doubles() {
        let duel = played(vec![bluff(2), streak(5)], vec![card(2), card(1)], 40, 99);

        assert_eq!(duel.hand(), 2 + 10);
    }

    #[test]
    fn greedy_scores_a_bluff_at_its_face_value() {
        // The Bluff's hit doesn't steer the pick: the plain 6 beats a Bluff 5.
        let draw = [bluff(5), card(6)];

        assert_eq!(
            greedy(&draw, 1, 1, 1),
            vec![Pick {
                card: 1,
                sacrifice: None
            }]
        );
    }
}
