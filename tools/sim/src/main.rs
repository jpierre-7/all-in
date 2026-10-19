//! The balance sim (#94): whole runs against the real `Duel`, headless, so a
//! change to a card, an enemy or a reward can be read as a win rate before it
//! ships. Every duel starts at `Duel::for_run`, the same door the game uses,
//! and walks the same `RUN` the overworld does.
//!
//! Two players, after the starter-deck prototype: **naive** plays its
//! biggest cards in a random order and never Pushes; **smart** tries every
//! row it can build from the Draw and Pushes only when it can't lose by it.
//! Real players land in between. See `tools/sim/README.md`.

use std::collections::BTreeMap;
use std::thread;

use all_in::combat::duel::{Duel, Phase, Played, needs_sacrifice, resolve_row, row_value};
use all_in::item::{self, Item, Rarity, When};
use all_in::legacy::{self, LegacyPerk};
use all_in::overworld::progression::RUN;
use all_in::run::{
    Card, CombatOutcome, Encounter, Floor, Pack, Perk, Reward, RewardOffer, RunState, Tell,
    xorshift64,
};
use all_in::wheel::{self, MAX_RANK, Spun, Square};

/// A duel still going after this many turns is a stall, counted as a loss.
const TURN_CAP: u32 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Player {
    Naive,
    Smart,
}

impl Player {
    fn name(self) -> &'static str {
        match self {
            Player::Naive => "naive",
            Player::Smart => "smart",
        }
    }
}

/// Which side of a boss's 1-of-2 the player takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pick {
    First,
    Second,
    Random,
}

/// What the player takes after beating a minion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Minion {
    Items,
    Cards,
    Skip,
    Random,
}

/// Which plain cards smart's Trim takes out (#192). Only a way to find out
/// why Trim is worth so much; the game lets the player choose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Trim {
    Lowest,
    Random,
    Highest,
    /// The lowest plain cards stay in the deck as a [`SWAP_FOR`]: the same
    /// number of cards, better ones, where Trim leaves fewer.
    Swap,
}

/// The Face Value [`Trim::Swap`] puts in for each card it takes out: about
/// the middle of the starter deck's plain cards (2 to 8).
const SWAP_FOR: u32 = 5;

/// A Draw card this small or smaller is a low one, for `--draw`.
const LOW: u32 = 3;

struct Args {
    runs: usize,
    seed: u64,
    players: Vec<Player>,
    pick: Pick,
    minion: Minion,
    /// The Wheel every run starts under, spun afresh each run. Empty is no
    /// Wheel at all.
    wheel: BTreeMap<Square, u8>,
    /// A duel lost before The House doesn't end the run: the player goes on
    /// at the Chips it sat down with and takes the reward as if it had won,
    /// so every run reaches The House.
    carry: bool,
    /// The Legacy Perk every run starts under. Victory Lap's deck is left
    /// empty here and dealt per run from [`victory_laps`].
    legacy: Option<LegacyPerk>,
    /// An Item every run starts holding, with every use it comes with: how
    /// one Item is weighed on its own.
    item: Option<&'static Item>,
    /// Every Opposing Card turns face up at the start of every turn: the
    /// most any Reveal could show.
    see_all: bool,
    /// Which plain cards smart's Trim takes out.
    trim: Trim,
    /// Report what sits unplayed in the Draw from turn to turn.
    draw: bool,
}

/// How a flag names something: `Card Shark's Sunglasses` is
/// `card-sharks-sunglasses`.
fn slug(name: &str) -> String {
    name.to_lowercase().replace('\'', "").replace(' ', "-")
}

/// `loaded-dice`: an Item by its [`slug`].
fn parse_item(name: &str) -> Result<&'static Item, String> {
    item::ITEMS
        .into_iter()
        .find(|item| slug(item.name) == name)
        .ok_or(format!("--item: no Item called {name}"))
}

/// `high-limit`: a Legacy Perk by its name in lower case with dashes, one of
/// the ones that do something yet.
fn parse_legacy(name: &str) -> Result<LegacyPerk, String> {
    let perk = LegacyPerk::all(&[])
        .into_iter()
        .find(|perk| slug(perk.name()) == name)
        .ok_or(format!("--legacy: no Legacy Perk called {name}"))?;
    if !perk.ready() {
        return Err(format!("--legacy: {name} doesn't do anything yet"));
    }
    Ok(perk)
}

