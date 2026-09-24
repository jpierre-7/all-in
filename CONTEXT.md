# All In

A text-adventure roguelike set in a casino. Every encounter is a card duel against the House: the enemy lays a row of cards down, the player covers it card for card, both rows turn over, and the lower total pays the difference.

## Language

### Fighters

**Chips**:
A fighter's remaining life. Only the player and the enemy have Chips; cards have a Face Value instead. The player's Chips carry between encounters and only come back by buying them with Cash.
_Avoid_: Stack, HP, health, life

**House Edge**:
The Opposing Cards' Stack Sum: the number The Hand has to beat this turn. Not a flat number the enemy carries around — it is whatever the enemy actually laid down, and the player only ever sees part of it before confirming.
_Avoid_: defense, armor, threshold

**Rising Blinds**:
Scheduled escalation as combat goes on: one more Opposing Card for ordinary enemies, the Margin for The House. Past the player's Plays the extra cards cannot be covered at all.
_Avoid_: scaling, difficulty ramp

**Margin**:
How far above the row it read The House sets its own. Rises with the Blinds.
_Avoid_: house cut, spread

**Hole Card**:
Against The House only: the last card in the player's row. The House keeps its own last Opposing Card back and fills it in on everything the player played before that one, so the Hole Card is the one card it could not see.
_Avoid_: last card, closer

### A turn

**Deal**:
How an enemy lays its Opposing Cards down: how many, the range of Face Values, the odds of a Tell and which Tells, and the odds of each card after the first being face down. Enemies have a Deal, not a deck.
_Avoid_: enemy deck, AI

**Opposing Cards**:
The enemy's row, laid down before the player plays anything. The first is always face up; each of the rest is dealt face down on a roll the enemy's own odds set. The player's cards sit one per slot across from them.
_Avoid_: enemy hand, their cards, board

**Slot**:
One column of the table: an Opposing Card and whatever the player put across from it. Which card faces which is what every Tell now reads.
_Avoid_: position, lane, index

**Stack Sum**:
What a row adds up to once every Tell in it has resolved. There is one per side of the table, and the side with the lower one loses the difference off its Chips.
_Avoid_: total, score, sum

**The Hand**:
The player's row, and its Stack Sum. Built by covering slots, and worth nothing until the rows turn over.
_Avoid_: score, total

**Lift**:
Taking a card back out of the row before Confirm. It returns to the Draw, and an All In hands back the card it burned.
_Avoid_: take back, undo, remove

**Confirm**:
The player saying the row is finished. Both rows turn over, every Tell resolves, and the Stack Sums are compared.
_Avoid_: submit, lock in, end turn

**Showdown**:
The moment the rows turn over on Confirm: every face-down card is shown, every Tell resolves, and the two Stack Sums are compared. What each card came to stays on the table until the next turn.
_Avoid_: reveal, resolution

**Draw**:
The cards the player is holding this turn. Refilled to 7 at the start of each turn.
_Avoid_: hand (reserved for The Hand), cards in hand

**Plays**:
The number of cards the player may put in the row in one turn. A row of Opposing Cards longer than this leaves slots the player cannot cover, and what sits in them counts for the enemy anyway.
_Avoid_: actions, energy, mana

**Push Your Luck**:
An optional coin flip offered once The Hand is final, whether it clears House Edge or falls short. On a clearing Hand, push and win: the Payout doubles; push and lose: The Hand becomes 0, a full Whiff. On a Whiff, push and win: the Whiff is forgiven; push and lose: it doubles. The Hand itself is the stake; there is no separate wager.
_Avoid_: gamble, double-or-nothing, wager

**Push / Hold**:
The two answers to Push Your Luck. Hold resolves the turn as normal.
_Avoid_: yes/no, gamble/pass

**Payout**:
Damage dealt to the enemy's Chips when The Hand beats House Edge: the difference between the two Stack Sums.
_Avoid_: damage, overflow

**Whiff**:
A Hand whose Stack Sum is the lower of the two. The difference is dealt to the player's own Chips.
_Avoid_: miss, fail, bust

### Cards

**Face Value**:
The number printed on a card: what it adds to The Hand before its Tell.
_Avoid_: Stack, base value, value, pips

**Tell**:
A single passive keyword on a card that modifies how it resolves. A card has at most one Tell.
_Avoid_: keyword, effect, ability, modifier

Every Tell reads the table by **slot**, not by the order cards were picked up, and every one of them reads a card's **Face Value** rather than what that card resolved to. That is what lets both rows be worked out in one pass, in either order, with a Flop on each side of the table reading the other without chasing it in a circle.

**Streak**:
Tell: doubles the card's Face Value if the card in the slot to its left has any Tell.

**All In**:
Tell: sacrifice another card from the Draw to add its Face Value to this card's. The only Tell that reads no slot, so it is worth the same wherever it sits.

**Copycat**:
Tell: takes the Face Value of the card in the slot to its right, and none of its Tell; its own if nothing follows it.

**Flop**:
Tell: takes the Face Value of the Opposing Card across from it, and none of its Tell; its own if nothing is across. Until the rows turn over, a face-down card counts as nothing across. Ties its own slot exactly, which makes it worth whatever the enemy kept hidden there.

**Peek**:
The tag that opens beside the card the player points at, mouse or keyboard, and says its Tell. The player can wave it off for the table and call it back.
_Avoid_: tooltip, hover text, popup, hint

### Run

**Fight or Fold**:
The choice before each encounter. Fight starts the duel; Fold abandons the run to the Lobby, resetting perks, items, Cash, and deck changes.
_Avoid_: engage/retreat, flee

**Perk**:
A run-long modifier chosen from a pair after beating a boss.

**Item**:
A run-long modifier dropped after beating a minion. The pool is Loaded Dice alone so far (#12); the drop is random once there is more than one thing in it.

**Loaded Dice**:
Item: +5 to The Hand for the next 2 Hands. Spent as The Hand is shown, and carried between encounters until it runs out.
_Avoid_: buff, charge

### Cash

**Cash**:
The currency of a run, spent in the Cage. Separate from Chips: losing Cash never hurts the player, and Chips are only Cash's merchandise. Resets on Fold or death.
_Avoid_: money, gold, coins, chips

**Side Bet**:
A mission revealed when the duel starts, three per encounter, drawn from a pool (e.g. play two All In cards; win without losing Chips); bosses draw from a pool of their own. Each one completed pays Cash when the encounter is won. Nothing is staked, and there are none against The House. The player can wave them off the table and call them back, like the Peek.
_Avoid_: quest, challenge, objective

**Interest**:
Cash paid on the Cash the player holds at the end of a won encounter, a fixed amount per block held, up to a cap.

**Comp**:
The free card after every won encounter: pick one of three to add to the deck, or pass.
_Avoid_: draft, pick, reward card

**Pack**:
A sealed set of four random cards; the player opens it and keeps two. A boss reward and sold in the Cage.
_Avoid_: booster, bundle

**The Cage**:
The shop, where Cash buys cards, Packs, Items, and Chips. One per floor, found by exploring it like any room.
_Avoid_: store, merchant
