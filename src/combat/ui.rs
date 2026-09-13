//! Draws the duel. Text-first, with an image slot per card and Tell that the
//! artist's files fill in when they exist under `assets/`.

use std::path::Path;

use bevy::prelude::*;

use super::duel::{Coin, Outcome, Phase, Push, TurnResult};
use super::plugin::ActiveDuel;
use crate::run::{EncounterId, LOADED_DICE_BONUS, Tell};
use crate::state::AppState;

const INK: Color = Color::srgb(0.90, 0.87, 0.80);
const FELT: Color = Color::srgb(0.05, 0.07, 0.06);
const NEON: Color = Color::srgb(0.85, 0.20, 0.30);
const DIM: Color = Color::srgb(0.55, 0.53, 0.48);
const GOLD: Color = Color::srgb(0.85, 0.70, 0.35);
const STREAK_BLUE: Color = Color::srgb(0.50, 0.72, 0.84);
const CARD_FACE: Color = Color::srgb(0.04, 0.08, 0.06);
/// Multiplied over the card frame to mark a pending All In sacrifice. Kept
/// light, because tinting green felt with a saturated red crushes it to black.
const SACRIFICE_TINT: Color = Color::srgb(1.0, 0.52, 0.56);

/// Portrait side, in pixels. The node is square; `assets/README.md` holds the
/// authoring contract that goes with it.
const PORTRAIT: f32 = 96.0;

/// Art that exists on disk. Anything `None` renders as text.
#[derive(Resource, Default)]
pub struct Art {
    pub frame: Option<Handle<Image>>,
    pub streak: Option<Handle<Image>>,
    pub all_in: Option<Handle<Image>>,
    pub backdrop: Option<Handle<Image>>,
    pub slotz: Option<Handle<Image>>,
    pub pit_boss: Option<Handle<Image>>,
    pub the_house: Option<Handle<Image>>,
}

impl Art {
    /// The face at the other side of the table. Only the three bosses sat for
    /// a portrait; the minions are a name and a Stack, and that is the whole
    /// of them.
    pub fn portrait(&self, id: EncounterId) -> Option<&Handle<Image>> {
        match id {
            EncounterId::Slotz => self.slotz.as_ref(),
            EncounterId::PitBoss => self.pit_boss.as_ref(),
            EncounterId::TheHouse => self.the_house.as_ref(),
            EncounterId::FloorMinion | EncounterId::PitMinion | EncounterId::Tutorial => None,
        }
    }
}

/// Checks `assets/` once at startup so a missing file is a fallback, not a
/// load error at the table.
pub fn load_art(mut commands: Commands, assets: Option<Res<AssetServer>>) {
    let load = |rel: &'static str| {
        let assets = assets.as_ref()?;
        Path::new("assets")
            .join(rel)
            .exists()
            .then(|| assets.load(rel))
    };
    commands.insert_resource(Art {
        frame: load("cards/frame.png"),
        streak: load("tells/streak.png"),
        all_in: load("tells/all_in.png"),
        backdrop: load("backdrops/combat.png"),
        slotz: load("portraits/slotz.png"),
        pit_boss: load("portraits/pit_boss.png"),
        the_house: load("portraits/the_house.png"),
    });
}

#[derive(Component)]
pub struct CombatScreen;

/// A card node in the Draw, by 0-based slot. Clickable (#58); the plugin
/// reads its `Interaction`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardSlot(pub usize);

const HOVER: Color = Color::srgb(1.0, 0.92, 0.70);

/// What `hover_cards` needs from a card node.
pub type HoveredCard = (
    &'static Interaction,
    &'static mut BorderColor,
    Option<&'static mut ImageNode>,
);

