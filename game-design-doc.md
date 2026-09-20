# All In

**Event:** HackRice 16 — Rice University, Sept 11–13, 2026
**Team:** 2 devs, 1 artist
**Engine:** Rust + Bevy

---

## 1. Elevator Pitch

*One sentence: what is this game?*

> A text adventure rogue-like where you fight your way through a casino toward the Big Shots table, resolving every encounter as a card duel where the House deals its row first — half of it face down — and you have to cover it card for card.

---

## 2. Story & Setting

**Premise:**
Players play as "Lucky Jack", a former professional gambler who bet it all at the big shots table 25 years ago. After losing everything Lucky Jack bet his first born son only to lose him too. 25 years later, Lucky Jack returns to the casino for revenge on those who destroyed his life. Players fight their way through the casino, floor by floor, to reach the **Big Shots table**. Upon defeating the House, Lucky Jack confronts the casino owner only to find that his own son is now the new owner. The game ends with Lucky Jack's son stating "After 25 years, I'd thought you'd of learned by now. The House Always Wins Pops. *Bang*"

**Framing / world flavor:**

- **Lobby** — Entry to the game after backstory (3 options: Secret Encounter Info Room, Tutorial Arcade Machine, and Begin Run).
- **The Floor** — Early low-stakes encounters (1 Minion + Boss: Slotz).
- **The Pit** — Mid-tier recurring enemies (1 Minion + Boss: Pit Boss).
- **The Back Room** — Optional secret encounter (Boss: The Man Who Beat the House [Plays +1] | Reward: Turns all cards in deck to "All In" tell).
- **The Big Shots Table** — Final boss (The House [reads your Hand before your last Play and sets House Edge to it plus a margin; your last Play is your Hole Card]).

**Tone:** [casino-noir / pulpy / darkly comic]

**Narrative delivery:** text adventure prose between encounters; combat can occur upon exploration.

---

## 3. Core Mechanics Glossary

| Term | Definition |
| --- | --- |
| **Stack/Chip Stack** | Health/life stat. Player, enemy, and individual cards each have their own Stack. |
| **Opposing Cards** | The enemy's row, laid down before you play. First card always face up, the rest a roll. Your cards go one per slot across from them. |
| **Slot** | One column of the table: an Opposing Card and whatever you put across from it. |
| **Stack Sum** | What a row adds up to once its Tells resolve. One per side; the lower one pays the difference. |
| **The Hand** | Your row, and its Stack Sum. |
| **House Edge** | The Opposing Cards' Stack Sum. Your Hand must beat it, and you only see part of it before you confirm. |
| **Payout** | Damage dealt to the enemy's Stack when your Hand beats theirs: the difference. |
| **Whiff** | A Hand that comes in lower. The difference is dealt to the player's own Chip Stack. |
| **Tell** | A single passive keyword on a card that modifies how it resolves. (Balatro/Inscryption-inspired, one per card.) Targets by **slot**, and always reads a **printed** Stack. |
| **Streak** | Tell: doubles this card if the card in the slot to its left has any Tell. |
| **All In** | Tell: sacrifice a card from the Draw to add its Stack to this card's. |
| **Copycat** | Tell: worth the printed Stack of the card in the slot to its right; its own if nothing follows. |
| **Flop** | Tell: worth the printed Stack of the Opposing Card across from it; its own if nothing is across. |
| **Rising Blinds** | Difficulty escalates as combat goes on: the enemy lays one more Opposing Card every 3 turns. Past your Plays you can't cover them all. |
| **Plays** | Limited number of cards you may put in your row per turn (Draw of 7, up to 5 placed by default). |
| **Push Your Luck** | Optional coin flip after both rows turn over and your Hand beats theirs: Push to double the Payout, or lose The Hand outright (Hand = 0, full Whiff). Coin is 45/55 in the House's favor. A tie is never offered it. |
| **Hole Card** | Against The House only: the last card in your row. The House keeps its own last Opposing Card back and fills it in on everything before yours. |
| **Fight or Fold** | Encounter choice. "Fight" initiates duel; "Fold" abandons the run and returns to the Lobby, resetting all perks, items, and temporary deck upgrades. |

