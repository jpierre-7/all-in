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
| `--carry`  | (no value)                            | off         |
| `--legacy` | `high-limit`, `marked-cards`, `lucky-streak`, `victory-lap` | none |
| `--item`   | an Item, e.g. `loaded-dice`, `card-sharks-sunglasses` | none |
| `--see-all` | (no value)                           | off         |
| `--trim`   | `lowest`, `random`, `highest`, `swap` (which plain cards smart's Trim takes out) | `lowest` |
| `--draw`   | (no value): also report what sits unplayed in the Draw | off |

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
takes (Slotz's first is its coin and its second Jackpot, the Pit Boss's
first Beam Swings Your Way and its second Read the Pan).

With `--wheel`, every run starts under that board the way the game starts
one once The Wheel is open (`wheel::start_run`): the spin picks a Hot Square
off the run's seed, then Trim and Pocket Change put their choices. Smart
trims its lowest plain cards and naive trims nothing; Pocket Change's pick
is kept the way a minion's Items are. Every run spins, Wheel or not, so a
run under a board plays the same deals as the same run without one. Fat
Pack needs nothing from the players: the minion's Pack just shows more
cards. With Second Look's rerolls, smart
rerolls a minion's three Items when all of them are common, and its Pack
unless it holds an 11, until it has none left; naive never rerolls
(`--reroll` and `--reroll-items` try other rules, see Second Look's
rerolls). Squares are named in lower case with dashes
(`early-read`), ranks 1 to 3, and only the squares that do something yet are
accepted.

With `--legacy`, every run starts under that Legacy Perk the way the game
starts the run after a House win (`legacy::apply`), before The Wheel's
spin. Victory Lap needs a deck that beat The House, so the sim first plays
smart runs of its own (same Wheel, no Legacy Perk, seeds of their own) and
keeps the decks of the ones that win, Flop Pack included, up to 500 of
them; each run then starts with one of those, naive runs too. A run that
beats The House opens its Flop Pack as in the game (`RewardOffer::Legacy`);
the pick itself has no effect on the run's numbers.

With `--item`, every run starts holding that Item with every use it comes
with, before any Wheel. With `--carry --minion skip` it is the only Item
in the run, which is how each Item's worth below was weighed.

With `--see-all`, every Opposing Card turns face up at the start of every
turn, before any Item is spent: the most any Reveal could show, so the
most one could be worth (#191). Against The House, which plays last, it
does nothing, the same as a real Reveal. Naive never reads the enemy's
row, so it only moves smart.

With `--carry`, a duel lost before The House doesn't end the run: the
player goes on at the Chips it sat down with and takes the reward as if it
had won, so every run reaches The House. Few runs reach it otherwise, so
this is how The House is tuned. A carried run is weaker than one that got
there by winning (it lost somewhere on the way), so The House's won% reads
lower than a real arrival's.

Two players, after the starter-deck prototype:

- **naive** plays the biggest cards it can, in a random order, with each All
  In burning the smallest card left over. It always Holds.
- **smart** tries every row the Draw can make (every order, every All In
  sacrifice) and plays the one with the best Hand over what it can see of
  the Opposing Cards, counting what its Perks add once the row resolves
  (Jackpot). It Pushes a clearing Hand only when the doubled Payout is the kill and the Hold
  isn't, and Pushes a Whiff only when Holding would kill it anyway.

Smart sees only what a player sees. A face-up Opposing Card it reads as
it is, Bluffs included (`Duel::bluffs_showing`). A face-down one it
weighs as any card of the enemy's whole Deck (`Duel::enemy_deck`), each
as likely as the next: what a hidden Lowball, Counterweight, Flop or Bluff
would do to the card across from it, and what its own would take off one.
So it keeps its biggest cards away from the face-down cards when it can,
in the slots past the end of the enemy's row. The Deck counts the cards
it can already see too, so the weighing is rough. The enemy's greedy
guesses at the player's row much the same way. Real players land between
the two. A duel still going after 300 turns is a stall, and it is
counted as a loss and flagged in the table.

New Tells mostly need nothing here: both players read cards through the
`Duel`. A Tell whose worth hangs on a face-down card (Lowball, Counterweight,
Flop, Bluff) needs smart's `face_down_worth` to weigh it.
A new decision does need a player model, like a new prompt or a choice
that isn't a 1-of-2.

Items are spent on demand in the game. Naive spends every Item it holds as
soon as it can. Smart holds them until it's behind on Chips (fewer than the
enemy), then spends all it can: the row-time ones before building the row,
so its search sees what a Reveal showed or Deep Pockets drew, and Weighted
Coin on a Push. Holding beat spending at once (11.7% against 10.3% of runs
won with `--minion items`, under #184's first Items). Each Item's uses and
offer weight are fields on its static in `src/item.rs`.

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

`main` at e345786 (the tuning pass, #184) with smart's reading of the
enemy's row from #191, `--runs 20000`, default seed, random picks:

```
naive: win 0.0%  mean encounters won 1.07  (20000 runs)
  encounter     reached   won%   chips   turns   hand   edge  whiff%   push%
  Floor minion    20000   96.6    50.0    11.2   10.8    9.2    25.9     0.0
  SLOTZ           19315    9.3    43.1    12.3   12.0   14.3    61.2     0.0
  Pit minion       1792   16.0    32.8    10.2   16.4   18.5    62.7     0.0
  THE PIT BOSS      286    1.7    18.1     5.8   18.7   21.5    71.6     0.0
  THE HOUSE           5    0.0    12.2     1.6   25.6   35.0    87.5     0.0

smart: win 14.0%  mean encounters won 3.08  (20000 runs)
  encounter     reached   won%   chips   turns   hand   edge  whiff%   push%
  Floor minion    20000   99.6    50.0     4.5   15.5    9.3    25.2    29.6
  SLOTZ           19920   91.1    42.4     5.4   17.0   12.3    32.1    26.5
  Pit minion      18153   72.3    35.0     7.0   22.3   19.4    34.7    15.5
  THE PIT BOSS    13129   58.3    25.5     7.7   24.6   22.3    37.3    14.2
  THE HOUSE        7657   36.6    16.6     4.9   32.9   31.7    40.9    26.4
```

Smart with fixed picks: `--pick first` 15.0%, `--pick second` 13.4%.

With `--carry`, The House row: smart 35.8% won (arriving with 17.5 Chips),
naive 0.7% (37.2 Chips).

No duel stalled. Before #191 smart won 13.6%: it counted a face-down card
as nothing, except across its own Lowball or Counterweight, and never
counted a Bluff (see Reveals, below). Naive is unchanged. Before #184
smart won 8.8% on a4e1f97; what moved it is below. Every table but
Reveals was measured before #191, against 13.6%.

### Minion rewards

Smart, `--minion` fixed:

| `--minion` | smart win |
| ---------- | --------- |
| `items`    | 17.1%     |
| `cards`    | 14.6%     |
| `skip`     | 9.1%      |

Before #184 a Pack (2 to 8) was worth no more than skipping. It now prints
5 to 11 (`MINION_PACK_FACES` in `src/run.rs`), above the starter deck.

### Items

Each Item on its own: `--item <name> --minion skip --carry`, smart, mean
encounters won of five, against 3.42 with no Item. The rarer an Item, the
more one is worth.

| Item                    | Uses | Weight | Rarity   | encounters won |
| ----------------------- | ---- | ------ | -------- | -------------- |
| Shiny Card Sleeve       | 1    | 3      | rare     | +0.16          |
| Deep Pockets            | 2    | 4      | rare     | +0.14          |
| Sleight of Hand         | 2    | 5      | uncommon | +0.14          |
| Ace Up the Sleeve       | 2    | 6      | uncommon | +0.10          |
| Loaded Dice             | 2    | 6      | uncommon | +0.07          |
| Insurance               | 4    | 8      | common   | +0.04          |
| Shaved Card             | 3    | 8      | common   | +0.02          |
| Weighted Coin           | 2    | 8      | common   | +0.02          |
| Card Shark's Sunglasses | 3    | 10     | common   | +0.01          |
| Two-Way Mirror          | 1    | 10     | common   | +0.01          |

Before #184 Deep Pockets (1 use, +0.06) was rare and the Two-Way Mirror
(weight 5) uncommon, so smart, keeping the rarest, took the Mirror over
Loaded Dice. Sleight of Hand, Deep Pockets, Shaved Card and Insurance got
more uses; Loaded Dice moved up to uncommon and the Mirror down to common.
Weighted Coin is only spent on smart's few Pushes, so its uses barely
matter here.

The two Reveals are worth next to nothing, and they still are under
#191's smart: see Reveals, next.

### Reveals

What a Reveal could be worth at most (#191): `--see-all`, every Opposing
Card face up every turn, against no Reveal. Smart, before and after #191
changed how it reads the enemy's row:

| smart                     | before #191 | after #191 |
| ------------------------- | ----------- | ---------- |
| no Reveal                 | 13.6%       | 14.0%      |
| `--see-all`               | 19.1%       | 20.0%      |
| `--legacy marked-cards`   | 15.7%       | 17.3%      |
| `--wheel early-read=1`    | 13.9%       | 14.5%      |
| `--wheel early-read=2`    | 14.2%       | 14.9%      |
| `--wheel early-read=3`    | 14.3%       | 15.3%      |

The Items, `--minion skip --carry`, mean encounters won against 3.44 with
no Item (3.42 before #191): Card Shark's Sunglasses +0.01, Two-Way Mirror
+0.01, and `--see-all` +0.22. Read the Pan has no number of its own yet.

**The player, not the game** (#191). Seeing the whole row every turn is
worth 6 points of runs won, about what Bankroll at rank 2 is worth, so the
enemy's row has something to act on. On the turns where seeing it changed
smart's row (counted once, before #191), a hidden Lowball was there to
muck the card across, or a hidden Counterweight (the Pit Boss's) to take
its Face Value, nearly every time.

Before #191 smart counted a face-down card as nothing, except across its
own Lowball or Counterweight, so it never kept a big card away from one;
and it never counted a Bluff, face up or not. Weighing each face-down card
as any card of the enemy's Deck is worth the 0.4 points of the new
baseline; counting the Bluffs moves nothing there, since without a Reveal
nothing is face up, but it is worth 0.9 points under `--see-all` (19.1%
without them).

Played without seeing, nothing says which slot holds the Lowball, so no
guess can get most of the 6 points back, and a turn's whole row seen is
worth under a Chip on average. A Reveal is worth what it turns over:
Marked Cards, one card every turn, gets half of `--see-all`; Early Read
climbs with its turns; the Items turn over one turn's row, or one card, a
few times a run, and are worth next to nothing for it.

### The minion template

`MinionTable` in `src/run.rs`. Smart, against the Pit minion at 32 Chips:

| Pit minion          | Pit minion won% | Pit Boss won% | smart win |
| ------------------- | --------------- | ------------- | --------- |
| 4 to 7 (before)     | 63.9            | 54.2          | 11.3%     |
| 4 to 7, 26 Chips    | 67.5            | 55.1          | 12.3%     |
| **3 to 7**          | 71.2            | 56.6          | 13.2%     |
| 3 to 7, 28 Chips    | 72.8            | 56.9          | 14.1%     |
| 4 to 6              | 77.0            | 57.8          | 14.9%     |

Taken with #184's Items and Pack, before every run spun. The Pit minion
was as hard as the Pit Boss; printing from 3 puts it between Slotz and the
Pit Boss, and Chips are left alone. The Floor minion is left as it is: it
costs about 8 Chips and is the first fight of the run. Size and Tell share
are unchanged on both. The Big Shots Table's minion isn't in `RUN` yet.

### The Blind ladder

The Floor 2, The Pit 3, The Big Shots Table 4, the same all duel. Duels
still end on time: smart's last 4.5 to 7.8 turns and naive's at most 12.3,
and none stalls. A steeper ladder only shortens them (`3/4/5`: smart about 15%,
Slotz over in 3 turns), so it stays.

### Legacy Perks

`--legacy`, no Wheel, against 13.6% with none:

| `--legacy`     | smart win | naive win |
| -------------- | --------- | --------- |
| `high-limit`   | 24.2%     | 0.0%      |
| `lucky-streak` | 24.0%     | 0.0%      |
| `marked-cards` | 15.7%     | 0.0%      |
| `victory-lap`  | 15.3%     | 0.0%      |

High Limit as first designed, +1 Blind all run, was 40.2% against 8.3%
(#168), so it was cut to the first two turns of each duel. Lucky Streak at
60/40 was 29.5%; it is 56/44 now (#184). Marked Cards is a Reveal (see
Reveals), and Victory Lap is a deck, so neither has a number to turn.

### The Wheel

Smart, one square at a time, against 13.6% with no Wheel:

| `--wheel`       | rank 1 | rank 2 | rank 3 |
| --------------- | ------ | ------ | ------ |
| `bankroll`      | 16.9%  | 20.0%  | 23.2%  |
| `pocket-change` | 14.8%  | 17.4%  | 18.6%  |
| `lucky-coin`    | 21.4%  | 29.6%  | 36.9%  |
| `trim`          | 21.8%  | 28.8%  | 35.1%  |
| `fat-pack`      | 13.8%  | 14.0%  | 14.4%  |
| `second-look`   | 14.5%  | 14.6%  | 14.8%  |
| `early-read`    | 13.9%  | 14.2%  | 14.3%  |
| `inside-man`    | 13.4%  | 13.9%  | 11.2%  |

Pocket Change's rank 3 was a pick of three that could all be commons, and
was worth less than rank 2's sure uncommon; it is now a pick of the three
uncommons, and rank 4 a pick with no commons (#184). Lucky Coin and Trim
are each worth more than two ranks of Bankroll at rank 1, the least they
can be. Smart Pushes when a Whiff would end the run, so one re-flip is
often a second life. Why Trim is worth so much is below. Fat Pack,
Second Look and Early Read are worth little at any rank. Inside Man cuts
both ways by design: rank 3 opens Flop to the minions.

#### Why Trim is worth so much (#192)

`--trim` changes which plain cards smart takes out, and `--draw` reports the
Draw. Smart, `--runs 20000`, against 14.0% with no Wheel:

| `--wheel`, `--trim`        | rank 1 | rank 2 |
| -------------------------- | ------ | ------ |
| `trim`, `lowest`           | 22.4%  | 29.5%  |
| `trim`, `random`           | 20.2%  | 25.8%  |
| `trim`, `highest`          | 15.1%  | 17.8%  |
| `trim`, `swap` (same size) | 16.8%  | 18.5%  |

`swap` keeps the deck's size: each lowest plain card is replaced by a
plain 5, about the middle of the starter's plain cards, instead of coming
out. Reading it:

- **Most of Trim is the smaller deck.** Swapping a low card for a 5 gets
  2.8 of rank 1's 8.4 points, and 4.5 of rank 2's 15.5 (the second swap
  barely adds). The rest, 67 to 71%, comes from the deck being
  smaller.
- **Which card goes matters less than that one goes.** Taking out the
  highest plain card, the worst choice, still adds 1.1 to 3.8 points, and a
  random one gets about three quarters of what the lowest does at rank 1.
  The lowest is still the best choice, by 2.2 points over random at rank 1
  and 3.7 at rank 2.
- **The Draw guess is true but small.** `--draw` counts, per turn, the
  low cards (face value 3 or less) held over unplayed in the Draw, and the
  ones that have sat there two turns or more. The Draw is 7.0 cards in
  every run. With no Wheel it holds 1.88 and 1.34.

  | `--trim`  | rank 1      | rank 2      |
  | --------- | ----------- | ----------- |
  | `lowest`  | 1.35, 0.90  | 0.82, 0.48  |
  | `random`  | 1.71, 1.16  | 1.58, 1.02  |
  | `highest` | 1.92, 1.34  | 1.94, 1.31  |
  | `swap`    | 1.33, 0.93  | 0.75, 0.48  |

  `swap` clears the Draw of low cards as well as Trim does and wins 5.6
  points less at rank 1, so the empty slot is worth at most what `swap`
  is worth, 2.8 points (it also adds a better card).

What the sim does not show is why a smaller deck is worth so much. The
Draw is the same 7 cards either way; what changes is how soon the deck
runs dry and reshuffles, and that each remaining card, the Streaks and All
Ins among them, comes round more often (Trim only takes plain cards, so
they are a bigger share of what is left). That fits the numbers, but it is
not measured here. For #189: a Trim that only improves cards (`swap`) is
worth about a third of Trim as it stands.

### Second Look's rerolls

Second Look's rerolls are a pool for the run, and a run meets two minions.
Smart's rule for spending them was a Pack with every card under 8 (a
minion's Pack prints 5 to 11) or three common Items, and it made Second
Look look weak (#190). Each rule below was tried alone, smart, `--minion
cards`, `--runs 20000`, against 14.6% with no Second Look (`--reroll`
takes `below:N`, `gap:N` or `late:N`):

| Pack rule                                        | rank 1 | rank 2 | rank 3 |
| ------------------------------------------------ | ------ | ------ | ------ |
| `below:7`, reroll with no card at 7 or more      | 14.7%  |        |        |
| `below:8`, the old rule                          | 14.8%  | 14.9%  | 14.9%  |
| `below:9`                                        | 15.1%  | 15.2%  | 15.3%  |
| `below:10`                                       | 15.4%  | 15.4%  | 15.7%  |
| `below:11`, keep only a Pack with an 11 in it    | 15.2%  | 15.4%  | 16.0%  |
| `below:12`, reroll every Pack, keep the last     |        |        | 14.6%  |
| `gap:4`, best card beats the deck's weakest by under 4 | 14.7% |   |        |
| `gap:6`                                          | 14.8%  | 14.9%  | 14.9%  |
| `gap:7`                                          | 15.1%  | 15.2%  | 15.3%  |
| `late:8`, only at the minion before a boss       | 14.6%  |        |        |
| `late:9`                                         |        | 14.8%  | 14.8%  |
| `late:10`                                        |        | 14.8%  | 14.9%  |
| `late:11`                                        |        | 14.8%  | 14.9%  |

The harder smart rerolls, the more it wins, up to `below:11`; `below:12`
spends the rerolls on the first offers and keeps the last blind, so it
wins what no rerolls do. Saving rerolls for the minion before a boss is
worse than spending them as they come: the first minion's Pack is as good
a place to improve as the second's. A gap to the deck's weakest card is
no better than a fixed number: `gap:6` and `gap:7` won what `below:8` and
`below:9` did. `below:11` is smart's rule now.

For Items, rerolling three commons (`--reroll-items uncommon`) won 18.8%
at rank 3 with `--minion items`; rerolling unless a rare shows
(`--reroll-items rare`) won 18.0%, so that rule stays.

Smart, `below:11`, by minion setup, against no Wheel:

| `--minion` | none  | `second-look` 1/2/3 | `fat-pack` 1/2/3 | both at 3 |
| ---------- | ----- | ------------------- | ---------------- | --------- |
| `cards`    | 14.6% | 15.2 / 15.4 / 16.0% | 15.0 / 15.6 / 16.1% | 17.0%  |
| `items`    | 17.1% | 18.2 / 18.6 / 18.8% | 17.1 / 17.1 / 17.1% | 18.8%  |
| `random`   | 13.6% | 14.5 / 14.6 / 14.8% | 13.8 / 14.0 / 14.4% | 15.3%  |

The baseline did not move: the rule only applies with rerolls, and Fat Pack
alone has none. Second Look is worth +0.6 to +1.4 points at best on Packs
and +1.1 to +1.7 on Items; the best rule moved rank 3 on `random` from
14.4% to 14.8%. No rule makes it worth near a point per rank, let alone
Bankroll's 16.9% to 23.2%, so #190 does need a redesign. Fat Pack matches
it on Packs and does nothing for Items.
