//! The balance sim (#94): whole runs against the real `Duel`, headless, so a
//! change to a card, an enemy or a reward can be read as a win rate before it
//! ships. Every duel starts at `Duel::for_run`, the same door the game uses,
//! and walks the same `RUN` the overworld does.
//!
//! Two players, after the starter-deck prototype: **naive** plays its
//! biggest cards in a random order and never Pushes; **smart** tries every
//! row it can build from the Draw and Pushes only when it can't lose by it.
//! Real players land in between. See `tools/sim/README.md`.

use std::thread;

use all_in::combat::duel::{Duel, Phase};
use all_in::overworld::progression::RUN;
use all_in::run::{
    Card, CombatOutcome, EncounterId, Enemy, Reward, RewardOffer, RunState, Tell, xorshift64,
};

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

struct Args {
    runs: usize,
    seed: u64,
    players: Vec<Player>,
    pick: Pick,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        runs: 2000,
        seed: 0x9e37_79b9_7f4a_7c15,
        players: vec![Player::Naive, Player::Smart],
        pick: Pick::Random,
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
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("unknown flag {other}")),
        }
    }
    Ok(args)
}

const USAGE: &str = "usage: cargo run --release -p sim -- [--runs N] [--seed N] \
[--player naive|smart|both] [--pick first|second|random]";

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
        let burn = if card.tell == Some(Tell::AllIn) {
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
/// it can see of the Opposing Cards. Face-down cards are the same whatever
/// it plays, so leaving them out moves every row by the same amount.
///
/// Against The House it also knows the Hole Card rule: the row it can't see
/// is set to everything but the last card plus the margin, so `prefix` (The
/// Hand before the last card went down) is what that read comes to.
fn score(duel: &Duel, prefix: u32) -> i64 {
    let (showing, _) = duel.showing();
    let edge = match duel.margin() {
        Some(margin) => showing.max(prefix + margin),
        None => showing,
    };
    i64::from(duel.hand()) - i64::from(edge)
}

/// Every row the Draw can make, by putting cards down and lifting them back
/// off, which is how a player rearranges a row on the table.
fn search(duel: &mut Duel, path: &mut Vec<Move>, prefix: u32, best: &mut (i64, Vec<Move>)) {
    let draw = duel.draw().to_vec();
    let mut tried: Vec<&Move> = Vec::new();
    let mut moves: Vec<Move> = Vec::new();
    if duel.plays_left() > 0 {
        for (i, card) in draw.iter().enumerate() {
            if card.tell == Some(Tell::AllIn) {
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
    let hand_before = duel.hand();
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
        search(duel, path, hand_before, best);
        path.pop();
        let last = duel.row().len() - 1;
        duel.lift(last).expect("the card just placed is in the row");
    }
    if !placed_any && !path.is_empty() {
        let s = score(duel, prefix);
        if s > best.0 {
            *best = (s, path.clone());
        }
    }
}

fn smart_row(duel: &mut Duel) {
    let mut best = (i64::MIN, Vec::new());
    search(duel, &mut Vec::new(), 0, &mut best);
    for mv in &best.1 {
        place(duel, mv);
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
}

impl Stats {
    fn merge(&mut self, o: &Stats) {
        self.runs += o.runs;
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
    player: Player,
    rng: &mut u64,
    at: usize,
    stats: &mut Stats,
) -> CombatOutcome {
    loop {
        match player {
            Player::Naive => naive_row(duel, rng),
            Player::Smart => smart_row(duel),
        }
        let result = match duel.confirm() {
            Some(result) => result,
            None => {
                debug_assert_eq!(duel.phase(), Phase::PushYourLuck);
                let push = pushes(duel, player);
                stats.pushes[at] += u64::from(push);
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

fn choose(offer: RewardOffer, pick: Pick, rng: &mut u64) -> Reward {
    match offer {
        RewardOffer::Drop(reward) => reward,
        RewardOffer::Pick(one, two) => match pick {
            Pick::First => one,
            Pick::Second => two,
            Pick::Random if xorshift64(rng) % 2 == 0 => one,
            Pick::Random => two,
        },
    }
}

fn play_run(player: Player, pick: Pick, mut rng: u64, stats: &mut Stats) {
    stats.runs += 1;
    let mut run = RunState::new();
    for (at, &id) in RUN.iter().enumerate() {
        stats.reached[at] += 1;
        stats.arrive_chips[at] += u64::from(run.chips);
        let mut duel = Duel::for_run(&run, Enemy::for_encounter(id), xorshift64(&mut rng));
        if play_duel(&mut duel, player, &mut rng, at, stats) == CombatOutcome::Lost {
            return;
        }
        stats.won[at] += 1;
        run.chips = duel.player_chips();
        run.set_loaded_dice(duel.dice_left());
        if let Some(offer) = RewardOffer::for_encounter(id) {
            let reward = choose(offer, pick, &mut rng);
            run.apply(reward, xorshift64(&mut rng));
        }
    }
}

/// Run `i`'s own seed, so the numbers don't depend on how many threads ran
/// them (splitmix64 over the base seed).
fn run_seed(base: u64, i: usize) -> u64 {
    let mut z = base.wrapping_add((i as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    (z ^ (z >> 31)) | 1
}

fn simulate(player: Player, args: &Args) -> Stats {
    let threads = thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(args.runs.max(1));
    let mut total = Stats::default();
    thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                s.spawn(move || {
                    let mut stats = Stats::default();
                    for i in (t..args.runs).step_by(threads) {
                        play_run(player, args.pick, run_seed(args.seed, i), &mut stats);
                    }
                    stats
                })
            })
            .collect();
        for h in handles {
            total.merge(&h.join().expect("a sim thread panicked"));
        }
    });
    total
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

fn label(id: EncounterId) -> &'static str {
    match id {
        EncounterId::FloorMinion => "Floor minion",
        EncounterId::Slotz => "Slotz",
        EncounterId::PitMinion => "Pit minion",
        EncounterId::PitBoss => "Pit Boss",
        EncounterId::TheHouse => "The House",
        EncounterId::Tutorial => "Tutorial",
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
    for (at, &id) in RUN.iter().enumerate() {
        let reached = s.reached[at] as u64;
        let turns = s.turns[at];
        println!(
            "  {:<13}{:>8}{:>7.1}{:>8.1}{:>8.1}{:>7.1}{:>7.1}{:>8.1}{:>8.1}{}",
            label(id),
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
    println!(
        "all-in sim: {} runs per player, seed {}, boss picks {:?}\n",
        args.runs, args.seed, args.pick
    );
    for &player in &args.players {
        report(player, &simulate(player, &args));
        println!();
    }
}
