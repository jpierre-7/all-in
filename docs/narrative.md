# All In: Narrative

Every string the overworld renders, in the order the player meets it. Each
heading is the constant name Dev 2 lifts into `src/overworld/narrative.rs`.
Keep lines short; this is Bevy text on a dark screen, not a novel.

Tone: casino-noir, pulpy, a little mean. Second person, present tense.
Lucky Jack is "you" and the House is always capitalised.

---

## TITLE

The first screen, before the Opening. The game's name at marquee size and
nothing else on the felt with it.

> **ALL IN**

## PRESS_SPACE

The footer under the marquee. Space, and only Space, gets you in — every other
screen in the shell takes any key, so this one names its key.

> *Press Space to play.*

## OPENING

Five frames, one scene each, any key advances, Esc skips to the Lobby.
`narrative.rs` lifts them as `OPENING_1` … `OPENING_5`; the art slots are
`assets/backstory/opening_N.png`. The *scene* line under each frame is for
the artist, not the screen.

### OPENING_1

> Twenty-five years ago you sat down at the Big Shots Table with everything
> you owned, and it took the House about an hour to take it off you.

*Scene: a younger Jack at the one table under the one lamp, chips stacked
high in front of him, a pair of hands across the felt.*

### OPENING_2

> That should've been the end of it. Instead you did the thing this whole
> building still whispers about when the shift changes: you put your boy on
> the felt.

*Scene: the same table, the chips gone, a small boy standing beside the
chair. The hands haven't moved.*

### OPENING_3

> Your firstborn, because you were sure the next hand was yours.
>
> It wasn't. The House took him the way it takes everything: without
> looking up.

*Scene: the cards turned over on the felt, the chair beside Jack empty, the
lamp the only light left.*

### OPENING_4

> That was twenty-five years ago. They still call you Lucky Jack. Nobody
> remembers why, and you've stopped correcting them.

*Scene: Jack now, older, in a coat that's seen better decades, under the
casino sign at night from across the street.*

### OPENING_5

> Tonight you walk back through the doors with a deck in your coat pocket
> and nothing else worth taking. Twenty-five years is a long time to owe
> someone, and you've come to collect.

*Scene: the casino doors from inside, swinging shut behind him, the lobby
carpet ahead.*

*(~130 words across five frames)*

## ANY_KEY_OR_SKIP

The footer under every Opening frame. It names Esc, because a skip nobody can
find is no skip — and the Opening is the one screen a returning player has
already read.

> *Press any key — or Esc to skip ahead.*

## LOBBY

> The lobby smells like carpet shampoo and old cigarettes; a slot machine
> near the coat check chirps at nobody in particular. Somewhere above you
> the Big Shots Table is set for one.

**Options**

1. `LOBBY_OPT_INFO`: **Case the joint** *(Info Room)*
2. `LOBBY_OPT_TUTORIAL`: **Warm up on the arcade** *(Tutorial)*
3. `LOBBY_OPT_BEGIN`: **Walk the Floor** *(Begin Run)*

## INFO_ROOM

> A cork board behind the coat check where somebody has pinned up the odds
> in pencil.
>
> **The Floor.** Small fish. Slotz runs the tables down here, loud and cheap
> and beatable if you don't get greedy.
>
> **The Pit.** Where the regulars lose their houses. The Pit Boss weighs
> every chip that crosses the rope, and it's never once been wrong.
>
> **The Big Shots Table.** The House sits here, and the House already knows
> what's in your Hand.
>
> *Press any key to go back.*

## TUTORIAL