/// `bankroll=3,lucky-coin=2`: each square by its name in lower case with
/// dashes, and its rank.
fn parse_wheel(spec: &str) -> Result<BTreeMap<Square, u8>, String> {
    let mut board = BTreeMap::new();
    for part in spec.split(',').filter(|p| !p.is_empty()) {
        let (name, rank) = part
            .split_once('=')
            .ok_or(format!("--wheel: {part} is not square=rank"))?;
        let square = Square::ALL
            .into_iter()
            .find(|sq| slug(sq.name()) == name)
            .ok_or(format!("--wheel: no square called {name}"))?;
        let rank: u8 = rank.parse().map_err(|e| format!("--wheel: {part}: {e}"))?;
        if !(1..=MAX_RANK).contains(&rank) {
            return Err(format!("--wheel: {part}: a rank is 1 to {MAX_RANK}"));
        }
        if !square.ready() {
            return Err(format!("--wheel: {name} doesn't do anything yet"));
        }
        board.insert(square, rank);
    }
    Ok(board)
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        runs: 2000,
        seed: 0x9e37_79b9_7f4a_7c15,
        players: vec![Player::Naive, Player::Smart],
        pick: Pick::Random,
        minion: Minion::Random,
        wheel: BTreeMap::new(),
        carry: false,
        legacy: None,
        item: None,
        see_all: false,
        trim: Trim::Lowest,
        draw: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--runs" => args.runs = value()?.parse().map_err(|e| format!("--runs: {e}"))?,
            "--seed" => args.seed = value()?.parse().map_err(|e| format!("--seed: {e}"))?,
            "--player" => {
                args.players = match value()?.as_str() {
                    "naive" => vec![Player::Naive],
                    "smart" => vec![Player::Smart],
                    "both" => vec![Player::Naive, Player::Smart],
                    other => return Err(format!("--player: no player called {other}")),
                }
            }
            "--pick" => {
                args.pick = match value()?.as_str() {
                    "first" => Pick::First,
                    "second" => Pick::Second,
                    "random" => Pick::Random,
                    other => return Err(format!("--pick: {other} is not first, second or random")),
                }
            }
            "--minion" => {
                args.minion = match value()?.as_str() {
                    "items" => Minion::Items,
                    "cards" => Minion::Cards,
                    "skip" => Minion::Skip,
                    "random" => Minion::Random,
                    other => {
                        return Err(format!(
                            "--minion: {other} is not items, cards, skip or random"
                        ));
                    }
                }
            }
            "--wheel" => args.wheel = parse_wheel(&value()?)?,
            "--carry" => args.carry = true,
            "--legacy" => args.legacy = Some(parse_legacy(&value()?)?),
            "--item" => args.item = Some(parse_item(&value()?)?),
            "--see-all" => args.see_all = true,
            "--trim" => {
                args.trim = match value()?.as_str() {
                    "lowest" => Trim::Lowest,
                    "random" => Trim::Random,
                    "highest" => Trim::Highest,
                    "swap" => Trim::Swap,
                    other => {
                        return Err(format!(
                            "--trim: {other} is not lowest, random, highest or swap"
                        ));
                    }
                }
            }
            "--draw" => args.draw = true,
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("unknown flag {other}")),
        }
    }
    Ok(args)
}

const USAGE: &str = "usage: cargo run --release -p sim -- [--runs N] [--seed N] \
[--player naive|smart|both] [--pick first|second|random] \
[--minion items|cards|skip|random] [--wheel square=rank,...] [--carry] \
[--legacy high-limit|marked-cards|lucky-streak|victory-lap] [--item loaded-dice|...] \
[--see-all] [--trim lowest|random|highest|swap] [--draw]";

// ---------------------------------------------------------------------------
// Building a row
// ---------------------------------------------------------------------------

/// One card put down, and what it burned.
type Move = (Card, Option<Card>);

fn index_of(duel: &Duel, card: &Card) -> usize {
    duel.draw()
        .iter()
        .position(|c| c == card)
        .expect("the card is still in the Draw")
}

fn place(duel: &mut Duel, (card, burn): &Move) {
    let i = index_of(duel, card);
    let b = burn.as_ref().map(|b| {
        // Two equal cards are one card as far as the row cares; burn the
        // other copy rather than the one being played.
        duel.draw()
            .iter()
            .enumerate()
            .position(|(j, c)| j != i && c == b)
            .expect("the sacrifice is still in the Draw")
    });
    duel.place(i, b).expect("the sim only makes legal plays");
}

/// The naive player: the biggest cards it can play, in a random order, each
/// All In burning the smallest card left over.
fn naive_row(duel: &mut Duel, rng: &mut u64) {
    while duel.plays_left() > 0 {
        let mut draw = duel.draw().to_vec();
        draw.sort_by(|a, b| b.face_value.cmp(&a.face_value));
        let take = usize::from(duel.plays_left()).min(draw.len());
        if take == 0 {
            return;
        }
        let card = draw[(xorshift64(rng) % take as u64) as usize].clone();
        let burn = if needs_sacrifice(duel.row(), &card) {
            // Smallest card that isn't this one; with nothing else in the
            // Draw an All In can't be played, so the row ends here.
            let i = draw.iter().position(|c| *c == card).unwrap();
            match (0..draw.len()).rev().find(|&j| j != i) {
                Some(j) => Some(draw[j].clone()),
                None => return,
            }
        } else {
            None
        };
        place(duel, &(card, burn));
    }
}

