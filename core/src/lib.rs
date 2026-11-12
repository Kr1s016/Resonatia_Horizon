// Resonantia's rules, free of game types so they build and test on their own.
//
// Everything in the world hums with eight tones laid out on a wheel. A tone and the one
// four steps away are opposites: where both sound, they cancel into dissonance. Two
// neighbours sounding together form a chord. Scanning a block or item with the tuning
// fork reads its profile and teaches the player insight in each tone it carries;
// insight is spent on attunements, which unlock the mod's devices and rituals.

use std::collections::BTreeSet;

/// The eight tones, in wheel order. `Tone::opposite` is four steps away.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tone {
    /// Heat, fire, smelting.
    Calor,
    /// Light, glow, clarity.
    Lux,
    /// Flow: water, wind, motion, change.
    Fluxus,
    /// Living force: growth, plants, creatures.
    Vigor,
    /// Cold, ice, preservation.
    Gelu,
    /// Shadow, the deep, the hidden.
    Umbra,
    /// Weight: stone, metal, stillness.
    Pondus,
    /// Decay: rot, bone, ending.
    Tabes,
}

pub const TONE_COUNT: usize = 8;

impl Tone {
    pub const ALL: [Tone; TONE_COUNT] = [
        Tone::Calor,
        Tone::Lux,
        Tone::Fluxus,
        Tone::Vigor,
        Tone::Gelu,
        Tone::Umbra,
        Tone::Pondus,
        Tone::Tabes,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_index(i: usize) -> Tone {
        Tone::ALL[i % TONE_COUNT]
    }

    pub fn opposite(self) -> Tone {
        Tone::from_index(self.index() + TONE_COUNT / 2)
    }

    /// The next tone clockwise.
    pub fn next(self) -> Tone {
        Tone::from_index(self.index() + 1)
    }

    /// Steps between two tones the short way round the wheel (0..=4).
    pub fn distance(self, other: Tone) -> usize {
        let d = self.index().abs_diff(other.index());
        d.min(TONE_COUNT - d)
    }

    /// Lower-case id, used in translation keys and commands.
    pub fn id(self) -> &'static str {
        match self {
            Tone::Calor => "calor",
            Tone::Lux => "lux",
            Tone::Fluxus => "fluxus",
            Tone::Vigor => "vigor",
            Tone::Gelu => "gelu",
            Tone::Umbra => "umbra",
            Tone::Pondus => "pondus",
            Tone::Tabes => "tabes",
        }
    }

    pub fn from_id(id: &str) -> Option<Tone> {
        Tone::ALL.into_iter().find(|t| t.id() == id)
    }

    /// Display colour, 0xRRGGBB.
    pub fn color(self) -> u32 {
        match self {
            Tone::Calor => 0xE8572A,
            Tone::Lux => 0xF6E27A,
            Tone::Fluxus => 0x5FC7E8,
            Tone::Vigor => 0x6CCB4F,
            Tone::Gelu => 0xB8E6F2,
            Tone::Umbra => 0x5B3F8C,
            Tone::Pondus => 0x8C8577,
            Tone::Tabes => 0x7A8C3F,
        }
    }
}

/// A chord: two neighbouring tones sounding together. There is one per pair of
/// neighbours, named by its lower tone (`Chord::of(Calor, Lux)` is `Ardor`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Chord {
    /// Calor + Lux: blaze, radiance that burns.
    Ardor,
    /// Lux + Fluxus: a ray, energy on the move.
    Radius,
    /// Fluxus + Vigor: sap, the flow of growth.
    Sucus,
    /// Vigor + Gelu: dormancy, life held in waiting.
    Somnus,
    /// Gelu + Umbra: the abyss, cold depths.
    Abyssus,
    /// Umbra + Pondus: the core, dense hidden ore.
    Nucleus,
    /// Pondus + Tabes: ash and ruin.
    Cinis,
    /// Tabes + Calor: fury, the burning end.
    Furor,
}

impl Chord {
    pub const ALL: [Chord; TONE_COUNT] = [
        Chord::Ardor,
        Chord::Radius,
        Chord::Sucus,
        Chord::Somnus,
        Chord::Abyssus,
        Chord::Nucleus,
        Chord::Cinis,
        Chord::Furor,
    ];

    /// The chord of two neighbouring tones, in either order; None for non-neighbours.
    pub fn of(a: Tone, b: Tone) -> Option<Chord> {
        if a.next() == b {
            Some(Chord::ALL[a.index()])
        } else if b.next() == a {
            Some(Chord::ALL[b.index()])
        } else {
            None
        }
    }

