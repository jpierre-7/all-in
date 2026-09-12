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
- **The Big Shots Table** — Final boss (The House [Reads player's running hand value, adds +1, sets to house edge]).

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
| **Rising Blinds** | Difficulty/cost escalates as combat goes on (turn-based scaling). E.g., +2 to House Edge every 2 turns. |
| **Plays** | Limited number of cards you may play per turn (Hand of 7 cards, up to 5 can be played by default). |
| **Fight or Fold** | Encounter choice. "Fight" initiates duel; "Fold" abandons the run and returns to the Lobby, resetting all perks, items, and temporary deck upgrades. |

---

## 4. Combat & Roguelike Loop

**Encounter Choice (Fight or Fold):**
Before each encounter, the player chooses to:

- **Fight:** Engage the enemy in card duel combat.
- **Fold:** Retreat to the Lobby to restart the run. All perks, items, and temporary deck modifications reset.

**Combat Structure:** turn-based, resolved instantly left-to-right (no stack, no targeting, no priority passing).

**Turn sequence (draft):**

1. Player draws/refills hand to 7 cards.
2. Player plays up to 5 cards (or up to limit modified by perks).
3. Each card resolves immediately, applying its base value + any Tell effect, adding to The Hand.
4. Compare The Hand total to enemy's House Edge.
   - Clear it → Payout (excess spills over as damage to enemy Stack).
   - Whiff (fall short) → the difference is dealt to the **player's** Chip Stack.
     Damage = House Edge − The Hand. No Payout to the enemy.
5. Enemy takes their turn: [TBD — see Enemy Design below]
6. Rising Blinds check: escalate House Edge every 2 turns (or per active perk modifier).
7. Repeat until player or enemy Stack hits 0.

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
  - **Option 1:** All "Push Your Luck" wagers become best 2-out-of-3, but odds shift slightly in the House's favor (49% player / 51% house).
  - **Option 2:** Adds 3x "Streak" Tell cards to the player's deck.

- **Boss: Pit Boss (The Pit)**
  - **Option 1:** You can play up to 6 cards per turn, but House Edge increases every turn by +4 instead.
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

**Stretch Tells (cut if behind schedule):** Echo, Brittle, [others TBD]

**Starter deck:** fixed, no deckbuilding meta-layer for MVP. [Aiming for ~15-20 cards. (might have to bump this up)]

| Card Name | Base Value | Tell | Notes/Flavor |
| --- | --- | --- | --- |
| | | | |
| | | | |

---

## 7. Enemy Design & Encounters

**MVP scope:** single enemy archetype, House Edge scales with Rising Blinds. No unique per-enemy behavior required for MVP.

**Stretch goal — reactive House:** enemy reveals one card from hand each turn (fixed or shuffled sequence). Effects could include:

- Raise their own House Edge
- Force player to discard
- Apply a negative Tell to the player's next play

**Encounter progression:**

1. **Lobby:** Story intro → Tutorial arcade / Info room / Start run.
2. **The Floor:** 1 Minion encounter → Boss: **Slotz**.
3. **The Pit:** 1 Minion encounter → Boss: **Pit Boss**.
4. **The Back Room (Optional/Secret):** Boss: **The Man Who Beat the House** (Plays +1, transforms deck).
5. **The Big Shots Table (Final Boss):** Boss: **The House** (Reads player's running Hand value, adds +1, sets to House Edge).

---

## 8. Stretch Goals (post-MVP, only if time allows)

- **Push Your Luck:** after building The Hand but before resolving, optionally wager part of your own Stack on a coin-flip to double the Payout or lose the Hand outright.
- **Card personas:** dealer's-commentary flavor lines on play (e.g. "The house always collects" on an All In card).
- Deckbuilding/reward layer between encounters.
- Additional Tells (Echo, Brittle).

*Cut order if behind schedule: deckbuilding layer → Push Your Luck → card personas → reactive House deck. Cut from the top of this list first; the reactive House deck is the highest-value addition and should be the last thing dropped.*

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
- [ ] Full starter deck list (~15-20 cards)
- [ ] Enemy encounter list and flavor
- [x] Opening/ending narrative beats: see `docs/narrative.md`
- [ ] Pitch/demo script for judging
