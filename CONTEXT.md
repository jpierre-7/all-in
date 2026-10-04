# All In

A text-adventure roguelike set in a casino. Every encounter is a card duel against the House: both sides draw from their own decks and lay a row of cards face down, both rows turn over, and the lower total pays the difference.

## Language

### Fighters

**Chips**:
A fighter's remaining life. Only the player and the enemy have Chips; cards have a Face Value instead. The player's Chips carry between encounters and only come back by buying them with Cash.
_Avoid_: Stack, HP, health, life

**Boss**:
An enemy written by hand in one place: its Deck, its Boss Tell, its Table Rule, its Boss Pack, and the Perk pair offered for beating it (The House offers a Legacy Perk choice instead). Beating it unlocks its Boss Tell for the rest of the run.
_Avoid_: elite, champion

**Minion**:
An enemy whose Deck is dealt from the standard template for its floor, its Tells drawn from every Tell the run has unlocked so far.
_Avoid_: mob, grunt, regular enemy

**House Edge**:
What the Opposing Cards add up to once every Tell in them has resolved: the number The Hand has to beat this turn. Not a flat number the enemy carries around: it is whatever the enemy actually laid down, and the player sees none of it before confirming unless a Reveal turns some of it over.
_Avoid_: defense, armor, threshold

**Table Rule**:
A boss's own rule for its fight: it bends how the duel plays while that boss is at the table, and never leaves it. One per boss. It may change how the boss chooses its cards, or when it lays them down.
_Avoid_: gimmick, mechanic, house rule

### A turn

**Deck**:
The cards a fighter draws from. Every enemy has one, as the player does: a boss's is written for it and carries its Boss Tell; a minion's is dealt from one standard template when the fight starts.
_Avoid_: Deal (retired), pool, library

**Opposing Cards**:
The enemy's row. The enemy draws and chooses its cards by the same rules as the player, and lays them down face down at the start of the turn, before the player has placed anything. The player sees how many there are and nothing else until Confirm, unless a Reveal turns some over. The player's cards sit one per slot across from them.
_Avoid_: enemy hand, their cards, board

**Slot**:
One column of the table: an Opposing Card and whatever the player put across from it. Both rows fill from the left, and there are as many slots as the larger of the two Blinds; a slot with nothing in it is worth nothing. Which card faces which is what every Tell now reads.
_Avoid_: position, lane, index

**The Hand**:
The player's row, and what it adds up to once every Tell in it has resolved. Built by covering slots, and worth nothing until the rows turn over. Whichever of The Hand and House Edge is lower, that side loses the difference off its Chips.
_Avoid_: score, total, Stack Sum

**Lift**:
Taking a card back out of the row before Confirm. It returns to the Draw, and an All In hands back the card it burned.
_Avoid_: take back, undo, remove

**Confirm**:
The player saying the row is finished. Both rows turn over, every Tell resolves, and The Hand meets House Edge.
_Avoid_: submit, lock in, end turn

**Showdown**:
The moment the rows turn over on Confirm: both rows are shown, every Tell resolves, and The Hand meets House Edge. What each card came to stays on the table until the next turn.
_Avoid_: reveal, resolution

**Draw**:
The cards a fighter is holding this turn; the enemy has one too, which the player never sees. Refilled to 7 from that fighter's Deck at the start of each turn; cards not played stay in it. Played cards go to that fighter's discard, which is shuffled back into the Deck when the Deck runs dry.
_Avoid_: hand (reserved for The Hand), cards in hand

**Blind**:
The most cards either side may put in its row in one turn. Set by the floor (2 on The Floor, one more on each floor after it) and the same for both sides; items, perks, and a Table Rule can raise or lower it for one side. Playing fewer is allowed.
_Avoid_: Plays, Rising Blinds, actions, energy, mana

**Reveal**:
An effect (an Item, a Perk, or a Table Rule) that turns chosen Opposing Cards face up before Confirm, for this turn only.
_Avoid_: peek (that is the tag), scout, spy

**Push Your Luck**:
An optional coin flip offered to the player, never the enemy, once The Hand is final, whether it clears House Edge or falls short. On a clearing Hand, push and win: the Payout doubles; push and lose: The Hand becomes 0, a full Whiff. On a Whiff, push and win: the Whiff is forgiven; push and lose: it doubles. The Hand itself is the stake; there is no separate wager.
_Avoid_: gamble, double-or-nothing, wager

**Push / Hold**:
The two answers to Push Your Luck. Hold resolves the turn as normal.
_Avoid_: yes/no, gamble/pass

**Payout**:
Damage dealt to the enemy's Chips when The Hand beats House Edge: the difference between them.
_Avoid_: damage, overflow

**Whiff**:
A Hand lower than House Edge. The difference is dealt to the player's own Chips.
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
Tell: takes the Face Values of the Opposing Card across from it and of that card's two neighbours, and none of their Tells; never its own. An empty slot counts as nothing, so it is strong against a long row and weak at the end of one. The House's Boss Tell.

**Boss Tell**:
The Tell a boss's deck is built around. Nobody else has it until that boss is beaten in the run; from then on it can turn up in minions' decks, Packs, and the Cage. Locked again on Fold or death.
_Avoid_: signature card, unlock

**Peek**:
The tag that opens beside the card the player points at, mouse or keyboard, and says its Tell. Where a Tell would read Opposing Cards that are still face down, it says what it knows and marks what it can't see. The player can wave it off for the table and call it back.
_Avoid_: tooltip, hover text, popup, hint

### Run

**Fight or Fold**:
The choice before each encounter. Fight starts the duel; Fold abandons the run to the Lobby, resetting perks, items, Cash, and deck changes.
_Avoid_: engage/retreat, flee

**Perk**:
A run-long modifier chosen from a pair after beating a boss.

**Item**:
A run-long modifier. After beating a minion the player is offered either three Items, keeping one, or a Pack; or may take nothing.

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

**Pack**:
A sealed set of cards; the player keeps some and the rest are gone. After a minion: three cards, keep one. After a boss, the Boss Pack: seven cards, keep two, a set number of them carrying that boss's Boss Tell.
_Avoid_: booster, bundle, Comp

**The Cage**:
The shop, where Cash buys cards, Packs, Items, and Chips. One per floor, found by exploring it like any room.
_Avoid_: store, merchant

### Between runs

**Golden Chips**:
What beating The House earns, kept from one run to the next, and spent at The Wheel. Always "Golden Chips" in full, never "Chips", which is a fighter's life.
_Avoid_: gold, tokens, meta currency

**The Wheel**:
The roulette table where Golden Chips are placed on squares; each square buys a permanent buff for future runs. Opens after The House is first beaten.
_Avoid_: shop, skill tree, upgrades

**Legacy Perk**:
A Perk chosen after beating The House that lasts for the next run only.
_Avoid_: special perk, bonus