    /// Its two tones, clockwise.
    pub fn tones(self) -> (Tone, Tone) {
        let low = Tone::from_index(self as usize);
        (low, low.next())
    }

    pub fn id(self) -> &'static str {
        match self {
            Chord::Ardor => "ardor",
            Chord::Radius => "radius",
            Chord::Sucus => "sucus",
            Chord::Somnus => "somnus",
            Chord::Abyssus => "abyssus",
            Chord::Nucleus => "nucleus",
            Chord::Cinis => "cinis",
            Chord::Furor => "furor",
        }
    }
}

/// The largest intensity a single tone reaches in a profile.
pub const MAX_INTENSITY: u8 = 12;
/// Both tones of a chord must sound at least this loud for it to form.
pub const CHORD_THRESHOLD: u8 = 2;

/// What a thing sounds like: an intensity per tone, after opposites cancelled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Profile {
    pub tones: [u8; TONE_COUNT],
    /// How much cancelled out between opposites. Dissonant things teach more
    /// but make attunements and rituals near them less stable.
    pub dissonance: u8,
}

impl Profile {
    /// Builds a profile from raw (pre-cancellation) amounts: each pair of opposites
    /// loses the smaller amount from both, which becomes dissonance; the rest is
    /// scaled so the loudest tone is at most `MAX_INTENSITY`.
    pub fn from_raw(raw: [u32; TONE_COUNT]) -> Profile {
        let mut tones = raw;
        let mut dissonance = 0u32;
        for i in 0..TONE_COUNT / 2 {
            let j = i + TONE_COUNT / 2;
            let cancelled = tones[i].min(tones[j]);
            tones[i] -= cancelled;
            tones[j] -= cancelled;
            dissonance += cancelled;
        }
        let loudest = tones.iter().copied().max().unwrap_or(0);
        let scale = |v: u32| -> u8 {
            if loudest <= MAX_INTENSITY as u32 {
                v as u8
            } else {
                // Round to nearest, but never let a sounding tone vanish.
                let s = ((v * MAX_INTENSITY as u32 + loudest / 2) / loudest) as u8;
                if v > 0 { s.max(1) } else { 0 }
            }
        };
        Profile {
            tones: tones.map(scale),
            dissonance: dissonance.min(u8::MAX as u32) as u8,
        }
    }

    pub fn get(&self, tone: Tone) -> u8 {
        self.tones[tone.index()]
    }

    pub fn is_silent(&self) -> bool {
        self.tones.iter().all(|&v| v == 0) && self.dissonance == 0
    }

    /// Sounding tones, loudest first (ties in wheel order).
    pub fn sounding(&self) -> Vec<(Tone, u8)> {
        let mut v: Vec<_> = Tone::ALL
            .into_iter()
            .map(|t| (t, self.get(t)))
            .filter(|&(_, n)| n > 0)
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v
    }

    /// Chords present, each with its strength (the quieter of its two tones).
    pub fn chords(&self) -> Vec<(Chord, u8)> {
        Chord::ALL
            .into_iter()
            .filter_map(|c| {
                let (a, b) = c.tones();
                let s = self.get(a).min(self.get(b));
                (s >= CHORD_THRESHOLD).then_some((c, s))
            })
            .collect()
    }

    /// Insight a first scan teaches: each tone's intensity, plus a bonus spread
    /// over the sounding tones for dissonance (a third of it, at least 1 when any).
    pub fn insight(&self) -> [u32; TONE_COUNT] {
        let mut out = self.tones.map(u32::from);
        let sounding: Vec<_> = (0..TONE_COUNT).filter(|&i| out[i] > 0).collect();
        if self.dissonance > 0 && !sounding.is_empty() {
            let bonus = (self.dissonance as u32 / 3).max(1);
            for i in sounding {
                out[i] += bonus;
            }
        }
        out
    }
}

/// One rule of the resonance table: an id whose path contains `word` (as a whole
/// `_`-separated part, or the whole path) adds `amount` of `tone`.
struct Rule {
    word: &'static str,
    tone: Tone,
    amount: u32,
}

const fn r(word: &'static str, tone: Tone, amount: u32) -> Rule {
    Rule { word, tone, amount }
}