/// Rebuilds the whole screen whenever the duel changes. Cheap enough at seven
/// cards, and it keeps every node derived from one source of truth.
pub fn redraw(
    mut commands: Commands,
    active: Res<ActiveDuel>,
    art: Res<Art>,
    existing: Query<Entity, With<CombatScreen>>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }

    let duel = &active.duel;
    let mut root = commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::all(px(40)),
            ..default()
        },
        BackgroundColor(FELT),
        CombatScreen,
        DespawnOnExit(AppState::Combat),
    ));

    if let Some(backdrop) = &art.backdrop {
        root.insert(ImageNode::new(backdrop.clone()));
    }

    root.with_children(|root| {
        // Top: the enemy.
        row(root, JustifyContent::SpaceBetween, |r| {
            // Portrait and name are one thing on the left of the row, so
            // SpaceBetween pushes the Stack and the Edge away from the pair
            // rather than through the middle of it.
            r.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(16),
                ..default()
            })
            .with_children(|who| {
                if let Some(portrait) = art.portrait(active.id) {
                    who.spawn((
                        Node {
                            width: px(PORTRAIT),
                            height: px(PORTRAIT),
                            ..default()
                        },
                        ImageNode::new(portrait.clone()),
                    ));
                }
                text(who, active.enemy_name, 30.0, NEON);
            });
            text(r, format!("Stack {}", duel.enemy_stack()), 26.0, GOLD);
            match (duel.margin(), duel.locked_edge()) {
                (None, _) => text(r, format!("House Edge {}", duel.house_edge()), 26.0, INK),
                (Some(margin), None) => text(r, format!("House Edge ?   (reads your Hand, +{margin})"), 26.0, INK),
                (Some(margin), Some(edge)) => text(r, format!("House Edge {edge}   (your Hand +{margin})"), 26.0, NEON),
            }
        });

        // Middle: The Hand and the feedback line.
        root.spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(10),
            ..default()
        })
        .with_children(|mid| {
            text(mid, format!("The Hand   {}", duel.hand()), 44.0, GOLD);
            text(mid, format!("Plays left  {}", duel.plays_left()), 20.0, DIM);
            if duel.dice_left() > 0 {
                let hands = duel.dice_left();
                let plural = if hands == 1 { "Hand" } else { "Hands" };
                text(mid, format!("Loaded Dice: +{LOADED_DICE_BONUS} on each of your next {hands} {plural}."), 18.0, GOLD);
            }
            if let Some(guide) = &active.guide {
                // The script speaks in gold; once it lets go, the hint sits back.
                let (size, color) = if guide.is_free_play() { (18.0, DIM) } else { (22.0, GOLD) };
                text(mid, guide.prompt(), size, color);
                text(mid, "Esc leaves the Arcade.", 14.0, DIM);
            }
            if duel.margin().is_some() && duel.plays_left() > 1 && duel.phase() == Phase::Playing {
                text(mid, "The House is watching. It sets the line after your fourth card; your last card is the one it can't see.", 18.0, DIM);
            }
            if duel.phase() == Phase::PushYourLuck {
                let held = duel.payout(None);
                let pushed = duel.payout(Some(Push::Won));
                text(mid, format!("Push Your Luck?   Hold for a Payout of {held}, or {pushed} if the coin lands your way."), 24.0, GOLD);
                text(mid, format!("Lose and The Hand is 0: a Whiff of {}, out of your own Stack.", duel.house_edge()), 18.0, NEON);
                text(mid, coin_line(duel.coin()), 18.0, DIM);
            }
            if let Some(turn) = &active.last_turn {
                text(mid, turn_line(turn), 20.0, INK);
                if turn.blinds_rose {
                    text(mid, "The Blinds rise.", 18.0, NEON);
                }
            }
            if let Some(notice) = &active.notice {
                text(mid, notice.clone(), 20.0, NEON);
            }
        });

        // The Draw.
        row(root, JustifyContent::Center, |r| {
            for (i, card) in duel.draw().iter().enumerate() {
                let sacrifice_pending = active.awaiting_sacrifice == Some(i);
                let mut node = r.spawn((
                    Node {
                        width: px(120),
                        height: px(170),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(px(8)),
                        margin: UiRect::all(px(6)),
                        border: UiRect::all(px(2)),
                        ..default()
                    },
                    BackgroundColor(CARD_FACE),
                    Button,
                    CardSlot(i),
                    // The frame art draws its own rounded bezel, so a square
                    // border on the same node leaves a gold notch at each
                    // corner. With art present the edge is the art's job and
                    // the pending state is a tint; without it, the border is
                    // the only edge there is.
                    BorderColor::all(match (&art.frame, sacrifice_pending) {
                        (Some(_), _) => Color::NONE,
                        (None, true) => NEON,
                        (None, false) => GOLD,
                    }),
                ));
                if let Some(frame) = &art.frame {
                    node.insert(ImageNode {
                        color: if sacrifice_pending {
                            SACRIFICE_TINT
                        } else {
                            Color::WHITE
                        },
                        ..ImageNode::new(frame.clone())
                    });
                }
                node.with_children(|c| {
                    text(c, format!("[{}]", i + 1), 16.0, DIM);
                    text(c, card.name, 16.0, INK);
                    text(c, card.stack.to_string(), 34.0, GOLD);
                    match card.tell {
                        Some(Tell::Streak) => tell(c, "Streak", STREAK_BLUE, art.streak.clone()),
                        Some(Tell::AllIn) => tell(c, "All In", NEON, art.all_in.clone()),
                        None => text(c, " ", 14.0, DIM),
                    }
                });
            }
        });

        // Bottom: you.
        row(root, JustifyContent::SpaceBetween, |r| {
            text(r, "Lucky Jack", 26.0, INK);
            text(r, format!("Stack {}", duel.player_stack()), 26.0, GOLD);
            let keys = match duel.phase() {
                Phase::PushYourLuck => "P push   H hold   I what the words mean",
                Phase::Playing => "1-7 play a card   Enter show your Hand   I what the words mean",
            };
            text(r, keys, 16.0, DIM);
        });
    });
}

