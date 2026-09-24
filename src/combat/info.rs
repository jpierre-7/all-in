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
        "Chips",
        "Your life, and the enemy's. Run out and the duel is over.",
    ),
    (
        "Face Value",
        "The number printed on a card: what it adds to The Hand before its Tell.",
    ),
    (
        "Draw",
        "The seven cards you're holding. Refilled at the start of every turn.",
    ),
    (
        "Plays",
        "How many cards you may put in your row this turn. Five, unless a perk says otherwise, and never more than the enemy laid down.",
    ),
    (
        "Opposing Cards",
        "The enemy's row, laid down before you play. The first is face up; the rest are a coin toss. Your cards sit one per slot across from them.",
    ),
    (
        "The Hand",
        "Your row, and what it adds up to once every Tell in it has resolved. The bigger of it and the House Edge wins.",
    ),
    (
        "House Edge",
        "What the Opposing Cards add up to. The number The Hand has to beat, and you only ever see part of it before you confirm.",
    ),
    (
        "Payout",
        "Beat the Edge and the difference comes off the enemy's Chips.",
    ),
    ("Whiff", "Fall short and the difference comes off yours."),
    ("Tell", "A keyword on a card. One per card, at most."),
    (
        "Streak",
        "Tell: doubles this card if the card in the slot to its left has any Tell.",
    ),
    (
        "All In",
        "Tell: burns another card from your Draw and adds its Face Value.",
    ),
    (
        "Copycat",
        "Tell: worth the Face Value of the card in the slot to its right. Its own if nothing follows it.",
    ),
    (
        "Flop",
        "Tell: worth the Face Value of the Opposing Card across from it. Its own if nothing is across.",
    ),
    (
        "Push Your Luck",
        "Once The Hand is shown: Push to flip a coin, or Hold to take the turn as it is. Cleared the Edge? Win doubles the Payout; lose and The Hand is 0. Fell short? Win forgives the Whiff; lose and it doubles.",
    ),
    (
        "Rising Blinds",
        "Every few turns the enemy lays another Opposing Card down. Past your Plays you can't cover them all. Nobody sits here forever.",
    ),
    ("Loaded Dice", "Item: +5 to your next two Hands."),
];

pub fn get_keywords() -> Vec<&'static str> {
    GLOSSARY.iter().map(|unit| unit.0).collect()
}

const BIG_SHOTS: &[(&str, &str)] = &[
    (
        "Margin",
        "How far above the row it read The House sets its own. Rises with the Blinds.",
    ),
    (
        "Hole Card",
        "The last card in your row. The House keeps its own last card back and fills it in on everything before yours, so the Hole Card is the only one it can't see.",
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