// The resonance table: Resonantia's own reading of the vanilla world. Words are matched
// against the parts of an id's path, so `deepslate_iron_ore` hears deepslate, iron and ore.
const RULES: &[Rule] = &[
    // Calor
    r("lava", Tone::Calor, 8),
    r("fire", Tone::Calor, 6),
    r("blaze", Tone::Calor, 6),
    r("magma", Tone::Calor, 6),
    r("campfire", Tone::Calor, 4),
    r("furnace", Tone::Calor, 4),
    r("coal", Tone::Calor, 3),
    r("charcoal", Tone::Calor, 3),
    r("netherrack", Tone::Calor, 3),
    r("torch", Tone::Calor, 1),
    r("smoker", Tone::Calor, 2),
    // Lux
    r("glowstone", Tone::Lux, 8),
    r("torch", Tone::Lux, 3),
    r("lantern", Tone::Lux, 4),
    r("glow", Tone::Lux, 4),
    r("sea", Tone::Lux, 1),
    r("lamp", Tone::Lux, 4),
    r("froglight", Tone::Lux, 6),
    r("gold", Tone::Lux, 3),
    r("glass", Tone::Lux, 2),
    r("quartz", Tone::Lux, 3),
    r("amethyst", Tone::Lux, 3),
    r("beacon", Tone::Lux, 10),
    r("sunflower", Tone::Lux, 3),
    // Fluxus
    r("water", Tone::Fluxus, 8),
    r("bucket", Tone::Fluxus, 1),
    r("feather", Tone::Fluxus, 4),
    r("wind", Tone::Fluxus, 6),
    r("breeze", Tone::Fluxus, 6),
    r("elytra", Tone::Fluxus, 8),
    r("rail", Tone::Fluxus, 3),
    r("redstone", Tone::Fluxus, 4),
    r("piston", Tone::Fluxus, 3),
    r("sand", Tone::Fluxus, 1),
    r("ender", Tone::Fluxus, 3),
    r("pearl", Tone::Fluxus, 3),
    r("kelp", Tone::Fluxus, 2),
    r("prismarine", Tone::Fluxus, 3),
    // Vigor
    r("log", Tone::Vigor, 3),
    r("wood", Tone::Vigor, 3),
    r("planks", Tone::Vigor, 1),
    r("leaves", Tone::Vigor, 4),
    r("sapling", Tone::Vigor, 6),
    r("grass", Tone::Vigor, 3),
    r("moss", Tone::Vigor, 4),
    r("flower", Tone::Vigor, 4),
    r("wheat", Tone::Vigor, 4),
    r("carrot", Tone::Vigor, 4),
    r("potato", Tone::Vigor, 4),
    r("seeds", Tone::Vigor, 5),
    r("apple", Tone::Vigor, 5),
    r("egg", Tone::Vigor, 6),
    r("honey", Tone::Vigor, 4),
    r("dirt", Tone::Vigor, 1),
    r("mushroom", Tone::Vigor, 2),
    r("vine", Tone::Vigor, 3),
    // Gelu
    r("ice", Tone::Gelu, 8),
    r("snow", Tone::Gelu, 6),
    r("powder", Tone::Gelu, 2),
    r("frost", Tone::Gelu, 6),
    r("packed", Tone::Gelu, 2),
    r("blue", Tone::Gelu, 1),
    r("stray", Tone::Gelu, 4),
    // Umbra
    r("obsidian", Tone::Umbra, 8),
    r("deepslate", Tone::Umbra, 3),
    r("sculk", Tone::Umbra, 8),
    r("black", Tone::Umbra, 2),
    r("ink", Tone::Umbra, 5),
    r("end", Tone::Umbra, 3),
    r("ender", Tone::Umbra, 3),
    r("crying", Tone::Umbra, 3),
    r("tinted", Tone::Umbra, 4),
    r("phantom", Tone::Umbra, 5),
    r("echo", Tone::Umbra, 6),
    // Pondus
    r("stone", Tone::Pondus, 4),
    r("cobblestone", Tone::Pondus, 4),
    r("deepslate", Tone::Pondus, 4),
    r("andesite", Tone::Pondus, 4),
    r("diorite", Tone::Pondus, 4),
    r("granite", Tone::Pondus, 4),
    r("tuff", Tone::Pondus, 4),
    r("iron", Tone::Pondus, 6),
    r("copper", Tone::Pondus, 4),
    r("ore", Tone::Pondus, 3),
    r("anvil", Tone::Pondus, 10),
    r("brick", Tone::Pondus, 3),
    r("bricks", Tone::Pondus, 3),
    r("gravel", Tone::Pondus, 3),
    r("diamond", Tone::Pondus, 5),
    r("netherite", Tone::Pondus, 8),
    r("clay", Tone::Pondus, 2),
    // Tabes
    r("rotten", Tone::Tabes, 8),
    r("bone", Tone::Tabes, 5),
    r("skeleton", Tone::Tabes, 6),
    r("wither", Tone::Tabes, 10),
    r("dead", Tone::Tabes, 6),
    r("spider", Tone::Tabes, 3),
    r("poisonous", Tone::Tabes, 6),
    r("soul", Tone::Tabes, 5),
    r("cracked", Tone::Tabes, 2),
    r("mossy", Tone::Tabes, 1),
    r("cobweb", Tone::Tabes, 4),
    r("gunpowder", Tone::Tabes, 3),
    r("tnt", Tone::Tabes, 4),
];