The Arcade is a scripted encounter (#40), not a text screen. It enters
combat as `EncounterId::Tutorial` against **THE DEMO DEALER** (Stack 30,
House Edge 20, no Blinds), with a fixed deal and a rigged coin. Esc at
any point goes back to the Lobby; the run is never touched.

### TUTORIAL_INTRO

> An arcade cabinet, sticky with spilled rum. The screen flickers on and
> deals you a hand. Somebody's scratched into the bezel: *play what it
> tells you, then play what you like.*
>
> *Esc leaves at any time.*

### Turn 1: gated. The prompt names the key; any other key repeats it.

The deal, in Draw order: Pawned Ring 8 · Last Dollar (All In 2) · Two of
Clubs 2 · Hot Streak 3 · Dealer Blinks 5 · Four of Hearts 4 · Cheap Seat 3.

| Step | Constant | Prompt | Key |
| --- | --- | --- | --- |
| 1 | `TUTORIAL_STEPS[0]` | Every card is worth its Stack. Press **1** to play Pawned Ring for 8. | 1 |
| 2 | `TUTORIAL_STEPS[1..=2]` | Last Dollar is **All In**: it burns another card and takes its chips. Press **1** to play it, then **2** to burn Two of Clubs (the All In stays in slot 1 until the burn is named). | 1, 2 |
| 3 | `TUTORIAL_STEPS[3]` | Hot Streak is a **Streak**: it doubles if the card before it had a Tell. It did. Press **1**. | 1 |
| 4 | `TUTORIAL_STEPS[4]` | Dealer Blinks is a Streak too, and the card before it had a Tell. Press **1** for 10. | 1 |
| 5 | `TUTORIAL_STEPS[5]` | Last Play. Press **1** to add Four of Hearts. Your Hand is 32 against a House Edge of 20. | 1 |
| 6 | `TUTORIAL_STEPS[6]` | Press **Enter** to show your Hand. | Enter |
| 7 | `TUTORIAL_STEPS[7]` | You cleared the Edge by 12: that's your **Payout**. Or **Push Your Luck**: press **P** and a coin flip doubles it, or zeroes your Hand. This coin is rigged your way; the real one is 45/55. | P |

Expected: 8 → +4 → +6 → +10 → +4 = 32. Push wins on the rigged coin.
Payout 24 leaves the dealer on 6.

### Turn 2: free play, one hint, until the dealer is done (a Hand of 26 finishes it).

`TUTORIAL_HINT`. Leaving early shows `TUTORIAL_LEFT` instead of `TUTORIAL_DONE`.

> Lead with a Tell so your Streaks double, and burn your smallest card to an
> All In. In a real duel the **Blinds rise** every few turns, so don't sit
> here all night.

### TUTORIAL_LEFT

> You walk away from the cabinet mid-hand. It doesn't seem to mind.

### TUTORIAL_DONE

> The cabinet spits out a paper ticket that says WINNER and nothing else.
> Upstairs, nobody rigs the coin.
>
> *Press any key.*

---

## FLOOR_INTRO: The Floor

> The main floor, where rows of slots blink like a migraine and the cocktail
> waitresses stopped smiling sometime in the nineties. This is where the
> House lets you win a little, so you'll stay.
>
> You aren't here to stay.

## PIT_INTRO: The Pit

> Down a short flight of stairs the noise dies off. The Pit is quiet money:
> velvet rope, real clay chips, dealers who don't blink. Everybody at these
> tables has lost something that wasn't cash, and most of them are still
> trying to win it back.
>
> You'd know.

## BIG_SHOTS_INTRO: The Big Shots Table

> One table under one lamp, and the felt is the exact green you've seen on
> the inside of your eyelids for twenty-five years.
>
> The House pulls out your chair. It remembers you.

---

## Encounter intros

### ENC_FLOOR_MINION

> A shill in a rented tux slides into the seat across from you. "House rules,
> pal. You play or you leave, and you don't look like the leaving type."

### ENC_SLOTZ

> **SLOTZ.** Three feet of chrome and neon on a rolling base, arms spinning,
> grinning the way only a machine can. It doesn't want your money; it wants
> your time, and it's got all night.

### ENC_PIT_MINION

> A dealer with a scar under one eye shuffles without looking down. "The
> Boss weighed you when you walked in. Came up light. So you don't go
> upstairs."

### ENC_PIT_BOSS

> **THE PIT BOSS.** A brass balance scale the height of a man, two pans
> hanging off a beam that creaks when it turns to look at you. One pan is
> already piled with chips; the other holds nothing yet. "Jack. Twenty-five
> years, and you came back with *that* Stack? Put it on the pan. Let's see
> what it's worth."

### ENC_THE_HOUSE

> **THE HOUSE.** No face, just a pair of hands resting on the felt and a
> voice that comes from the walls. "Sit down, Jack. Let's see what you
> learned. Play four. I'll set the line. Then show me your last card."

## FIGHT_OR_FOLD

> **Fight**: sit down and play.
> **Fold**: walk back to the lobby, and leave behind everything you picked
> up tonight.

---

## Encounter outcomes

### WIN_MINION

> The chair scrapes back empty and you pocket whatever they left on the felt.

### WIN_SLOTZ

> The reels spin once more, land on nothing, and the neon goes out. Somewhere
> off in the dark, something bigger clears its throat.

### WIN_PIT_BOSS

> The beam tips your way and stays there, and for a long moment the only
> sound in the Pit is brass settling. "Upstairs," it says at last. "He's
> been expecting you."

### WIN_THE_HOUSE

> The House's hands go still on the felt.
>
> For the first time in twenty-five years, the table is yours.

### LOSE

> Your Stack hits the felt and stops, and the dealer doesn't bother looking
> up.
>
> "House always wins, pal."

### FOLD

> You push back from the table and nobody stops you. Nobody ever does.
>
> The lobby smells the same as when you left it.

---

## ENDING

> A door opens behind the table. Footsteps. Then a young man in a good suit
> steps into the lamplight wearing your eyes and your jaw and twenty-five
> years of somebody else's raising.
>
> "Hey, Pops."
>
> He picks a chip off the felt and turns it over in his fingers. "I run this
> place now. Took me a long while to work out how you could put me on the
> table like that, and then it hit me: you didn't think you could lose."
>
> His hand goes under his jacket.
>
> "After twenty-five years, I thought you'd have learned by now."
>
> "The House always wins, Pops."
>
> **BANG.**

## GAME_OVER

The same two words as the marquee, which is the joke: the game is named after
the way you lose it. `GAME_OVER` is `TITLE` in code.

> **ALL IN**
>
> *Press any key.*