/// What the smart player thinks a finished row is worth: The Hand less what
/// it can see of the Opposing Cards, plus what the Bluffs it can read will
/// take (`Duel::bluffs_showing`), plus what it stands to win or lose across
/// the face-down ones ([`face_down_worth`], weighed against `deck`). It
/// counts what its Perks add to The Hand once the row resolves (Jackpot),
/// read against the face-up cards alone.
fn score(duel: &Duel, perks: &[&Perk], deck: &[Card]) -> f64 {
    let (showing, _) = duel.showing();
    let across: Vec<Option<Card>> = duel
        .opposing()
        .iter()
        .map(|o| o.face_up.then(|| o.card.clone()))
        .collect();
    let resolved = resolve_row(duel.row(), &across);
    let plain = row_value(&resolved);
    let bent = perks.iter().fold(plain, |hand, perk| {
        perk.modifier.after_showdown(hand, 0, &resolved)
    });
    f64::from(duel.hand()) + f64::from(bent) - f64::from(plain) - f64::from(showing)
        + duel.bluffs_showing() as f64
        + face_down_worth(duel, &resolved, deck)
}

/// What `row`, resolved against the face-up Opposing Cards, stands to win
/// or lose across the face-down ones, in Chips, each face-down card weighed
/// as any card of `deck`, the enemy's whole Deck (`Duel::enemy_deck`), every
/// card as likely as the next: a Lowball that would muck the card across, a
/// Counterweight that would take its Face Value, a Flop that would read it,
/// a Bluff, and what its own Lowballs, Counterweights, Flops and Bluffs
/// would take off one. The Deck counts the cards it can already see too,
/// so the weighing is rough. A face-down card's own Face Value moves every
/// row by the same amount, so it is left out. The enemy's greedy guesses at
/// the player's row much the same way, with an average card of the
/// player's Deck.
fn face_down_worth(duel: &Duel, row: &[Played], deck: &[Card]) -> f64 {
    if deck.is_empty() {
        return 0.0;
    }
    // A Mucked card is nothing for a Flop to read.
    let standing = |slot: usize| {
        row.get(slot)
            .filter(|p| !p.mucked)
            .map_or(0, |p| p.card.face_value)
    };
    let mut total = 0.0;
    for (slot, _) in duel
        .opposing()
        .iter()
        .enumerate()
        .filter(|(_, o)| !o.face_up)
    {
        let reads = slot.saturating_sub(1)..=slot + 1;
        // Smart's own Flops that read this slot.
        let flops = reads
            .clone()
            .filter_map(|r| row.get(r))
            .filter(|p| p.plays_as == Some(Tell::Flop) && !p.mucked)
            .count() as f64;
        let mine = row.get(slot);
        for card in deck {
            let theirs = card.face_value;
            let own = mine.map_or(0, |p| p.card.face_value);
            // A Mucked card's Tell does nothing, a Bluff's hit included, and
            // smart's Flops reading its slot find it empty.
            if mine.is_some_and(|p| p.plays_as == Some(Tell::Lowball) && own < theirs) {
                total += f64::from(theirs);
                continue;
            }
            total += f64::from(theirs) * flops;
            if card.tell == Some(Tell::Flop) {
                total -= f64::from(reads.clone().map(standing).sum::<u32>());
            }
            let Some(mine) = mine else { continue };
            if card.tell == Some(Tell::Lowball) && theirs < own {
                total -= f64::from(mine.value);
                continue;
            }
            if card.tell == Some(Tell::Counterweight) {
                total -= f64::from(own.saturating_sub(theirs));
            }
            if mine.plays_as == Some(Tell::Counterweight) {
                // Across a face-down card it resolved as its own Face Value,
                // the one an Echo copied included.
                total += f64::from(theirs.saturating_sub(mine.value));
            }
            if card.tell == Some(Tell::Bluff) || mine.plays_as == Some(Tell::Bluff) {
                total += f64::from(own) - f64::from(theirs);
            }
        }
    }
    total / deck.len() as f64
}

/// Every row the Draw can make, by putting cards down and lifting them back
/// off, which is how a player rearranges a row on the table.
fn search(
    duel: &mut Duel,
    perks: &[&Perk],
    deck: &[Card],
    path: &mut Vec<Move>,
    best: &mut (f64, Vec<Move>),
) {
    let draw = duel.draw().to_vec();
    let mut tried: Vec<&Move> = Vec::new();
    let mut moves: Vec<Move> = Vec::new();
    if duel.plays_left() > 0 {
        for (i, card) in draw.iter().enumerate() {
            if needs_sacrifice(duel.row(), card) {
                for (j, burn) in draw.iter().enumerate() {
                    if j != i {
                        moves.push((card.clone(), Some(burn.clone())));
                    }
                }
            } else {
                moves.push((card.clone(), None));
            }
        }
    }
    let mut placed_any = false;
    for mv in &moves {
        // Equal cards make equal rows.
        if tried.contains(&mv) {
            continue;
        }
        tried.push(mv);
        placed_any = true;
        place(duel, mv);
        path.push(mv.clone());
        search(duel, perks, deck, path, best);
        path.pop();
        let last = duel.row().len() - 1;
        duel.lift(last).expect("the card just placed is in the row");
    }
    if !placed_any && !path.is_empty() {
        let s = score(duel, perks, deck);
        if s > best.0 {
            *best = (s, path.clone());
        }
    }
}

