//! Resonantia's glue to the game: the launcher links this crate into the game and calls
//! `hooks()`. The rules live in `resonantia-core`; this file only reads the game, calls
//! them and tells the player (docs/native-mods.md, "Trait mods").
//!
//! - Right-click a block with the tuning fork: hear its tones, learn insight the first time.
//! - Right-click in the air with the tuning fork: your insight on the action bar.
//! - `/resonantia`: your knowledge.
//! - `/resonantia riddle <name>`: an attunement's riddle and its starting board.
//! - `/resonantia tune <name> <slot0> … <slot7>`: solve it with tone tokens (`-` for an
//!   empty slot), spending the tokens' insight. The codex screen will build the same board.
use horizon_client::mod_api::{
    AttachmentCodec, AttachmentError, AttachmentType, Compound, Flow, Holder, Hooks, Host,
    NativeCommand, PerformCommandEvent, RegisterCommandsEvent, Tag, Text, UseItemEvent,
    UseItemOnEvent,
};
use resonantia_core::{
    AttuneError, Board, Knowledge, Profile, Riddle, RiddleError, ScanResult, TONE_COUNT, Tone,
};

pub const TUNING_FORK: &str = "resonantia:tuning_fork";

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Resonantia)
}

struct Resonantia;

/// A player's knowledge, saved with the player and kept through death.
static KNOWLEDGE: AttachmentType<Knowledge> = AttachmentType {
    namespace: "resonantia",
    name: "knowledge",
    default: Knowledge::default,
    codec: Some(AttachmentCodec {
        encode: encode_knowledge,
        decode: decode_knowledge,
    }),
    copy_on_death: true,
    sync: false,
};

fn strings(set: &std::collections::BTreeSet<String>) -> Tag {
    Tag::List(set.iter().map(|s| Tag::String(s.as_str().into())).collect())
}

fn encode_knowledge(k: &Knowledge) -> Result<Compound, AttachmentError> {
    let insight = k
        .insight
        .iter()
        .map(|&n| n.min(i32::MAX as u32) as i32)
        .collect();
    Ok([
        ("insight".into(), Tag::IntArray(insight)),
        ("scanned".into(), strings(&k.scanned)),
        ("attuned".into(), strings(&k.attuned)),
    ]
    .into())
}

fn read_strings(
    data: &Compound,
    key: &str,
) -> Result<std::collections::BTreeSet<String>, AttachmentError> {
    match data.get(&key.into()) {
        None => Ok(Default::default()),
        Some(Tag::List(items)) => items
            .iter()
            .map(|t| match t {
                Tag::String(s) => s.to_string().map_err(|_| AttachmentError::Decode),
                _ => Err(AttachmentError::Decode),
            })
            .collect(),
        Some(_) => Err(AttachmentError::Decode),
    }
}

fn decode_knowledge(data: &Compound) -> Result<Knowledge, AttachmentError> {
    let mut k = Knowledge::default();
    match data.get(&"insight".into()) {
        Some(Tag::IntArray(v)) if v.len() == TONE_COUNT => {
            for (have, &n) in k.insight.iter_mut().zip(v) {
                *have = n.max(0) as u32;
            }
        }
        None => {}
        Some(_) => return Err(AttachmentError::Decode),
    }
    k.scanned = read_strings(data, "scanned")?;
    k.attuned = read_strings(data, "attuned")?;
    Ok(k)
}

fn tone_text(tone: Tone, amount: u32) -> Text {
    // "Calor 6", coloured as the tone; the name is translated on the player's client.
    Text::empty()
        .append(Text::translate(
            &format!("resonantia.tone.{}", tone.id()),
            &[],
        ))
        .append(Text::literal(format!(" {amount}")))
        .with_color(tone.color())
}

/// "Calor 6, Lux 3" for the non-zero amounts; "none" when all are zero.
fn amounts_text(amounts: &[u32; TONE_COUNT]) -> Text {
    let parts: Vec<Text> = Tone::ALL
        .into_iter()
        .filter(|t| amounts[t.index()] > 0)
        .map(|t| tone_text(t, amounts[t.index()]))
        .collect();
    join(parts)
}

