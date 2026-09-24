# All In: pitch and demo script

Judges get minutes. This is what to say and what to press. The build is
at **https://voraciousjp.itch.io/all-in** (Linux zip; unzip, run
`all-in.sh`). The demo is the Arcade: the tutorial deals a fixed hand, so every judge sees the
same turn and every mechanic fires once. Have the game open at the Lobby
before they arrive (`cargo run`).

## The 60-second pitch

> Twenty-five years ago Lucky Jack bet everything at the Big Shots Table,
> lost it, and then bet his son. Tonight he walks back in to collect.
>
> All In is a card-combat roguelike. Every fight is a hand of cards against
> the House. You draw seven, play up to five, and the cards you play stack
> into your Hand. Clear the House Edge and the extra comes off the enemy's
> chips. Fall short and the difference comes off yours. That's the whole
> game, and it's enough, because two card types bend it: a Streak doubles
> if the card before it had a Tell, and an All In burns another card from
> your hand and takes its chips. So the order you play in is the skill.
>
> Then the House. It reads your Hand after your fourth card and sets its
> Edge just above it. Your fifth card is the one it can't see. Everything
> the game just taught you about leading with your best card is wrong at
> that table, and you find that out the hard way.
>
> Two developers, one artist, Rust and Bevy, thirty-six hours. It's a full
> run: three floors, four bosses, a rigged coin you can push your luck on,
> and an ending we won't spoil.

Say it standing up. If they cut you off at "the order you play in is the
skill", you've said the important part.

## The 3-minute demo: the Arcade

Sit them in front of the Lobby. Hand them the keyboard; you narrate.

"Press 2. This is our tutorial; it deals you a fixed hand and walks you
through one turn. You can't press a wrong key; the gold line tells you
the next one."

They press what the prompt says: **1, 1, 2, 1, 1, 1, Enter, P**.

What to say as it happens:

- After the 8: "Every card is worth its Face Value. That's your Hand
  building up top."
- After the burn: "That was an All In. It ate the 2 next to it and took
  its chips."
- After the two Streaks: "Both doubled, because the card before each had
  a Tell. Same five cards in a different order and they're worth half.
  The order is the skill."
- At the Push prompt: "You cleared the Edge by 12. Push and a coin flip
  doubles it, or zeroes your Hand. The tutorial's coin is rigged your way.
  The real one is 45/55, the House's."
- When the 24 floats up off the dealer: "That's the whole loop: build a
  Hand, beat the line, take the difference out of them."

Then the free turn. The dealer has 6 chips left and the prompt lets go.
Let them play it however they like; a Hand of 26 finishes it. If they
lead with a vanilla card, say nothing until the Streaks don't double,
then: "See? Same cards. Different order."

The ticket says WINNER and they're back in the Lobby. If you have thirty
seconds left, press **I** on any table for the glossary and say: "Every
term in the game fits on one screen."

Then the part they don't play:

"Upstairs there are three floors and four bosses, a perk after each
boss, and the House at the top. The House reads your Hand after your
fourth card and sets the line one above it, so your last card is the
only one it can't see. Everything the tutorial just taught you about
leading with your best card gets you killed up there. That's the game."

If they ask to see it: "Walk the Floor" (3) starts a real run, and the
first fight is a shill with 25 chips. Two turns. Don't go further than
that unless they ask again.

## The two lines that matter

When the House locks its Edge:

> "Play four. I'll set the line. Then show me your last card."

When the son walks in:

> "After twenty-five years, I thought you'd have learned by now. The
> House always wins, Pops."

## While they play: the concept-art deck

Have `docs/concept-art.pdf` open full-screen on the second screen, or on
the laptop if there's only one and nobody's holding the keyboard. Ten
slides, 16:9. One line when you point at it:

> "Everything on screen is drawn in code — no stock art, no generators.
> That deck is the design work behind it, including what we cut."

The three-tries and what-it-replaced slides are the interesting ones if a
judge asks a follow-up; they show the card face and the Opening frames
being rejected and redrawn.

## If something goes wrong

- Stale build after a pull: `cargo clean -p all-in && cargo run`.
- The window opens small: it's a windowed 1920×1080 layout; maximise it.
- A judge presses a key the tutorial isn't waiting for: nothing happens,
  the prompt stays. That's by design; point at the gold line.
- Esc in the Arcade goes back to the Lobby from anywhere. In a real run,
  Fold (2) at the next Fight or Fold prompt.
- The concept-art deck (`docs/concept-art.pdf`) is the fallback if the
  build won't start: pitch over the slides, then the tutorial on someone
  else's laptop.
