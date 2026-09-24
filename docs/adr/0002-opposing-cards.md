# The turn is two rows facing each other, and Tells read the slot

_Terms updated by #123: a fighter's Stack is now **Chips**, a card's is its **Face Value**, and "Stack Sum" is gone: The Hand and House Edge name each side's row._

The enemy used to do nothing on its turn: it was a pile of Chips, a flat House Edge, and a Rising Blinds schedule, and the player built The Hand against a number that was decided before the duel started. Now the enemy lays a row of **Opposing Cards** down first — the first face up, each of the rest face down on a roll — the player covers that row slot for slot out of the Draw, both rows turn over on **Confirm**, and the side that comes up short loses the difference. House Edge survives as a term but changes meaning: it is whatever the Opposing Cards add up to, not a number the enemy carries.

The consequence that drove the rest of the design: **a Tell targets by slot, not by play order**. Streak reads one slot left, Copycat one slot right, **Flop** reads straight across at the Opposing Card. Cards no longer resolve as they are played — a row is worth nothing until it is finished and every Tell knows its neighbours — so the player can put a card down, take it back out, and rearrange until they confirm.

## Considered Options

- **Keep the flat House Edge and add a revealed enemy card as flavour.** Rejected: the stretch goal in §7 of the design doc, and it doesn't give the player a decision. The hidden cards are the whole gamble; a decorative enemy row is a number with a picture on it.
- **Tells resolve as each card is placed, as they used to.** Rejected: it makes the row immutable, so lifting a card back out is either impossible or a rollback, and Flop could not read a slot the player hasn't chosen yet. Resolving the whole row at Confirm is one pure function over `(row, across)`, which is also the easiest thing in the codebase to test.
- **Tells read resolved values rather than Face Values.** Rejected: a Flop on each side of the table would read the other one and the two rows would have no fixed point. Face Values throughout make the pass order irrelevant, and Copycat already worked this way (#110).
- **Enemies get real decks of named cards.** Rejected for now: a `Deal` — a Face Value range, the odds of a Tell, which Tells, and the odds of a card being face down — is six numbers per enemy instead of eighteen cards, and it is the surface a designer actually wants to turn. Ranges were first picked so each enemy's expected row sum landed on the flat Edge it used to carry; after balancing they no longer do (see Consequences).
- **Cap the enemy's row at the player's Plays.** Rejected: letting the row outgrow Plays is what Rising Blinds now *are*. Slots the player can't cover count for the enemy anyway, which keeps the Blinds a clock instead of something that stops mattering at five cards.

## Consequences

- `Enemy` loses `house_edge` and gains `deal: Deal`. `RisingBlinds.increase` (chips) becomes `RisingBlinds.cards` (Opposing Cards). The Pit Boss perk's price is another card every turn rather than +2 Edge every turn.
- The Hole Card rule (#5) rides on `Enemy::hole_card` rather than on `EncounterId`, so `Duel` no longer needs to be told who it is playing. The House keeps its **last Opposing Card** blank and face down and fills it in at the showdown so its row reads the player's row minus their last card, plus the margin — the same rule as before, expressed as a card instead of a threshold. It deals itself no Copycat: that is the one Tell that would have to read the card it hasn't decided on yet.
- **Plays are capped at the enemy's row.** The player can't put down more cards than there are Opposing Cards: 3 Plays against the Floor minion and Slotz, 4 against the Pit and The House. The Pit Boss perk's sixth Play is dead until the Blinds add a slot, and against The House, whose Blinds raise the margin rather than the row, it never comes alive.
- **Rising Blinds tick every 5 turns, not 3.** One more Opposing Card is a bigger step than the old +2 Edge, so the clock slows to match.
- **Row sums no longer land on the old Edges.** Measured over 4000 deals: Floor minion ~10.8 (was 18), Slotz ~15.6 (20), Pit minion ~22.8 (22), Pit Boss ~27 (24). The early floor is easier and the Pit Boss harder than §7 was tuned for; retuning waits on the balance sim (#94).
- Ties are possible now and pay nobody. `Outcome::Payout(0)` is a standoff, and the coin is never offered on one.
- The screen grew a row: Opposing Cards, the player's row under them at the same card size, then the Draw. `CardSlot` carries a `Zone` so a click can mean "cover a slot" or "take that back", and the Peek can read a face-up Opposing Card the same way it reads the Draw.
- `Duel::house_edge` knows the whole truth including face-down cards; `Duel::showing` is what the screen is allowed to say. A face-up card whose Tell reads a face-down one counts for its Face Value alone, so the row never leaks more than the fact that something is hidden.