fn profile_text(p: &Profile) -> Text {
    let mut parts: Vec<Text> = p
        .sounding()
        .into_iter()
        .map(|(t, n)| tone_text(t, n.into()))
        .collect();
    if p.dissonance > 0 {
        parts.push(Text::translate(
            "resonantia.scan.dissonance",
            &[Text::literal(p.dissonance.to_string())],
        ));
    }
    join(parts)
}

fn join(parts: Vec<Text>) -> Text {
    if parts.is_empty() {
        return Text::translate("resonantia.none", &[]);
    }
    let mut out = Text::empty();
    for (i, p) in parts.into_iter().enumerate() {
        if i > 0 {
            out = out.append(Text::literal(", "));
        }
        out = out.append(p);
    }
    out
}

fn attunement_name(id: &str) -> Text {
    Text::translate(&format!("resonantia.attunement.{id}"), &[])
}

/// What a scan of `id` tells the player, line by line.
pub fn scan_lines(result: &ScanResult, id: &str) -> Vec<Text> {
    let what = Text::literal(id.to_owned());
    match result {
        ScanResult::Silent => vec![Text::translate("resonantia.scan.silent", &[what])],
        ScanResult::Known(p) => {
            let mut v = vec![Text::translate(
                "resonantia.scan.known",
                &[what, profile_text(p)],
            )];
            v.extend(chords_line(p));
            v
        }
        ScanResult::Learned { profile, gained } => {
            let mut v = vec![Text::translate(
                "resonantia.scan.learned",
                &[what, profile_text(profile)],
            )];
            v.extend(chords_line(profile));
            v.push(Text::translate(
                "resonantia.scan.gained",
                &[amounts_text(gained)],
            ));
            v
        }
    }
}

fn chords_line(p: &Profile) -> Option<Text> {
    let chords = p.chords();
    if chords.is_empty() {
        return None;
    }
    let parts = chords
        .into_iter()
        .map(|(c, s)| {
            Text::empty()
                .append(Text::translate(
                    &format!("resonantia.chord.{}", c.id()),
                    &[],
                ))
                .append(Text::literal(format!(" {s}")))
        })
        .collect();
    Some(Text::translate("resonantia.chords", &[join(parts)]))
}

fn knowledge_lines(k: &Knowledge) -> Vec<Text> {
    let names = |ids: Vec<&str>| join(ids.into_iter().map(attunement_name).collect());
    vec![
        Text::translate("resonantia.insight", &[amounts_text(&k.insight)]),
        Text::translate(
            "resonantia.scanned",
            &[Text::literal(k.scanned.len().to_string())],
        ),
        Text::translate(
            "resonantia.attuned",
            &[names(k.attuned.iter().map(String::as_str).collect())],
        ),
        Text::translate(
            "resonantia.available",
            &[names(k.available().iter().map(|a| a.id).collect())],
        ),
    ]
}

fn tone_name(tone: Tone) -> Text {
    Text::translate(&format!("resonantia.tone.{}", tone.id()), &[]).with_color(tone.color())
}

fn chord_name(c: resonantia_core::Chord) -> Text {
    Text::translate(&format!("resonantia.chord.{}", c.id()), &[])
}

/// The board as eight short cells: `[Lux] [ - ] …`.
fn board_text(board: &Board) -> Text {
    let mut out = Text::empty();
    for (i, slot) in board.iter().enumerate() {
        if i > 0 {
            out = out.append(Text::literal(" "));
        }
        out = out.append(Text::literal(format!("{i}:")));
        out = match slot {
            Some(t) => out.append(tone_name(*t)),
            None => out.append(Text::literal("-")),
        };
    }
    out
}

fn riddle_lines(id: &str, r: &Riddle) -> Vec<Text> {
    vec![
        Text::translate(
            "resonantia.riddle.chords",
            &[
                attunement_name(id),
                join(r.chords.iter().map(|&c| chord_name(c)).collect()),
            ],
        ),
        Text::translate(
            "resonantia.riddle.dissonance",
            &[
                Text::literal(r.min_dissonance.to_string()),
                Text::literal(r.max_dissonance.to_string()),
            ],
        ),
        Text::translate(
            "resonantia.riddle.token_cost",
            &[Text::literal(r.token_cost.to_string())],
        ),
        Text::translate("resonantia.riddle.board", &[board_text(&r.start())]),
    ]
}

