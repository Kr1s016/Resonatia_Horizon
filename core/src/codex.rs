// The Resonance Codex screen, without drawing: its layout, what a click hits, what the
// screen remembers, and the bytes its client and server halves exchange. The game glue
// draws `Layout`'s rectangles and feeds clicks to `Codex::click`.

use crate::{ATTUNEMENTS, Board, Knowledge, TONE_COUNT, Tone, attunement, read_board};

/// The panel's size in GUI pixels.
pub const PANEL: [f32; 2] = [300.0, 196.0];
/// A ring slot's side.
pub const SLOT: f32 = 20.0;
/// The ring's radius, centre to slot centre.
pub const RING_RADIUS: f32 = 54.0;
/// An attunement row's height in the list.
pub const ROW: f32 = 12.0;

/// `[x, y, width, height]` in GUI pixels.
pub type Rect = [f32; 4];

pub fn contains(r: Rect, p: [f32; 2]) -> bool {
    p[0] >= r[0] && p[0] < r[0] + r[2] && p[1] >= r[1] && p[1] < r[1] + r[3]
}

/// Where everything is, for a panel whose top left corner is `origin`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    pub origin: [f32; 2],
}

impl Layout {
    /// The panel centred in `screen` (`[x, y, w, h]`).
    pub fn centred(screen: Rect) -> Layout {
        Layout {
            origin: [
                (screen[0] + (screen[2] - PANEL[0]) / 2.0).floor(),
                (screen[1] + (screen[3] - PANEL[1]) / 2.0).floor(),
            ],
        }
    }

    fn at(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        [self.origin[0] + x, self.origin[1] + y, w, h]
    }

    pub fn panel(&self) -> Rect {
        self.at(0.0, 0.0, PANEL[0], PANEL[1])
    }

    /// Row `i` of the attunement list, on the left.
    pub fn attunement_row(&self, i: usize) -> Rect {
        self.at(8.0, 20.0 + i as f32 * ROW, 88.0, ROW)
    }

    /// Where the riddle's description starts, under the list.
    pub fn riddle_text(&self) -> [f32; 2] {
        let r = self.at(8.0, 96.0, 0.0, 0.0);
        [r[0], r[1]]
    }

    pub fn ring_centre(&self) -> [f32; 2] {
        let r = self.at(152.0, 94.0, 0.0, 0.0);
        [r[0], r[1]]
    }

    /// Ring slot `i`: slot 0 at the top, then clockwise, 45° apart.
    pub fn slot(&self, i: usize) -> Rect {
        let [cx, cy] = self.ring_centre();
        let angle = (i as f32 * 45.0 - 90.0).to_radians();
        let x = cx + RING_RADIUS * angle.cos();
        let y = cy + RING_RADIUS * angle.sin();
        [
            (x - SLOT / 2.0).round(),
            (y - SLOT / 2.0).round(),
            SLOT,
            SLOT,
        ]
    }

    /// A tone's button in the palette, two columns of four on the right.
    pub fn palette(&self, tone: Tone) -> Rect {
        let i = tone.index();
        let (col, row) = (i % 2, i / 2);
        self.at(
            218.0 + col as f32 * 38.0,
            20.0 + row as f32 * 18.0,
            36.0,
            15.0,
        )
    }

    pub fn clear_button(&self) -> Rect {
        self.at(218.0, 98.0, 74.0, 15.0)
    }

    pub fn tune_button(&self) -> Rect {
        self.at(218.0, 116.0, 74.0, 15.0)
    }

    /// Where the status line (the last result) is drawn.
    pub fn status(&self) -> [f32; 2] {
        let r = self.at(218.0, 138.0, 0.0, 0.0);
        [r[0], r[1]]
    }

    /// Insight for `tone`, along the bottom.
    pub fn insight(&self, tone: Tone) -> Rect {
        self.at(8.0 + tone.index() as f32 * 36.0, 178.0, 34.0, 12.0)
    }

