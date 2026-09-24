# Onboarding

You have the repo; this is the order to read it in and how work is handed out.
Budget an hour for the reading and an afternoon for the first build — Bevy
compiles are slow the first time (see "Build times" in the README).

## Day one

1. **Play the game.** The itch build (linked from the README) or `cargo run
   --release` from the repo root. One full run, Lobby to ending. Everything
   below assumes you have.
2. **Read `CONTEXT.md`.** It is the game's vocabulary — Chips, Face Value, The Hand, House
   Edge, Whiff, Tell, and the rest — and it is short. Code, tickets, and
   conversation all use these words exactly; the `_Avoid_` lines are the
   synonyms we do not use. Read it before naming anything.
3. **Read `game-design-doc.md`.** The game we are building toward. The
   hackathon shipped its MVP; the current work restores what was cut.
4. **Skim `docs/adr/`.** One ADR so far: the combat ↔ overworld seam. It is
   the one boundary in the codebase you should know before touching either
   side.
5. **Build it.** README → "Running it". The Nix devshell is the path of least
   pain; without it, install the system libraries the README lists. Then
   `cargo test` — the whole suite should be green.

## Where the work is

All work is tracked on the GitHub issue tracker as a **wayfinder map**: a
single issue labelled `wayfinder:map` whose child issues are the tickets.

The current map is **[All In: the design-doc
game](https://github.com/jpierre-7/all-in/issues/86)**. Its body is the
low-resolution view: the destination, the standing notes, the decisions made so
far (one line each, linking to the ticket that holds the detail), and the fog —
work we know is coming but cannot ticket yet. Read the map body once; it is
kept current.

Tickets carry a `wayfinder:<type>` label:

| Type | What it is | Who |
| --- | --- | --- |
| `task` | Build or setup work that is already specified. Nothing to decide. | Anyone |
| `grilling` | A design decision, resolved in conversation with whoever holds the design. | Usually John |
| `prototype` | A rough artifact built to answer "how should it behave / look". | Anyone, with the design holder reacting |
| `research` | A fact-finding read of docs or APIs, written up under `docs/research/`. | Anyone, often an agent |

**The frontier** is the set of open tickets with no open blocker and no
assignee. GitHub shows blocking natively — a blocked ticket says so at the top
of its page — so the frontier is visible without opening the map.

## Claiming a ticket

Assign yourself. That is the whole protocol:

```sh
gh issue edit <number> --add-assignee @me
```

An open, unassigned ticket is unclaimed; an assigned one is taken. Claim before
you start, not after, so nobody else picks it up in parallel. If you stop
working on it, unassign yourself so it returns to the frontier.

Pick from the frontier; do not start a blocked ticket. If a ticket's question
turns out to depend on a decision nobody has made, say so on the ticket rather
than deciding it yourself.

Good first tickets are the `task` ones: self-contained, fully specified, and
touching different files from each other.

## Finishing a ticket

1. Open a PR to `main`, small and frequent. `cargo build` and `cargo test`
   green before merge; the `ci` workflow runs both on every PR.
2. Post the outcome as a comment on the ticket — what was done, and any fact a
   later ticket will need (numbers, paths, URLs) — and close it.
3. Add one line to the map's **Decisions so far**: the ticket title as a link,
   then a one-line gist. The map is an index, not a store: the detail stays on
   the ticket.

If the work touched the game's vocabulary, update `CONTEXT.md` in the same PR.
If it changed the combat ↔ overworld seam, it needs an ADR (`docs/adr/`).

## Working with an agent

The repo is set up for Claude Code: `CLAUDE.md` points at the skill docs in
`docs/agents/`, and the wayfinder map is what `/wayfinder 86` loads. An agent
session resolves one ticket at a time and follows the same claim → resolve →
index protocol above. Research tickets are the ones it does well alone;
grilling tickets need a human who can answer for the design.

## Matt Pocock Skills

This is the skills suite we use to drive our development with agents. You can find the repo and documentation below.

[mattpocock-skills](https://github.com/mattpocock/skills)

[AI Skills for Real Engineers (Docs)](https://www.aihero.dev/skills)

## Who is who

| | GitHub | Role |
| --- | --- | --- |
| John | @jpierre-7 | Leads; drives the map; holds the design |
| Nelson | @HefKer | Dev |
| Andrei | @ambornstein | Dev |
| Rui | @ruiiiijiiiiang | Dev |
| Madison | @MEmshousen | Art |

Ownership is per ticket, not per module. The hackathon's module split (combat
vs overworld) is history; the seam it left behind is documented in the ADR.