/// Parses eight slots: tone ids (`lux`) or `-`/`_` for empty.
pub fn parse_board<'a>(words: impl Iterator<Item = &'a str>) -> Option<Board> {
    let slots: Vec<&str> = words.collect();
    if slots.len() != TONE_COUNT {
        return None;
    }
    let mut board = [None; TONE_COUNT];
    for (cell, word) in board.iter_mut().zip(slots) {
        *cell = match word {
            "-" | "_" => None,
            w => Some(Tone::from_id(&w.to_ascii_lowercase())?),
        };
    }
    Some(board)
}

fn riddle_error_line(e: &RiddleError) -> Text {
    match e {
        RiddleError::AnchorMoved { slot, tone } => Text::translate(
            "resonantia.riddle.anchor",
            &[Text::literal(slot.to_string()), tone_name(*tone)],
        ),
        RiddleError::MissingChords(missing) => Text::translate(
            "resonantia.riddle.missing",
            &[join(missing.iter().map(|&c| chord_name(c)).collect())],
        ),
        RiddleError::TooHarmonious(d) => Text::translate(
            "resonantia.riddle.too_harmonious",
            &[Text::literal(d.to_string())],
        ),
        RiddleError::TooDissonant(d) => Text::translate(
            "resonantia.riddle.too_dissonant",
            &[Text::literal(d.to_string())],
        ),
    }
}

fn attune_error_line(id: &str, e: AttuneError) -> Text {
    match e {
        AttuneError::Unknown => {
            Text::translate("resonantia.attune.unknown", &[Text::literal(id.to_owned())])
        }
        AttuneError::AlreadyAttuned => {
            Text::translate("resonantia.attune.already", &[attunement_name(id)])
        }
        AttuneError::Requires(missing) => Text::translate(
            "resonantia.attune.requires",
            &[
                attunement_name(id),
                join(missing.into_iter().map(attunement_name).collect()),
            ],
        ),
        AttuneError::Riddle(e) => riddle_error_line(&e),
        AttuneError::Lacking(lacking) => Text::translate(
            "resonantia.attune.lacking",
            &[attunement_name(id), amounts_text(&lacking)],
        ),
    }
}

fn riddle_command(k: &Knowledge, id: &str) -> Vec<Text> {
    match k.may_attempt(id) {
        Ok(a) => riddle_lines(a.id, &a.riddle),
        Err(e) => vec![attune_error_line(id, e)],
    }
}

fn tune_command(k: &mut Knowledge, id: &str, board: &Board) -> Vec<Text> {
    match k.attune(id, board) {
        Ok(a) => vec![Text::translate(
            "resonantia.attune.done",
            &[attunement_name(a.id)],
        )],
        Err(e) => vec![attune_error_line(id, e)],
    }
}

fn with_knowledge<R>(
    host: &mut dyn Host,
    uuid: u128,
    f: impl FnOnce(&mut Knowledge) -> R,
) -> Result<R, AttachmentError> {
    let mut k = KNOWLEDGE.get(host, Holder::Player(uuid))?;
    let before = k.clone();
    let out = f(&mut k);
    if k != before {
        KNOWLEDGE.set(host, Holder::Player(uuid), k)?;
    }
    Ok(out)
}

impl Hooks for Resonantia {
    /// Tuning fork on a block: hear it.
    fn use_item_on(&self, host: &mut dyn Host, event: &mut UseItemOnEvent) -> Flow {
        if &*event.stack.item != TUNING_FORK {
            return Flow::Continue;
        }
        // Server side only: the client's own call has no acting player.
        let Some(player) = host.acting_player() else {
            return Flow::Continue;
        };
        let block = host.block(event.pos);
        // `minecraft:oak_log[axis=y]` → `minecraft:oak_log`.
        let id = block.split('[').next().unwrap_or(&block).to_owned();
        match with_knowledge(host, player.uuid, |k| k.scan(&id)) {
            Ok(result) => {
                for line in scan_lines(&result, &id) {
                    host.tell(player.uuid, line);
                }
            }
            Err(e) => eprintln!(
                "[resonantia] could not read {}'s knowledge: {e:?}",
                player.name
            ),
        }
        Flow::Cancel
    }