fn smart_row(duel: &mut Duel, perks: &[&Perk]) {
    let deck = duel.enemy_deck();
    let mut best = (f64::NEG_INFINITY, Vec::new());
    search(duel, perks, &deck, &mut Vec::new(), &mut best);
    for mv in &best.1 {
        place(duel, mv);
    }
}

/// Whether `player` spends its Items this turn. Naive spends them as soon as
/// it can. Smart holds them until it's behind on Chips, which beat spending
/// at once and saving them for the bosses under #184's first Items (11.7%
/// of runs won against 10.3% for both with `--minion items`: an Item mostly
/// arrives just before a boss).
fn spends_items(duel: &Duel, player: Player) -> bool {
    match player {
        Player::Naive => true,
        Player::Smart => duel.player_chips() < duel.enemy_chips(),
    }
}

/// Spend every Item that can be spent now: the row-time ones before the row
/// is built, so the smart player's search sees what a Reveal turned over or
/// Deep Pockets drew, and Weighted Coin on a Push.
fn spend_items(duel: &mut Duel, when: When) {
    for i in 0..duel.items().len() {
        if duel.items()[i].item.when == when {
            // Refused when it has no uses left or is already working.
            let _ = duel.spend(i);
        }
    }
}

/// Push Your Luck. Naive always Holds. Smart Pushes a clearing Hand only
/// when the double is the kill and the Hold isn't, and a Whiff only when
/// Holding would kill it anyway.
fn pushes(duel: &Duel, player: Player) -> bool {
    match player {
        Player::Naive => false,
        Player::Smart if duel.hand() >= duel.house_edge() => {
            let chips = duel.enemy_chips();
            duel.payout(None) < chips && duel.payout(Some(all_in::combat::duel::Push::Won)) >= chips
        }
        Player::Smart => duel.whiff(None) >= duel.player_chips(),
    }
}

// ---------------------------------------------------------------------------
// Playing a run
// ---------------------------------------------------------------------------

/// Everything counted for one player, per encounter in `RUN` order.
#[derive(Default, Clone)]
struct Stats {
    runs: usize,
    reached: [usize; 5],
    won: [usize; 5],
    stalled: [usize; 5],
    turns: [u64; 5],
    arrive_chips: [u64; 5],
    hand: [u64; 5],
    edge: [u64; 5],
    whiffs: [u64; 5],
    pushes: [u64; 5],
    /// `--draw`: turns counted, and per turn the cards in the Draw, the ones
    /// carried over unplayed from the turn before, how many of those are low,
    /// and how many low ones have sat there two turns or more.
    draw_turns: u64,
    drawn: u64,
    carried: u64,
    carried_low: u64,
    stuck_low: u64,
}

impl Stats {
    fn merge(&mut self, o: &Stats) {
        self.runs += o.runs;
        self.draw_turns += o.draw_turns;
        self.drawn += o.drawn;
        self.carried += o.carried;
        self.carried_low += o.carried_low;
        self.stuck_low += o.stuck_low;
        for i in 0..5 {
            self.reached[i] += o.reached[i];
            self.won[i] += o.won[i];
            self.stalled[i] += o.stalled[i];
            self.turns[i] += o.turns[i];
            self.arrive_chips[i] += o.arrive_chips[i];
            self.hand[i] += o.hand[i];
            self.edge[i] += o.edge[i];
            self.whiffs[i] += o.whiffs[i];
            self.pushes[i] += o.pushes[i];
        }
    }
}