/// What the coin is, in the player's terms.
fn coin_line(coin: Coin) -> String {
    let house = 100 - coin.player_pct;
    match coin.best_of {
        1 => format!("One flip, {}/{house} the House's way.", coin.player_pct),
        n => format!("Best {} of {n} at {}/{house}.", n / 2 + 1, coin.player_pct),
    }
}

/// The one-line story of the turn that just resolved.
fn turn_line(turn: &TurnResult) -> String {
    let edge = turn.house_edge;
    match (turn.pyl, turn.kind) {
        (Some(Push::Won), Outcome::Payout(n)) => {
            format!("The coin is yours. Payout doubled to {n}.")
        }
        (Some(Push::Lost), Outcome::Whiff(n)) => {
            format!("The coin is the House's. The Hand is 0: Whiff. You lose {n}.")
        }
        (_, Outcome::Payout(n)) => format!("{} vs House Edge {edge}: Payout {n}.", turn.hand),
        (_, Outcome::Whiff(n)) => {
            format!("{} vs House Edge {edge}: Whiff. You lose {n}.", turn.hand)
        }
    }
}

/// Brightens the card under the mouse so the click target is obvious: the
/// border when the card is drawn plain, the frame's tint when the art is in.
/// A pending sacrifice keeps its own colour. The redraw rebuilds the nodes
/// on every change and the focus system re-reports hover on the new node
/// the next frame, so there is nothing to carry over.
pub fn hover_cards(mut cards: Query<HoveredCard, (With<CardSlot>, Changed<Interaction>)>) {
    for (interaction, mut border, image) in &mut cards {
        let hovered = matches!(interaction, Interaction::Hovered | Interaction::Pressed);
        match image {
            Some(mut image) => {
                if image.color != SACRIFICE_TINT {
                    image.color = if hovered { HOVER } else { Color::WHITE };
                }
            }
            None => {
                if *border != BorderColor::all(NEON) {
                    *border = BorderColor::all(if hovered { HOVER } else { GOLD });
                }
            }
        }
    }
}

fn row(
    parent: &mut ChildSpawnerCommands,
    justify: JustifyContent,
    f: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            justify_content: justify,
            align_items: AlignItems::Center,
            column_gap: px(24),
            ..default()
        })
        .with_children(f);
}

fn text(parent: &mut ChildSpawnerCommands, s: impl Into<String>, size: f32, color: Color) {
    parent.spawn((
        Text::new(s),
        TextFont::from_font_size(size),
        TextColor(color),
    ));
}

/// A Tell label, or the artist's icon for it when the file exists.
fn tell(parent: &mut ChildSpawnerCommands, label: &str, color: Color, icon: Option<Handle<Image>>) {
    match icon {
        Some(icon) => {
            parent.spawn((
                Node {
                    width: px(28),
                    height: px(28),
                    ..default()
                },
                ImageNode::new(icon),
            ));
        }
        None => text(parent, label, 14.0, color),
    }
}

#[cfg(test)]
mod tests {
    use bevy::asset::uuid_handle;
    use bevy::prelude::*;

    use super::Art;
    use crate::run::EncounterId;

    const SLOTZ: Handle<Image> = uuid_handle!("2f9d4f2e-0e5f-4a23-9a1a-0b0c1d2e3f40");
    const PIT_BOSS: Handle<Image> = uuid_handle!("2f9d4f2e-0e5f-4a23-9a1a-0b0c1d2e3f41");
    const THE_HOUSE: Handle<Image> = uuid_handle!("2f9d4f2e-0e5f-4a23-9a1a-0b0c1d2e3f42");

    /// Every portrait on disk, so the only thing under test is the match.
    fn hung() -> Art {
        Art {
            slotz: Some(SLOTZ),
            pit_boss: Some(PIT_BOSS),
            the_house: Some(THE_HOUSE),
            ..Art::default()
        }
    }

    #[test]
    fn each_boss_gets_its_own_face() {
        let art = hung();

        assert_eq!(art.portrait(EncounterId::Slotz), Some(&SLOTZ));
        assert_eq!(art.portrait(EncounterId::PitBoss), Some(&PIT_BOSS));
        assert_eq!(art.portrait(EncounterId::TheHouse), Some(&THE_HOUSE));
    }

    #[test]
    fn the_minions_never_sat_for_one() {
        let art = hung();

        assert_eq!(art.portrait(EncounterId::FloorMinion), None);
        assert_eq!(art.portrait(EncounterId::PitMinion), None);
    }

    #[test]
    fn a_missing_file_is_no_portrait_rather_than_a_broken_one() {
        let art = Art::default();

        for id in [
            EncounterId::Slotz,
            EncounterId::PitBoss,
            EncounterId::TheHouse,
        ] {
            assert_eq!(art.portrait(id), None, "{id:?} falls back to no portrait");
        }
    }
}
