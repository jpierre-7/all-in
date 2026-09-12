//! Type and window identity: the fonts the whole game draws with.
//!
//! `bevy_text` installs FiraMono into `Assets<Font>` at the default asset id,
//! and every `TextFont` that names no font resolves there. Replacing that one
//! entry re-types the entire game — combat and overworld alike — without
//! touching a single call site, so body text is handled here rather than
//! threaded through `Screen` and the combat UI.
//!
//! The display face is opt-in: tag a text entity with [`DisplayText`] and it is
//! swapped over on the next frame.

use bevy::asset::AssetId;
use bevy::prelude::*;

/// Barlow Condensed: the body face, and the new default for all text.
const BODY: &[u8] = include_bytes!("../assets/fonts/BarlowCondensed-Regular.ttf");
/// Limelight: an art-deco display face, for titles and marquee lines.
const DISPLAY: &[u8] = include_bytes!("../assets/fonts/Limelight-Regular.ttf");

/// Fonts are embedded rather than loaded from `assets/`, so they are never
/// missing and never race the first frame.
#[derive(Resource)]
pub struct Fonts {
    pub display: Handle<Font>,
}

/// Marks text that should be drawn in the display face.
#[derive(Component)]
pub struct DisplayText;

pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        let display = {
            let mut fonts = app.world_mut().resource_mut::<Assets<Font>>();
            // Overwrites the FiraMono that `TextPlugin` put here.
            fonts
                .insert(AssetId::default(), Font::from_bytes(BODY.to_vec()))
                .expect("the default font id is always writable");
            fonts.add(Font::from_bytes(DISPLAY.to_vec()))
        };
        app.insert_resource(Fonts { display })
            .add_systems(Update, apply_display_font);
    }
}

/// Swaps tagged text over to the display face, then drops the tag so each
/// entity is only touched once.
fn apply_display_font(
    mut commands: Commands,
    fonts: Res<Fonts>,
    mut tagged: Query<(Entity, &mut TextFont), With<DisplayText>>,
) {
    for (entity, mut font) in &mut tagged {
        font.font = fonts.display.clone().into();
        commands.entity(entity).remove::<DisplayText>();
    }
}
