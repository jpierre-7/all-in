//! One screen shape for the whole overworld: a title, a block of prose, and a
//! footer telling the player which keys do something. Everything spawned here
//! is scoped to the state that spawned it, so leaving the state clears it.

use std::path::Path;

use bevy::prelude::*;

use crate::overworld::narrative;
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
    /// One frame per paragraph of the Opening, in paging order. A `None` here
    /// pages as text only — the frames are the heaviest files in the repo and
    /// the Opening has to read without them.
    pub opening: [Option<Handle<Image>>; narrative::OPENING.len()],
}

impl OverworldArt {
    /// The frame behind Opening frame `index`, if the artist drew one.
    pub fn opening_frame(&self, index: usize) -> Option<&Handle<Image>> {
        self.opening.get(index).and_then(Option::as_ref)
    }
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
        // Spelled out rather than built from the index, so the five slots are
        // greppable against the five files in `assets/backstory/`.
        opening: [
            on_disk("backstory/opening_1.png"),
            on_disk("backstory/opening_2.png"),
            on_disk("backstory/opening_3.png"),
            on_disk("backstory/opening_4.png"),
            on_disk("backstory/opening_5.png"),
        ],
    });
}

/// Which backdrop a screen hangs behind itself.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Backdrop {
    #[default]
    Lobby,
    /// The title's own art, or the lobby's if the artist has not drawn one.
    Title,
    /// One Opening frame, by its index in `narrative::OPENING`. Unlike the
    /// other two this has no stand-in: a missing frame is bare felt, because
    /// the lobby carpet behind the backstory would be a lie.
    Opening(usize),
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
            Backdrop::Opening(index) => art.opening_frame(index),
        };
        let mut screen = commands.entity(entity);
        if let Some(image) = image {
            screen.insert(ImageNode::new(image.clone()));
        }
        screen.remove::<WantsBackdrop>();
    }
}

/// Ordinary screen titles.
const TITLE_SIZE: f32 = 38.0;
/// The marquee: the game's own name, and the only text on its screen.
const MARQUEE_SIZE: f32 = 128.0;

/// Where a screen's content sits against its backdrop.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Anchor {
    /// Down the middle, for screens whose backdrop is only atmosphere.
    #[default]
    Center,
    /// Against the bottom edge. The backstory frames are composed with their
    /// lower third dark and their subject in the upper two thirds, so prose
    /// down the middle would sit across the thing being drawn (#48).
    LowerThird,
}

/// A screen under construction. Build it up, then `spawn` it.
pub struct Screen {
    title: Option<(&'static str, f32)>,
    blocks: Vec<(String, Color)>,
    footer: Option<String>,
    backdrop: Backdrop,
    anchor: Anchor,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            title: None,
            blocks: Vec::new(),
            footer: None,
            backdrop: Backdrop::default(),
            anchor: Anchor::default(),
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

    /// Sit the content somewhere other than the middle of the screen.
    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
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

    /// Put the screen on the felt, scoped to `state`. The root comes back so
    /// a caller can tag it with whatever it needs to find it again.
    pub fn spawn(self, commands: &mut Commands, state: AppState) -> Entity {
        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    flex_direction: FlexDirection::Column,
                    justify_content: match self.anchor {
                        Anchor::Center => JustifyContent::Center,
                        Anchor::LowerThird => JustifyContent::FlexEnd,
                    },
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
            })
            .id()
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

#[cfg(test)]
mod tests {
    use bevy::asset::uuid_handle;
    use bevy::prelude::*;

    use super::{OverworldArt, narrative};

    const FIRST: Handle<Image> = uuid_handle!("6c1b8f7a-1d2e-4b3c-8a90-0f1e2d3c4b51");
    const LAST: Handle<Image> = uuid_handle!("6c1b8f7a-1d2e-4b3c-8a90-0f1e2d3c4b52");

    /// The first and last frames drawn, the three between them missing: enough
    /// to tell "frame N" apart from "any frame at all".
    fn part_hung() -> OverworldArt {
        let mut art = OverworldArt::default();
        art.opening[0] = Some(FIRST);
        art.opening[narrative::OPENING.len() - 1] = Some(LAST);
        art
    }

    #[test]
    fn every_frame_of_the_opening_has_an_art_slot() {
        assert_eq!(
            OverworldArt::default().opening.len(),
            narrative::OPENING.len()
        );
    }

    #[test]
    fn each_frame_hangs_its_own_image() {
        let art = part_hung();

        assert_eq!(art.opening_frame(0), Some(&FIRST));
        assert_eq!(art.opening_frame(narrative::OPENING.len() - 1), Some(&LAST));
    }

    #[test]
    fn a_frame_with_no_file_pages_as_text_only() {
        let art = part_hung();

        assert_eq!(art.opening_frame(1), None, "an undrawn frame is bare felt");
        assert_eq!(art.opening_frame(2), None);
    }

    /// The index comes from the paging code, so an off-by-one should be no art
    /// rather than a panic on the most cinematic screen in the game.
    #[test]
    fn an_index_past_the_last_frame_is_no_art_rather_than_a_crash() {
        assert_eq!(part_hung().opening_frame(narrative::OPENING.len()), None);
    }
}