/// Ids the table reads wrongly or not at all, given whole.
const OVERRIDES: &[(&str, [u32; TONE_COUNT])] = &[
    //                       Calor Lux Flux Vig Gelu Umb Pond Tab
    ("minecraft:air", [0, 0, 0, 0, 0, 0, 0, 0]),
    ("minecraft:bedrock", [0, 0, 0, 0, 0, 4, 12, 0]),
    ("minecraft:sponge", [0, 0, 6, 3, 0, 0, 0, 0]),
    ("minecraft:wet_sponge", [0, 0, 9, 3, 0, 0, 0, 0]),
    ("minecraft:cake", [1, 0, 0, 6, 0, 0, 0, 0]),
    ("minecraft:dragon_egg", [3, 0, 0, 10, 0, 10, 0, 0]),
    ("minecraft:nether_star", [0, 10, 0, 0, 0, 0, 0, 8]),
    ("minecraft:totem_of_undying", [0, 6, 0, 12, 0, 0, 0, 0]),
    ("minecraft:spawner", [0, 0, 3, 6, 0, 4, 0, 6]),
    ("minecraft:enchanting_table", [0, 6, 0, 0, 0, 6, 4, 0]),
    ("minecraft:heart_of_the_sea", [0, 4, 10, 0, 0, 0, 0, 0]),
];

/// Reads the resonance profile of a namespaced block or item id. Ids outside
/// `minecraft:` are read by the same table, so other mods' blocks get a guess.
pub fn profile_of(id: &str) -> Profile {
    if let Some((_, raw)) = OVERRIDES.iter().find(|(k, _)| *k == id) {
        return Profile::from_raw(*raw);
    }
    let path = id.split_once(':').map_or(id, |(_, p)| p);
    let parts: Vec<&str> = path.split(['_', '/']).collect();
    let mut raw = [0u32; TONE_COUNT];
    for rule in RULES {
        if path == rule.word || parts.contains(&rule.word) {
            raw[rule.tone.index()] += rule.amount;
        }
    }
    Profile::from_raw(raw)
}

/// The attunement board: eight slots in a ring, each empty or holding a tone token.
/// Slot `i` neighbours `i - 1` and `i + 1` (mod 8).
pub type Board = [Option<Tone>; TONE_COUNT];

/// What two tokens in neighbouring slots do, by their distance on the tone wheel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pairing {
    /// The same tone: no effect.
    Unison,
    /// Neighbours on the wheel: their chord sounds.
    Chord(Chord),
    /// Two steps apart: no effect.
    Neutral,
    /// Three steps apart: grating, 1 dissonance.
    Harsh,
    /// Opposites: 2 dissonance.
    Clash,
}

pub fn pairing(a: Tone, b: Tone) -> Pairing {
    match a.distance(b) {
        0 => Pairing::Unison,
        1 => Pairing::Chord(Chord::of(a, b).expect("distance 1 is a chord")),
        2 => Pairing::Neutral,
        3 => Pairing::Harsh,
        _ => Pairing::Clash,
    }
}

/// How a board sounds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reading {
    /// Chords sounding, in wheel order, without repeats.
    pub chords: Vec<Chord>,
    pub dissonance: u32,
}

pub fn read_board(board: &Board) -> Reading {
    let mut chords = BTreeSet::new();
    let mut dissonance = 0;
    for i in 0..TONE_COUNT {
        let (Some(a), Some(b)) = (board[i], board[(i + 1) % TONE_COUNT]) else {
            continue;
        };
        match pairing(a, b) {
            Pairing::Chord(c) => {
                chords.insert(c);
            }
            Pairing::Harsh => dissonance += 1,
            Pairing::Clash => dissonance += 2,
            Pairing::Unison | Pairing::Neutral => {}
        }
    }
    Reading {
        chords: chords.into_iter().collect(),
        dissonance,
    }
}

/// The puzzle an attunement sets: chords that must sound on the board, the dissonance
/// allowed, and tokens already in place (free, and fixed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Riddle {
    pub chords: &'static [Chord],
    pub min_dissonance: u32,
    pub max_dissonance: u32,
    pub anchors: &'static [(usize, Tone)],
    /// Insight each token the player places costs, in its own tone.
    pub token_cost: u32,
}

