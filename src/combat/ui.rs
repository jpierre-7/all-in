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
const CARD_FACE: Color = Color::srgb(0.04, 0.08, 0.06);
/// Multiplied over the card frame to mark a pending All In sacrifice. Kept
/// light, because tinting green felt with a saturated red crushes it to black.
const SACRIFICE_TINT: Color = Color::srgb(1.0, 0.52, 0.56);

/// The card node's size, in pixels. The Peek hangs its tag off these.
pub const CARD_WIDTH: f32 = 150.0;
pub const CARD_HEIGHT: f32 = 210.0;

/// Portrait side, in pixels. The node is square; `assets/README.md` holds the
/// authoring contract that goes with it.
const PORTRAIT: f32 = 96.0;

/// Art that exists on disk. Anything `None` renders as text.
#[derive(Resource, Default)]
pub struct Art {
    pub frame: Option<Handle<Image>>,
    pub streak: Option<Handle<Image>>,
    pub all_in: Option<Handle<Image>>,
    pub copycat: Option<Handle<Image>>,
    pub backdrop: Option<Handle<Image>>,
    pub slotz: Option<Handle<Image>>,
    pub pit_boss: Option<Handle<Image>>,
    pub the_house: Option<Handle<Image>>,
    /// Per-card face art under `assets/cards/faces/`, keyed by the card's
    /// name as a file stem (`bus_ticket_home.png`). Whatever is there.
    pub faces: std::collections::HashMap<String, Handle<Image>>,
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

    /// The art in the middle of a card, if the artist drew one for it.
    pub fn face(&self, card_name: &str) -> Option<&Handle<Image>> {
        self.faces.get(&face_stem(card_name))
    }
}

/// "Bus Ticket Home" -> "bus_ticket_home".
fn face_stem(card_name: &str) -> String {
    card_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
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
    let faces = std::fs::read_dir("assets/cards/faces")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let stem = path.file_stem()?.to_str()?.to_string();
            if path.extension()?.to_str()? != "png" {
                return None;
            }
            let handle = assets.as_ref()?.load(format!("cards/faces/{stem}.png"));
            Some((stem, handle))
        })
        .collect();
    commands.insert_resource(Art {
        faces,
        frame: load("cards/frame.png"),
        streak: load("tells/streak.png"),
        all_in: load("tells/all_in.png"),
        copycat: load("tells/copycat.png"),
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

/// The frame art layer inside a card node; hover tints it.
#[derive(Component)]
pub struct CardFrame;

const HOVER: Color = Color::srgb(1.0, 0.92, 0.70);

/// What `hover_cards` needs from a card node.
pub type HoveredCard = (
    &'static Interaction,
    &'static Children,
    &'static mut BorderColor,
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
            // Enough that the corner rows read as sitting on the table, not
            // pinned to the window edge.
            padding: UiRect::axes(px(88), px(60)),
            ..default()
        },
        BackgroundColor(FELT),
        CombatScreen,
        DespawnOnExit(AppState::Combat),
    ));

    root.with_children(|root| {
        // The backdrop is its own layer behind everything, filling the root
        // regardless of padding. An ImageNode on the root itself would be
        // drawn inside the padding, pushing the art in instead of the text.
        if let Some(backdrop) = &art.backdrop {
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px(0),
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                ImageNode::new(backdrop.clone()).with_mode(NodeImageMode::Stretch),
                ZIndex(-1),
            ));
        }

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
                if duel.hand() >= duel.house_edge() {
                    let held = duel.payout(None);
                    let pushed = duel.payout(Some(Push::Won));
                    text(mid, format!("Push Your Luck?   Hold for a Payout of {held}, or {pushed} if the coin lands your way."), 24.0, GOLD);
                    text(mid, format!("Lose and The Hand is 0: a Whiff of {}, out of your own Stack.", duel.house_edge()), 18.0, NEON);
                } else {
                    let held = duel.whiff(None);
                    let lost = duel.whiff(Some(Push::Lost));
                    text(mid, format!("Push Your Luck?   Hold and take the Whiff of {held}, or Push: the coin lands your way and it's forgiven."), 24.0, GOLD);
                    text(mid, format!("Lose and the Whiff doubles to {lost}, out of your own Stack."), 18.0, NEON);
                }
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

        // The Draw. Each slot is a column: the card, then its key underneath.
        row(root, JustifyContent::Center, |r| {
            for (i, card) in duel.draw().iter().enumerate() {
                let sacrifice_pending = active.awaiting_sacrifice == Some(i);
                r.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(6),
                    margin: UiRect::horizontal(px(6)),
                    ..default()
                })
                .with_children(|slot| {
                    slot.spawn((
                        Node {
                            width: px(CARD_WIDTH),
                            height: px(CARD_HEIGHT),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            // Inside the frame art's bezel, which is about
                            // 20px deep on every side at this size.
                            padding: UiRect::axes(px(26), px(22)),
                            border: UiRect::all(px(2)),
                            ..default()
                        },
                        // With the frame art in, the art is the whole face:
                        // no felt showing around its rounded corners and no
                        // square border notching them. Without it, the plain
                        // face and border are all the edge there is.
                        BackgroundColor(if art.frame.is_some() { Color::NONE } else { CARD_FACE }),
                        Button,
                        CardSlot(i),
                        BorderColor::all(match (&art.frame, sacrifice_pending) {
                            (Some(_), _) => Color::NONE,
                            (None, true) => NEON,
                            (None, false) => GOLD,
                        }),
                    ))
                    .with_children(|c| {
                        // The frame is its own layer filling the card, so the
                        // padding insets the face content and not the art
                        // (an ImageNode on the padded node is drawn inside
                        // the padding).
                        if let Some(frame) = &art.frame {
                            c.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: px(0),
                                    top: px(0),
                                    width: percent(100),
                                    height: percent(100),
                                    ..default()
                                },
                                ImageNode {
                                    color: if sacrifice_pending { SACRIFICE_TINT } else { Color::WHITE },
                                    ..ImageNode::new(frame.clone()).with_mode(NodeImageMode::Stretch)
                                },
                                ZIndex(-1),
                                CardFrame,
                            ));
                        }
                        // Playing-card layout: the value in two corners and
                        // the art (or the Tell, large) in the middle. No name
                        // on the face; it doesn't fit, and the prompts say it
                        // when it matters.
                        let value = card.stack.to_string();
                        corner_row(c, JustifyContent::FlexStart, &value);
                        centre(c, card, &art);
                        corner_row(c, JustifyContent::FlexEnd, &value);
                    });
                    // The keyboard pointer (the Peek, #106) brackets its slot.
                    if active.pointer == Some(i) {
                        text(slot, format!("[{}]", i + 1), 16.0, HOVER);
                    } else {
                        text(slot, (i + 1).to_string(), 16.0, DIM);
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
                Phase::Playing => "1-7 play a card   Enter show your Hand   T peek   I what the words mean",
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
        // A lost Push on a Hand that cleared the Edge zeroed it.
        (Some(Push::Lost), Outcome::Whiff(n)) if turn.hand >= edge => {
            format!("The coin is the House's. The Hand is 0: Whiff. You lose {n}.")
        }
        (Some(Push::Won), Outcome::Whiff(_)) => {
            format!(
                "{} vs House Edge {edge}. The coin is yours: the Whiff is forgiven.",
                turn.hand
            )
        }
        (Some(Push::Lost), Outcome::Whiff(n)) => {
            format!(
                "{} vs House Edge {edge}. The coin is the House's: the Whiff doubles. You lose {n}.",
                turn.hand
            )
        }
        (_, Outcome::Payout(n)) => format!("{} vs House Edge {edge}: Payout {n}.", turn.hand),
        (_, Outcome::Whiff(n)) => {
            format!("{} vs House Edge {edge}: Whiff. You lose {n}.", turn.hand)
        }
    }
}