fn play_duel(
    duel: &mut Duel,
    perks: &[&Perk],
    player: Player,
    see_all: bool,
    rng: &mut u64,
    at: usize,
    stats: &mut Stats,
) -> CombatOutcome {
    // The cards left in the Draw after each row, with the turns each has sat
    // there unplayed.
    let mut left: Vec<(Card, u32)> = Vec::new();
    loop {
        if see_all {
            // Against The House there is nothing down to turn over yet.
            let every: Vec<usize> = (0..duel.opposing().len()).collect();
            duel.reveal(&every);
        }
        let mut held: Vec<(Card, u32)> = Vec::new();
        for card in duel.draw() {
            let age = left
                .iter()
                .position(|(c, _)| c == card)
                .map_or(0, |i| left.swap_remove(i).1);
            held.push((card.clone(), age));
        }
        stats.draw_turns += 1;
        stats.drawn += held.len() as u64;
        for (card, age) in &held {
            stats.carried += u64::from(*age > 0);
            stats.carried_low += u64::from(*age > 0 && card.face_value <= LOW);
            stats.stuck_low += u64::from(*age > 1 && card.face_value <= LOW);
        }
        let spends = spends_items(duel, player);
        if spends {
            spend_items(duel, When::Row);
        }
        match player {
            Player::Naive => naive_row(duel, rng),
            Player::Smart => smart_row(duel, perks),
        }
        left = duel
            .draw()
            .iter()
            .map(|card| {
                let age = held
                    .iter()
                    .position(|(c, _)| c == card)
                    .map_or(0, |i| held.swap_remove(i).1);
                (card.clone(), age + 1)
            })
            .collect();
        let result = match duel.confirm() {
            Some(result) => result,
            None => {
                debug_assert_eq!(duel.phase(), Phase::PushYourLuck);
                let push = pushes(duel, player);
                stats.pushes[at] += u64::from(push);
                if push && spends {
                    spend_items(duel, When::Prompt);
                }
                if push { duel.push() } else { duel.hold() }.expect("the prompt is up")
            }
        };
        stats.turns[at] += 1;
        stats.hand[at] += u64::from(result.hand);
        stats.edge[at] += u64::from(result.house_edge);
        stats.whiffs[at] +=
            u64::from(matches!(result.kind, all_in::combat::duel::Outcome::Whiff(n) if n > 0));
        if let Some(outcome) = duel.outcome() {
            return outcome;
        }
        if duel.turn() > TURN_CAP {
            stats.stalled[at] += 1;
            return CombatOutcome::Lost;
        }
    }
}

fn choose(one: Reward, two: Reward, pick: Pick, rng: &mut u64) -> Reward {
    match pick {
        Pick::First => one,
        Pick::Second => two,
        Pick::Random if xorshift64(rng) % 2 == 0 => one,
        Pick::Random => two,
    }
}

/// A Pack whose best card prints under this is one smart rerolls with
/// Second Look: the middle of what a minion's Pack deals (5 to 11).
const REROLL_BELOW: u32 = 8;

