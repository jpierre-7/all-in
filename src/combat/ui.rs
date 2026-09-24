//! Draws the duel. Text-first, with an image slot per card and Tell that the
//! artist's files fill in when they exist under `assets/`.
//!
//! Three rows of cards, top to bottom: the Opposing Cards the enemy laid
//! down, the row the player is building across from them, and the Draw the
//! player is building it out of. The two rows are the same width and the same
//! card size, and slot `i` of one sits directly over slot `i` of the other,
//! because which card is across from which is the whole game now.

use std::path::Path;

use bevy::prelude::*;

use super::duel::{Coin, Duel, Opposing, Outcome, Phase, Placed, Push, TurnResult};
use super::plugin::ActiveDuel;
use crate::run::{Card, EncounterId, LOADED_DICE_BONUS, Tell};
use crate::state::AppState;

const INK: Color = Color::srgb(0.90, 0.87, 0.80);
const FELT: Color = Color::srgb(0.05, 0.07, 0.06);
const NEON: Color = Color::srgb(0.85, 0.20, 0.30);
const DIM: Color = Color::srgb(0.55, 0.53, 0.48);
const GOLD: Color = Color::srgb(0.85, 0.70, 0.35);
const CARD_FACE: Color = Color::srgb(0.04, 0.08, 0.06);
/// The back of a face-down Opposing Card, when there is no art for one.
const CARD_BACK: Color = Color::srgb(0.10, 0.05, 0.07);
/// Multiplied over the card frame to mark a pending All In sacrifice. Kept
/// light, because tinting green felt with a saturated red crushes it to black.
const SACRIFICE_TINT: Color = Color::srgb(1.0, 0.52, 0.56);
/// The empty slots in the player's row, waiting to be covered.
const EMPTY_SLOT: Color = Color::srgb(0.16, 0.20, 0.17);

/// The Draw's card size, in pixels. The Peek hangs its tag off these.
pub const CARD_WIDTH: f32 = 120.0;
pub const CARD_HEIGHT: f32 = 168.0;
/// The two facing rows are drawn smaller: there are three rows of cards on
/// the table now, and the Draw is the one the player is reaching into.
pub const ROW_WIDTH: f32 = 88.0;
pub const ROW_HEIGHT: f32 = 123.0;

/// Portrait side, in pixels. The node is square; `assets/README.md` holds the
/// authoring contract that goes with it.
const PORTRAIT: f32 = 72.0;

/// Which of the three rows a card node is in. The plugin reads this to tell a
/// card being picked up out of the Draw from one being taken back out of the
/// row, and the Peek reads it to know how big a card it is hanging a tag off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    /// The cards the player is holding.
    Draw,
    /// The row the player is building. Clicking one takes it back.
    Row,
    /// The enemy's row. Nothing to click; the Peek still reads the face-up ones.
    Opposing,
}

impl Zone {
    /// The card size this row is drawn at.
    pub fn card_size(self) -> (f32, f32) {
        match self {
            Zone::Draw => (CARD_WIDTH, CARD_HEIGHT),
            Zone::Row | Zone::Opposing => (ROW_WIDTH, ROW_HEIGHT),
        }
    }
}

/// Art that exists on disk. Anything `None` renders as text.
#[derive(Resource, Default)]
pub struct Art {
    pub frame: Option<Handle<Image>>,
    pub back: Option<Handle<Image>>,
    pub streak: Option<Handle<Image>>,
    pub all_in: Option<Handle<Image>>,
    pub copycat: Option<Handle<Image>>,
    pub flop: Option<Handle<Image>>,
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

    /// The icon a Tell is drawn with, if there is one.
    pub fn tell(&self, tell: Tell) -> Option<&Handle<Image>> {
        match tell {
            Tell::Streak => self.streak.as_ref(),
            Tell::AllIn => self.all_in.as_ref(),
            Tell::Copycat => self.copycat.as_ref(),
            Tell::Flop => self.flop.as_ref(),
        }
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
        back: load("cards/back.png"),
        streak: load("tells/streak.png"),
        all_in: load("tells/all_in.png"),
        copycat: load("tells/copycat.png"),
        flop: load("tells/flop.png"),
        backdrop: load("backdrops/combat.png"),
        slotz: load("portraits/slotz.png"),
        pit_boss: load("portraits/pit_boss.png"),
        the_house: load("portraits/the_house.png"),
    });
}

