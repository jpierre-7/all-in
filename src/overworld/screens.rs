//! One screen shape for the whole overworld: a title, a block of prose, and a
//! footer telling the player which keys do something. Everything spawned here
//! is scoped to the state that spawned it, so leaving the state clears it.

use std::path::Path;

use bevy::prelude::*;

use crate::state::AppState;

const INK: Color = Color::srgb(0.90, 0.87, 0.80);
const FELT: Color = Color::srgb(0.05, 0.07, 0.06);
const NEON: Color = Color::srgb(0.85, 0.20, 0.30);
const DIM: Color = Color::srgb(0.55, 0.53, 0.48);

/// Backdrop art for the prose screens, when the artist's file is on disk.
///
/// Mirrors `combat::ui::Art`: the file is checked once at startup so a missing
/// one is a fallback to bare felt, not a load error mid-run.
#[derive(Resource, Default)]
pub struct OverworldArt {
    pub lobby: Option<Handle<Image>>,
}

pub fn load_overworld_art(mut commands: Commands, assets: Option<Res<AssetServer>>) {
    const LOBBY: &str = "backdrops/lobby.png";
    let lobby = assets
        .as_ref()
        .filter(|_| Path::new("assets").join(LOBBY).exists())
        .map(|assets| assets.load(LOBBY));
    commands.insert_resource(OverworldArt { lobby });
}

/// A screen root that has not been given its backdrop yet.
#[derive(Component)]
pub struct WantsBackdrop;

/// Hangs the backdrop on every screen root that asked for one.
///
/// `OnEnter` runs in the state-transition schedule, ahead of `Update` in the
/// same frame, so the image lands before the screen is ever drawn and there is
/// no flash of bare felt.
pub fn apply_backdrop(
    mut commands: Commands,
    art: Res<OverworldArt>,
    screens: Query<Entity, With<WantsBackdrop>>,
) {
    for entity in &screens {
        let mut screen = commands.entity(entity);
        if let Some(lobby) = &art.lobby {
            screen.insert(ImageNode::new(lobby.clone()));
        }
        screen.remove::<WantsBackdrop>();
    }
}

/// A screen under construction. Build it up, then `spawn` it.
pub struct Screen {
    title: Option<&'static str>,
    blocks: Vec<(String, Color)>,
    footer: Option<String>,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            title: None,
            blocks: Vec::new(),
            footer: None,
        }
    }

    pub fn title(mut self, title: &'static str) -> Self {
        self.title = Some(title);
        self
    }

    /// A paragraph of prose.
    pub fn prose(mut self, body: impl Into<String>) -> Self {
        self.blocks.push((body.into(), INK));
        self
    }

    /// A numbered menu option. The number is the key that picks it.
    pub fn option(mut self, number: u8, label: &str) -> Self {
        self.blocks.push((format!("[{number}]  {label}"), NEON));
        self
    }

    /// The key hint along the bottom.
    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    /// Put the screen on the felt, scoped to `state`.
    pub fn spawn(self, commands: &mut Commands, state: AppState) {
        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    row_gap: px(16),
                    padding: UiRect::all(px(64)),
                    ..default()
                },
                BackgroundColor(FELT),
                WantsBackdrop,
                DespawnOnExit(state),
            ))
            .with_children(|root| {
                if let Some(title) = self.title {
                    root.spawn((
                        Text::new(title),
                        TextFont::from_font_size(38.0),
                        TextColor(NEON),
                    ));
                }

                for (body, color) in self.blocks {
                    root.spawn((
                        Text::new(body),
                        TextFont::from_font_size(20.0),
                        TextColor(color),
                        TextLayout::justify(Justify::Left),
                    ));
                }

                if let Some(footer) = self.footer {
                    root.spawn((
                        Text::new(footer),
                        TextFont::from_font_size(16.0),
                        TextColor(DIM),
                    ));
                }
            });
    }
}

impl Default for Screen {
    fn default() -> Self {
        Self::new()
    }
}

/// Any key at all, for prose screens that just need dismissing.
pub fn any_key(keys: &ButtonInput<KeyCode>) -> bool {
    keys.get_just_pressed().next().is_some()
}

/// Enter, for taking the option a menu leads with.
pub fn confirm(keys: &ButtonInput<KeyCode>) -> bool {
    keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter)
}

/// The number key just pressed, top row or numpad, for menus.
pub fn digit_pressed(keys: &ButtonInput<KeyCode>) -> Option<u8> {
    keys.get_just_pressed().find_map(|key| match key {
        KeyCode::Digit1 | KeyCode::Numpad1 => Some(1),
        KeyCode::Digit2 | KeyCode::Numpad2 => Some(2),
        KeyCode::Digit3 | KeyCode::Numpad3 => Some(3),
        _ => None,
    })
}