/// Which of `offered` the player keeps: naive any of them, smart the
/// rarest, since the stronger Items are the rarer ones. `None` when nothing
/// is offered.
fn keep_item(offered: &[&'static Item], player: Player, rng: &mut u64) -> Option<&'static Item> {
    match player {
        _ if offered.is_empty() => None,
        Player::Naive => Some(offered[(xorshift64(rng) % offered.len() as u64) as usize]),
        Player::Smart => offered.iter().copied().min_by_key(|item| item.weight),
    }
}

/// A minion's reward, taken as the overworld offers it: three Items or a
/// Pack, keep one, or nothing, the Item kept by [`keep_item`]. With Second
/// Look, smart rerolls three common Items, or a Pack with nothing at
/// [`REROLL_BELOW`] or better, while it has rerolls left. Naive never
/// rerolls.
fn take_minion_reward(run: &mut RunState, player: Player, minion: Minion, rng: &mut u64) {
    let minion = match minion {
        Minion::Random => {
            [Minion::Items, Minion::Cards, Minion::Skip][(xorshift64(rng) % 3) as usize]
        }
        chosen => chosen,
    };
    let smart = player == Player::Smart;
    match minion {
        Minion::Items => {
            let mut offered = item::offer(&run.items, 3, xorshift64(rng));
            while smart
                && offered.iter().all(|item| item.rarity() == Rarity::Common)
                && run.reroll()
            {
                offered = item::offer(&run.items, 3, xorshift64(rng));
            }
            if let Some(taken) = keep_item(&offered, player, rng) {
                run.apply(Reward::Item(taken), xorshift64(rng));
            }
        }
        Minion::Cards => {
            let mut pack = Pack::minion(run, xorshift64(rng));
            while smart
                && pack.cards.iter().all(|card| card.face_value < REROLL_BELOW)
                && run.reroll()
            {
                pack = Pack::minion(run, xorshift64(rng));
            }
            let picks = keep(&pack, player, rng);
            assert!(
                run.keep(&pack, &picks),
                "the sim keeps what the Pack allows"
            );
        }
        Minion::Skip | Minion::Random => {}
    }
}

/// Which cards of a Pack the player keeps. The naive player keeps any of
/// them; the smart one keeps the highest Face Values, a Tell breaking a tie.
fn keep(pack: &Pack, player: Player, rng: &mut u64) -> Vec<usize> {
    let mut order: Vec<usize> = (0..pack.cards.len()).collect();
    match player {
        Player::Naive => {
            for i in (1..order.len()).rev() {
                order.swap(i, (xorshift64(rng) % (i as u64 + 1)) as usize);
            }
        }
        Player::Smart => order.sort_by_key(|&i| {
            let card = &pack.cards[i];
            std::cmp::Reverse((card.face_value, card.tell.is_some()))
        }),
    }
    order.truncate(pack.keep);
    order
}

/// The plain cards of `deck` that `trim` takes out, `count` of them.
fn trim_picks(deck: &[Card], count: usize, trim: Trim, rng: &mut u64) -> Vec<usize> {
    let mut plain: Vec<usize> = (0..deck.len())
        .filter(|&i| deck[i].tell.is_none())
        .collect();
    match trim {
        Trim::Lowest | Trim::Swap => plain.sort_by_key(|&i| deck[i].face_value),
        Trim::Highest => plain.sort_by_key(|&i| std::cmp::Reverse(deck[i].face_value)),
        Trim::Random => {
            for i in (1..plain.len()).rev() {
                plain.swap(i, (xorshift64(rng) % (i as u64 + 1)) as usize);
            }
        }
    }
    plain.truncate(count);
    plain
}

/// The run a Legacy Perk and The Wheel start: the perk, `--item`'s Item,
/// the spin, then Trim and Pocket Change. Smart trims its lowest plain
/// cards, or the ones `--trim` says; naive trims nothing. Pocket Change's
/// pick is kept by [`keep_item`]. Every run spins, Wheel or not, so a run
/// under a board plays the same deals as the same run without one.
fn start_run(
    board: &BTreeMap<Square, u8>,
    perk: Option<&LegacyPerk>,
    item: Option<&'static Item>,
    player: Player,
    trim: Trim,
    rng: &mut u64,
) -> RunState {
    let mut run = RunState::new();
    if let Some(perk) = perk {
        legacy::apply(&mut run, perk);
    }
    if let Some(item) = item {
        run.apply(Reward::Item(item), 0);
    }
    let spun = Spun::new(board, wheel::spin(xorshift64(rng)));
    let mut run = wheel::start_run(run, spun, xorshift64(rng));
    if player == Player::Smart {
        // Only the random pick draws from the rng, so the others play the
        // same deals as before.
        let picks = trim_picks(&run.deck, run.wheel.trims(), trim, rng);
        let swapped = picks.len();
        assert!(
            wheel::trim(&mut run, &picks),
            "the sim trims what Trim allows"
        );
        if trim == Trim::Swap {
            run.deck.extend((0..swapped).map(|_| Card {
                name: "Stand-in",
                face_value: SWAP_FOR,
                tell: None,
            }));
        }
    }
    let pick = run.wheel.pocket_pick(xorshift64(rng));
    if let Some(item) = keep_item(&pick, player, rng) {
        run.apply(Reward::Item(item), 0);
    }
    run
}

/// Play one run. A run that beats The House hands back the deck it beat it
/// with, its Flop Pack kept: what Victory Lap starts the next run with.
fn play_run(
    player: Player,
    args: &Args,
    perk: Option<&LegacyPerk>,
    mut rng: u64,
    stats: &mut Stats,
) -> Option<Vec<Card>> {
    stats.runs += 1;
    let mut run = start_run(&args.wheel, perk, args.item, player, args.trim, &mut rng);
    for (at, &encounter) in RUN.iter().enumerate() {
        stats.reached[at] += 1;
        stats.arrive_chips[at] += u64::from(run.chips);
        let mut duel = Duel::for_run(&run, encounter, xorshift64(&mut rng));
        if play_duel(
            &mut duel,
            &run.perks,
            player,
            args.see_all,
            &mut rng,
            at,
            stats,
        ) == CombatOutcome::Won
        {
            stats.won[at] += 1;
            run.chips = duel.player_chips();
        } else if !args.carry || at + 1 == RUN.len() {
            return None;
        }
        run.put_back(duel.items_left());
        run.reflips = duel.reflips_left();
        if let Encounter::Boss { boss, .. } = encounter {
            run.beat(boss);
        }
        match encounter.reward_offer() {
            Some(RewardOffer::ItemsOrPack) => {
                take_minion_reward(&mut run, player, args.minion, &mut rng);
            }
            Some(RewardOffer::Pick(one, two)) => {
                // A boss opens its Pack before the Perk pick, as in the game.
                if let Some(pack) = encounter.boss_pack(&run, xorshift64(&mut rng)) {
                    let picks = keep(&pack, player, &mut rng);
                    assert!(
                        run.keep(&pack, &picks),
                        "the sim keeps what the Pack allows"
                    );
                }
                let reward = choose(one, two, args.pick, &mut rng);
                run.apply(reward, xorshift64(&mut rng));
            }
            Some(RewardOffer::Legacy) => {
                // The House's Flop Pack, then the run is over.
                if let Some(pack) = encounter.boss_pack(&run, xorshift64(&mut rng)) {
                    let picks = keep(&pack, player, &mut rng);
                    assert!(
                        run.keep(&pack, &picks),
                        "the sim keeps what the Pack allows"
                    );
                }
                return Some(run.deck);
            }
            None => {}
        }
    }
    None
}

/// How many decks [`victory_laps`] gathers at most; runs share them round
/// the table after that.
const VICTORY_LAPS: usize = 500;

/// The decks Victory Lap starts runs with: the ones smart runs, under the
/// same Wheel and no Legacy Perk, beat The House with, Flop Pack kept. Only
/// a winning run has one, so naive runs borrow smart's. Gathered on seeds of
/// their own, in seed order, so the set doesn't hang on the thread count.
fn victory_laps(args: &Args) -> Vec<Vec<Card>> {
    let want = VICTORY_LAPS.min(args.runs.max(1));
    let base = args.seed.rotate_left(32);
    let mut decks = Vec::new();
    let mut tried = 0;
    // Smart beats The House about one run in twelve; give up well past that.
    while decks.len() < want && tried < want * 200 {
        let batch = (want - decks.len()) * 16;
        let (_, found) = in_parallel(tried..tried + batch, |i, stats| {
            play_run(Player::Smart, args, None, run_seed(base, i), stats)
        });
        decks.extend(found);
        tried += batch;
    }
    decks.truncate(want);
    decks
}

/// Run `i`'s own seed, so the numbers don't depend on how many threads ran
/// them (splitmix64 over the base seed).
fn run_seed(base: u64, i: usize) -> u64 {
    let mut z = base.wrapping_add((i as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    (z ^ (z >> 31)) | 1
}

/// Play run `i` for every `i` in `runs`, across every thread. Hands back the
/// merged stats and whatever the runs found, in run order.
fn in_parallel<T: Send>(
    runs: std::ops::Range<usize>,
    play: impl Fn(usize, &mut Stats) -> Option<T> + Sync,
) -> (Stats, Vec<T>) {
    let threads = thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(runs.len().max(1));
    let mut total = Stats::default();
    let mut found = Vec::new();
    thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let (runs, play) = (runs.clone(), &play);
                s.spawn(move || {
                    let mut stats = Stats::default();
                    let mut found = Vec::new();
                    for i in runs.skip(t).step_by(threads) {
                        if let Some(x) = play(i, &mut stats) {
                            found.push((i, x));
                        }
                    }
                    (stats, found)
                })
            })
            .collect();
        for h in handles {
            let (stats, f) = h.join().expect("a sim thread panicked");
            total.merge(&stats);
            found.extend(f);
        }
    });
    found.sort_by_key(|(i, _)| *i);
    (total, found.into_iter().map(|(_, x)| x).collect())
}