impl Riddle {
    /// The board with only the anchors placed: where the player starts.
    pub fn start(&self) -> Board {
        let mut b = [None; TONE_COUNT];
        for &(slot, tone) in self.anchors {
            b[slot] = Some(tone);
        }
        b
    }

    /// Insight the player's tokens on `board` cost (anchors are free).
    pub fn cost(&self, board: &Board) -> [u32; TONE_COUNT] {
        let mut cost = [0; TONE_COUNT];
        for (slot, tone) in board.iter().enumerate() {
            if let Some(t) = tone
                && !self.anchors.iter().any(|&(s, _)| s == slot)
            {
                cost[t.index()] += self.token_cost;
            }
        }
        cost
    }

    /// Whether `board` solves the riddle, and if not, why.
    pub fn check(&self, board: &Board) -> Result<Reading, RiddleError> {
        if let Some(&(slot, tone)) = self.anchors.iter().find(|&&(s, t)| board[s] != Some(t)) {
            return Err(RiddleError::AnchorMoved { slot, tone });
        }
        let reading = read_board(board);
        let missing: Vec<Chord> = self
            .chords
            .iter()
            .copied()
            .filter(|c| !reading.chords.contains(c))
            .collect();
        if !missing.is_empty() {
            return Err(RiddleError::MissingChords(missing));
        }
        if reading.dissonance < self.min_dissonance {
            return Err(RiddleError::TooHarmonious(reading.dissonance));
        }
        if reading.dissonance > self.max_dissonance {
            return Err(RiddleError::TooDissonant(reading.dissonance));
        }
        Ok(reading)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RiddleError {
    /// An anchor's slot doesn't hold its tone.
    AnchorMoved {
        slot: usize,
        tone: Tone,
    },
    MissingChords(Vec<Chord>),
    /// Dissonance below the riddle's minimum.
    TooHarmonious(u32),
    /// Dissonance above the riddle's maximum.
    TooDissonant(u32),
}

/// An attunement: what it needs first, and the riddle that unlocks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attunement {
    pub id: &'static str,
    /// Attunements that must come first.
    pub requires: &'static [&'static str],
    pub riddle: Riddle,
}

/// The first branch of the attunement tree. Later branches (devices, rituals) build on it.
pub const ATTUNEMENTS: &[Attunement] = &[
    Attunement {
        id: "first_chord",
        // Hearing chords at all: sap and ray, two at once.
        requires: &[],
        riddle: Riddle {
            chords: &[Chord::Radius, Chord::Sucus],
            min_dissonance: 0,
            max_dissonance: 0,
            anchors: &[],
            token_cost: 2,
        },
    },
    Attunement {
        id: "resonant_lens",
        // A lens that shows profiles in tooltips: light at its centre, burning and flowing.
        requires: &["first_chord"],
        riddle: Riddle {
            chords: &[Chord::Ardor, Chord::Radius],
            min_dissonance: 0,
            max_dissonance: 0,
            anchors: &[(0, Tone::Lux)],
            token_cost: 3,
        },
    },
    Attunement {
        id: "tone_jar",
        // Bottling a tone: life held in cold, sealed by the dense core. Shadow sits apart.
        requires: &["first_chord"],
        riddle: Riddle {
            chords: &[Chord::Somnus, Chord::Nucleus],
            min_dissonance: 0,
            max_dissonance: 1,
            anchors: &[(4, Tone::Umbra)],
            token_cost: 3,
        },
    },
    Attunement {
        id: "dissonance_ward",
        // Turning dissonance against itself: ash and ray, with a clash between them.
        requires: &["first_chord", "tone_jar"],
        riddle: Riddle {
            chords: &[Chord::Cinis, Chord::Radius],
            min_dissonance: 2,
            max_dissonance: 3,
            anchors: &[(2, Tone::Fluxus), (3, Tone::Pondus)],
            token_cost: 3,
        },
    },
    Attunement {
        id: "harmonic_altar",
        // The ritual altar: every chord at once, the whole wheel in harmony.
        requires: &["resonant_lens", "tone_jar"],
        riddle: Riddle {
            chords: &Chord::ALL,
            min_dissonance: 0,
            max_dissonance: 0,
            anchors: &[(0, Tone::Calor), (4, Tone::Gelu)],
            token_cost: 4,
        },
    },
];

pub fn attunement(id: &str) -> Option<&'static Attunement> {
    ATTUNEMENTS.iter().find(|a| a.id == id)
}

