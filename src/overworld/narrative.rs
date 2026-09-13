//! Every string the overworld renders, lifted from `docs/narrative.md`.
//! Prose only — which screen shows what is `mod.rs`'s business.

/// The marquee.
pub const TITLE: &str = "ALL IN";

/// The Opening, one frame per screen, lifted from the `## OPENING` section of
/// `docs/narrative.md` (split there by #37). The art slots are
/// `assets/backstory/opening_N.png`, numbered to match.
pub const OPENING_1: &str = "\
Twenty-five years ago you sat down at the Big Shots Table with everything
you owned, and it took the House about an hour to take it off you.";

pub const OPENING_2: &str = "\
That should've been the end of it. Instead you did the thing this whole
building still whispers about when the shift changes: you put your boy on
the felt.";

pub const OPENING_3: &str = "\
Your firstborn, because you were sure the next hand was yours.

It wasn't. The House took him the way it takes everything: without
looking up.";

pub const OPENING_4: &str = "\
That was twenty-five years ago. They still call you Lucky Jack. Nobody
remembers why, and you've stopped correcting them.";

pub const OPENING_5: &str = "\
Tonight you walk back through the doors with a deck in your coat pocket
and nothing else worth taking. Twenty-five years is a long time to owe
someone, and you've come to collect.";

/// The frames in the order they are paged through. Everything that walks the
/// Opening — the screens, the art slots, the tests — counts from this.
pub const OPENING: [&str; 5] = [OPENING_1, OPENING_2, OPENING_3, OPENING_4, OPENING_5];

pub const LOBBY: &str = "\
The lobby smells like carpet shampoo and old cigarettes; a slot machine
near the coat check chirps at nobody in particular. Somewhere above you
the Big Shots Table is set for one.";

pub const LOBBY_OPT_INFO: &str = "Case the joint";
pub const LOBBY_OPT_TUTORIAL: &str = "Warm up on the arcade";
pub const LOBBY_OPT_BEGIN: &str = "Walk the Floor";

pub const INFO_ROOM: &str = "\
A cork board behind the coat check where somebody has pinned up the odds
in pencil.

The Floor. Small fish. Slotz runs the tables down here, loud and cheap
and beatable if you don't get greedy.

The Pit. Where the regulars lose their houses. The Pit Boss weighs
every chip that crosses the rope, and it's never once been wrong.

The Big Shots Table. The House sits here, and the House already knows
what's in your Hand.";

pub const TUTORIAL_INTRO: &str = "\
An arcade cabinet, sticky with spilled rum. The screen flickers on and
deals you a hand. Somebody's scratched into the bezel: play what it
tells you, then play what you like.";

/// The gated first turn, one prompt per keypress (#40).
pub const TUTORIAL_STEPS: [&str; 8] = [
    "Every card is worth its Stack. Press 1 to play Pawned Ring for 8.",
    "Last Dollar is All In: it burns another card and takes its chips. Press 1 to play it.",
    "Now press 2 to burn Two of Clubs. Its 2 chips join Last Dollar's.",
    "Hot Streak is a Streak: it doubles if the card before it had a Tell. It did. Press 1.",
    "Dealer Blinks is a Streak too, and the card before it had a Tell. Press 1 for 10.",
    "Last Play. Press 1 to add Four of Hearts. That's 32 against a House Edge of 20.",
    "Press Enter to show your Hand.",
    "You cleared the Edge by 12: your Payout. Or Push Your Luck: press P and a coin flip doubles it, or zeroes your Hand. This coin is rigged your way. The real one is 45/55.",
];

pub const TUTORIAL_HINT: &str = "\
Free play. Lead with a Tell so your Streaks double, and burn your smallest
card to an All In. In a real duel the Blinds rise every few turns, so
don't sit here all night.";

pub const TUTORIAL_DONE: &str = "\
The cabinet spits out a paper ticket that says WINNER and nothing else.
Upstairs, nobody rigs the coin.";

pub const TUTORIAL_LEFT: &str = "\
You walk away from the cabinet mid-hand. It doesn't seem to mind.";

pub const FLOOR_INTRO: &str = "\
The main floor, where rows of slots blink like a migraine and the cocktail
waitresses stopped smiling sometime in the nineties. This is where the
House lets you win a little, so you'll stay.

