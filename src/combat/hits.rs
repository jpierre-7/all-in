//! Damage feedback (#67): when a turn resolves, a "-N" floats up from the
//! Chips that took the hit and fades. A Bluff's hit floats up as the rows
//! turn over, before the Payout or the Whiff. Markers are their own root entities,
//! not part of the table's `CombatScreen`, so the per-change redraw leaves
//! them alone and they ride out their own clock.

use bevy::prelude::*;

use super::duel::Outcome;
use super::plugin::ActiveDuel;
pub use crate::modifier::Side;
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
            (spawn_markers, spawn_bluff_markers, animate_markers)
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
    spawn_marker(&mut commands, side, format!("-{amount}"));
}

/// Watches for rows that have just turned over and floats whatever their
/// Bluffs took, one marker per side. Keyed on the showdown's turn: the rows
/// are read while the Push Your Luck prompt is up, or, when the turn
/// resolved on the spot, from what it left on the table.
fn spawn_bluff_markers(
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
    let Some(showdown) = active.duel.showdown().or(active.duel.last_showdown()) else {
        return;
    };
    if *last_seen == Some(showdown.turn) {
        return;
    }
    *last_seen = Some(showdown.turn);

    for side in [Side::Enemy, Side::Player] {
        let amount: u32 = showdown
            .bluffs
            .iter()
            .filter(|hit| hit.loser == side)
            .map(|hit| hit.amount)
            .sum();
        if amount > 0 {
            spawn_marker(&mut commands, side, format!("Bluff -{amount}"));
        }
    }
}

fn spawn_marker(commands: &mut Commands, side: Side, label: String) {
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
                Text::new(label),
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