---

## 4. Combat & Roguelike Loop

**Encounter Choice (Fight or Fold):**
Before each encounter, the player chooses to:

- **Fight:** Engage the enemy in card duel combat.
- **Fold:** Retreat to the Lobby to restart the run. All perks, items, and temporary deck modifications reset.

**Combat Structure:** turn-based, two rows facing each other. No stack, no priority passing; targeting is by **slot** and nothing else.

**Turn sequence:**

1. Player refills the Draw to 7 cards.
2. **The enemy goes first.** It lays down its Opposing Cards — 3 or 4 to start, depending on the enemy, and one more per Rising Blinds tick. The first is face up; each of the rest is face down on a roll the enemy's own odds set. Enemies play Tells too, at their own configured rate.
3. Player covers the row: click a card in the Draw to put it in the next empty slot, click a card already in the row to take it (and anything it burned) back out. Up to 5 cards (or the limit set by perks). Nothing resolves yet — a row is worth nothing until it is finished.
4. Player **confirms**. Both rows turn over.
5. Each row resolves left to right. Every Tell reads its slot: Streak looks one left, Copycat one right, Flop straight across. All of them read *printed* Stacks, so neither row depends on the other resolving first.
6. Items that modify The Hand apply (Loaded Dice: +5). Both Stack Sums are now final.
7. **Push Your Luck** (only if The Hand > House Edge): the player sees both totals and chooses **Push** or **Hold**.
   - Hold → step 8 as normal.
   - Push → flip the coin (45% player / 55% House by default; Slotz Option 1 makes it best-2-of-3 at 49/51 ≈ 48.5%).
     - Win → the Payout in step 8 is **doubled**.
     - Lose → The Hand becomes **0**; step 8 is a full Whiff for the entire House Edge.
8. Compare the two Stack Sums. The lower side loses the difference off its Stack.
   - Yours higher → Payout (the difference, ×2 if PYL won) is dealt to the enemy's Stack.
   - Yours lower → Whiff: the difference is dealt to the **player's** Stack.
   - Dead even → nobody pays.
9. Rising Blinds check: one more Opposing Card every 3 turns (or per active perk modifier). Once the row is longer than your Plays, the slots you can't cover count for the enemy anyway.
10. Repeat until player or enemy Stack hits 0.

**Loss condition:** player Stack reaches 0 (run ends / back to Lobby).
**Win condition:** enemy Stack reaches 0 (grants rewards based on enemy type).

---

## 5. Perk & Item System

Upon defeating an enemy, the player earns rewards depending on the enemy type. All collected perks and items persist through the current run and reset upon **Folding** or dying.

### Minion Rewards (Random Item Drop)

Defeating a minion grants 1 random item from the pool:

- **Lucky Pocket Card:** Adds a random Tell to a card in the player's deck that does not already have a Tell.
- **Loaded Dice:** Adds +5 to the next 2 Hand totals.
- **Card Shark's Sunglasses:** Reveals the next encounter's starting House Edge beforehand.

### Boss Rewards (Pick 1 of 2 Perks)

Defeating a floor boss presents a choice between two powerful run-altering perks:

- **Boss: Slotz (The Floor)**
  - **Option 1:** All "Push Your Luck" flips become best 2-out-of-3 at 49% player / 51% House (≈48.5% overall, up from the base 45%).
  - **Option 2:** Adds 3x "Streak" Tell cards to the player's deck.

- **Boss: Pit Boss (The Pit)**
  - **Option 1:** You can play up to 6 cards per turn, but Rising Blinds become +2 every turn (3× the base rate). Pays in short fights, punishes long ones.
  - **Option 2:** Adds 4x random cards to deck (2x with random Tells, 2x normal/vanilla cards).

- **Secret Boss: The Man Who Beat the House (The Back Room)**
  - **Reward:** Guaranteed transformation — turns **all cards** in player's deck to have the **"All In"** Tell.

---

## 6. Card Design

**Card anatomy:**

- Name
- Base value (contribution to The Hand)
- Tell (one keyword, or none)
- [Flavor text / persona line — stretch goal]