/// What a scan did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScanResult {
    /// First time: the profile and the insight it taught.
    Learned {
        profile: Profile,
        gained: [u32; TONE_COUNT],
    },
    /// Scanned before: nothing new to learn.
    Known(Profile),
    /// It makes no sound at all (air, or nothing the table hears).
    Silent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttuneError {
    Unknown,
    AlreadyAttuned,
    /// The attunements still missing.
    Requires(Vec<&'static str>),
    /// The board doesn't solve the riddle.
    Riddle(RiddleError),
    /// The insight still missing, per tone, for the tokens on the board.
    Lacking([u32; TONE_COUNT]),
}

/// A player's knowledge: insight per tone, what they have scanned and attuned.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Knowledge {
    pub insight: [u32; TONE_COUNT],
    pub scanned: BTreeSet<String>,
    pub attuned: BTreeSet<String>,
}

impl Knowledge {
    pub fn scan(&mut self, id: &str) -> ScanResult {
        let profile = profile_of(id);
        if profile.is_silent() {
            return ScanResult::Silent;
        }
        if !self.scanned.insert(id.to_owned()) {
            return ScanResult::Known(profile);
        }
        let gained = profile.insight();
        for (have, got) in self.insight.iter_mut().zip(gained) {
            *have = have.saturating_add(got);
        }
        ScanResult::Learned { profile, gained }
    }

