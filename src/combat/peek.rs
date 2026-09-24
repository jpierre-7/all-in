//! The Peek (#106): the tag that opens beside the card the player points at,
//! mouse or keyboard, and says its Tell. Variant B of the #102 prototype:
//! the Tell's rules text alone, one level of nested terms, cards only. The
//! rest of the table keeps the `I` overlay. `T` waves it off and calls it
//! back.
//!
//! The tag is a child of the card node it explains, so the redraw that
//! rebuilds the Draw takes the tag with it and [`show`] puts it back on the
//! next frame. Nothing is carried over between frames but what the player is
//! pointing at.

use bevy::prelude::*;
use bevy::ui::FocusPolicy;

use super::duel::Duel;
use super::info::{self, InfoOpen, get_keywords};
use super::plugin::ActiveDuel;
use super::ui::{CardSlot, Zone};
use crate::run::{Card, Tell};

const INK: Color = Color::srgb(0.90, 0.87, 0.80);
const GOLD: Color = Color::srgb(0.85, 0.70, 0.35);
const DIM: Color = Color::srgb(0.55, 0.53, 0.48);
const NEON: Color = Color::srgb(0.85, 0.20, 0.30);
const HOVER: Color = Color::srgb(1.0, 0.92, 0.70);
/// Near-black felt, opaque: the tag sits over the table's middle and has to
/// read against whatever the feedback line is saying.
const PANEL: Color = Color::srgb(0.02, 0.06, 0.04);

const TAG_WIDTH: f32 = 260.0;
const NEST_WIDTH: f32 = 230.0;
/// Between the tag's bottom edge and the card's top.
const GAP: f32 = 8.0;
/// Between the words of the rule, each its own node so a term can be pointed at.
const WORD_GAP: f32 = 4.0;

/// Whether the Peek is up for the table, and which term the keyboard has
/// opened inside it. Lives with the session: there is no save file, so it is
/// not remembered between launches, but it survives every table of a run.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Peek {
    pub on: bool,
    /// `Up` steps through the terms in the open tag; `Down` closes them.
    /// Counted, not named, so it is the nth term of whatever card is pointed
    /// at. Reset when the pointer moves.
    term: Option<usize>,
    /// The last card the mouse was over, anywhere on the table. The tag
    /// stays on it after the cursor leaves, so crossing the gap up to the tag
    /// (or off onto the felt) does not snap it back to the keyboard's card.
    /// The keyboard takes over again the moment it moves.
    mouse: Option<CardSlot>,
}

impl Default for Peek {
    fn default() -> Self {
        Self {
            on: true,
            term: None,
            mouse: None,
        }
    }
}

/// The tag on the table, and what it is showing, so the next frame can tell
/// whether it is still the right one.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tag {
    /// Which card on the table it is explaining, in whichever row.
    pub card: CardSlot,
    /// The term whose own tag is open beside this one.
    pub term: Option<&'static str>,
    /// While an All In waits for its sacrifice: what burning this card adds.
    pub burn: Option<u32>,
}

/// The card in a slot, as the player is entitled to see it. A face-down
/// Opposing Card has nothing to say about itself, so nothing is what the
/// Peek says about it.
fn card_at(duel: &Duel, slot: CardSlot) -> Option<Card> {
    match slot.zone {
        Zone::Draw => duel.draw().get(slot.slot).cloned(),
        Zone::Row => duel.row().get(slot.slot).map(|p| p.card.clone()),
        Zone::Opposing => duel
            .opposing()
            .get(slot.slot)
            .filter(|opposing| opposing.face_up)
            .map(|opposing| opposing.card.clone()),
    }
}

/// A term inside the rules text. Point at it and it gets a tag of its own.
#[derive(Component, Debug, Clone, Copy)]
pub struct Term(pub &'static str);

fn terms(tell: Tell) -> impl Iterator<Item = &'static str> {
    let text = tell.rule_text();
    text.into_iter().filter(|piece| is_keyword(piece))
}

fn is_keyword(string: &str) -> bool {
    get_keywords().contains(&string)
}

/// The key that waves the Peek off and calls it back.
pub fn toggled(keys: &ButtonInput<KeyCode>) -> bool {
    keys.just_pressed(KeyCode::KeyT)
}

