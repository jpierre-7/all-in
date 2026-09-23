# All In

A text-adventure roguelike set in a casino. Every encounter is a card duel against the House, resolved by building a running total and clearing the enemy's threshold.

## Language

### Fighters

**Chips**:
A fighter's remaining life. Only the player and the enemy have Chips; cards have a Face Value instead. The player's Chips carry between encounters and only come back by buying them with Cash.
_Avoid_: Stack, HP, health, life

**House Edge**:
The enemy's defense threshold for one turn. The Hand must reach it.
_Avoid_: defense, armor, threshold

**Rising Blinds**:
Scheduled escalation as combat goes on: House Edge for ordinary enemies, the Margin for The House.
_Avoid_: scaling, difficulty ramp

**Margin**:
How far above the player's read Hand The House sets its Edge. Rises with the Blinds.
_Avoid_: house cut, spread

**Hole Card**:
Against The House only: the player's final Play of the turn, made after the House has locked its Edge on everything played before it.
_Avoid_: last card, closer

### A turn

**The Hand**:
The running total built by the cards played this turn.
_Avoid_: score, total

**Draw**:
The cards the player is holding this turn. Refilled to 7 at the start of each turn.
_Avoid_: hand (reserved for The Hand), cards in hand

**Plays**:
The number of cards the player may play from the Draw in one turn.
_Avoid_: actions, energy, mana

**Push Your Luck**:
An optional coin flip offered once The Hand is final, whether it clears House Edge or falls short. On a clearing Hand, push and win: the Payout doubles; push and lose: The Hand becomes 0, a full Whiff. On a Whiff, push and win: the Whiff is forgiven; push and lose: it doubles. The Hand itself is the stake; there is no separate wager.
_Avoid_: gamble, double-or-nothing, wager

**Push / Hold**:
The two answers to Push Your Luck. Hold resolves the turn as normal.
_Avoid_: yes/no, gamble/pass

**Payout**:
Damage dealt to the enemy's Chips when The Hand clears House Edge: the excess over it.
_Avoid_: damage, overflow

**Whiff**:
A Hand that falls short of House Edge. The shortfall is dealt to the player's own Chips.
_Avoid_: miss, fail, bust

### Cards

**Face Value**:
The number printed on a card: what it adds to The Hand before its Tell.
_Avoid_: Stack, base value, value, pips

**Tell**:
A single passive keyword on a card that modifies how it resolves. A card has at most one Tell.
_Avoid_: keyword, effect, ability, modifier

**Streak**:
Tell: doubles the card's Face Value if the previous card played this turn had any Tell.

**All In**:
Tell: sacrifice another card from the Draw to add its Face Value to The Hand.

**Copycat**:
Tell: takes the Face Value of the next card played this turn, and none of its Tell; its own if no card follows.

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
