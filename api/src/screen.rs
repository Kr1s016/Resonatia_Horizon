// The Resonance Codex screen, client side: drawing `resonantia_core::codex::Layout` with the
// game's GUI helpers and passing clicks to `Codex`. The rules and layout are tested in core;
// this file only paints and routes.

use std::sync::Mutex;

use horizon_client::mod_api::{Host, HudBatch, Text};
use resonantia_core::codex::{Codex, Hit, Layout, Rect, RowState, Snapshot, TuneRequest};
use resonantia_core::{ATTUNEMENTS, Pairing, TONE_COUNT, Tone, pairing};

pub const MOD_ID: &str = "resonantia";
/// Server → client: knowledge, and open the codex.
pub const OPEN_CHANNEL: &str = "resonantia:codex_open";
/// Server → client: knowledge changed, refresh an open codex.
pub const UPDATE_CHANNEL: &str = "resonantia:codex_update";
/// Client → server: a `TuneRequest`.
pub const TUNE_CHANNEL: &str = "resonantia:tune";

/// What the window's client keeps: the open codex and requests waiting for a tick to send.
struct Client {
    codex: Option<Codex>,
    outbox: Vec<TuneRequest>,
}

static CLIENT: Mutex<Client> = Mutex::new(Client {
    codex: None,
    outbox: Vec::new(),
});

fn client() -> std::sync::MutexGuard<'static, Client> {
    // A panic elsewhere while drawing must not lock the codex for good.
    CLIENT.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn is_open(host: &dyn Host) -> bool {
    host.mod_screen() == Some(MOD_ID)
}

/// The server sent knowledge to open the codex with.
pub fn open(host: &mut dyn Host, data: &[u8]) {
    let Some(snapshot) = Snapshot::from_bytes(data) else {
        eprintln!("[resonantia] the server sent a codex this client can't read");
        return;
    };
    let mut c = client();
    match &mut c.codex {
        // Reopening keeps the board the player left.
        Some(codex) => codex.update(snapshot),
        None => c.codex = Some(Codex::new(snapshot)),
    }
    drop(c);
    host.mod_screen_open(MOD_ID);
}

pub fn update(data: &[u8]) {
    if let (Some(snapshot), Some(codex)) = (Snapshot::from_bytes(data), client().codex.as_mut()) {
        codex.update(snapshot);
    }
}

/// A mouse press over the codex: true when it was ours.
pub fn click(host: &dyn Host, screen_rect: Rect, button: i32) -> bool {
    if !is_open(host) {
        return false;
    }
    let layout = Layout::centred(screen_rect);
    let mut c = client();
    let Some(codex) = c.codex.as_mut() else {
        return true;
    };
    if let Some(hit) = layout.hit(host.cursor())
        && let Some(request) = codex.click(hit, button)
    {
        c.outbox.push(request);
    }
    true
}

/// Sends what clicks queued; called from the client tick, where the host can send.
pub fn flush(host: &mut dyn Host) {
    let requests = std::mem::take(&mut client().outbox);
    for r in requests {
        if let Err(e) = host.net_send_to_server(TUNE_CHANNEL, &r.to_bytes()) {
            eprintln!("[resonantia] could not send a tune: {e}");
        }
    }
}

fn tr(key: &str) -> String {
    Text::translate(key, &[]).plain()
}

fn tr_args(key: &str, args: &[String]) -> String {
    let args: Vec<Text> = args.iter().cloned().map(Text::literal).collect();
    Text::translate(key, &args).plain()
}

fn tone_name(t: Tone) -> String {
    tr(&format!("resonantia.tone.{}", t.id()))
}

/// The first two letters of a tone's name, for its slot and palette button.
fn tone_short(t: Tone) -> String {
    tone_name(t).chars().take(2).collect()
}

const WHITE: u32 = 0xFFFFFF;
const GREY: u32 = 0x8A8A9A;
const GOOD: u32 = 0x7FDB6A;
const BAD: u32 = 0xE86A5A;
const GOLD: u32 = 0xE0C060;

fn argb(rgb: u32) -> u32 {
    0xFF00_0000 | rgb
}

fn outline(host: &dyn Host, batch: &mut HudBatch, r: Rect, rgb: u32) {
    let [x, y, w, h] = r;
    let c = argb(rgb);
    host.fill(batch, [x, y, w, 1.0], c);
    host.fill(batch, [x, y + h - 1.0, w, 1.0], c);
    host.fill(batch, [x, y, 1.0, h], c);
    host.fill(batch, [x + w - 1.0, y, 1.0, h], c);
}