/// `laps` are Victory Lap's decks, dealt round the runs; empty unless
/// `--legacy victory-lap`.
fn simulate(player: Player, args: &Args, laps: &[Vec<Card>]) -> Stats {
    in_parallel(0..args.runs, |i, stats| {
        let perk = match &args.legacy {
            Some(LegacyPerk::VictoryLap { .. }) => {
                Some(LegacyPerk::victory_lap(&laps[i % laps.len()]))
            }
            other => other.clone(),
        };
        play_run(player, args, perk.as_ref(), run_seed(args.seed, i), stats);
        None::<()>
    })
    .0
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

fn label(encounter: Encounter) -> &'static str {
    match encounter {
        Encounter::Boss { boss, .. } => boss.name,
        Encounter::Minion {
            floor: Floor::TheFloor,
        } => "Floor minion",
        Encounter::Minion {
            floor: Floor::ThePit,
        } => "Pit minion",
        Encounter::Minion {
            floor: Floor::BigShotsTable,
        } => "Big Shots minion",
        Encounter::Practice => "Practice",
    }
}

fn per(n: u64, d: u64) -> f64 {
    if d == 0 { 0.0 } else { n as f64 / d as f64 }
}

fn report(player: Player, s: &Stats) {
    let wins = s.won[4];
    let depth: usize = s.won.iter().sum();
    println!(
        "{}: win {:.1}%  mean encounters won {:.2}  ({} runs)",
        player.name(),
        100.0 * per(wins as u64, s.runs as u64),
        per(depth as u64, s.runs as u64),
        s.runs
    );
    println!(
        "  {:<13}{:>8}{:>7}{:>8}{:>8}{:>7}{:>7}{:>8}{:>8}",
        "encounter", "reached", "won%", "chips", "turns", "hand", "edge", "whiff%", "push%"
    );
    for (at, &encounter) in RUN.iter().enumerate() {
        let reached = s.reached[at] as u64;
        let turns = s.turns[at];
        println!(
            "  {:<13}{:>8}{:>7.1}{:>8.1}{:>8.1}{:>7.1}{:>7.1}{:>8.1}{:>8.1}{}",
            label(encounter),
            reached,
            100.0 * per(s.won[at] as u64, reached),
            per(s.arrive_chips[at], reached),
            per(turns, reached),
            per(s.hand[at], turns),
            per(s.edge[at], turns),
            100.0 * per(s.whiffs[at], turns),
            100.0 * per(s.pushes[at], turns),
            if s.stalled[at] > 0 {
                format!("  ({} stalled)", s.stalled[at])
            } else {
                String::new()
            },
        );
    }
}