    /// Checks prerequisites only: whether the player may try the riddle.
    pub fn may_attempt(&self, id: &str) -> Result<&'static Attunement, AttuneError> {
        let a = attunement(id).ok_or(AttuneError::Unknown)?;
        if self.attuned.contains(a.id) {
            return Err(AttuneError::AlreadyAttuned);
        }
        let missing: Vec<_> = a
            .requires
            .iter()
            .copied()
            .filter(|r| !self.attuned.contains(*r))
            .collect();
        if !missing.is_empty() {
            return Err(AttuneError::Requires(missing));
        }
        Ok(a)
    }

    /// Whether `board` would attune `id` now: prerequisites, the riddle, the insight.
    pub fn can_attune(&self, id: &str, board: &Board) -> Result<&'static Attunement, AttuneError> {
        let a = self.may_attempt(id)?;
        a.riddle.check(board).map_err(AttuneError::Riddle)?;
        let cost = a.riddle.cost(board);
        let lacking: [u32; TONE_COUNT] =
            std::array::from_fn(|i| cost[i].saturating_sub(self.insight[i]));
        if lacking.iter().any(|&n| n > 0) {
            return Err(AttuneError::Lacking(lacking));
        }
        Ok(a)
    }

    /// Solves the riddle with `board`: spends its tokens' insight and records the attunement.
    pub fn attune(&mut self, id: &str, board: &Board) -> Result<&'static Attunement, AttuneError> {
        let a = self.can_attune(id, board)?;
        let cost = a.riddle.cost(board);
        for (have, spent) in self.insight.iter_mut().zip(cost) {
            *have -= spent;
        }
        self.attuned.insert(a.id.to_owned());
        Ok(a)
    }

    /// Attunements whose prerequisites are met and that aren't attuned yet.
    pub fn available(&self) -> Vec<&'static Attunement> {
        ATTUNEMENTS
            .iter()
            .filter(|a| !self.attuned.contains(a.id))
            .filter(|a| a.requires.iter().all(|r| self.attuned.contains(*r)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opposites_are_four_steps_apart_and_symmetric() {
        for t in Tone::ALL {
            assert_eq!(t.opposite().opposite(), t);
            assert_eq!(t.distance(t.opposite()), 4);
            assert_eq!(t.distance(t.next()), 1);
        }
        assert_eq!(Tone::Calor.opposite(), Tone::Gelu);
        assert_eq!(Tone::Lux.opposite(), Tone::Umbra);
        assert_eq!(Tone::Fluxus.opposite(), Tone::Pondus);
        assert_eq!(Tone::Vigor.opposite(), Tone::Tabes);
    }

    #[test]
    fn ids_round_trip() {
        for t in Tone::ALL {
            assert_eq!(Tone::from_id(t.id()), Some(t));
        }
        assert_eq!(Tone::from_id("aqua"), None);
    }

    #[test]
    fn chords_are_neighbours_in_either_order() {
        assert_eq!(Chord::of(Tone::Calor, Tone::Lux), Some(Chord::Ardor));
        assert_eq!(Chord::of(Tone::Lux, Tone::Calor), Some(Chord::Ardor));
        assert_eq!(Chord::of(Tone::Tabes, Tone::Calor), Some(Chord::Furor));
        assert_eq!(Chord::of(Tone::Calor, Tone::Fluxus), None);
        for c in Chord::ALL {
            let (a, b) = c.tones();
            assert_eq!(Chord::of(a, b), Some(c));
        }
    }

    #[test]
    fn opposites_cancel_into_dissonance() {
        let mut raw = [0; TONE_COUNT];
        raw[Tone::Calor.index()] = 7;
        raw[Tone::Gelu.index()] = 3;
        let p = Profile::from_raw(raw);
        assert_eq!(p.get(Tone::Calor), 4);
        assert_eq!(p.get(Tone::Gelu), 0);
        assert_eq!(p.dissonance, 3);
    }

    #[test]
    fn loud_profiles_scale_down_without_losing_quiet_tones() {
        let mut raw = [0; TONE_COUNT];
        raw[Tone::Pondus.index()] = 48;
        raw[Tone::Umbra.index()] = 1;
        let p = Profile::from_raw(raw);
        assert_eq!(p.get(Tone::Pondus), MAX_INTENSITY);
        assert_eq!(p.get(Tone::Umbra), 1);
    }

    #[test]
    fn the_table_hears_compound_ids() {
        let p = profile_of("minecraft:deepslate_iron_ore");
        assert_eq!(p.get(Tone::Pondus), 12); // deepslate 4 + iron 6 + ore 3 = 13, scaled
        assert!(p.get(Tone::Umbra) > 0);
        assert_eq!(p.chords().first().map(|c| c.0), Some(Chord::Nucleus));
        // Whole parts only: "stone" is not heard inside "glowstone".
        assert_eq!(profile_of("minecraft:glowstone").get(Tone::Pondus), 0);
    }

    #[test]
    fn overrides_win_and_silence_is_silent() {
        assert!(profile_of("minecraft:air").is_silent());
        assert!(profile_of("minecraft:barrier").is_silent());
        assert_eq!(profile_of("minecraft:bedrock").get(Tone::Pondus), 12);
    }

    #[test]
    fn a_torch_is_warm_light() {
        let p = profile_of("minecraft:torch");
        assert_eq!(p.sounding()[0], (Tone::Lux, 3));
        assert_eq!(p.chords(), vec![]); // Calor 1 is under the chord threshold
        let s = profile_of("minecraft:soul_torch");
        assert!(s.get(Tone::Tabes) > 0);
    }

    #[test]
    fn first_scan_teaches_then_it_is_known() {
        let mut k = Knowledge::default();
        let ScanResult::Learned { gained, .. } = k.scan("minecraft:ice") else {
            panic!("ice teaches");
        };
        assert_eq!(gained[Tone::Gelu.index()], 8);
        assert_eq!(k.insight[Tone::Gelu.index()], 8);
        assert!(matches!(k.scan("minecraft:ice"), ScanResult::Known(_)));
        assert_eq!(k.insight[Tone::Gelu.index()], 8);
        assert_eq!(k.scan("minecraft:air"), ScanResult::Silent);
    }

    #[test]
    fn dissonance_teaches_a_bonus() {
        // Magma: Calor; a frosted magma would cancel. Build one by hand.
        let mut raw = [0; TONE_COUNT];
        raw[Tone::Calor.index()] = 9;
        raw[Tone::Gelu.index()] = 6;
        raw[Tone::Lux.index()] = 2;
        let p = Profile::from_raw(raw);
        let i = p.insight();
        assert_eq!(i[Tone::Calor.index()], 3 + 2);
        assert_eq!(i[Tone::Lux.index()], 2 + 2);
        assert_eq!(i[Tone::Gelu.index()], 0);
    }

    use Tone::*;

    fn board(slots: [Option<Tone>; TONE_COUNT]) -> Board {
        slots
    }

    /// A known solution to every riddle, so none is unsolvable.
    fn solution(id: &str) -> Board {
        match id {
            "first_chord" => board([
                Some(Lux),
                Some(Fluxus),
                Some(Vigor),
                None,
                None,
                None,
                None,
                None,
            ]),
            "resonant_lens" => board([
                Some(Lux),
                Some(Fluxus),
                None,
                None,
                None,
                None,
                None,
                Some(Calor),
            ]),
            "tone_jar" => board([
                None,
                None,
                Some(Vigor),
                Some(Gelu),
                Some(Umbra),
                Some(Pondus),
                None,
                None,
            ]),
            "dissonance_ward" => board([
                None,
                Some(Lux),
                Some(Fluxus),
                Some(Pondus),
                Some(Tabes),
                None,
                None,
                None,
            ]),
            "harmonic_altar" => Tone::ALL.map(Some),
            _ => panic!("no solution for {id}"),
        }
    }

    #[test]
    fn neighbouring_slots_pair_by_wheel_distance() {
        assert_eq!(pairing(Calor, Calor), Pairing::Unison);
        assert_eq!(pairing(Calor, Lux), Pairing::Chord(Chord::Ardor));
        assert_eq!(pairing(Calor, Fluxus), Pairing::Neutral);
        assert_eq!(pairing(Calor, Vigor), Pairing::Harsh);
        assert_eq!(pairing(Calor, Gelu), Pairing::Clash);
        // The ring wraps: slot 7 neighbours slot 0.
        let mut b = [None; TONE_COUNT];
        b[7] = Some(Tabes);
        b[0] = Some(Calor);
        assert_eq!(read_board(&b).chords, vec![Chord::Furor]);
        b[1] = Some(Gelu);
        assert_eq!(read_board(&b).dissonance, 2);
    }

    #[test]
    fn every_riddle_has_a_solution() {
        for a in ATTUNEMENTS {
            let b = solution(a.id);
            assert_eq!(a.riddle.check(&b).map(|_| ()), Ok(()), "{}", a.id);
        }
    }

    #[test]
    fn anchors_are_fixed_and_free() {
        let r = attunement("resonant_lens").unwrap().riddle;
        let start = r.start();
        assert_eq!(start[0], Some(Lux));
        assert_eq!(r.cost(&start), [0; TONE_COUNT]);
        let mut moved = solution("resonant_lens");
        moved[0] = Some(Gelu);
        assert_eq!(
            r.check(&moved),
            Err(RiddleError::AnchorMoved { slot: 0, tone: Lux })
        );
        let cost = r.cost(&solution("resonant_lens"));
        assert_eq!(cost[Calor.index()], 3);
        assert_eq!(cost[Fluxus.index()], 3);
        assert_eq!(cost[Lux.index()], 0);
    }

    #[test]
    fn riddles_bound_dissonance_both_ways() {
        let ward = attunement("dissonance_ward").unwrap().riddle;
        let mut calm = solution("dissonance_ward");
        calm[2] = None; // removes the anchor too
        assert!(matches!(
            ward.check(&calm),
            Err(RiddleError::AnchorMoved { .. })
        ));
        let jar = attunement("tone_jar").unwrap().riddle;
        let mut loud = solution("tone_jar");
        loud[6] = Some(Fluxus); // Pondus next to Fluxus: a clash
        assert_eq!(jar.check(&loud), Err(RiddleError::TooDissonant(2)));
        let first = attunement("first_chord").unwrap().riddle;
        assert_eq!(
            first.check(&board([
                Some(Lux),
                Some(Fluxus),
                None,
                None,
                None,
                None,
                None,
                None
            ])),
            Err(RiddleError::MissingChords(vec![Chord::Sucus]))
        );
    }

    #[test]
    fn attunements_need_prerequisites_a_solution_and_insight() {
        let mut k = Knowledge::default();
        let empty = [None; TONE_COUNT];
        assert_eq!(k.attune("nope", &empty), Err(AttuneError::Unknown));
        assert_eq!(
            k.attune("tone_jar", &empty),
            Err(AttuneError::Requires(vec!["first_chord"]))
        );
        assert!(matches!(
            k.attune("first_chord", &empty),
            Err(AttuneError::Riddle(RiddleError::MissingChords(_)))
        ));
        let b = solution("first_chord");
        let Err(AttuneError::Lacking(l)) = k.attune("first_chord", &b) else {
            panic!("costs insight");
        };
        assert_eq!(l, [0, 2, 2, 2, 0, 0, 0, 0]);
        k.insight = [5; TONE_COUNT];
        assert!(k.attune("first_chord", &b).is_ok());
        assert_eq!(k.insight, [5, 3, 3, 3, 5, 5, 5, 5]);
        assert_eq!(
            k.attune("first_chord", &b),
            Err(AttuneError::AlreadyAttuned)
        );
        let ids: Vec<_> = k.available().iter().map(|a| a.id).collect();
        assert_eq!(ids, ["resonant_lens", "tone_jar"]);
    }

    #[test]
    fn every_attunement_requirement_exists_and_tree_is_acyclic() {
        for a in ATTUNEMENTS {
            for r in a.requires {
                let pos_r = ATTUNEMENTS.iter().position(|x| x.id == *r);
                let pos_a = ATTUNEMENTS.iter().position(|x| x.id == a.id);
                assert!(pos_r.is_some(), "{} requires unknown {r}", a.id);
                assert!(pos_r < pos_a, "{} must come after {r}", a.id);
            }
        }
    }
}