    /// Tuning fork in the air: show insight on the action bar.
    fn use_item(&self, host: &mut dyn Host, event: &mut UseItemEvent) -> Flow {
        if &*event.stack.item != TUNING_FORK {
            return Flow::Continue;
        }
        let Some(player) = host.acting_player() else {
            return Flow::Continue;
        };
        if let Ok(insight) = with_knowledge(host, player.uuid, |k| k.insight) {
            host.action_bar(
                player.uuid,
                Text::translate("resonantia.insight", &[amounts_text(&insight)]),
            );
        }
        Flow::Cancel
    }

    fn register_commands(&self, _host: &mut dyn Host, event: &mut RegisterCommandsEvent) {
        event.commands.push(NativeCommand::new("resonantia"));
        event
            .commands
            .push(NativeCommand::new("resonantia riddle <name:word>"));
        event.commands.push(NativeCommand::new(
            "resonantia tune <name:word> <slots:greedy>",
        ));
    }

    fn perform_command(&self, host: &mut dyn Host, event: &mut PerformCommandEvent) -> Flow {
        let mut words = event.command.split_whitespace();
        if words.next() != Some("resonantia") {
            return Flow::Continue;
        }
        let uuid = event.player.uuid;
        let lines = match (words.next(), words.next()) {
            (None, _) => with_knowledge(host, uuid, |k| knowledge_lines(k)),
            (Some("riddle"), Some(name)) => with_knowledge(host, uuid, |k| riddle_command(k, name)),
            (Some("tune"), Some(name)) => match parse_board(words) {
                Some(board) => with_knowledge(host, uuid, |k| tune_command(k, name, &board)),
                None => Ok(vec![Text::translate("resonantia.riddle.slots", &[])]),
            },
            _ => Ok(vec![Text::translate("resonantia.usage", &[])]),
        };
        match lines {
            Ok(lines) => {
                for line in lines {
                    host.feedback(line);
                }
            }
            Err(e) => eprintln!("[resonantia] could not read knowledge: {e:?}"),
        }
        Flow::Cancel
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use resonantia_core::attunement;

    #[test]
    fn knowledge_survives_its_codec() {
        let mut k = Knowledge::default();
        k.scan("minecraft:ice");
        k.scan("minecraft:torch");
        k.insight[Tone::Umbra.index()] = 40;
        k.attuned.insert("first_chord".into());
        let back = decode_knowledge(&encode_knowledge(&k).unwrap()).unwrap();
        assert_eq!(back, k);
    }

    #[test]
    fn boards_parse_from_eight_words() {
        let b = parse_board("lux FLUXUS vigor - - _ - -".split_whitespace()).unwrap();
        assert_eq!(b[0], Some(Tone::Lux));
        assert_eq!(b[1], Some(Tone::Fluxus));
        assert_eq!(b[3], None);
        assert_eq!(parse_board("lux fluxus".split_whitespace()), None);
        assert_eq!(parse_board("aqua - - - - - - -".split_whitespace()), None);
    }

    #[test]
    fn tuning_solves_the_first_riddle() {
        let mut k = Knowledge {
            insight: [4; TONE_COUNT],
            ..Default::default()
        };
        let b = parse_board("lux fluxus vigor - - - - -".split_whitespace()).unwrap();
        tune_command(&mut k, "first_chord", &b);
        assert!(k.attuned.contains("first_chord"));
        assert!(attunement("first_chord").is_some());
    }

    #[test]
    fn an_empty_compound_is_fresh_knowledge() {
        assert_eq!(
            decode_knowledge(&Compound::new()).unwrap(),
            Knowledge::default()
        );
    }
}