/// `--draw`: how much of the Draw is cards that were there last turn too.
fn report_draw(s: &Stats) {
    let turns = s.draw_turns;
    println!(
        "  Draw per turn: {:.2} cards, {:.2} held over unplayed, {:.2} of them low (face value {LOW} or less), {:.2} low ones held two turns or more",
        per(s.drawn, turns),
        per(s.carried, turns),
        per(s.carried_low, turns),
        per(s.stuck_low, turns),
    );
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("{e}");
            }
            eprintln!("{USAGE}");
            std::process::exit(if e.is_empty() { 0 } else { 2 });
        }
    };
    let wheel = if args.wheel.is_empty() {
        "no Wheel".to_string()
    } else {
        let squares: Vec<String> = args
            .wheel
            .iter()
            .map(|(square, rank)| format!("{} {rank}", square.name()))
            .collect();
        format!("Wheel {}", squares.join(", "))
    };
    let laps = match args.legacy {
        Some(LegacyPerk::VictoryLap { .. }) => {
            let laps = victory_laps(&args);
            if laps.is_empty() {
                eprintln!("--legacy victory-lap: no smart run beat The House to give it a deck");
                std::process::exit(1);
            }
            laps
        }
        _ => Vec::new(),
    };
    let perk = match &args.legacy {
        Some(LegacyPerk::VictoryLap { .. }) => {
            format!(", Legacy Victory Lap ({} winning decks)", laps.len())
        }
        Some(perk) => format!(", Legacy {}", perk.name()),
        None => String::new(),
    };
    let item = args.item.map_or(String::new(), |item| {
        format!(", starting with {}", item.name)
    });
    let trim = if args.trim == Trim::Lowest {
        String::new()
    } else {
        format!(", Trim takes {:?}", args.trim)
    };
    println!(
        "all-in sim: {} runs per player, seed {}, boss picks {:?}, {wheel}{trim}{perk}{item}{}{}\n",
        args.runs,
        args.seed,
        args.pick,
        if args.carry {
            ", losses carried to The House"
        } else {
            ""
        },
        if args.see_all {
            ", every Opposing Card face up"
        } else {
            ""
        }
    );
    for &player in &args.players {
        let stats = simulate(player, &args, &laps);
        report(player, &stats);
        if args.draw {
            report_draw(&stats);
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use all_in::run::Enemy;

    fn card(face_value: u32, tell: Option<Tell>) -> Card {
        Card {
            name: "card",
            face_value,
            tell,
        }
    }

    /// Smart holding `draw` at Blind 2, against an enemy whose whole Deck is
    /// `enemy` and whose row is `across`, each card face up or not.
    fn smart_plays(draw: Vec<Card>, enemy: Vec<Card>, across: Vec<(Card, bool)>) -> Vec<u32> {
        let enemy = Enemy {
            name: "shill",
            chips: 99,
            deck: enemy,
            table_rule: None,
        };
        let mut duel = Duel::new(draw, 40, 2, enemy).with_opposing(across);
        smart_row(&mut duel, &[]);
        duel.row().iter().map(|p| p.card.face_value).collect()
    }

    #[test]
    fn smart_keeps_its_big_card_away_from_a_face_down_card_that_could_be_a_lowball() {
        // Every card the enemy owns is a Lowball 1, so whatever sits across
        // the face-down card is Mucked; the slot past the enemy's row is safe.
        let lowball = card(1, Some(Tell::Lowball));
        let played = smart_plays(
            vec![card(2, None), card(9, None)],
            vec![lowball.clone(); 20],
            vec![(lowball, false)],
        );

        assert_eq!(played, vec![2, 9]);
    }

    #[test]
    fn smart_sets_its_big_card_against_a_face_up_bluff() {
        // The 9 across the Bluff 6 takes 3 off the enemy; the 2 would cost 4.
        let played = smart_plays(
            vec![card(9, None), card(2, None)],
            vec![card(6, None); 20],
            vec![(card(6, Some(Tell::Bluff)), true), (card(6, None), true)],
        );

        assert_eq!(played, vec![9, 2]);
    }

    #[test]
    fn smart_counts_no_flop_on_a_face_down_card_its_lowball_mucks() {
        // The Lowball mucks the face-down 9 either way. A Flop beside it
        // then reads an empty slot, so the 5 is the better second card.
        let played = smart_plays(
            vec![
                card(1, Some(Tell::Lowball)),
                card(2, Some(Tell::Flop)),
                card(5, None),
            ],
            vec![card(9, None); 20],
            vec![(card(9, None), false)],
        );

        assert_eq!(played, vec![1, 5]);
    }
}
