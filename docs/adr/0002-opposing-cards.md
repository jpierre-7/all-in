# The turn is two rows facing each other, and Tells read the slot

The enemy used to do nothing on its turn: it was a Stack, a flat House Edge, and a Rising Blinds schedule, and the player built The Hand against a number that was decided before the duel started. Now the enemy lays a row of **Opposing Cards** down first — the first face up, each of the rest face down on a roll — the player covers that row slot for slot out of the Draw, both rows turn over on **Confirm**, and the side with the lower **Stack Sum** loses the difference. House Edge survives as a term but changes meaning: it is whatever the Opposing Cards add up to, not a number the enemy carries.

The consequence that drove the rest of the design: **a Tell targets by slot, not by play order**. Streak reads one slot left, Copycat one slot right, **Flop** reads straight across at the Opposing Card. Cards no longer resolve as they are played — a row is worth nothing until it is finished and every Tell knows its neighbours — so the player can put a card down, take it back out, and rearrange until they confirm.

## Considered Options

- **Keep the flat House Edge and add a revealed enemy card as flavour.** Rejected: the stretch goal in §7 of the design doc, and it doesn't give the player a decision. The hidden cards are the whole gamble; a decorative enemy row is a number with a picture on it.
- **Tells resolve as each card is placed, as they used to.** Rejected: it makes the row immutable, so lifting a card back out is either impossible or a rollback, and Flop could not read a slot the player hasn't chosen yet. Resolving the whole row at Confirm is one pure function over `(row, across)`, which is also the easiest thing in the codebase to test.
- **Tells read resolved values rather than printed Stacks.** Rejected: a Flop on each side of the table would read the other one and the two rows would have no fixed point. Printed Stacks throughout make the pass order irrelevant, and Copycat already worked this way (#110).
- **Enemies get real decks of named cards.** Rejected for now: a `Deal` — a Stack range, the odds of a Tell, which Tells, and the odds of a card being face down — is six numbers per enemy instead of eighteen cards, and it is the surface a designer actually wants to turn. Ranges are picked so each enemy's expected row sum lands on the flat Edge it used to carry, so §7's tuning survives.
- **Cap the enemy's row at the player's Plays.** Rejected: letting the row outgrow Plays is what Rising Blinds now *are*. Slots the player can't cover count for the enemy anyway, which keeps the Blinds a clock instead of something that stops mattering at five cards.

## Consequences

- `Enemy` loses `house_edge` and gains `deal: Deal`. `RisingBlinds.increase` (chips) becomes `RisingBlinds.cards` (Opposing Cards). The Pit Boss perk's price is another card every turn rather than +2 Edge every turn.
- The Hole Card rule (#5) rides on `Enemy::hole_card` rather than on `EncounterId`, so `Duel` no longer needs to be told who it is playing. The House keeps its **last Opposing Card** blank and face down and fills it in at the showdown so its row reads the player's row minus their last card, plus the margin — the same rule as before, expressed as a card instead of a threshold. It deals itself no Copycat: that is the one Tell that would have to read the card it hasn't decided on yet.
- Ties are possible now and pay nobody. `Outcome::Payout(0)` is a standoff, and the coin is never offered on one.
- The screen grew a row: Opposing Cards, the player's row under them at the same card size, then the Draw. `CardSlot` carries a `Zone` so a click can mean "cover a slot" or "take that back", and the Peek can read a face-up Opposing Card the same way it reads the Draw.
- `Duel::house_edge` knows the whole truth including face-down cards; `Duel::showing` is what the screen is allowed to say. A face-up card whose Tell reads a face-down one counts for its printed Stack alone, so the row never leaks more than the fact that something is hidden.
