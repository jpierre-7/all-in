# All In

**Event:** HackRice 16 — Rice University, Sept 11–13, 2026
**Team:** 2 devs, 1 artist
**Engine:** Rust + Bevy

---

## 1. Elevator Pitch

*One sentence: what is this game?*

> A text adventure rogue-like where you fight your way through a casino toward the Big Shots table, resolving every encounter as a card-combat duel against the House.

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
| **The Hand** | The running total built by the cards you play in a turn. |
| **House Edge** | The enemy's defense threshold. Your Hand must clear it. |
| **Payout** | Damage dealt to the enemy's Stack when your Hand clears their House Edge. |
| **Whiff** | A Hand that falls short of the House Edge. The shortfall (House Edge − Hand) is dealt to the player's own Chip Stack. |
| **Tell** | A single passive keyword on a card that modifies how it resolves. (Balatro/Inscryption-inspired, one per card, no stack/targeting.) |
| **Streak** | Tell: doubles a card's value if the previous card played shared a Tell. |
| **All In** | Tell: sacrifice a card from hand to add its value to the current Hand. |
| **Rising Blinds** | Difficulty/cost escalates as combat goes on (turn-based scaling). +2 to House Edge every 3 turns. |
| **Plays** | Limited number of cards you may play per turn (Hand of 7 cards, up to 5 can be played by default). |
| **Push Your Luck** | Optional coin flip after The Hand is final and clears House Edge: Push to double the Payout, or lose The Hand outright (Hand = 0, full Whiff). Coin is 45/55 in the House's favor. |
| **Hole Card** | Against The House only: the player's final Play of the turn, made after the House has locked its Edge on everything played before it. |
| **Fight or Fold** | Encounter choice. "Fight" initiates duel; "Fold" abandons the run and returns to the Lobby, resetting all perks, items, and temporary deck upgrades. |

---

## 4. Combat & Roguelike Loop

**Encounter Choice (Fight or Fold):**
Before each encounter, the player chooses to:

- **Fight:** Engage the enemy in card duel combat.
- **Fold:** Retreat to the Lobby to restart the run. All perks, items, and temporary deck modifications reset.

**Combat Structure:** turn-based, resolved instantly left-to-right (no stack, no targeting, no priority passing).

**Turn sequence:**

1. Player refills the Draw to 7 cards.
2. Player plays up to 5 cards (or up to the limit set by perks), one at a time.
3. Each card resolves immediately, applying its Stack + any Tell effect, adding to The Hand.
4. Items that modify The Hand apply (Loaded Dice: +5). The Hand is now final.
5. **Push Your Luck** (only if The Hand ≥ House Edge): the player sees the House Edge and chooses **Push** or **Hold**.
   - Hold → step 6 as normal.
   - Push → flip the coin (45% player / 55% House by default; Slotz Option 1 makes it best-2-of-3 at 49/51 ≈ 48.5%).
     - Win → the Payout in step 6 is **doubled**.
     - Lose → The Hand becomes **0**; step 6 is a full Whiff for the entire House Edge.
6. Compare The Hand to the enemy's House Edge.
   - Clear it → Payout (excess over House Edge, ×2 if PYL won) is dealt to the enemy's Stack.
   - Whiff (fall short) → the difference is dealt to the **player's** Stack. Damage = House Edge − The Hand. No Payout.
7. Enemy turn: nothing (MVP). Enemies are a Stack + House Edge + Rising Blinds.
8. Rising Blinds check: escalate House Edge +2 every 3 turns (or per active perk modifier).
9. Repeat until player or enemy Stack hits 0.

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

**MVP Tell set (2 only):**

- **Streak**
- **All In**

**Stretch Tells (cut if behind schedule):** Echo, Brittle, Copycat (built, #110), [others TBD]

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

**MVP scope:** single enemy archetype, House Edge scales with Rising Blinds. No unique per-enemy behavior required for MVP.

**Combat numbers** (first pass, tuned on the `prototype/starter-deck` sim with 300 full runs per player model; playtesting adjusts):

| | Player | Floor minion | Slotz | Pit minion | Pit Boss | The House |
| --- | --- | --- | --- | --- | --- | --- |
| Stack | **50** | 25 | 32 | 32 | 40 | 35 |
| House Edge | | 18 | 20 | 22 | 24 | Hand + margin |
| Rising Blinds | | +2 / 3 turns | +2 / 3 turns | +2 / 3 turns | +2 / 3 turns | margin +1, +2 / 2 turns |

What the sim said: a careless player (top five cards, random order) beats the Pit Boss 82% of the time and the House 19%; a player who leads with All In then Streaks beats everything up to the House with a full Stack, then loses to the House every time because that habit makes the hole card a low vanilla card; a player who saves the biggest Tell for last beats the House in ~4 turns. The House inverts the habit the Tutorial teaches, on purpose. Blinds every 2 turns (the earlier draft) made every fight past 6 turns unwinnable; every 3 is the difference.

**Stretch goal — reactive House:** enemy reveals one card from hand each turn (fixed or shuffled sequence). Effects could include:

- Raise their own House Edge
- Force player to discard
- Apply a negative Tell to the player's next play

**Encounter progression:**

1. **Lobby:** Story intro → Tutorial arcade / Info room / Start run.
2. **The Floor:** 1 Minion encounter → Boss: **Slotz**.
3. **The Pit:** 1 Minion encounter → Boss: **Pit Boss**.
4. **The Back Room (Optional/Secret):** Boss: **The Man Who Beat the House** (Plays +1, transforms deck).
5. **The Big Shots Table (Final Boss):** Boss: **The House**. See "The House" below.

**The House (the Hole Card rule):**

- The House does not have a fixed House Edge. When the player has **one Play remaining** (after 4 of 5 Plays, or 5 of 6 with the Pit Boss perk), the House reads The Hand so far and locks House Edge = The Hand + **margin**.
- The player's final Play is the **Hole Card**: the one card the House can't see. Payout = the Hole Card's resolved value − margin. Streak doubling, All In sacrifice, and Loaded Dice all land after the lock, so they are the whole strategy.
- Push Your Luck works unchanged: it is offered when the Hole Card beats the margin.
- Rising Blinds for The House raise the **margin**, not the Edge: margin starts at +1 and rises +2 every 2 turns (1, 1, 3, 3, 5, 5, 7…). The margin is the clock; the House wins by outlasting the deck, not by big Whiffs.
- Ending the turn with more than one Play unused still locks the Edge at "one Play remaining"; with no Hole Card played, the turn Whiffs by the margin.
- Intro line hook: "Play four. I'll set the line. Then show me your last card."

---

## 8. Stretch Goals (post-MVP, only if time allows)

- **Push Your Luck on a Whiff:** offer PYL when The Hand falls short too — win forgives the Whiff, lose doubles it. (Base PYL is MVP; see §4.)
- **Card personas:** dealer's-commentary flavor lines on play (e.g. "The house always collects" on an All In card).
- Deckbuilding/reward layer between encounters.
- Additional Tells (Echo, Brittle).

*Cut order if behind schedule: deckbuilding layer → PYL on a Whiff → card personas → reactive House deck. Cut from the top of this list first; the reactive House deck is the highest-value addition and should be the last thing dropped.*

---

## 9. Tech & Architecture

**Stack:** Rust, Bevy 0.19

**Rough module breakdown:**

- `Card` struct — name, base value, `Tell` enum
- `Perk` / `Item` structs — active run modifiers and inventory triggers
- Resolution loop — applies Tells, builds The Hand, compares to House Edge
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
