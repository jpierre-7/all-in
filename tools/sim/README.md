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

The same seed gives the same numbers, whatever the thread count, so a diff
against the baseline is the change you made and nothing else.

## What it plays

A run is `overworld::progression::RUN`, the five encounters the game walks:
the Floor minion, Slotz, the Pit minion, the Pit Boss, The House. Each duel
starts at `Duel::for_run`, the same function the game calls, so the deck,
the floor's Blind, the Perks and Loaded Dice come from the `RunState` exactly
as they would in play, and the enemy commits its row the way it does in play.
Between encounters the run takes its reward through `Encounter::reward_offer`
and `RunState::apply`, and a beaten boss unlocks its Boss Tell. A minion's
drop is automatic, and `--pick` decides which side of a boss's 1-of-2 the
player takes (the first is the perk: Slotz's coin, the Pit Boss's +1 Blind).

Two players, after the starter-deck prototype:

- **naive** plays the biggest cards it can, in a random order, with each All
  In burning the smallest card left over. It always Holds.
- **smart** tries every row the Draw can make (every order, every All In
  sacrifice) and plays the one with the best Hand over what it can see of
  the Opposing Cards. It Pushes a clearing Hand only when the doubled Payout is the kill and the Hold
  isn't, and Pushes a Whiff only when Holding would kill it anyway.

Smart sees only what a player sees: face-down Opposing Cards count for
nothing, and so does a Flop's read of them. Real players land between the
two. A duel still going after 300 turns is a stall, and it is counted as a
loss and flagged in the table.

New Tells need nothing here: both players read cards through the `Duel`.
A new decision does need a player model, like a new prompt or a choice
that isn't a 1-of-2.

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

`main` at 3a8da7a (Flop with neighbours, #149), `--runs 20000`, default seed,
random picks:

```
naive: win 0.0%  mean encounters won 2.00  (20000 runs)
  encounter     reached   won%   chips   turns   hand   edge  whiff%   push%
  Floor minion    20000  100.0    50.0     4.1   17.8   10.9     2.2     0.0
  Slotz           20000   99.1    49.8     8.6   21.4   18.3    25.1     0.0
  Pit minion      19828    1.0    41.6     8.3   25.2   29.9    68.1     0.0
  Pit Boss          190    0.0    17.2     4.0   26.2   30.8    69.3     0.0
  The House           0    0.0     0.0     0.0    0.0    0.0     0.0     0.0

smart: win 8.3%  mean encounters won 2.84  (20000 runs)
  encounter     reached   won%   chips   turns   hand   edge  whiff%   push%
  Floor minion    20000   99.2    50.0     2.5   24.7   10.9    32.8    59.1
  Slotz           19842   91.3    41.9     3.4   27.3   16.2    27.0    41.7
  Pit minion      18110   71.7    33.6     5.4   31.3   26.8    31.4    20.3
  Pit Boss        12984   32.0    25.9     7.5   33.0   32.3    40.5    19.4
  The House        4151   39.9    19.1     6.6   25.6   22.9    31.3    21.5
```

Smart with fixed picks: `--pick first` 3.4%, `--pick second` 14.2%.
