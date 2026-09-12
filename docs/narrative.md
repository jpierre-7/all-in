# All In: Narrative

Every string the overworld renders, in the order the player meets it. Each
heading is the constant name Dev 2 lifts into `src/overworld/narrative.rs`.
Keep lines short; this is Bevy text on a dark screen, not a novel.

Tone: casino-noir, pulpy, a little mean. Second person, present tense.
Lucky Jack is "you" and the House is always capitalised.

---

## OPENING

> Twenty-five years ago you sat down at the Big Shots Table with everything
> you owned, and it took the House about an hour to take it off you.
>
> That should've been the end of it. Instead you did the thing this whole
> building still whispers about when the shift changes: you put your boy on
> the felt, your firstborn, because you were sure the next hand was yours.
>
> It wasn't.
>
> They still call you Lucky Jack. Nobody remembers why, and you've stopped
> correcting them.
>
> Tonight you walk back through the doors with a deck in your coat pocket
> and nothing else worth taking. Twenty-five years is a long time to owe
> someone, and you've come to collect.

*(~120 words)*

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

> An arcade cabinet, sticky with spilled rum, with a cracked screen that
> explains the only game in this building that matters.
>
> Draw 7 and play up to 5; the cards you play stack into your **Hand**.
> Clear the **House Edge** and whatever's left over comes out of *their*
> Stack. Fall short, and the difference comes out of yours.
>
> **Streak** cards double if the card before them had a Tell. **All In**
> cards burn a card from your Draw and add its chips to the pile.
>
> Every couple of turns the **Blinds rise**, because nobody gets to sit at
> this table forever.
>
> *Press any key to go back.*

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

> **ALL IN**
>
> *Press any key.*