fn centred_text(host: &dyn Host, batch: &mut HudBatch, r: Rect, s: &str, rgb: u32) {
    let w = host.text_width(s);
    let x = (r[0] + (r[2] - w) / 2.0).round();
    let y = (r[1] + (r[3] - 8.0) / 2.0).round();
    host.text(batch, s, [x, y], rgb, false);
}

fn centre(r: Rect) -> [f32; 2] {
    [r[0] + r[2] / 2.0, r[1] + r[3] / 2.0]
}

/// A line `width` thick between two points, as a quad.
fn line(host: &dyn Host, batch: &mut HudBatch, a: [f32; 2], b: [f32; 2], width: f32, argb: u32) {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = (dx * dx + dy * dy).sqrt().max(0.001);
    let (nx, ny) = (-dy / len * width / 2.0, dx / len * width / 2.0);
    host.quad(
        batch,
        [
            [a[0] + nx, a[1] + ny],
            [b[0] + nx, b[1] + ny],
            [b[0] - nx, b[1] - ny],
            [a[0] - nx, a[1] - ny],
        ],
        argb,
    );
}

fn button(host: &dyn Host, batch: &mut HudBatch, r: Rect, label: &str, hovered: bool) {
    host.fill(batch, r, if hovered { 0xFF4E4E6E } else { 0xFF34344A });
    outline(host, batch, r, 0x6B5B95);
    centred_text(host, batch, r, label, WHITE);
}