    pub fn hit(&self, p: [f32; 2]) -> Option<Hit> {
        if let Some(i) = (0..ATTUNEMENTS.len()).find(|&i| contains(self.attunement_row(i), p)) {
            return Some(Hit::Attunement(i));
        }
        if let Some(i) = (0..TONE_COUNT).find(|&i| contains(self.slot(i), p)) {
            return Some(Hit::Slot(i));
        }
        if let Some(t) = Tone::ALL
            .into_iter()
            .find(|&t| contains(self.palette(t), p))
        {
            return Some(Hit::Palette(t));
        }
        if contains(self.clear_button(), p) {
            return Some(Hit::Clear);
        }
        if contains(self.tune_button(), p) {
            return Some(Hit::Tune);
        }
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Attunement(usize),
    Slot(usize),
    Palette(Tone),
    Clear,
    Tune,
}

/// Mouse buttons as the game numbers them.
pub const LEFT: i32 = 0;
pub const RIGHT: i32 = 1;

/// What the server tells the codex: the player's insight, attunements and how many
/// things they have heard. Enough for the client to show and pre-check riddles.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub insight: [u32; TONE_COUNT],
    pub attuned: Vec<String>,
    pub scanned: u32,
}

impl Snapshot {
    pub fn of(k: &Knowledge) -> Snapshot {
        Snapshot {
            insight: k.insight,
            attuned: k.attuned.iter().cloned().collect(),
            scanned: k.scanned.len() as u32,
        }
    }

    /// The knowledge it stands for (without the scanned ids, which the client never needs).
    pub fn knowledge(&self) -> Knowledge {
        Knowledge {
            insight: self.insight,
            scanned: Default::default(),
            attuned: self.attuned.iter().cloned().collect(),
        }
    }

    const VERSION: u8 = 1;

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = vec![Self::VERSION];
        for n in self.insight {
            out.extend_from_slice(&n.to_le_bytes());
        }
        out.extend_from_slice(&self.scanned.to_le_bytes());
        // Only known attunement ids travel, so each is short.
        let ids: Vec<&String> = self
            .attuned
            .iter()
            .filter(|id| id.len() <= u8::MAX as usize)
            .collect();
        out.push(ids.len().min(u8::MAX as usize) as u8);
        for id in ids.into_iter().take(u8::MAX as usize) {
            out.push(id.len() as u8);
            out.extend_from_slice(id.as_bytes());
        }
        out
    }

    pub fn from_bytes(data: &[u8]) -> Option<Snapshot> {
        let mut r = Reader(data);
        if r.u8()? != Self::VERSION {
            return None;
        }
        let mut insight = [0u32; TONE_COUNT];
        for n in &mut insight {
            *n = r.u32()?;
        }
        let scanned = r.u32()?;
        let count = r.u8()?;
        let mut attuned = Vec::with_capacity(count as usize);
        for _ in 0..count {
            attuned.push(r.string()?);
        }
        r.0.is_empty().then_some(Snapshot {
            insight,
            attuned,
            scanned,
        })
    }
}

struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        if self.0.len() < n {
            return None;
        }
        let (head, rest) = self.0.split_at(n);
        self.0 = rest;
        Some(head)
    }
    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }
    fn u32(&mut self) -> Option<u32> {
        self.take(4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn string(&mut self) -> Option<String> {
        let n = self.u8()? as usize;
        String::from_utf8(self.take(n)?.to_vec()).ok()
    }
}

/// The client asks the server to solve `attunement` with `board`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TuneRequest {
    pub attunement: String,
    pub board: Board,
}

impl TuneRequest {
    /// `[id length][id][8 slots: 0 empty, 1..=8 tone index + 1]`.
    pub fn to_bytes(&self) -> Vec<u8> {
        let id = self.attunement.as_bytes();
        let mut out = vec![id.len().min(u8::MAX as usize) as u8];
        out.extend_from_slice(&id[..id.len().min(u8::MAX as usize)]);
        out.extend(
            self.board
                .iter()
                .map(|s| s.map_or(0, |t| t.index() as u8 + 1)),
        );
        out
    }

    /// Checked: a client's bytes are a player's own input.
    pub fn from_bytes(data: &[u8]) -> Option<TuneRequest> {
        let mut r = Reader(data);
        let attunement = r.string()?;
        let slots = r.take(TONE_COUNT)?;
        let mut board = [None; TONE_COUNT];
        for (cell, &b) in board.iter_mut().zip(slots) {
            *cell = match b {
                0 => None,
                1..=8 => Some(Tone::from_index(b as usize - 1)),
                _ => return None,
            };
        }
        r.0.is_empty().then_some(TuneRequest { attunement, board })
    }
}

/// What the codex screen remembers between frames.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Codex {
    pub snapshot: Snapshot,
    /// Index into `ATTUNEMENTS`.
    pub selected: Option<usize>,
    pub board: Board,
    /// The tone a left click places.
    pub brush: Option<Tone>,
    /// A translation key for the last thing that happened, shown under the buttons.
    pub status: Option<&'static str>,
}