/// The keyboard's side of pointing: `Left` and `Right` walk the Draw, `Esc`
/// lets go, `Up` and `Down` step through the terms in the open tag, `T`
/// toggles. Runs before the table takes its input, so an `Esc` that backs
/// out of an All In is that and nothing else. The pointer lives on the duel
/// so the redraw can mark the slot, and is only written when it moves.
pub fn walk(
    keys: Res<ButtonInput<KeyCode>>,
    active: Option<ResMut<ActiveDuel>>,
    mut peek: ResMut<Peek>,
    info: Option<Res<InfoOpen>>,
) {
    let Some(mut active) = active else { return };
    // The glossary is up, or is about to be: the table hears nothing.
    if info.is_some() || info::toggled(&keys) {
        return;
    }
    if toggled(&keys) {
        peek.on = !peek.on;
    }

    let len = active.duel.draw().len();
    let was = active.pointer;
    // A play shrank the Draw under the pointer: it lands on the nearest slot.
    let mut pointer = match was {
        Some(p) if p >= len => len.checked_sub(1),
        other => other,
    };
    let left = keys.just_pressed(KeyCode::ArrowLeft);
    let right = keys.just_pressed(KeyCode::ArrowRight);
    if (left || right) && len > 0 {
        pointer = Some(match pointer {
            None if right => 0,
            None => len - 1,
            Some(p) if right => (p + 1) % len,
            Some(p) => (p + len - 1) % len,
        });
        peek.term = None;
        peek.mouse = None;
    }
    // Esc has other jobs first: backing out of a sacrifice, leaving the Arcade.
    if keys.just_pressed(KeyCode::Escape)
        && active.awaiting_sacrifice.is_none()
        && active.guide.is_none()
    {
        pointer = None;
        peek.term = None;
        peek.mouse = None;
    }
    if pointer != was {
        active.pointer = pointer;
    }

    if keys.just_pressed(KeyCode::ArrowUp) {
        peek.term = Some(peek.term.map_or(0, |t| t + 1));
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        peek.term = None;
    }
}

/// What the tag would show right now, if anything. Notes the card under the
/// mouse on the way, for the frames after it leaves.
fn wanted(
    peek: &mut Peek,
    active: &ActiveDuel,
    cards: &Query<(Entity, &Interaction, &CardSlot)>,
    terms_hovered: &Query<(&Interaction, &Term)>,
) -> Option<Tag> {
    if !peek.on {
        return None;
    }
    let hovered = |i: &Interaction| matches!(i, Interaction::Hovered | Interaction::Pressed);
    let under_mouse = cards
        .iter()
        .find(|(_, i, _)| hovered(i))
        .map(|(_, _, slot)| *slot);
    if under_mouse.is_some() && under_mouse != peek.mouse {
        peek.mouse = under_mouse;
    }
    // The mouse's card, now or lately, wins over the keyboard's: it is the
    // thing the player most recently pointed at, until the keyboard moves.
    // The keyboard only ever walks the Draw.
    let keyboard = active.pointer.map(|index| CardSlot {
        zone: Zone::Draw,
        slot: index,
    });
    let card = under_mouse
        .or_else(|| {
            peek.mouse
                .filter(|slot| card_at(&active.duel, *slot).is_some())
        })
        .or(keyboard)?;
    let held = card_at(&active.duel, card)?;
    let term = terms_hovered
        .iter()
        .find(|(i, _)| hovered(i))
        .map(|(_, t)| t.0)
        .or_else(|| {
            let tell = held.tell?;
            let all: Vec<_> = terms(tell).collect();
            peek.term.map(|t| all[t % all.len()])
        });
    // Only a card still in the Draw can be burned to a waiting All In.
    let burn = match active.awaiting_sacrifice {
        Some(all_in) if card.zone == Zone::Draw && all_in != card.slot => Some(held.face_value),
        _ => None,
    };
    Some(Tag { card, term, burn })
}

