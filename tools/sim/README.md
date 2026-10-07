# The balance sim

Plays whole runs of All In headless, against the real `Duel`, and prints how
far players get. Every feature is tuned here before it ships: change a card,
an enemy or a reward, run the sim, and compare against the baseline below.

```sh
cargo run --release -p sim                    # 2000 runs each for naive and smart
cargo run --release -p sim -- --runs 20000    # the baseline's size, about 40 s on 16 threads
cargo run --release -p sim -- --player smart --pick second
```

Use `--release`. The debug build works (CI runs 20 runs to keep it honest),
but it's too slow to tune with.

| Flag       | Values                                | Default     |
| ---------- | ------------------------------------- | ----------- |
| `--runs`   | runs per player                       | 2000        |
| `--seed`   | any `u64`                             | fixed       |
| `--player` | `naive`, `smart`, `both`              | `both`      |
| `--pick`   | `first`, `second`, `random`           | `random`    |
| `--minion` | `items`, `cards`, `skip`, `random`    | `random`    |
| `--wheel`  | `square=rank,...`, e.g. `bankroll=3,lucky-coin=2` | no Wheel |

The same seed gives the same numbers, whatever the thread count, so a diff
against the baseline is the change you made and nothing else.

## What it plays

A run is `overworld::progression::RUN`, the five encounters the game walks:
the Floor minion, Slotz, the Pit minion, the Pit Boss, The House. Each duel
starts at `Duel::for_run`, the same function the game calls, so the deck,
the floor's Blind, the Perks and the Items come from the `RunState` exactly
as they would in play, and the enemy commits its row the way it does in play.
Between encounters the run takes its reward through `Encounter::reward_offer`
and `RunState::apply`, and a beaten boss unlocks its Boss Tell. After a
minion, `--minion` picks Items, cards or nothing (`random` is each a third
of the time). Items deals three with `item::offer`: naive keeps any, smart
the rarest, since the stronger Items are rarer. Cards opens `Pack::minion`
and keeps one the same way as a Boss Pack, below. A boss opens its Boss Pack first (`Encounter::boss_pack`,
`RunState::keep`): naive keeps two at random, smart the two highest Face
Values. Then `--pick` decides which side of the boss's 1-of-2 the player
takes (Slotz's first is its coin, the Pit Boss's first Beam Swings Your
Way and its second Read the Pan).

With `--wheel`, every run starts under that board the way the game starts
one once The Wheel is open (`wheel::start_run`): the spin picks a Hot Square
off the run's seed, then Trim and Pocket Change put their choices. Smart
trims its lowest plain cards and naive trims nothing; both take the first
Item Pocket Change offers. Squares are named in lower case with dashes
(`early-read`), ranks 1 to 3, and only the squares that do something yet are
accepted.

Two players, after the starter-deck prototype:

- **naive** plays the biggest cards it can, in a random order, with each All
  In burning the smallest card left over. It always Holds.
- **smart** tries every row the Draw can make (every order, every All In
  sacrifice) and plays the one with the best Hand over what it can see of
  the Opposing Cards. It Pushes a clearing Hand only when the doubled Payout is the kill and the Hold
  isn't, and Pushes a Whiff only when Holding would kill it anyway.

Smart sees only what a player sees: face-down Opposing Cards count for
nothing, and so does a Flop's read of them. The one guess it makes is
across a Lowball or a Counterweight: a face-down card there counts as the
enemy Deck's average Face Value (`Duel::enemy_average`), mucked if the
Lowball prints under it and taken if the Counterweight prints under it,
the way the enemy's greedy guesses at the player's row. Real players land between the
two. A duel still going after 300 turns is a stall, and it is counted as a
loss and flagged in the table.

New Tells mostly need nothing here: both players read cards through the
`Duel`. A Tell whose worth hangs on a face-down card (Lowball, Counterweight)
needs smart's `score` to guess at it.
A new decision does need a player model, like a new prompt or a choice
that isn't a 1-of-2.

Items are spent on demand in the game. Both players spend every Item they
hold as soon as they can: the row-time ones before building the row, so
smart's search sees what a Reveal showed or Deep Pockets drew, and Weighted
Coin on every Push. Each Item's uses and offer weight are fields on its
static in `src/item.rs`, for the sim to tune.

## Reading the output

```
smart: win 8.3%  mean encounters won 2.84  (20000 runs)
  encounter     reached   won%   chips   turns   hand   edge  whiff%   push%
  Floor minion    20000   99.2    50.0     2.5   24.7   10.9    32.8    59.1
```

- **win**: runs that beat The House. **mean encounters won**: out of five.
- **reached**: runs that sat down at this encounter. **won%**: of those.
- **chips**: the player's mean Chips on arrival.
- **turns**: mean turns per duel.
- **hand** / **edge**: mean Hand and House Edge per turn, as the rows turned
  over. Edge is the enemy's real row, face-down cards included.
- **whiff%**: turns where the player paid. A lost Push on a clearing Hand
  counts.
- **push%**: turns the player Pushed.

## Baseline

`main` at 9f5aa3f with the Pit Boss built (#161), `--runs 20000`, default
seed, random picks:

```
naive: win 0.0%  mean encounters won 0.97  (20000 runs)
  encounter     reached   won%   chips   turns   hand   edge  whiff%   push%
  Floor minion    20000   95.5    50.0    11.6   10.6    9.2    29.0     0.0
  SLOTZ           19091    1.7    42.2    16.5   11.9   14.0    71.9     0.0
  Pit minion        322    3.7    38.5     9.6   15.8   19.4    74.8     0.0
  THE PIT BOSS       12    0.0    18.7     5.3   17.6   20.9    81.2     0.0
  THE HOUSE           0    0.0     0.0     0.0    0.0    0.0     0.0     0.0

smart: win 9.2%  mean encounters won 2.66  (20000 runs)
  encounter     reached   won%   chips   turns   hand   edge  whiff%   push%
  Floor minion    20000   99.5    50.0     4.6   15.2    9.3    26.5    28.4
  SLOTZ           19895   92.7    42.1    10.9   16.5   14.0    28.8    10.4
  Pit minion      18438   49.9    30.2     8.3   21.3   20.3    40.3    13.9
  THE PIT BOSS     9197   42.1    19.9     8.2   23.7   22.2    37.8    15.1
  THE HOUSE        3875   47.4    12.4     3.7   31.4   26.9    34.3    30.2
```

Smart with fixed picks: `--pick first` 10.8%, `--pick second` 8.0%. With
The Beam Swings Back switched off, smart beats the Pit Boss 66.0% of the
time instead of 42.1%, and wins 16.1% of runs.

The worktrees share one `target/`, and every worktree's sim builds to the
same `target/release/sim`, so `cargo run` can start a binary built from
another worktree's source and call it fresh. Touch `src/lib.rs` and
`tools/sim/src/main.rs` before a run that a baseline will be taken from.
