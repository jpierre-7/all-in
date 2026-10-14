# Both sides play the same turn: a Draw, a Blind, and a hidden row

_Supersedes [ADR-0002](0002-opposing-cards.md). Decided in #141._

Under ADR-0002 the enemy didn't play cards: it was dealt a row off a `Deal` (a Face Value range and some odds), laid it down first with the first card face up, and grew it with Rising Blinds. Now the enemy plays by the player's rules:
- It has a **Deck**, draws up to 7 into its own **Draw** each turn, and plays up to the floor's **Blind** into its row.
- Cards it didn't play stay in its Draw, and played and burned cards go to its own discard, which is reshuffled into the Deck when the Deck runs dry. A side whose Deck and discard can't fill its Draw plays with what it has. Running out never loses a fight on its own.
- The enemy commits its row face down at the start of the turn, before the player places anything. The player sees one card back per card committed, and nothing more unless a **reveal** turns some of them over.
- Both rows turn over at Confirm.

The enemy choosing for itself is the change everything else follows from. An enemy now needs a **strategy**: a pure, seeded function from what it can see to the row it commits, so the sim and tests can replay any fight.

## The turn

1. Each side draws up to 7.
2. The enemy commits up to its Blind, face down, by its strategy.
3. The player places and Lifts cards up to their own Blind. Reveals act here.
4. **Confirm**, then the **Showdown**. The table is as wide as the larger Blind. Both rows fill from the left, and slot *i* faces slot *i*. An empty slot is worth 0 to either side, and a Flop reading it gets 0. Every Tell resolves on Face Values.
5. **Push Your Luck**, offered to the player only.
6. The Payout or Whiff is dealt, and each side's row goes to its own discard.

A boss's **Table Rule** may change step 2 for its own fight: swap the strategy, or move when the enemy commits. The House acting last is the motivating case (see below).

## Greedy, the default strategy

Greedy looks only at this turn, its own Draw, and its own Blind. It tries every ordered pick of up to the Blind, including every All In with every sacrifice (the enemy has a Draw now, so All In works for it). It keeps the pick whose row resolves highest. A tie goes to the earliest pick in Draw order.

Greedy can't see the player's row, so it scores a Flop as Face Value + the average Face Value of the player's whole Deck for each neighbouring slot the player could fill. That average only steers the *choice*: a Flop still resolves on the real cards across from it at the Showdown. Reading the player's Deck isn't cheating, because a player watching the table could work out the same number.

## Reveals

A reveal effect (an Item, a Perk, or a Table Rule) has one form: it turns chosen Opposing Cards face up for the current turn, before Confirm. It can turn over one slot, several, or all of them. The enemy's Draw or Deck may become a second target of the same form later. Nothing reveals the player's row to the enemy, because under the default strategy the enemy has already chosen.

The Flop Peek line on a card in the player's Draw counts the revealed Opposing Cards it would read as `+N`, adds `+ ?` for each hidden one, and adds nothing for an empty slot. It gets exact as reveals stack up.

## Kept from ADR-0002

- Tells read slots, by Face Value.
- The row is worth nothing until Confirm, so the player can Lift and rearrange freely.
- A tie pays nobody.
- Push Your Luck comes after the Showdown.
- House Edge is whatever the enemy's row resolves to.
- What the screen may say (`showing`) is still separate from the truth.

## Considered Options

- **The enemy commits at Confirm, seeing the player's row.** Rejected as the default. It's simpler, but it's cheating, and it leaves a reveal nothing to show. It is kept as something a Table Rule may grant, and is proposed for The House.
- **Personalities (greedy, hoarder, reckless) per enemy.** Deferred. The strategy seam can carry them, but only greedy is built until a boss proposal asks for another.
- **How The House plays Flop properly.** Its deck is built around Flop, and greedy only estimates Flop. The alternatives were:
  - **Flop goes in last, for everyone:** held Flops drop into empty slots after both rows turn over. It's symmetric and elegant, but it gives the Showdown two beats and an extra phase.
  - **A Button that alternates who commits first:** played hidden, it doesn't help Flop. Played open, it breaks "both rows hidden" on half the turns.

  Chosen instead: The House acts last, as its Table Rule. That gives it exact Flops and leaves the default turn alone. The final pick of The House's Table Rule belongs to the boss proposals ticket (#146).
- **Cap both rows at the smaller Blind.** Rejected: a +1 Blind effect for one side would do nothing.

## Consequences

- `Deal`, `RisingBlinds`, `Duel::plays`/`plays_left` against the enemy's row, `hidden_pct` and the face-up first card all go. `row_size` becomes each side's Blind, set by the floor.
- `refill` and the discard become per side.
- Fights no longer get harder on a clock. Whether duels still end on time is a question for the balance sim (#94).
- Against The House, a reveal shows nothing until its Table Rule is decided. Its Hole Card and margin are removed separately (#145).