/// Brightens the card under the mouse so the click target is obvious: the
/// frame's tint when the art is in, the border when the card is drawn plain.
/// A pending sacrifice keeps its own colour. The redraw rebuilds the nodes
/// on every change and the focus system re-reports hover on the new node
/// the next frame, so there is nothing to carry over.
pub fn hover_cards(
    mut cards: Query<HoveredCard, (With<CardSlot>, Changed<Interaction>)>,
    mut frames: Query<&mut ImageNode, With<CardFrame>>,
) {
    for (interaction, children, mut border) in &mut cards {
        let hovered = matches!(interaction, Interaction::Hovered | Interaction::Pressed);
        let frame = children.iter().find(|child| frames.contains(*child));
        match frame {
            Some(frame) => {
                let mut image = frames.get_mut(frame).expect("found above");
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

/// A corner of the card face: the value, top-left or bottom-right.
fn corner_row(parent: &mut ChildSpawnerCommands, justify: JustifyContent, value: &str) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            justify_content: justify,
            ..default()
        })
        .with_children(|row| text(row, value, 20.0, GOLD));
}

/// The middle of a card: the artist's face art for this card, else the
/// Tell's icon drawn large, else nothing.
fn centre(parent: &mut ChildSpawnerCommands, card: &crate::run::Card, art: &Art) {
    let image = art.face(card.name).cloned().or_else(|| match card.tell {
        Some(Tell::Streak) => art.streak.clone(),
        Some(Tell::AllIn) => art.all_in.clone(),
        Some(Tell::Copycat) => art.copycat.clone(),
        None => None,
    });
    let mut node = parent.spawn(Node {
        width: px(72),
        height: px(72),
        ..default()
    });
    if let Some(image) = image {
        node.insert(ImageNode::new(image));
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
