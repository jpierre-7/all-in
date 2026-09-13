//! One screen shape for the whole overworld: a title, a block of prose, and a
//! footer telling the player which keys do something. Everything spawned here
//! is scoped to the state that spawned it, so leaving the state clears it.

use std::path::Path;

use bevy::prelude::*;

use crate::state::AppState;
use crate::theme::DisplayText;

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
    /// The title screen's own art. Nothing is on disk for it yet, so the
    /// marquee falls back to the lobby backdrop.
    pub title: Option<Handle<Image>>,
}

pub fn load_overworld_art(mut commands: Commands, assets: Option<Res<AssetServer>>) {
    let on_disk = |path: &'static str| {
        assets
            .as_ref()
            .filter(|_| Path::new("assets").join(path).exists())
            .map(|assets| assets.load(path))
    };
    commands.insert_resource(OverworldArt {
        lobby: on_disk("backdrops/lobby.png"),
        title: on_disk("backdrops/title.png"),
    });
}

/// Which backdrop a screen hangs behind itself.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Backdrop {
    #[default]
    Lobby,
    /// The title's own art, or the lobby's if the artist has not drawn one.
    Title,
}

/// A screen root that has not been given its backdrop yet.
#[derive(Component)]
pub struct WantsBackdrop(pub Backdrop);

/// Hangs the backdrop on every screen root that asked for one.
///
/// `OnEnter` runs in the state-transition schedule, ahead of `Update` in the
/// same frame, so the image lands before the screen is ever drawn and there is
/// no flash of bare felt.
pub fn apply_backdrop(
    mut commands: Commands,
    art: Res<OverworldArt>,
    screens: Query<(Entity, &WantsBackdrop)>,
) {
    for (entity, wants) in &screens {
        let image = match wants.0 {
            Backdrop::Lobby => art.lobby.as_ref(),
            Backdrop::Title => art.title.as_ref().or(art.lobby.as_ref()),
        };
        let mut screen = commands.entity(entity);
        if let Some(image) = image {
            // Its own layer behind the text, filling the root regardless of
            // padding; an ImageNode on the root would be drawn inside the
            // padding and shrink the art instead of insetting the words.
            screen.with_children(|root| {
                root.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px(0),
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                    ImageNode::new(image.clone()).with_mode(NodeImageMode::Stretch),
                    ZIndex(-1),
                ));
            });
        }
        screen.remove::<WantsBackdrop>();
    }
}

/// Ordinary screen titles.
const TITLE_SIZE: f32 = 38.0;
/// The marquee: the game's own name, and the only text on its screen.
const MARQUEE_SIZE: f32 = 128.0;

/// A screen under construction. Build it up, then `spawn` it.
pub struct Screen {
    title: Option<(&'static str, f32)>,
    blocks: Vec<(String, Color)>,
    footer: Option<String>,
    backdrop: Backdrop,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            title: None,
            blocks: Vec::new(),
            footer: None,
            backdrop: Backdrop::default(),
        }
    }

    pub fn title(mut self, title: &'static str) -> Self {
        self.title = Some((title, TITLE_SIZE));
        self
    }

    /// A title at marquee size, for the one screen that is nothing else.
    pub fn marquee(mut self, title: &'static str) -> Self {
        self.title = Some((title, MARQUEE_SIZE));
        self
    }

    /// Hang something other than the lobby behind this screen.
    pub fn backdrop(mut self, backdrop: Backdrop) -> Self {
        self.backdrop = backdrop;
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
                WantsBackdrop(self.backdrop),
                DespawnOnExit(state),
            ))
            .with_children(|root| {
                if let Some((title, size)) = self.title {
                    root.spawn((
                        Text::new(title),
                        TextFont::from_font_size(size),
                        TextColor(NEON),
                        DisplayText,
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

/// Space, and nothing else: the one key the Title screen answers to.
pub fn space(keys: &ButtonInput<KeyCode>) -> bool {
    keys.just_pressed(KeyCode::Space)
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
