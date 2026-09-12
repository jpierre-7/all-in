# Combat and overworld meet at a resource-in, resource-out seam

Two devs build in parallel over one weekend, so the seam between combat (Dev 1) and overworld (Dev 2) has to be small enough to freeze on day one. Overworld inserts an `Encounter` resource and sets `AppState::Combat`; combat runs the whole duel including its UI, mutates `RunState.stack` in place, inserts `CombatOutcome`, and sets `AppState::PostCombat` — the only transition it ever makes. Overworld routes from there. All shared types (`RunState`, `Card`, `Enemy`, `Reward`, the seam resources) live in `src/run.rs`, owned by Dev 1; `src/state.rs` holds the one flat `AppState` and is frozen.

## Considered Options

- **Overworld drives a pure `Duel` state machine and renders it.** Rejected: puts card rendering on Dev 2's plate and makes every combat change a two-person change. The pure `Duel` still exists, but as combat's internal module (the test surface for TDD), not the seam.
- **Nested / sub-states for combat phases.** Rejected: a flat enum with two combat-facing variants (`Combat`, `PostCombat`) is all overworld needs to know; combat's turn phases are its own business.

## Consequences

- The reward screen is split by the seam: overworld renders the pick, `RunState::apply(Reward)` in `run.rs` does the mutation, combat only consumes perks and items during a duel.
- Combat's write-back is the Stack **and** anything a duel spends (#12): Loaded Dice are counted down at the table and the remainder rides back out on `RunState`. `apply` takes a seed alongside the `Reward`, for the same reason `Duel::new` takes its deck pre-shuffled — the randomness stays the caller's, so `run.rs` is deterministic under test.
- Perks bend the enemy at the table, not in the table: `Enemy::for_encounter` is still the one place the numbers are *authored*, and `RunState::plays`/`RunState::blinds` are the one place a perk changes them on the way into a duel (#12).
- Enemy numbers live in one function, `Enemy::for_encounter`, so balancing is a one-file change.
- Editing `run.rs` or `state.rs` from the overworld side goes through a PR to Dev 1.