/// How an attunement row is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowState {
    Attuned,
    Available,
    Locked,
}

impl Codex {
    pub fn new(snapshot: Snapshot) -> Codex {
        let mut c = Codex {
            snapshot,
            brush: Some(Tone::Calor),
            ..Default::default()
        };
        // Start on the first riddle the player can try.
        let k = c.snapshot.knowledge();
        c.select(ATTUNEMENTS.iter().position(|a| k.may_attempt(a.id).is_ok()));
        c
    }

    /// New knowledge from the server: keep the board unless the riddle was just solved.
    pub fn update(&mut self, snapshot: Snapshot) {
        let solved_now = self.selected.is_some_and(|i| {
            let id = ATTUNEMENTS[i].id;
            snapshot.attuned.iter().any(|a| a == id)
                && !self.snapshot.attuned.iter().any(|a| a == id)
        });
        self.snapshot = snapshot;
        if solved_now {
            self.status = Some("resonantia.codex.solved");
        }
    }

    pub fn row_state(&self, i: usize) -> RowState {
        let a = &ATTUNEMENTS[i];
        let k = self.snapshot.knowledge();
        if k.attuned.contains(a.id) {
            RowState::Attuned
        } else if k.may_attempt(a.id).is_ok() {
            RowState::Available
        } else {
            RowState::Locked
        }
    }

    fn select(&mut self, i: Option<usize>) {
        self.selected = i;
        self.board = i.map_or([None; TONE_COUNT], |i| ATTUNEMENTS[i].riddle.start());
        self.status = None;
    }

    fn is_anchor(&self, slot: usize) -> bool {
        self.selected.is_some_and(|i| {
            ATTUNEMENTS[i]
                .riddle
                .anchors
                .iter()
                .any(|&(s, _)| s == slot)
        })
    }

    /// A click on `hit` with mouse `button`. A tune click returns the request to send.
    pub fn click(&mut self, hit: Hit, button: i32) -> Option<TuneRequest> {
        match hit {
            Hit::Attunement(i) => {
                if self.row_state(i) != RowState::Locked {
                    self.select(Some(i));
                } else {
                    self.status = Some("resonantia.codex.locked");
                }
            }
            Hit::Palette(t) => self.brush = Some(t),
            Hit::Slot(s) => {
                if self.selected.is_none() || self.is_anchor(s) {
                    return None;
                }
                self.status = None;
                self.board[s] = match (button, self.board[s], self.brush) {
                    (RIGHT, _, _) => None,
                    // Clicking a slot with the brush's own tone takes it out again.
                    (_, Some(have), Some(b)) if have == b => None,
                    (_, _, brush) => brush,
                };
            }
            Hit::Clear => {
                if let Some(i) = self.selected {
                    self.board = ATTUNEMENTS[i].riddle.start();
                    self.status = None;
                }
            }
            Hit::Tune => {
                let i = self.selected?;
                let id = ATTUNEMENTS[i].id;
                if self.row_state(i) == RowState::Attuned {
                    self.status = Some("resonantia.codex.already");
                    return None;
                }
                // Check here first so a wrong board shows why at once; the server decides.
                if let Err(e) = self.snapshot.knowledge().can_attune(id, &self.board) {
                    self.status = Some(status_key(&e));
                    return None;
                }
                self.status = Some("resonantia.codex.sent");
                return Some(TuneRequest {
                    attunement: id.to_owned(),
                    board: self.board,
                });
            }
        }
        None
    }

    /// The selected riddle's chords, each with whether it sounds on the board now.
    pub fn chord_progress(&self) -> Vec<(crate::Chord, bool)> {
        let Some(i) = self.selected else {
            return vec![];
        };
        let reading = read_board(&self.board);
        ATTUNEMENTS[i]
            .riddle
            .chords
            .iter()
            .map(|&c| (c, reading.chords.contains(&c)))
            .collect()
    }

    /// The board's dissonance and whether it is inside the riddle's range.
    pub fn dissonance(&self) -> Option<(u32, bool)> {
        let r = &ATTUNEMENTS[self.selected?].riddle;
        let d = read_board(&self.board).dissonance;
        Some((d, (r.min_dissonance..=r.max_dissonance).contains(&d)))
    }

    /// What the board costs and whether the player can pay each tone.
    pub fn cost(&self) -> Option<[(u32, bool); TONE_COUNT]> {
        let r = &ATTUNEMENTS[self.selected?].riddle;
        let cost = r.cost(&self.board);
        Some(std::array::from_fn(|i| {
            (cost[i], cost[i] <= self.snapshot.insight[i])
        }))
    }
}