**Tell set as built:**

- **Streak** — reads the slot to its left
- **All In** — reads no slot; burns from the Draw
- **Copycat** — reads the slot to its right
- **Flop** — reads the Opposing Card across from it

**Stretch Tells (cut if behind schedule):** Echo, Brittle, Copycat (built, #110), Flop (built, #88), [others TBD]

**Starter deck:** fixed, no deckbuilding meta-layer for MVP. 18 cards, 8 with a Tell (44%). Prototyped on the `prototype/starter-deck` branch: a careless player fires Streak ~1/3 of the time, a careful one ~94%, a 6-chip gap in mean Hand from sequencing alone. Mean Hand ~28 (naive) / ~34 (smart), range 18–52. Whiffs start appearing around House Edge 26.

| Card Name | Stack | Tell | Notes/Flavor |
| --- | --- | --- | --- |
| Two of Clubs | 2 | — | |
| Cheap Seat | 3 | — | |
| Comped Drink | 3 | — | |
| Four of Hearts | 4 | — | |
| Bus Ticket Home | 4 | — | |
| Five of Spades | 5 | — | |
| Borrowed Watch | 5 | — | |
| Six of Diamonds | 6 | — | |
| Marked Card | 7 | — | |
| Pawned Ring | 8 | — | Biggest vanilla; the natural All In fuel |
| Hot Streak | 3 | Streak | |
| Lucky Seat | 4 | Streak | |
| Dealer Blinks | 5 | Streak | |
| Table Runs Hot | 6 | Streak | |
| Last Dollar | 2 | All In | |
| Car Keys | 3 | All In | |
| Deed to the House | 4 | All In | |
| Firstborn | 5 | All In | Yes, that one |

---

## 7. Enemy Design & Encounters

**How an enemy is authored:** an enemy is a Stack, a Rising Blinds schedule, and a **Deal** — six numbers saying how it fills its row. No named decks; regular enemies don't have them.

| Deal field | What it does |
| --- | --- |
| `row` | Opposing Cards laid down on turn one |
| `low` / `high` | the Stack range a card is dealt from, inclusive |
| `tell_pct` | chance in 100 that a dealt card carries a Tell at all |
| `tells` | which Tells this enemy plays; one is drawn at random when `tell_pct` hits. **Never All In** — an enemy has no Draw to burn from |
| `hidden_pct` | chance in 100 that a card is face down. The first Opposing Card is always face up |

**Combat numbers.** The ranges are picked so each enemy's *expected* row sum lands on the flat House Edge it used to carry, so the `prototype/starter-deck` tuning below still applies:

| | Player | Floor minion | Slotz | Pit minion | Pit Boss | The House |
| --- | --- | --- | --- | --- | --- | --- |
| Stack | **50** | 25 | 32 | 32 | 40 | 35 |
| Deal | | 3 × 4–8 | 3 × 5–9 | 4 × 4–7 | 4 × 4–8 | 4 × 1–4, last kept back |
| Expected row | | ~18 | ~21 | ~22 | ~24 | read + margin |
| Tells | | 20% Streak | 35% Streak/Copycat | 30% Streak/Flop | 40% Streak/Copycat/Flop | 35% Streak/Flop |
| Face down | | 50% | 50% | 50% | 55% | 40%, + the Hole Card |
| Rising Blinds | | +1 card / 3 turns | +1 card / 3 turns | +1 card / 3 turns | +1 card / 3 turns | margin +1, +2 / 2 turns |

What the sim said: a careless player (top five cards, random order) beats the Pit Boss 82% of the time and the House 19%; a player who leads with All In then Streaks beats everything up to the House with a full Stack, then loses to the House every time because that habit makes the hole card a low vanilla card; a player who saves the biggest Tell for last beats the House in ~4 turns. The House inverts the habit the Tutorial teaches, on purpose. Blinds every 2 turns (the earlier draft) made every fight past 6 turns unwinnable; every 3 is the difference.

**Built (#88):** the reactive enemy is no longer a stretch goal — laying the Opposing Cards down *is* the enemy's turn, and House Edge is what they add up to. See `docs/adr/0002-opposing-cards.md`.

**Encounter progression:**

1. **Lobby:** Story intro → Tutorial arcade / Info room / Start run.
2. **The Floor:** 1 Minion encounter → Boss: **Slotz**.
3. **The Pit:** 1 Minion encounter → Boss: **Pit Boss**.
4. **The Back Room (Optional/Secret):** Boss: **The Man Who Beat the House** (Plays +1, transforms deck).
5. **The Big Shots Table (Final Boss):** Boss: **The House**. See "The House" below.

**The House (the Hole Card rule):**

- The House lays its row down like anyone else, but deals itself scraps (1–4) and keeps its **last Opposing Card** blank and face down.
- At the showdown it fills that card in so its whole row reads **the player's row minus the player's last card, plus the margin**.
- The last card in the player's row is the **Hole Card**: the one card The House can't see. Payout = the Hole Card's resolved value − margin. Streak doubling, All In sacrifice, and Loaded Dice all land after it fills in, so they are the whole strategy.
- Push Your Luck works unchanged: it is offered when the Hole Card beats the margin.
- Rising Blinds for The House raise the **margin**, not the row: margin starts at +1 and rises +2 every 2 turns (1, 1, 3, 3, 5, 5, 7…). The margin is the clock; the House wins by outlasting the deck, not by big Whiffs.
- Confirming a short row still works the same way: everything but the last card you placed is what it reads. An empty row Whiffs by the margin.
- The House deals itself no Copycat — that is the one Tell that would have to read the card it hasn't decided on yet.
- Intro line hook: "Fill your row. I'll fill my last card in after. Then show me the one I couldn't see."

---

## 8. Stretch Goals (post-MVP, only if time allows)

- **Push Your Luck on a Whiff:** offer PYL when The Hand falls short too — win forgives the Whiff, lose doubles it. (Base PYL is MVP; see §4.)
- **Card personas:** dealer's-commentary flavor lines on play (e.g. "The house always collects" on an All In card).
- Deckbuilding/reward layer between encounters.
- Additional Tells (Echo, Brittle).

*Cut order if behind schedule: deckbuilding layer → PYL on a Whiff → card personas. Cut from the top of this list first. The reactive enemy deck used to sit at the bottom of this list as the last thing to drop; it landed in #88 and is now the core loop.*

---

## 9. Tech & Architecture

**Stack:** Rust, Bevy 0.19

**Rough module breakdown:**

- `Card` struct — name, base value, `Tell` enum
- `Perk` / `Item` structs — active run modifiers and inventory triggers
- `Deal` struct — how an enemy fills its row of Opposing Cards
- Resolution loop — one pure pass per row, applying Tells by slot, then compares the two Stack Sums
- Overworld/text adventure layer — navigation, encounter triggers ("Fight" vs "Fold"), narrative text
- Combat UI — renders Stack, Plays remaining, The Hand as it builds

*TODO: finalize struct definitions, module boundaries, save/state format if needed.*

---

## 10. Team & Roles

| Role | Owner | Focus |
| --- | --- | --- |
| Combat engine + combat UI | Dev 1 | `Card`/`Tell` structs, resolution loop, perks/items; renders combat state (Stack, Plays, The Hand) |
| Overworld / text adventure + shell | Dev 2 | Navigation, encounter triggers ("Fight" / "Fold"), narrative text; app setup, scene/state transitions, input |
| Art | Artist | Card frames/icons scoped to MVP Tells; casino-themed UI elements |

**Shared:** playtesting and balancing once the core loop is live — all three, no dedicated owner.

**Coverage note:** with no third dev, rendering is owned by whoever owns the state it draws
(Dev 1 for combat, Dev 2 for overworld) rather than being a separate role. If the schedule
slips, cut from Section 11's stretch items before splitting attention further.

---

## 11. Open Questions / TODO

- [x] Final title: All In
- [x] Full starter deck list: see §6
- [ ] Enemy encounter list and flavor
- [x] Opening/ending narrative beats: see `docs/narrative.md`
- [ ] Pitch/demo script for judging