You aren't here to stay.";

pub const PIT_INTRO: &str = "\
Down a short flight of stairs the noise dies off. The Pit is quiet money:
velvet rope, real clay chips, dealers who don't blink. Everybody at these
tables has lost something that wasn't cash, and most of them are still
trying to win it back.

You'd know.";

pub const BIG_SHOTS_INTRO: &str = "\
One table under one lamp, and the felt is the exact green you've seen on
the inside of your eyelids for twenty-five years.

The House pulls out your chair. It remembers you.";

pub const ENC_FLOOR_MINION: &str = "\
A shill in a rented tux slides into the seat across from you. \"House rules,
pal. You play or you leave, and you don't look like the leaving type.\"";

pub const ENC_SLOTZ: &str = "\
SLOTZ. Three feet of chrome and neon on a rolling base, arms spinning,
grinning the way only a machine can. It doesn't want your money; it wants
your time, and it's got all night.";

pub const ENC_PIT_MINION: &str = "\
A dealer with a scar under one eye shuffles without looking down. \"The
Boss weighed you when you walked in. Came up light. So you don't go
upstairs.\"";

pub const ENC_PIT_BOSS: &str = "\
THE PIT BOSS. A brass balance scale the height of a man, two pans
hanging off a beam that creaks when it turns to look at you. One pan is
already piled with chips; the other holds nothing yet. \"Jack. Twenty-five
years, and you came back with that Stack? Put it on the pan. Let's see
what it's worth.\"";

pub const ENC_THE_HOUSE: &str = "\
THE HOUSE. No face, just a pair of hands resting on the felt and a
voice that comes from the walls. \"Sit down, Jack. Let's see what you
learned. Play four. I'll set the line. Then show me your last card.\"";

pub const FIGHT_OR_FOLD: &str = "\
Fight: sit down and play.
Fold: walk back to the lobby, and leave behind everything you picked
up tonight.";

pub const WIN_MINION: &str =
    "The chair scrapes back empty and you pocket whatever they left on the felt.";

pub const WIN_SLOTZ: &str = "\
The reels spin once more, land on nothing, and the neon goes out. Somewhere
off in the dark, something bigger clears its throat.";

pub const WIN_PIT_BOSS: &str = "\
The beam tips your way and stays there, and for a long moment the only
sound in the Pit is brass settling. \"Upstairs,\" it says at last. \"He's
been expecting you.\"";

pub const WIN_THE_HOUSE: &str = "\
The House's hands go still on the felt.

For the first time in twenty-five years, the table is yours.";

pub const LOSE: &str = "\
Your Stack hits the felt and stops, and the dealer doesn't bother looking
up.

\"House always wins, pal.\"";

pub const FOLD: &str = "\
You push back from the table and nobody stops you. Nobody ever does.

The lobby smells the same as when you left it.";

pub const ENDING: &str = "\
A door opens behind the table. Footsteps. Then a young man in a good suit
steps into the lamplight wearing your eyes and your jaw and twenty-five
years of somebody else's raising.

\"Hey, Pops.\"

He picks a chip off the felt and turns it over in his fingers. \"I run this
place now. Took me a long while to work out how you could put me on the
table like that, and then it hit me: you didn't think you could lose.\"

His hand goes under his jacket.

\"After twenty-five years, I thought you'd have learned by now.\"

\"The House always wins, Pops.\"

BANG.";

pub const ITEM_DROP: &str = "\
Nobody's coming back for what's left on the felt, so it goes in your coat
pocket on the way past.";

pub const PERK_PICK: &str = "\
Something about how you play changes from here. You only get to change one
thing, and you don't get to change it back.";

/// The same two words as the marquee, which is the joke: the game is named
/// after the way you lose it.
pub const GAME_OVER: &str = TITLE;

pub const PRESS_SPACE: &str = "Press Space to play.";
pub const ANY_KEY: &str = "Press any key.";
pub const ANY_KEY_BACK: &str = "Press any key to go back.";
/// The Opening names Esc, because a skip nobody can find is no skip.
pub const ANY_KEY_OR_SKIP: &str = "Press any key — or Esc to skip ahead.";
