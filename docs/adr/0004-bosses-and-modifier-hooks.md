# Bosses are one file each, and every modifier bends the duel through one set of hooks

_Decided in #142. Replaces two consequences of ADR-0001: enemy numbers no longer live in one `Enemy::for_encounter`, and Perks no longer bend the enemy through `RunState::plays`/`RunState::blinds`._

A boss is its Deck, Boss Tell, Table Rule, Perk pair and Boss Pack, and today those are spread across `Enemy::for_encounter`, `RewardOffer::for_encounter`, the overworld's narrative and portrait tables, and a Hole Card field the duel was taught about by hand. Every one of them matches on `EncounterId`. We want adding a boss to mean writing one definition and nothing else, so the first boss added by someone who didn't build the framework (The Back Room's) touches no other file.

So each boss is one file under `src/boss/`, exporting a single `Boss` value: name, Chips, portrait, intro and win text, its hand-written Deck, its Boss Tell, its Table Rule, its Perk pair (The House: its Legacy Perk choice), and `pack_tells`, the number of Boss Tell cards in its Boss Pack (3 unless the boss says otherwise). `BOSSES` in `boss/mod.rs` lists them. An encounter is `Boss(&'static Boss)`, `Minion { floor }` or `Practice`, and nothing outside `boss/` matches on which boss it is. The boss does not know its floor; the floor layout lists bosses.

A Table Rule, a Perk and an Item are the same kind of thing to the duel: a **modifier** that answers some of a fixed set of hooks and ignores the rest. The hooks are `blind` (each side's Blind), `choose` (the enemy's choice of cards), `reveal` (what the player is shown before Confirm), `before_showdown` (bends Face Values or slots before any Tell resolves, so Tells still read Face Values in one pass), `coin` (Push Your Luck) and `after_showdown` (bends The Hand or House Edge once Tells have resolved). Each defaults to doing nothing. The duel takes one list of modifiers, applied in a fixed order: the Table Rule, then Perks in the order taken, then Items, each seeing the previous answer; a Blind never drops below 1. A hook is added only when a chosen boss, Perk or Item needs it, and removed when nothing answers it.

The run's unlocked Tells are not stored: `RunState` records the bosses beaten, and the pool is the Tells open from the start (Streak, All In) plus each beaten boss's Tell. Minion decks, Packs and the Cage all read that pool, and Fold resets it with everything else.

## Considered Options

- **One table in `run.rs`, as `Enemy::for_encounter` is today.** Rejected: rewards, narrative and portraits would still each need a match on the boss, so a new boss is a change in four files owned by two people.
- **Bosses as data files (RON/TOML).** Rejected: a Table Rule is behaviour, not numbers. A data file would name its rule, and some code would match that name to the rule, which is the match we're removing.
- **An enum of Table Rule kinds, matched in its own module.** Rejected: the duel wouldn't know the boss, but every new boss would still edit the shared enum and its match arms instead of only its own file.
- **Perks and Items stay enums, matched where they apply.** Rejected: `CONTEXT.md` already says items, Perks and Table Rules can all change the Blind, and three mechanisms for one effect is how `Coin::for_perks`, `RunState::plays` and `RunState::blinds` came to exist. One hook set means a boss's Perks live in the boss's file too.

## Consequences

- `Enemy::hole_card`, `Coin::for_perks`, `RunState::plays`, `RunState::blinds` and `RewardOffer::for_encounter` go away. The Hole Card is the first Table Rule written as a modifier, until The House loses it.
- The Arcade's demo dealer is `Practice`, never part of a run: it unlocks nothing, gives no reward, and doesn't read the pool.
- One test over `BOSSES` checks every boss: its Deck holds its Boss Tell and no other boss's, and `pack_tells` is at most 7. The Pit Boss carries Streak as a stand-in until its Boss Tell is picked, so the check that a Boss Tell is not open from the start goes in when that happens.
- The hook signatures wait on ADR-0003, which settles how an enemy chooses its row and what a reveal is.
- The Item pool (#153) adds `draw` (each side's Draw size) to the hook set for Deep Pockets, and building it (#158) adds `whiff` for Insurance: asked after every `after_showdown`, so it halves the Whiff actually left whatever order the Items were taken in. Items are spent on demand, so a modifier from an Item is in the duel's list only on the Hands, or for the rest of the encounter, that the player spent a use on.
- The Wheel (#167) adds its run-long modifiers after the Perks and before the Items, and Early Read gives `reveal` the turn it's asked on, so a reveal can last only the first turns of a duel.