/// The translation key that explains why `e` stops a tune.
pub fn status_key(e: &crate::AttuneError) -> &'static str {
    use crate::{AttuneError, RiddleError};
    match e {
        AttuneError::Unknown => "resonantia.codex.unknown",
        AttuneError::AlreadyAttuned => "resonantia.codex.already",
        AttuneError::Requires(_) => "resonantia.codex.locked",
        AttuneError::Lacking(_) => "resonantia.codex.lacking",
        AttuneError::Riddle(RiddleError::AnchorMoved { .. }) => "resonantia.codex.anchor",
        AttuneError::Riddle(RiddleError::MissingChords(_)) => "resonantia.codex.missing",
        AttuneError::Riddle(RiddleError::TooHarmonious(_)) => "resonantia.codex.too_harmonious",
        AttuneError::Riddle(RiddleError::TooDissonant(_)) => "resonantia.codex.too_dissonant",
    }
}

/// Whether `id` names an attunement, for the server to check before it answers.
pub fn known(id: &str) -> bool {
    attunement(id).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use Tone::*;

    fn layout() -> Layout {
        Layout::centred([0.0, 0.0, 400.0, 260.0])
    }

    fn centre(r: Rect) -> [f32; 2] {
        [r[0] + r[2] / 2.0, r[1] + r[3] / 2.0]
    }

    #[test]
    fn the_panel_is_centred_and_everything_fits_inside_it() {
        let l = layout();
        assert_eq!(l.origin, [50.0, 32.0]);
        let p = l.panel();
        let inside = |r: Rect| {
            r[0] >= p[0] && r[1] >= p[1] && r[0] + r[2] <= p[0] + p[2] && r[1] + r[3] <= p[1] + p[3]
        };
        let mut rects: Vec<Rect> = (0..TONE_COUNT).map(|i| l.slot(i)).collect();
        rects.extend((0..ATTUNEMENTS.len()).map(|i| l.attunement_row(i)));
        rects.extend(Tone::ALL.map(|t| l.palette(t)));
        rects.extend(Tone::ALL.map(|t| l.insight(t)));
        rects.push(l.clear_button());
        rects.push(l.tune_button());
        for r in &rects {
            assert!(inside(*r), "{r:?} outside {p:?}");
        }
    }

    #[test]
    fn nothing_overlaps_so_every_click_has_one_meaning() {
        let l = layout();
        let mut hits = vec![];
        for i in 0..TONE_COUNT {
            hits.push((l.slot(i), Hit::Slot(i)));
        }
        for i in 0..ATTUNEMENTS.len() {
            hits.push((l.attunement_row(i), Hit::Attunement(i)));
        }
        for t in Tone::ALL {
            hits.push((l.palette(t), Hit::Palette(t)));
        }
        hits.push((l.clear_button(), Hit::Clear));
        hits.push((l.tune_button(), Hit::Tune));
        for (r, want) in &hits {
            assert_eq!(l.hit(centre(*r)), Some(*want));
        }
        assert_eq!(l.hit(l.ring_centre()), None);
    }

    #[test]
    fn slot_zero_is_on_top_and_the_ring_goes_clockwise() {
        let l = layout();
        let [cx, cy] = l.ring_centre();
        let c0 = centre(l.slot(0));
        let c2 = centre(l.slot(2));
        assert!((c0[0] - cx).abs() < 1.0 && c0[1] < cy);
        assert!(c2[0] > cx && (c2[1] - cy).abs() < 1.0);
    }

    #[test]
    fn snapshots_and_requests_survive_their_bytes() {
        let s = Snapshot {
            insight: [1, 2, 3, 400_000, 0, 6, 7, 8],
            attuned: vec!["first_chord".into(), "tone_jar".into()],
            scanned: 42,
        };
        assert_eq!(Snapshot::from_bytes(&s.to_bytes()), Some(s.clone()));
        let mut bytes = s.to_bytes();
        bytes.push(0);
        assert_eq!(Snapshot::from_bytes(&bytes), None, "trailing bytes");
        assert_eq!(Snapshot::from_bytes(&[]), None);

        let t = TuneRequest {
            attunement: "first_chord".into(),
            board: [
                Some(Lux),
                Some(Fluxus),
                Some(Vigor),
                None,
                None,
                None,
                None,
                Some(Tabes),
            ],
        };
        assert_eq!(TuneRequest::from_bytes(&t.to_bytes()), Some(t.clone()));
        let mut bad = t.to_bytes();
        *bad.last_mut().unwrap() = 9;
        assert_eq!(TuneRequest::from_bytes(&bad), None, "tone out of range");
        assert_eq!(
            TuneRequest::from_bytes(&bad[..bad.len() - 1]),
            None,
            "short"
        );
    }

    fn rich() -> Snapshot {
        Snapshot {
            insight: [20; TONE_COUNT],
            ..Default::default()
        }
    }

    #[test]
    fn a_new_codex_opens_on_the_first_riddle_it_can_try() {
        let c = Codex::new(rich());
        assert_eq!(c.selected, Some(0));
        assert_eq!(c.row_state(0), RowState::Available);
        assert_eq!(c.row_state(1), RowState::Locked);
        let mut s = rich();
        s.attuned.push("first_chord".into());
        let c = Codex::new(s);
        assert_eq!(c.selected, Some(1));
        assert_eq!(c.row_state(0), RowState::Attuned);
    }

    #[test]
    fn clicks_paint_the_board_and_tune_sends_a_solution() {
        let mut c = Codex::new(rich());
        // A wrong board is stopped on the client, with the reason.
        assert_eq!(c.click(Hit::Tune, LEFT), None);
        assert_eq!(c.status, Some("resonantia.codex.missing"));
        for (slot, tone) in [(0, Lux), (1, Fluxus), (2, Vigor)] {
            c.click(Hit::Palette(tone), LEFT);
            c.click(Hit::Slot(slot), LEFT);
        }
        assert_eq!(c.chord_progress().iter().filter(|p| p.1).count(), 2);
        assert_eq!(c.dissonance(), Some((0, true)));
        let sent = c.click(Hit::Tune, LEFT).expect("solved");
        assert_eq!(sent.attunement, "first_chord");
        assert_eq!(sent.board[1], Some(Fluxus));
        // Right click empties a slot; the brush's own tone toggles it.
        c.click(Hit::Slot(0), RIGHT);
        assert_eq!(c.board[0], None);
        c.click(Hit::Slot(2), LEFT);
        assert_eq!(c.board[2], None, "brush is Vigor, slot held Vigor");
    }

    #[test]
    fn anchors_cannot_be_painted_and_clear_restores_them() {
        let mut s = rich();
        s.attuned.push("first_chord".into());
        let mut c = Codex::new(s);
        let lens = ATTUNEMENTS
            .iter()
            .position(|a| a.id == "resonant_lens")
            .unwrap();
        c.click(Hit::Attunement(lens), LEFT);
        c.click(Hit::Palette(Gelu), LEFT);
        c.click(Hit::Slot(0), LEFT);
        assert_eq!(c.board[0], Some(Lux), "anchor stays");
        c.click(Hit::Slot(3), LEFT);
        assert_eq!(c.board[3], Some(Gelu));
        c.click(Hit::Clear, LEFT);
        assert_eq!(c.board, ATTUNEMENTS[lens].riddle.start());
    }

    #[test]
    fn locked_rows_cannot_be_selected() {
        let mut c = Codex::new(rich());
        let altar = ATTUNEMENTS.len() - 1;
        c.click(Hit::Attunement(altar), LEFT);
        assert_eq!(c.selected, Some(0));
        assert_eq!(c.status, Some("resonantia.codex.locked"));
    }

    #[test]
    fn an_update_that_attunes_the_selected_riddle_says_so() {
        let mut c = Codex::new(rich());
        let mut s = rich();
        s.attuned.push("first_chord".into());
        c.update(s);
        assert_eq!(c.status, Some("resonantia.codex.solved"));
        assert_eq!(c.row_state(0), RowState::Attuned);
    }

    #[test]
    fn costs_show_what_the_player_cannot_pay() {
        let mut c = Codex::new(Snapshot {
            insight: [0, 2, 1, 2, 0, 0, 0, 0],
            ..Default::default()
        });
        for (slot, tone) in [(0, Lux), (1, Fluxus), (2, Vigor)] {
            c.click(Hit::Palette(tone), LEFT);
            c.click(Hit::Slot(slot), LEFT);
        }
        let cost = c.cost().unwrap();
        assert_eq!(cost[Lux.index()], (2, true));
        assert_eq!(cost[Fluxus.index()], (2, false));
        assert_eq!(c.click(Hit::Tune, LEFT), None);
        assert_eq!(c.status, Some("resonantia.codex.lacking"));
    }
}