/// Keeps the one tag on the table matching what the player points at:
/// nothing to do while it does, else the old one goes and the new one is
/// spawned under its card.
pub fn show(
    mut commands: Commands,
    mut peek: ResMut<Peek>,
    active: Option<Res<ActiveDuel>>,
    info: Option<Res<InfoOpen>>,
    cards: Query<(Entity, &Interaction, &CardSlot)>,
    tags: Query<(Entity, &Tag)>,
    terms_hovered: Query<(&Interaction, &Term)>,
) {
    let Some(active) = active else { return };
    let wanted = if info.is_some() {
        None
    } else {
        wanted(&mut peek, &active, &cards, &terms_hovered)
    };
    let current = tags.iter().next().map(|(entity, tag)| (entity, *tag));
    match (current, wanted) {
        (Some((_, showing)), Some(wanted)) if showing == wanted => return,
        (Some((entity, _)), _) => commands.entity(entity).despawn(),
        (None, _) => {}
    }
    let Some(tag) = wanted else { return };
    let Some((card_node, _, _)) = cards.iter().find(|(_, _, slot)| **slot == tag.card) else {
        return;
    };
    let Some(held) = card_at(&active.duel, tag.card) else {
        return;
    };
    // The tag opens above the card it explains, whichever row that is and
    // whatever size that row is drawn at. The Opposing Cards sit at the top
    // of the table with nothing above them, so theirs opens downwards.
    let (card_width, card_height) = tag.card.zone.card_size();
    let mut node = Node {
        position_type: PositionType::Absolute,
        left: px((card_width - TAG_WIDTH) / 2.0),
        width: px(TAG_WIDTH),
        flex_direction: FlexDirection::Column,
        row_gap: px(4),
        padding: UiRect::axes(px(8), px(6)),
        border: UiRect::all(px(2)),
        ..default()
    };
    if tag.card.zone == Zone::Opposing {
        node.top = px(card_height + GAP);
    } else {
        node.bottom = px(card_height + GAP);
    }
    commands.entity(card_node).with_children(|card| {
        card.spawn((
            node,
            BackgroundColor(PANEL),
            BorderColor::all(GOLD),
            // Solid to the cursor, so a card behind it never lights up.
            FocusPolicy::Block,
            GlobalZIndex(5),
            tag,
        ))
        .with_children(|body| {
            match held.tell {
                Some(tell) => {
                    text(body, tell.name(), 20.0, GOLD);
                    body.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: px(WORD_GAP),
                        row_gap: px(2),
                        align_items: AlignItems::Baseline,
                        ..default()
                    })
                    .with_children(|line| {
                        for piece in tell.rule_text() {
                            if is_keyword(piece) {
                                let open = tag.term == Some(piece);
                                line.spawn((
                                    Text::new(piece),
                                    TextFont::from_font_size(16.0),
                                    TextColor(if open { HOVER } else { GOLD }),
                                    Button,
                                    Term(piece),
                                ));
                            } else {
                                for word in piece.split_whitespace() {
                                    // Punctuation closes up to the term before it.
                                    let closing = word.starts_with(['.', ',']);
                                    line.spawn((
                                        Text::new(word),
                                        TextFont::from_font_size(16.0),
                                        TextColor(INK),
                                        Node {
                                            margin: UiRect::left(px(if closing {
                                                -WORD_GAP
                                            } else {
                                                0.0
                                            })),
                                            ..default()
                                        },
                                    ));
                                }
                            }
                        }
                    });
                }
                None => {
                    text(body, "No Tell", 20.0, DIM);
                    text(
                        body,
                        format!("{} chips, what it says.", held.face_value),
                        16.0,
                        DIM,
                    );
                }
            }
            if let Some(burn) = tag.burn {
                text(body, format!("Burn for +{burn}"), 16.0, NEON);
            }
            if let Some(term) = tag.term {
                nest(body, tag.card.slot, term);
            }
        });
    });
}

/// The second tag: a term's glossary line, beside the first. To the right
/// for the left half of the Draw and to the left for the right half, so the
/// last cards' tags stay on the table.
fn nest(parent: &mut ChildSpawnerCommands, card: usize, term: &'static str) {
    let mut node = Node {
        position_type: PositionType::Absolute,
        top: px(-2),
        width: px(NEST_WIDTH),
        flex_direction: FlexDirection::Column,
        row_gap: px(4),
        padding: UiRect::axes(px(8), px(6)),
        border: UiRect::all(px(2)),
        ..default()
    };
    if card < 4 {
        node.left = px(TAG_WIDTH - 2.0 + 6.0);
    } else {
        node.right = px(TAG_WIDTH - 2.0 + 6.0);
    }
    parent
        .spawn((node, BackgroundColor(PANEL), BorderColor::all(DIM)))
        .with_children(|body| {
            text(body, term, 18.0, INK);
            let line = info::define(term).unwrap_or("");
            body.spawn((
                Text::new(line),
                TextFont::from_font_size(16.0),
                TextColor(INK),
                Node {
                    width: px(NEST_WIDTH - 20.0),
                    ..default()
                },
            ));
        });
}

fn text(parent: &mut ChildSpawnerCommands, s: impl Into<String>, size: f32, color: Color) {
    parent.spawn((
        Text::new(s),
        TextFont::from_font_size(size),
        TextColor(color),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_term_a_tell_leans_on_has_a_glossary_line() {
        let tells = [Tell::Streak, Tell::AllIn, Tell::Copycat, Tell::Flop];
        for tell in tells {
            let found: Vec<_> = terms(tell).collect();
            assert!(!found.is_empty(), "{} marks no terms", tell.name());
            for term in found {
                assert!(info::define(term).is_some(), "{term} has no line");
            }
        }
        assert!(terms(Tell::Streak).any(|t| t == "Tell"));
        assert!(terms(Tell::AllIn).any(|t| t == "The Hand"));
    }
}
