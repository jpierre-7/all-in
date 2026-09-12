# All In

A text-adventure roguelike set in a casino. Every encounter is a card duel against the House, resolved by building a running total and clearing the enemy's threshold.

## Language

### Fighters

**Stack**:
A quantity of chips. Player and enemy each have a Stack (their remaining life); a card has a Stack (the value it contributes to The Hand). Everything in the game is chips.
_Avoid_: HP, health, life, base value

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
An optional coin flip offered once The Hand is final and clears House Edge. Push and win: the Payout doubles. Push and lose: The Hand becomes 0, a full Whiff. The Hand itself is the stake; there is no separate wager.
_Avoid_: gamble, double-or-nothing, wager

**Push / Hold**:
The two answers to Push Your Luck. Hold resolves the turn as normal.
_Avoid_: yes/no, gamble/pass

**Payout**:
Damage dealt to the enemy's Stack when The Hand clears House Edge: the excess over it.
_Avoid_: damage, overflow

**Whiff**:
A Hand that falls short of House Edge. The shortfall is dealt to the player's own Stack.
_Avoid_: miss, fail, bust

### Cards

**Tell**:
A single passive keyword on a card that modifies how it resolves. A card has at most one Tell.
_Avoid_: keyword, effect, ability, modifier

**Streak**:
Tell: doubles the card's Stack if the previous card played this turn had any Tell.

**All In**:
Tell: sacrifice another card from the Draw to add its Stack to The Hand.

### Run

**Fight or Fold**:
The choice before each encounter. Fight starts the duel; Fold abandons the run to the Lobby, resetting perks, items, and deck changes.
_Avoid_: engage/retreat, flee

**Perk**:
A run-long modifier chosen from a pair after beating a boss.

**Item**:
A run-long modifier dropped at random after beating a minion.

**Loaded Dice**:
Item: +5 to The Hand for the next 2 Hands. Spent as The Hand is shown, and carried between encounters until it runs out.
_Avoid_: buff, charge