#[derive(Component)]
pub struct CombatScreen;

/// A card node on the table: which row it is in and where. Clickable (#58);
/// the plugin reads its `Interaction` and its `zone` together.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardSlot {
    pub zone: Zone,
    pub index: usize,
}

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

/// Rebuilds the whole screen whenever the duel changes. Cheap enough at three
/// short rows, and it keeps every node derived from one source of truth.
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
            align_items: AlignItems::Center,
            // Enough that the corner rows read as sitting on the table, not
            // pinned to the window edge.
            padding: UiRect::axes(px(56), px(24)),
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

        // Top: the enemy, and what its row adds up to as far as anyone can see.
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
                text(who, active.enemy_name, 26.0, NEON);
            });
            text(r, format!("Stack {}", duel.enemy_stack()), 24.0, GOLD);
            edge_line(r, duel);
        });

        // The Opposing Cards, then the row the player is building under them.
        // Same card size and same order, so a slot is a column of the table.
        facing_rows(root, &active, &art);

        // The middle: what the two rows read, and whatever the table is
        // saying about the turn.
        root.spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(6),
            ..default()
        })
        .with_children(|mid| {
            text(mid, format!("The Hand   {}", duel.hand()), 34.0, GOLD);

            if duel.dice_left() > 0 {
                let hands = duel.dice_left();
                let plural = if hands == 1 { "Hand" } else { "Hands" };
                text(mid, format!("Loaded Dice: +{LOADED_DICE_BONUS} on each of your next {hands} {plural}."), 16.0, GOLD);
            }
            if let Some(guide) = &active.guide {
                // The script speaks in gold; once it lets go, the hint sits back.
                let (size, color) = if guide.is_free_play() { (16.0, DIM) } else { (20.0, GOLD) };
                text(mid, guide.prompt(), size, color);
                text(mid, "Esc leaves the Arcade.", 14.0, DIM);
            }
            if let Some(margin) = duel.margin()
                && duel.phase() == Phase::Playing
            {
                text(mid, format!("The House keeps its last card back. It fills it in on everything but your last card, plus {margin} - so your last card is the one it can't see."), 16.0, DIM);
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
                text(mid, turn_line(turn), 18.0, INK);
                // Both rows are off the table by the time this is read, so
                // the line has to carry what they came to, slot by slot.

                if turn.blinds_rose {
                    text(mid, blinds_line(duel), 16.0, NEON);
                }
            }
            if let Some(notice) = &active.notice {
                text(mid, notice.clone(), 18.0, NEON);
            }
        });

        // The Draw. Each slot is a column: the card, then its key underneath.
        row(root, JustifyContent::Center, |r| {
            for (i, card) in duel.draw().iter().enumerate() {
                let sacrifice_pending = active.awaiting_sacrifice == Some(i);
                r.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(4),
                    margin: UiRect::horizontal(px(5)),
                    ..default()
                })
                .with_children(|slot| {
                    card_node(
                        slot,
                        &art,
                        CardSlot {
                            zone: Zone::Draw,
                            index: i,
                        },
                        Some(card),
                        sacrifice_pending,
                        None,
                    );
                    // The keyboard pointer (the Peek, #106) brackets its slot.
                    if active.pointer == Some(i) {
                        text(slot, format!("[{}]", i + 1), 15.0, HOVER);
                    } else {
                        text(slot, (i + 1).to_string(), 15.0, DIM);
                    }
                });
            }
        });

        // Bottom: you.
        row(root, JustifyContent::SpaceBetween, |r| {
            text(r, "Lucky Jack", 24.0, INK);
            text(r, format!("Stack {}", duel.player_stack()), 24.0, GOLD);
            let keys = match duel.phase() {
                Phase::PushYourLuck => "P: Push   H: Hold   I: Help and Terms",
                Phase::Playing => {
                    "1-7: Place Card  Backspace/Esc: Cancel  Enter: Confirm  T: Toggle Peek  I: Help and Terms"
                }
            };
            text(r, keys, 14.0, DIM);
        });
    });
}

