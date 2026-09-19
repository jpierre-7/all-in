//! The Info overlay (#43): the game's terms on one screen, over the table or
//! over the Fight or Fold prompt. `I` toggles it, Esc closes it, and while it
//! is up nothing underneath sees a key.

use bevy::prelude::*;

use crate::state::AppState;

const INK: Color = Color::srgb(0.90, 0.87, 0.80);
const GOLD: Color = Color::srgb(0.85, 0.70, 0.35);
const DIM: Color = Color::srgb(0.55, 0.53, 0.48);
const NEON: Color = Color::srgb(0.85, 0.20, 0.30);
/// Near-black felt at 94%, so the table shows through as a hint of where you are.
const SHADE: Color = Color::srgba(0.03, 0.05, 0.04, 0.94);

/// Present while the overlay is up. Input systems under it check for this
/// and stand down.
#[derive(Resource)]
pub struct InfoOpen;

#[derive(Component)]
pub struct InfoOverlay;

/// One line per term, in the order a new player meets them. Text follows
/// `CONTEXT.md`. The Peek's nested tags read from the same table.
const GLOSSARY: &[(&str, &str)] = &[
    (
        "Stack",
        "Chips. Yours and the enemy's are your lives; a card's is what it adds to The Hand.",
    ),
    (
        "Draw",
        "The seven cards you're holding. Refilled at the start of every turn.",
    ),
    (
        "Plays",
        "How many cards you may play this turn. Five, unless a perk says otherwise.",
    ),
    (
        "The Hand",
        "The running total of the cards you've played this turn.",
    ),
    ("House Edge", "The number The Hand has to reach this turn."),
    (
        "Payout",
        "Clear the Edge and the excess comes off the enemy's Stack.",
    ),
    ("Whiff", "Fall short and the shortfall comes off yours."),
    ("Tell", "A keyword on a card. One per card, at most."),
    (
        "Streak",
        "Tell: doubles this card if the card before it had any Tell.",
    ),
    (
        "All In",
        "Tell: burns another card from your Draw and adds its chips.",
    ),
    (
        "Copycat",
        "Tell: worth the next card's printed Stack, not its Tell. Its own if no card follows.",
    ),
    (
        "Push Your Luck",
        "Once The Hand is shown: Push to flip a coin, or Hold to take the turn as it is. Cleared the Edge? Win doubles the Payout; lose and The Hand is 0. Fell short? Win forgives the Whiff; lose and it doubles.",
    ),
    (
        "Rising Blinds",
        "Every few turns the House Edge climbs. Nobody sits here forever.",
    ),
    ("Loaded Dice", "Item: +5 to your next two Hands."),
];

const BIG_SHOTS: &[(&str, &str)] = &[
    (
        "Margin",
        "How far above your Hand The House sets its Edge. Rises with the Blinds.",
    ),
    (
        "Hole Card",
        "Your last Play. The House locks its Edge on everything before it, so the Hole Card is the only card it can't see.",
    ),
];

/// The glossary line for a term, if it has one.
pub fn define(term: &str) -> Option<&'static str> {
    GLOSSARY
        .iter()
        .chain(BIG_SHOTS)
        .find(|(name, _)| *name == term)
        .map(|(_, line)| *line)
}

pub struct InfoPlugin;

impl Plugin for InfoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            toggle.run_if(in_state(AppState::Combat).or_else(in_state(AppState::FightOrFold))),
        )
        .add_systems(OnExit(AppState::Combat), close)
        .add_systems(OnExit(AppState::FightOrFold), close);
    }
}

/// The key that opens and closes the overlay.
pub fn toggled(keys: &ButtonInput<KeyCode>) -> bool {
    keys.just_pressed(KeyCode::KeyI)
}

fn toggle(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    open: Option<Res<InfoOpen>>,
    state: Res<State<AppState>>,
    overlays: Query<Entity, With<InfoOverlay>>,
) {
    let closing = open.is_some() && (toggled(&keys) || keys.just_pressed(KeyCode::Escape));
    let opening = open.is_none() && toggled(&keys);
    if closing {
        for entity in &overlays {
            commands.entity(entity).despawn();
        }
        commands.remove_resource::<InfoOpen>();
    } else if opening {
        commands.insert_resource(InfoOpen);
        spawn(&mut commands, *state.get());
    }
}

/// Leaving the state takes the overlay with it (the node is scoped), so only
/// the flag needs clearing.
fn close(mut commands: Commands) {
    commands.remove_resource::<InfoOpen>();
}

fn spawn(commands: &mut Commands, state: AppState) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: px(6),
                padding: UiRect::all(px(48)),
                ..default()
            },
            BackgroundColor(SHADE),
            GlobalZIndex(10),
            InfoOverlay,
            DespawnOnExit(state),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("How the table works"),
                TextFont::from_font_size(30.0),
                TextColor(NEON),
            ));
            for (term, line) in GLOSSARY {
                entry(root, term, line);
            }
            root.spawn((
                Text::new("At the Big Shots Table"),
                TextFont::from_font_size(18.0),
                TextColor(GOLD),
                Node {
                    margin: UiRect::top(px(10)),
                    ..default()
                },
            ));
            for (term, line) in BIG_SHOTS {
                entry(root, term, line);
            }
            root.spawn((
                Text::new("I or Esc closes this."),
                TextFont::from_font_size(14.0),
                TextColor(DIM),
                Node {
                    margin: UiRect::top(px(14)),
                    ..default()
                },
            ));
        });
}

fn entry(parent: &mut ChildSpawnerCommands, term: &str, line: &str) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: px(14),
            width: px(860),
            max_width: percent(100),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(term),
                TextFont::from_font_size(16.0),
                TextColor(GOLD),
                Node {
                    width: px(140),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            row.spawn((
                Text::new(line),
                TextFont::from_font_size(16.0),
                TextColor(INK),
            ));
        });
}
