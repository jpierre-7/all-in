# All In

A text-adventure roguelike set in a casino. Every encounter is a card duel against the House: the enemy lays a row of cards down, the player covers it card for card, both rows turn over, and the lower total pays the difference.

## Language

### Fighters

**Stack**:
A quantity of chips. Player and enemy each have a Stack (their remaining life); a card has a Stack (the value it contributes to its row's Stack Sum). Everything in the game is chips.
_Avoid_: HP, health, life, base value

**House Edge**:
The Opposing Cards' Stack Sum: the number The Hand has to beat this turn. Not a flat threshold the enemy carries around — it is whatever the enemy actually laid down, and the player only ever sees part of it before confirming.
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

**Opposing Cards**:
The enemy's row, laid down before the player plays anything. The first is always face up; each of the rest is dealt face down on a roll the enemy's own odds set. The player's cards sit one per slot across from them.
_Avoid_: enemy hand, their cards, board

**Slot**:
One column of the table: an Opposing Card and whatever the player put across from it. Which card faces which is what every Tell now reads.
_Avoid_: position, lane, index

**Stack Sum**:
What a row adds up to once every Tell in it has resolved. There is one per side of the table, and the side with the lower one loses the difference off its Stack.
_Avoid_: total, score, sum

**The Hand**:
The player's row, and its Stack Sum. Built by covering slots, and worth nothing until the rows turn over.
_Avoid_: score, total

**Confirm**:
The player saying the row is finished. Both rows turn over, every Tell resolves, and the Stack Sums are compared.
_Avoid_: submit, lock in, end turn

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
Damage dealt to the enemy's Stack when The Hand beats House Edge: the difference between the two Stack Sums.
_Avoid_: damage, overflow

**Whiff**:
A Hand whose Stack Sum is the lower of the two. The difference is dealt to the player's own Stack.
_Avoid_: miss, fail, bust

### Cards

**Tell**:
A single passive keyword on a card that modifies how it resolves. A card has at most one Tell.
_Avoid_: keyword, effect, ability, modifier

Every Tell reads the table by **slot**, not by the order cards were picked up, and every one of them reads a card's **printed** Stack rather than what that card resolved to. That is what lets both rows be worked out in one pass, in either order, with a Flop on each side of the table reading the other without chasing it in a circle.

**Streak**:
Tell: doubles the card's Stack if the card in the slot to its left has any Tell.

**All In**:
Tell: sacrifice another card from the Draw to add its Stack to this card's. The only Tell that reads no slot, so it is worth the same wherever it sits.

**Copycat**:
Tell: takes the printed Stack of the card in the slot to its right, and none of its Tell; its own if nothing follows it.

**Flop**:
Tell: takes the printed Stack of the Opposing Card across from it, and none of its Tell; its own if nothing is across. Ties its own slot exactly, which makes it worth whatever the enemy kept hidden there.

**Peek**:
The tag that opens beside the card the player points at, mouse or keyboard, and says its Tell. The player can wave it off for the table and call it back.
_Avoid_: tooltip, hover text, popup, hint

### Run

**Fight or Fold**:
The choice before each encounter. Fight starts the duel; Fold abandons the run to the Lobby, resetting perks, items, and deck changes.
_Avoid_: engage/retreat, flee

**Perk**:
A run-long modifier chosen from a pair after beating a boss.

**Item**:
A run-long modifier dropped after beating a minion. The pool is Loaded Dice alone so far (#12); the drop is random once there is more than one thing in it.

**Loaded Dice**:
Item: +5 to The Hand for the next 2 Hands. Spent as The Hand is shown, and carried between encounters until it runs out.
_Avoid_: buff, charge