/// The two rows facing each other: the Opposing Cards, then the player's row
/// directly under them, one column per slot.
fn facing_rows(root: &mut ChildSpawnerCommands, active: &ActiveDuel, art: &Art) {
    let duel = &active.duel;
    // What each card turned out to be worth, but only while both rows are
    // face up and the turn has not yet been answered.
    let showdown = duel.showdown();

    root.spawn(Node {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        row_gap: px(4),
        ..default()
    })
    .with_children(|table| {
        row(table, JustifyContent::Center, |r| {
            for (i, opposing) in duel.opposing().iter().enumerate() {
                let value = showdown.and_then(|s| s.theirs.get(i)).map(|p| p.value);
                slot_column(
                    r,
                    art,
                    Zone::Opposing,
                    i,
                    opposing_face(opposing),
                    value,
                    None,
                );
            }
        });
        row(table, JustifyContent::Center, |r| {
            for i in 0..duel.slots() {
                let placed: Option<&Placed> = duel.row().get(i);
                let value = showdown.and_then(|s| s.yours.get(i)).map(|p| p.value);
                // Past the player's Plays there is no slot to fill, only a
                // card of theirs nobody is covering.
                let coverable = i < usize::from(duel.plays_left()) + duel.row().len();
                let label = (placed.is_none() && !coverable).then_some("uncovered");
                slot_column(
                    r,
                    art,
                    Zone::Row,
                    i,
                    placed.map(|p| p.card.clone()),
                    value,
                    label,
                );
            }
        });
    });
}

/// One column of a facing row: the card (or the empty slot), and the number
/// under it once the rows have turned over.
fn slot_column(
    parent: &mut ChildSpawnerCommands,
    art: &Art,
    zone: Zone,
    index: usize,
    card: Option<Card>,
    value: Option<u32>,
    label: Option<&str>,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(2),
            margin: UiRect::horizontal(px(5)),
            ..default()
        })
        .with_children(|column| {
            card_node(
                column,
                art,
                CardSlot { zone, index },
                card.as_ref(),
                false,
                label,
            );
            match value {
                Some(value) => text(column, value.to_string(), 17.0, GOLD),
                None => text(column, " ", 17.0, DIM),
            }
        });
}

/// An Opposing Card as the player is allowed to see it: the card itself when
/// it is face up, and nothing at all when it isn't.
fn opposing_face(opposing: &Opposing) -> Option<Card> {
    opposing.revealed.then(|| opposing.card.clone())
}

/// One card node. `card` of `None` draws an empty slot — a face-down Opposing
/// Card, or a slot in the player's row nobody has covered yet.
fn card_node(
    parent: &mut ChildSpawnerCommands,
    art: &Art,
    slot: CardSlot,
    card: Option<&Card>,
    sacrifice_pending: bool,
    label: Option<&str>,
) {
    let (width, height) = slot.zone.card_size();
    // The Draw's cards carry the frame art; the two facing rows are drawn
    // plain, small and bordered, so a column reads as a column.
    let framed = slot.zone == Zone::Draw && art.frame.is_some();
    let face_down = card.is_none() && slot.zone == Zone::Opposing;
    let empty = card.is_none() && slot.zone != Zone::Opposing;
    let background = match (framed, face_down, empty) {
        (true, _, _) => Color::NONE,
        (_, true, _) => CARD_BACK,
        (_, _, true) => Color::NONE,
        _ => CARD_FACE,
    };
    let border = match (framed, sacrifice_pending, empty) {
        (true, _, _) => Color::NONE,
        (_, true, _) => NEON,
        (_, _, true) => EMPTY_SLOT,
        _ => GOLD,
    };

    parent
        .spawn((
            Node {
                width: px(width),
                height: px(height),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                // Inside the frame art's bezel, which is about a sixth of the
                // card's width on every side at this size.
                padding: UiRect::axes(px(width / 7.0), px(height / 10.0)),
                border: UiRect::all(px(2)),
                ..default()
            },
            // With the frame art in, the art is the whole face: no felt
            // showing around its rounded corners and no square border
            // notching them. Without it, the plain face and border are all
            // the edge there is.
            BackgroundColor(background),
            Button,
            slot,
            BorderColor::all(border),
        ))
        .with_children(|c| {
            if framed && let Some(frame) = &art.frame {
                // The frame is its own layer filling the card, so the padding
                // insets the face content and not the art (an ImageNode on
                // the padded node is drawn inside the padding).
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
                        color: if sacrifice_pending {
                            SACRIFICE_TINT
                        } else {
                            Color::WHITE
                        },
                        ..ImageNode::new(frame.clone()).with_mode(NodeImageMode::Stretch)
                    },
                    ZIndex(-1),
                    CardFrame,
                ));
            }
            match card {
                // Playing-card layout: the value in two corners and the art
                // (or the Tell, large) in the middle. No name on the face; it
                // doesn't fit, and the prompts say it when it matters.
                Some(card) => {
                    let value = card.stack.to_string();
                    let corner = if slot.zone == Zone::Draw { 18.0 } else { 15.0 };
                    corner_row(c, JustifyContent::FlexStart, &value, corner);
                    centre(c, card, art, height * 0.36);
                    corner_row(c, JustifyContent::FlexEnd, &value, corner);
                }
                // Face down: the back art if there is one, else the mark the
                // house prints on it.
                None if face_down => match &art.back {
                    Some(back) => {
                        c.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(0),
                                top: px(0),
                                width: percent(100),
                                height: percent(100),
                                ..default()
                            },
                            ImageNode::new(back.clone()).with_mode(NodeImageMode::Stretch),
                        ));
                    }
                    None => {
                        c.spawn(Node {
                            width: percent(100),
                            height: percent(100),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|face| text(face, "?", height * 0.30, NEON));
                    }
                },
                // An empty slot in the player's row.
                None => {
                    c.spawn(Node {
                        width: percent(100),
                        height: percent(100),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|face| {
                        text(face, label.unwrap_or("+"), 13.0, EMPTY_SLOT);
                    });
                }
            }
        });
}

