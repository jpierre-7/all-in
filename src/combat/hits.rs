//! Damage feedback (#67): when a turn resolves, a "-N" floats up from the
//! Chips that took the hit and fades. Markers are their own root entities,
//! not part of the table's `CombatScreen`, so the per-change redraw leaves
//! them alone and they ride out their own clock.

use bevy::prelude::*;

use super::duel::Outcome;
use super::plugin::ActiveDuel;
use crate::state::AppState;

/// Hotter than the table's neon so it reads at a glance.
const HIT: Color = Color::srgb(1.0, 0.30, 0.38);
/// How long a marker lives, in seconds.
const LIFETIME: f32 = 0.9;
/// How far it rises over its life, in px.
const RISE: f32 = 48.0;
/// Where the two Chips readouts sit, as a fraction of the window height.
/// Matches the table's rows: enemy at the top, you at the bottom.
const ENEMY_ROW: f32 = 0.11;
const PLAYER_ROW: f32 = 0.86;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Enemy,
    Player,
}

#[derive(Component, Debug)]
pub struct HitMarker {
    pub side: Side,
    /// Seconds since it was spawned. Public so a test can run the clock out.
    pub age: f32,
}

pub struct HitsPlugin;

impl Plugin for HitsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (spawn_markers, animate_markers)
                .chain()
                .run_if(in_state(AppState::Combat)),
        );
    }
}

/// Watches the duel for a newly resolved turn and floats its number. Keyed on
/// the turn counter, so a redraw for any other reason spawns nothing.
fn spawn_markers(
    mut commands: Commands,
    active: Option<Res<ActiveDuel>>,
    mut last_seen: Local<Option<u32>>,
) {
    let Some(active) = active else {
        *last_seen = None;
        return;
    };
    if !active.is_changed() {
        return;
    }
    let Some(turn) = active.last_turn else {
        return;
    };
    // `turn` on the duel has already advanced past the one that resolved.
    let resolved = active.duel.turn().saturating_sub(1);
    if *last_seen == Some(resolved) {
        return;
    }
    *last_seen = Some(resolved);

    let (side, amount) = match turn.kind {
        Outcome::Payout(n) => (Side::Enemy, n),
        Outcome::Whiff(n) => (Side::Player, n),
    };
    if amount == 0 {
        return;
    }
    let row = match side {
        Side::Enemy => ENEMY_ROW,
        Side::Player => PLAYER_ROW,
    };
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: percent(row * 100.0),
                width: percent(100),
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(5),
            HitMarker { side, age: 0.0 },
            DespawnOnExit(AppState::Combat),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(format!("-{amount}")),
                TextFont::from_font_size(56.0),
                TextColor(HIT),
                Node {
                    // Sits just beside the Chips readout, not on top of it.
                    margin: UiRect::left(px(160)),
                    ..default()
                },
            ));
        });
}

/// Rises, fades, and despawns at the end of its life.
fn animate_markers(
    mut commands: Commands,
    time: Res<Time>,
    mut markers: Query<(Entity, &mut HitMarker, &mut Node, &Children)>,
    mut colors: Query<&mut TextColor>,
) {
    for (entity, mut marker, mut node, children) in &mut markers {
        marker.age += time.delta_secs();
        let t = (marker.age / LIFETIME).clamp(0.0, 1.0);
        if t >= 1.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let row = match marker.side {
            Side::Enemy => ENEMY_ROW,
            Side::Player => PLAYER_ROW,
        };
        node.top = Val::Px(0.0);
        node.top = percent(row * 100.0);
        node.margin = UiRect::top(px(-RISE * t));
        for child in children.iter() {
            if let Ok(mut color) = colors.get_mut(child) {
                color.0 = HIT.with_alpha(1.0 - t * t);
            }
        }
    }
}