pub fn draw(host: &dyn Host, batch: &mut HudBatch, screen_rect: Rect) {
    if !is_open(host) {
        return;
    }
    let c = client();
    let Some(codex) = c.codex.as_ref() else {
        return;
    };
    let l = Layout::centred(screen_rect);
    let mouse = host.cursor();
    let hovered = l.hit(mouse);

    // Panel.
    let p = l.panel();
    host.fill(batch, p, 0xF0141420);
    outline(host, batch, p, 0x6B5B95);
    host.text(
        batch,
        &tr("resonantia.codex.title"),
        [p[0] + 8.0, p[1] + 6.0],
        GOLD,
        true,
    );
    let heard = tr_args(
        "resonantia.codex.heard",
        &[codex.snapshot.scanned.to_string()],
    );
    let w = host.text_width(&heard);
    host.text(
        batch,
        &heard,
        [p[0] + p[2] - 8.0 - w, p[1] + 6.0],
        GREY,
        false,
    );

    // Attunement list.
    for (i, a) in ATTUNEMENTS.iter().enumerate() {
        let r = l.attunement_row(i);
        if codex.selected == Some(i) {
            host.fill(batch, r, 0x40FFFFFF);
        } else if hovered == Some(Hit::Attunement(i)) {
            host.fill(batch, r, 0x20FFFFFF);
        }
        let (mark, color) = match codex.row_state(i) {
            RowState::Attuned => ("+ ", GOOD),
            RowState::Available => ("  ", WHITE),
            RowState::Locked => ("  ", 0x5A5A68),
        };
        let name = format!("{mark}{}", tr(&format!("resonantia.attunement.{}", a.id)));
        host.text(batch, &name, [r[0] + 2.0, r[1] + 2.0], color, false);
    }

    // The selected riddle: chords with progress, dissonance.
    let [tx, mut ty] = l.riddle_text();
    match codex.selected {
        None => host.text(batch, &tr("resonantia.codex.pick"), [tx, ty], GREY, false),
        Some(i) => {
            host.text(batch, &tr("resonantia.codex.chords"), [tx, ty], GREY, false);
            ty += 10.0;
            for (chord, sounding) in codex.chord_progress() {
                let name = tr(&format!("resonantia.chord.{}", chord.id()));
                let (mark, color) = if sounding { ("+ ", GOOD) } else { ("- ", GREY) };
                host.text(batch, &format!("{mark}{name}"), [tx, ty], color, false);
                ty += 10.0;
            }
            if let Some((d, ok)) = codex.dissonance() {
                let r = &ATTUNEMENTS[i].riddle;
                let s = tr_args(
                    "resonantia.codex.dissonance",
                    &[
                        d.to_string(),
                        r.min_dissonance.to_string(),
                        r.max_dissonance.to_string(),
                    ],
                );
                host.text(
                    batch,
                    &s,
                    [tx, ty + 2.0],
                    if ok { GOOD } else { BAD },
                    false,
                );
            }
        }
    }

    // The ring: links first, slots over them.
    let anchors: &[(usize, Tone)] = codex
        .selected
        .map_or(&[], |i| ATTUNEMENTS[i].riddle.anchors);
    for i in 0..TONE_COUNT {
        let j = (i + 1) % TONE_COUNT;
        let (a, b) = (centre(l.slot(i)), centre(l.slot(j)));
        let color = match (codex.board[i], codex.board[j]) {
            (Some(x), Some(y)) => match pairing(x, y) {
                Pairing::Chord(_) => argb(GOOD),
                Pairing::Harsh => argb(0xE8A04A),
                Pairing::Clash => argb(BAD),
                Pairing::Unison | Pairing::Neutral => 0xFF50506A,
            },
            _ => 0xFF2E2E3E,
        };
        let width = if color == argb(GOOD) || color == argb(BAD) {
            3.0
        } else {
            1.0
        };
        line(host, batch, a, b, width, color);
    }
    for i in 0..TONE_COUNT {
        let r = l.slot(i);
        match codex.board[i] {
            Some(t) => {
                host.fill(batch, r, argb(t.color()));
                centred_text(host, batch, r, &tone_short(t), 0x101010);
            }
            None => host.fill(batch, r, 0xFF22222E),
        }
        let edge = if anchors.iter().any(|&(s, _)| s == i) {
            GOLD
        } else if hovered == Some(Hit::Slot(i)) {
            WHITE
        } else {
            0x50506A
        };
        outline(host, batch, r, edge);
    }

    // Palette and buttons.
    for t in Tone::ALL {
        let r = l.palette(t);
        host.fill(batch, r, argb(t.color()));
        centred_text(host, batch, r, &tone_short(t), 0x101010);
        if codex.brush == Some(t) {
            outline(host, batch, r, WHITE);
        }
    }
    button(
        host,
        batch,
        l.clear_button(),
        &tr("resonantia.codex.clear"),
        hovered == Some(Hit::Clear),
    );
    button(
        host,
        batch,
        l.tune_button(),
        &tr("resonantia.codex.tune"),
        hovered == Some(Hit::Tune),
    );
    if let Some(key) = codex.status {
        let [sx, sy] = l.status();
        let good = key == "resonantia.codex.solved" || key == "resonantia.codex.sent";
        // The status can be longer than the column: wrap on spaces.
        let mut line_y = sy;
        let mut current = String::new();
        for word in tr(key).split(' ') {
            let next = if current.is_empty() {
                word.to_owned()
            } else {
                format!("{current} {word}")
            };
            if host.text_width(&next) > 76.0 && !current.is_empty() {
                host.text(
                    batch,
                    &current,
                    [sx, line_y],
                    if good { GOOD } else { BAD },
                    false,
                );
                line_y += 10.0;
                current = word.to_owned();
            } else {
                current = next;
            }
        }
        host.text(
            batch,
            &current,
            [sx, line_y],
            if good { GOOD } else { BAD },
            false,
        );
    }

    // Insight along the bottom, with what the board would cost.
    let cost = codex.cost();
    for t in Tone::ALL {
        let r = l.insight(t);
        host.fill(batch, [r[0], r[1] + 2.0, 6.0, 6.0], argb(t.color()));
        let have = codex.snapshot.insight[t.index()];
        let (s, color) = match cost.map(|c| c[t.index()]) {
            Some((n, ok)) if n > 0 => (format!("{have}-{n}"), if ok { WHITE } else { BAD }),
            _ => (have.to_string(), GREY),
        };
        host.text(batch, &s, [r[0] + 8.0, r[1] + 1.0], color, false);
    }

    // Tooltip: the full name of the hovered tone.
    if let Some(Hit::Palette(t)) = hovered {
        let name = tone_name(t);
        let w = host.text_width(&name) + 6.0;
        let r = [mouse[0] + 8.0, mouse[1] - 12.0, w, 11.0];
        host.fill(batch, r, 0xF0000000);
        host.text(
            batch,
            &name,
            [r[0] + 3.0, r[1] + 2.0],
            argb(t.color()) & 0xFFFFFF,
            false,
        );
    }
}