/// What the player is allowed to know about the Opposing Cards' Stack Sum.
fn edge_line(parent: &mut ChildSpawnerCommands, duel: &Duel) {
    // Once the rows are face up the Edge is simply the Edge.
    if duel.phase() == Phase::PushYourLuck {
        text(
            parent,
            format!("House Edge {}", duel.house_edge()),
            24.0,
            NEON,
        );
        return;
    }
    let (showing, hidden) = duel.showing();
    let line = match (hidden, duel.margin()) {
        (0, _) => format!("House Edge {showing}"),
        (1, Some(margin)) => {
            format!("House Edge {showing} + the card it kept back   (your row, +{margin})")
        }
        (_, None) => format!("House Edge {showing} + ?"),
        (_, _) => format!("House Edge {showing} + ?"),
    };
    text(parent, line, 24.0, INK);
}

/// What the Blinds just did, which is not the same thing for The House.
fn blinds_line(duel: &Duel) -> String {
    match duel.margin() {
        Some(margin) => format!("The Blinds rise. The margin is {margin}."),
        None => format!("The Blinds rise. {} Opposing Cards now.", duel.slots()),
    }
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
        (_, Outcome::Payout(0)) => format!("Tie. Your Hand was equal to the House Edge"),
        (_, Outcome::Payout(n)) => format!("{} vs House Edge {edge}: Payout {n}.", turn.hand),
        (_, Outcome::Whiff(n)) => {
            format!("{} against {edge}: Whiff. You lose {n}.", turn.hand)
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
                // Empty slots and face-down cards keep their own edge; only a
                // card that can actually be clicked lights up.
                if *border != BorderColor::all(NEON) && *border != BorderColor::all(EMPTY_SLOT) {
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
            column_gap: px(20),
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
fn corner_row(parent: &mut ChildSpawnerCommands, justify: JustifyContent, value: &str, size: f32) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            justify_content: justify,
            ..default()
        })
        .with_children(|row| text(row, value, size, GOLD));
}

/// The middle of a card: the artist's face art for this card, else the
/// Tell's icon drawn large, else nothing.
fn centre(parent: &mut ChildSpawnerCommands, card: &Card, art: &Art, size: f32) {
    let image = art
        .face(card.name)
        .cloned()
        .or_else(|| card.tell.and_then(|tell| art.tell(tell).cloned()));
    let mut node = parent.spawn(Node {
        width: px(size),
        height: px(size),
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

    use super::{Art, Zone};
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

    #[test]
    fn the_two_facing_rows_are_drawn_at_the_same_size() {
        // A slot is a column of the table: the Opposing Card and the card
        // covering it have to line up, whatever the Draw is drawn at.
        assert_eq!(Zone::Row.card_size(), Zone::Opposing.card_size());
        assert_ne!(Zone::Draw.card_size(), Zone::Row.card_size());
    }
}
