//! A frame of the game into advice and marks: read the board, follow the instruments through the game, ask the
//! advisor, lay out the marks for the screen the overlay covers. Shared by the live app and --preview.

use crate::draw::{plan_for, Plan};
use vorax::advisor::{advise_plan, Advice};
use vorax_scan::layout::layout_at;
use vorax_scan::live::carried_round;
use vorax_scan::pixels::Img;
use vorax_scan::reading::{Problem, Reading};
use vorax_scan::titles::{pictures_in, read_titles_in, Script};
use vorax_scan::track::{track_instruments, InstrumentTracker};
use vorax_scan::window::{crop_to, windowed_game};
use vorax_scan::{scan, scan_data};

pub type TextReader<'a> = Option<&'a mut dyn FnMut(&Img) -> String>;

/// Futures per option the Solver plays for the overlay's advice. More futures make close calls less a matter of
/// luck in the Solver's own playouts. On the same 3,000 seeds with the full Solver (Oct 24, 2026): the linear board
/// value at 1024 futures reached 840k in 81.5% of games, the trained trees at 256 in 79.2% and at 512 in 81.0%;
/// the linear one is about five times faster to evaluate, so 1024 of its futures take less time than 256 with the
/// trees (the hardest recorded board: about 0.2 s on 12 threads).
pub const FUTURES: u32 = 1024;

pub enum Outcome {
    /// not the Surgical Preparation screen
    NotGame,
    /// a card flying off as it's played: the last advice still stands
    Animating,
    /// the game closing after round 13's last pick (its end animation, the rewards): not a board
    Closing,
    /// advice for the board, with its marks
    Advice { reading: Box<Reading>, advice: Box<Advice>, plan: Plan, read_ms: f64 },
}

/// A card the reader didn't make out, for the player to pick (the toolbox's picker; not in upstream).
#[derive(Clone, Debug)]
pub struct Ask {
    /// its place in the hand, 0-based
    pub slot: usize,
    /// the kind of pick: "potion" or "instrument"
    pub phase: &'static str,
    /// what the text reader made of its title
    pub text: String,
    /// the cards it most likely is (English names), best first
    pub candidates: Vec<String>,
    /// the card's middle, in the picture's pixels
    pub at: (f64, f64),
}

pub struct Pipeline {
    pub tracker: InstrumentTracker,
    pub shelley: i32,
    /// instrument rerolls left, as last read on an instrument offer this game (a potion board doesn't show them)
    instrument_refreshes: Option<i32>,
    /// the last board advised on, with the size of its picture, for advising again when a setting changes (and
    /// for its round, when the next board's can't be read)
    last: Option<(Reading, usize, usize)>,
    /// this game reached the Surgical Protocols (rounds 11-13): a board after them with no cards in hand is the game
    /// closing (its end animation, the rewards). Read as a board it gave six 1×1 monsters in "round 1"; round 13's
    /// own board may not be read first (caught mid-animation, or its number read as 12)
    protocols: bool,
    /// the last read was the Vorax screen opened again on the game under way (Escape and back, to skip long
    /// animations), not a new game
    pub resumed: bool,
    /// the writing the card titles are read in (a Chinese client's: matched to the cards' Chinese names)
    pub script: Script,
    /// the picker (not in upstream): cards picked by the player for this board (`pick_key`), the titles' texts
    /// learned from earlier picks, and the cards still not made out on the last board
    picks: std::collections::BTreeMap<usize, String>,
    pick_key: String,
    pub learned: crate::learn::Learned,
    pub asks: Vec<Ask>,
    pub ask_key: String,
    /// every card asked about on this board (`ask_key`), by place: a pick counts even when a read since made the
    /// card out
    asked: std::collections::BTreeMap<usize, Ask>,
    /// the picker's test (Troubleshooting): the first card counts as not read, its title half read
    pub test_unknown: bool,
}

/// Whether the screen opening is the game under way going on rather than a new one, from the round of the board
/// before (`before`) and the round now read for sure (`now`). A new game starts in round 1, and none goes on past
/// round 13. Icons in the instrument panel prove nothing: a new game can start with an item from the map in it
/// (a boss drop), read as the nearest instrument; taken for the game under way, it stayed in as an instrument.
fn resumes(before: Option<i32>, now: Option<i32>) -> bool {
    match (before, now) {
        (Some(b), Some(n)) => n > 1 && b < 13 && n >= b,
        _ => false,
    }
}

/// Instrument rerolls a game starts with, by Intern Shelley level (src/data/shelley.ts).
pub fn shelley_rerolls(level: i32) -> i32 {
    [0, 1, 1, 2, 2, 2].get((level - 1).clamp(0, 5) as usize).copied().unwrap_or(0)
}

impl Pipeline {
    pub fn new(shelley: i32) -> Pipeline {
        Pipeline { tracker: InstrumentTracker::default(), shelley, instrument_refreshes: None, last: None, protocols: false, resumed: false, script: Script::Latin, picks: Default::default(), pick_key: String::new(), learned: Default::default(), asks: Vec::new(), ask_key: String::new(), asked: Default::default(), test_unknown: false }
    }

    /// The board's instrument rerolls: as read on an instrument offer, else carried from the last one this game,
    /// else what the Shelley level starts a game with. (Upstream counts them as 0 on a potion board.)
    fn fill_rerolls(&mut self, reading: &mut Reading) {
        match reading.state.phase {
            vorax::advisor::ScreenPhase::Instrument => {
                if reading.fields.get("refreshes").is_some_and(|f| f.sure) {
                    self.instrument_refreshes = reading.state.instrument_refreshes;
                }
            }
            _ => {
                if reading.state.instrument_refreshes.is_none() {
                    reading.state.instrument_refreshes = Some(self.instrument_refreshes.unwrap_or(shelley_rerolls(self.shelley)));
                }
            }
        }
    }

    /// The last board advised again (the Shelley level changed): its marks, for a picture of this size.
    /// The cards on the last board not made out, left to right (a card the data doesn't have isn't one).
    fn unknown_slots(reading: &Reading) -> Vec<(usize, String)> {
        let data = scan_data();
        let mut out: Vec<(usize, String)> = reading
            .problems
            .iter()
            .filter_map(|p| match p {
                Problem::UnknownArt { field, text, .. } => {
                    let j = field.strip_prefix("hand")?.parse::<usize>().ok()?;
                    let t = text.clone().unwrap_or_default();
                    vorax_scan::titles::han_unsupported(data, &t).is_none().then_some((j, t))
                }
                _ => None,
            })
            .collect();
        out.sort_by_key(|(j, _)| *j);
        out
    }

    fn phase_of(reading: &Reading) -> Option<&'static str> {
        match reading.state.phase {
            vorax::advisor::ScreenPhase::Potion => Some("potion"),
            vorax::advisor::ScreenPhase::Instrument => Some("instrument"),
            vorax::advisor::ScreenPhase::Protocol => None,
        }
    }

    /// The number of cards in the hand read.
    fn hand_size(reading: &Reading) -> usize {
        reading.hand_ranks.as_ref().map(|r| r.len()).filter(|&n| n > 0).or(reading.hand_size).unwrap_or(3)
    }

    /// Which board this is, for keeping the player's picks and the picker's card while it stays: the round, the
    /// kind of pick, the rerolls left and the hand's size. Not the cards made out: a read again that makes out one
    /// more is the same board (it closed the picker's card before the player could pick).
    fn board_key(reading: &Reading) -> String {
        let st = &reading.state;
        format!("{:?}|{:?}|{:?}|{:?}|{}", st.round, st.phase, st.potion_refreshes, st.instrument_refreshes, Self::hand_size(reading))
    }

    /// A card picked (or learned) for place `slot`: in the hand in its place, no longer "not read"; a card made out
    /// there is replaced (the player's pick stands).
    fn put_card(reading: &mut Reading, slot: usize, name: &str) {
        let key = format!("hand{slot}");
        let size = Self::hand_size(reading);
        if slot >= size {
            return;
        }
        // the hand lists the cards made out in their order: this one goes after those left of it
        let all_unknown: Vec<usize> = reading.problems.iter().filter_map(|p| match p {
            Problem::UnknownArt { field, .. } => field.strip_prefix("hand").and_then(|j| j.parse::<usize>().ok()),
            _ => None,
        }).collect();
        let at = (0..slot).filter(|j| !all_unknown.contains(j)).count();
        if all_unknown.contains(&slot) {
            reading.problems.retain(|p| !matches!(p, Problem::UnknownArt { field, .. } if *field == key));
            let at = at.min(reading.state.hand.len());
            reading.state.hand.insert(at, name.to_string());
        } else if let Some(card) = reading.state.hand.get_mut(at) {
            *card = name.to_string();
        } else {
            return;
        }
        reading.fields.insert(key, vorax_scan::reading::FieldRead { sure: false, guess: Some(name.to_string()), ..Default::default() });
    }

    /// The picker's test: the first and the last card taken out as not read, with their titles half read (a
    /// character or a word dropped), as cards the reader missed would be.
    fn hide_first(&self, reading: &mut Reading) {
        if Self::phase_of(reading).is_none() || reading.state.hand.is_empty() || reading.problems.iter().any(|p| matches!(p, Problem::UnknownArt { .. })) {
            return;
        }
        let size = reading.state.hand.len();
        // the last first, so the first's place in the hand stays
        for slot in [size - 1, 0] {
            if slot == 0 && size == 1 && reading.state.hand.is_empty() {
                break;
            }
            let name = reading.state.hand.remove(slot);
            let text = if self.script == Script::Han {
                let zh: Vec<char> = crate::draw::zh_name(reading.state.phase, &name).chars().collect();
                zh.iter().enumerate().filter(|(i, _)| *i != 1).map(|(_, c)| *c).collect::<String>()
            } else {
                name.split(' ').skip(1).collect::<Vec<_>>().join(" ")
            };
            let key = format!("hand{slot}");
            reading.fields.remove(&key);
            reading.problems.push(Problem::UnknownArt { field: key, crop: vorax_scan::pixels::Rect::new(0.0, 0.0, 1.0, 1.0), hand: None, text: Some(text) });
        }
    }

    /// The player's picks for this board, and cards learned from earlier picks, put in; what's still not made out
    /// becomes `asks`.
    fn fill_picks(&mut self, reading: &mut Reading, w: usize, h: usize) {
        self.asks.clear();
        let Some(phase) = Self::phase_of(reading) else { return };
        let key = Self::board_key(reading);
        if key != self.pick_key {
            self.picks.clear();
            self.pick_key = key.clone();
        }
        if key != self.ask_key {
            self.asked.clear();
        }
        self.ask_key = key;
        // the player's picks for this board stand, made out since or not
        for (j, n) in self.picks.clone() {
            Self::put_card(reading, j, &n);
        }
        let unknown = Self::unknown_slots(reading);
        if unknown.is_empty() {
            return;
        }
        for (j, text) in &unknown {
            if let Some(n) = self.learned.get(phase, text) {
                Self::put_card(reading, *j, &n);
            }
        }
        let size = Self::hand_size(reading);
        let pos = layout_at(w, h, reading.ui()).shifted_hand(reading.hand_dy.unwrap_or(0.0)).hand_at(size).to_vec();
        for (j, text) in Self::unknown_slots(reading) {
            let pictures = reading.hand_ranks.as_ref().and_then(|r| r.get(j)).cloned().unwrap_or_default();
            let candidates = vorax_scan::titles::candidates(scan_data(), phase, &text, self.script, &pictures, 5);
            let at = pos.get(j).map_or((w as f64 / 2.0, h as f64 / 2.0), |c| (c.cx, c.cy));
            let ask = Ask { slot: j, phase, text, candidates, at };
            self.asked.insert(j, ask.clone());
            self.asks.push(ask);
        }
    }

    /// The player picked `name` for the card at `slot` on the last board: kept while the board stays, learned for
    /// its title's text, and the advice given again with it.
    pub fn pick(&mut self, slot: usize, name: &str) -> Option<(Plan, usize, usize)> {
        // asked about on this board, even if a read since made it out (the pick still stands)
        let ask = self.asks.iter().find(|a| a.slot == slot).or_else(|| self.asked.get(&slot))?.clone();
        self.picks.insert(slot, name.to_string());
        self.learned.put(ask.phase, &ask.text, name);
        self.asks.retain(|a| a.slot != slot);
        if let Some((reading, ..)) = self.last.as_mut() {
            Self::put_card(reading, slot, name);
        }
        self.readvise()
    }

    /// The size of the last board's picture.
    pub fn last_size(&self) -> Option<(usize, usize)> {
        self.last.as_ref().map(|(_, w, h)| (*w, *h))
    }

    /// The last board as read (with the picks in).
    /// The interface scale and hand shift the last board was read at (1 and 0 before any).
    pub fn last_place(&self) -> (f64, f64) {
        self.last.as_ref().map_or((1.0, 0.0), |(r, ..)| (r.ui(), r.hand_dy.unwrap_or(0.0)))
    }

    pub fn last_reading(&self) -> Option<&Reading> {
        self.last.as_ref().map(|(r, ..)| r)
    }

    pub fn readvise(&mut self) -> Option<(Plan, usize, usize)> {
        let (mut reading, w, h) = self.last.clone()?;
        reading.state.shelley = Some(self.shelley);
        if reading.state.phase != vorax::advisor::ScreenPhase::Instrument {
            reading.state.instrument_refreshes = None;
        }
        self.fill_rerolls(&mut reading);
        let advice = advise_plan(&reading.state, Some(FUTURES), true);
        Some((plan_for(&advice, &reading.state, &reading, &layout_at(w, h, reading.ui()).shifted_hand(reading.hand_dy.unwrap_or(0.0)), self.script == Script::Han && !crate::zh::english()), w, h))
    }

    /// `frame`: the captured game window; `trigger`: "screen" when the log says a new game opened.
    pub fn run(&mut self, frame: &Img, trigger: Option<&str>, ocr: TextReader) -> Outcome {
        let t = std::time::Instant::now();
        self.resumed = false;
        let data = scan_data();
        let img = match windowed_game(frame) {
            Some(a) => crop_to(frame, &a),
            None => frame.clone(),
        };
        // with a text reader: the cards by their printed names, by their pictures only where a name doesn't settle
        let mut raw = match ocr {
            Some(read) => {
                let names = scan(&img, data, false);
                let (dy, ui) = (names.hand_dy.unwrap_or(0.0), names.ui());
                let script = self.script;
                let pics = |phase: &str, size: usize, slots: &[usize]| pictures_in(&img, data, phase, size, slots, dy, ui, script);
                read_titles_in(&img, names, data, read, Some(&pics), script)
            }
            None => scan(&img, data, true),
        };
        if raw.has(|p| matches!(p, Problem::Animating)) {
            return Outcome::Animating;
        }
        // not in upstream: rounds 11-13 are always the Surgical Protocols; one read for sure there whose cards weren't
        // made out yet (still flying in) is one, not a potion board with an empty hand ("Board not fully read")
        if raw.fields.get("round").is_some_and(|f| f.sure) && raw.state.round.is_some_and(|r| r >= 11) && raw.state.phase != vorax::advisor::ScreenPhase::Protocol && raw.state.hand.is_empty() {
            raw.state.phase = vorax::advisor::ScreenPhase::Protocol;
        }
        if raw.has(|p| matches!(p, Problem::NotFound)) {
            return Outcome::NotGame;
        }
        // the screen opened again on the game under way (Escape and back skips long animations): what's known of it
        // (instruments and their rounds, rerolls) is kept
        let round_sure = raw.fields.get("round").is_some_and(|f| f.sure);
        let before = self.last.as_ref().and_then(|(r, ..)| r.state.round);
        self.resumed = trigger == Some("screen") && resumes(before, round_sure.then_some(raw.state.round.unwrap_or(1)));
        let trigger = if self.resumed { None } else { trigger };
        if trigger == Some("screen") {
            self.instrument_refreshes = None;
            self.protocols = false;
        }
        // a new game opens with "screen" and an instrument offer: cards in hand
        let protocol = raw.state.phase == vorax::advisor::ScreenPhase::Protocol;
        if self.protocols && !protocol && raw.state.hand.is_empty() {
            return Outcome::Closing;
        }
        self.protocols |= protocol;
        let tr = track_instruments(&self.tracker, &raw, trigger);
        self.tracker = tr.tracker.clone();
        let mut reading = raw;
        // not in upstream: the player's picks and the learned titles, before the advice
        if self.test_unknown {
            self.hide_first(&mut reading);
        }
        reading.state.instruments = tr.instruments.clone();
        reading.state.instrument_rounds = Some(tr.rounds.clone());
        // a round not read for sure: the one before stands in, not round 1 (before any board, round 1 as upstream's
        // Solver starts). So does one that can't be: a protocol board below round 11, or a round going back when the
        // screen opened again on the game under way (still fading in, round 12 read as 2)
        let before_round = self.last.as_ref().and_then(|(r, ..)| r.state.round);
        let read_round = reading.state.round.unwrap_or(1);
        let impossible = (reading.state.phase == vorax::advisor::ScreenPhase::Protocol && read_round < 11) || (self.resumed && before_round.is_some_and(|b| read_round < b));
        if let Some(f) = reading.fields.get("round").filter(|f| !f.sure || impossible) {
            let before = self.last.as_ref().map_or(Some(1), |(r, ..)| r.state.round);
            reading.state.round = Some(carried_round(before, reading.state.phase, f.guess.as_deref()));
        }
        // after the round is settled: the picker's board is known by it
        self.fill_picks(&mut reading, img.width, img.height);
        reading.state.shelley = Some(self.shelley);
        self.fill_rerolls(&mut reading);
        let read_ms = t.elapsed().as_secs_f64() * 1000.0;
        let advice = advise_plan(&reading.state, Some(FUTURES), true);
        let plan = plan_for(&advice, &reading.state, &reading, &layout_at(img.width, img.height, reading.ui()).shifted_hand(reading.hand_dy.unwrap_or(0.0)), self.script == Script::Han && !crate::zh::english());
        self.last = Some((reading.clone(), img.width, img.height));
        Outcome::Advice { reading: Box::new(reading), advice: Box::new(advice), plan, read_ms }
    }

    /// A pick the tracker isn't sure of yet, first seen on this board: worth another look once the icon has landed.
    pub fn wants_relook(&self) -> bool {
        self.tracker.unsure.iter().any(|q| q.looks == 0)
    }
}

pub fn decode_png(path: &std::path::Path) -> Result<Img, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut dec = png::Decoder::new(file);
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let (w, h) = (info.width as usize, info.height as usize);
    let ch = info.color_type.samples();
    let mut img = Img::new(w, h);
    for i in 0..w * h {
        let p = &buf[i * ch..i * ch + ch];
        let px = match ch {
            1 => [p[0], p[0], p[0], 255],
            2 => [p[0], p[0], p[0], p[1]],
            3 => [p[0], p[1], p[2], 255],
            _ => [p[0], p[1], p[2], p[3]],
        };
        img.data[i * 4..i * 4 + 4].copy_from_slice(&px);
    }
    Ok(img)
}

pub fn encode_png(path: &std::path::Path, w: u32, h: u32, rgba: &[u8]) -> Result<(), String> {
    encode_png_with(path, w, h, rgba, png::Compression::Default)
}

/// A picture saved quickly (bigger file): the toolbox's last pictures, kept for reporting a misread.
pub fn encode_png_fast(path: &std::path::Path, w: u32, h: u32, rgba: &[u8]) -> Result<(), String> {
    encode_png_with(path, w, h, rgba, png::Compression::Fast)
}

fn encode_png_with(path: &std::path::Path, w: u32, h: u32, rgba: &[u8], level: png::Compression) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_compression(level);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| e.to_string())?;
    wr.write_image_data(rgba).map_err(|e| e.to_string())
}

/// Not in upstream: whether a picture still shows the Vorax screen, cheaply (the hand's header, else the round's
/// number around its place; no tanks, cards or text reader), at the interface scale and hand shift the last board
/// was read at. For the marks left over the town when the log didn't say the screen closed.
pub fn still_vorax(frame: &Img, ui: f64, hand_dy: f64) -> bool {
    use vorax_scan::text::{read_text, TextKind};
    let img = match windowed_game(frame) {
        Some(a) => crop_to(frame, &a),
        None => frame.clone(),
    };
    let data = scan_data();
    let l = layout_at(img.width, img.height, ui);
    if vorax_scan::cards::read_phase(&img, &l.shifted_hand(hand_dy), data).0.is_some() {
        return true;
    }
    let g = vorax_scan::pixels::gray(&img);
    let zh = data.digits_with_zh("round");
    for refs in [&data.digits["round"], &zh] {
        for dy in [0.0, 4.0, 8.0, 12.0, 16.0, 20.0, -4.0, -8.0] {
            for dx in [0.0, -6.0, 6.0, -12.0, 12.0] {
                let r = vorax_scan::pixels::Rect::new(l.round.x + dx * l.k, l.round.y + dy * l.k, l.round.w, l.round.h);
                let Some(t) = read_text(&img, &g, &r, TextKind::Round, refs) else { continue };
                let n: f64 = t.text.parse().unwrap_or(f64::NAN);
                if n.fract() == 0.0 && (1.0..=13.0).contains(&n) && t.score >= 0.75 && t.score - t.second >= 0.05 {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{resumes, Pipeline};
    use vorax_scan::reading::Problem;

    #[test]
    fn a_picked_card_goes_back_in_its_place() {
        // a board read with every card made out: its middle card taken out as not read, then picked back
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/frames");
        let mut paths: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "png")).collect();
        paths.sort();
        let data = vorax_scan::scan_data();
        let reading = paths.iter().find_map(|p| {
            let img = super::decode_png(p).ok()?;
            let r = vorax_scan::scan(&img, data, true);
            (r.state.hand.len() == 3 && !r.problems.iter().any(|q| matches!(q, Problem::UnknownArt { .. })) && r.state.phase != vorax::advisor::ScreenPhase::Protocol).then_some(r)
        });
        let Some(whole) = reading else { panic!("no fully read three-card board in fixtures/frames") };
        let mut r = whole.clone();
        let middle = r.state.hand.remove(1);
        r.problems.push(Problem::UnknownArt { field: "hand1".into(), crop: vorax_scan::pixels::Rect::new(0.0, 0.0, 1.0, 1.0), hand: None, text: Some("麻 、 異 魔".into()) });
        assert_eq!(Pipeline::unknown_slots(&r), vec![(1, "麻 、 異 魔".to_string())]);
        Pipeline::put_card(&mut r, 1, &middle);
        assert_eq!(r.state.hand, whole.state.hand);
        assert!(Pipeline::unknown_slots(&r).is_empty());
        assert_eq!(r.fields["hand1"].guess.as_deref(), Some(middle.as_str()));
        // a card the data doesn't have isn't asked about
        let mut r = whole.clone();
        r.state.hand.remove(2);
        r.problems.push(Problem::UnknownArt { field: "hand2".into(), crop: vorax_scan::pixels::Rect::new(0.0, 0.0, 1.0, 1.0), hand: None, text: Some("解剖浸液：病軀".into()) });
        assert!(Pipeline::unknown_slots(&r).is_empty());
    }

    #[test]
    fn a_card_made_out_on_a_read_again_is_the_same_board_and_a_pick_stands() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/frames");
        let mut paths: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "png")).collect();
        paths.sort();
        let data = vorax_scan::scan_data();
        let whole = paths
            .iter()
            .find_map(|p| {
                let r = vorax_scan::scan(&super::decode_png(p).ok()?, data, true);
                (r.state.hand.len() == 3 && !r.problems.iter().any(|q| matches!(q, Problem::UnknownArt { .. })) && r.state.phase != vorax::advisor::ScreenPhase::Protocol).then_some(r)
            })
            .expect("a fully read three-card board in fixtures/frames");
        // the first read missed the middle card; a read again made it out: the same board for the picker
        let mut first = whole.clone();
        first.state.hand.remove(1);
        first.problems.push(Problem::UnknownArt { field: "hand1".into(), crop: vorax_scan::pixels::Rect::new(0.0, 0.0, 1.0, 1.0), hand: None, text: Some("麻 、 異 魔".into()) });
        assert_eq!(Pipeline::board_key(&first), Pipeline::board_key(&whole));
        // the player's pick for a card made out since replaces it, in its place
        let mut r = whole.clone();
        Pipeline::put_card(&mut r, 1, "Picked");
        assert_eq!(r.state.hand, [whole.state.hand[0].as_str(), "Picked", whole.state.hand[2].as_str()]);
        // another round is another board
        let mut next = whole.clone();
        next.state.round = next.state.round.map(|n| n + 1);
        assert_ne!(Pipeline::board_key(&next), Pipeline::board_key(&whole));
    }

    #[test]
    fn candidates_come_from_the_text_and_the_pictures() {
        let data = vorax_scan::scan_data();
        let c = vorax_scan::titles::candidates(data, "potion", "麻 、 異 魔", vorax_scan::titles::Script::Han, &[("Spinal Solution - Aemberon".to_string(), 0.9)], 4);
        assert_eq!(c.first().map(String::as_str), Some("Anesthetic Tincture - Aemberon"));
        assert!(c.iter().any(|n| n == "Spinal Solution - Aemberon"));
        assert!(c.len() <= 4);
        // nothing read and no pictures: nothing to offer but the list of every card
        assert!(vorax_scan::titles::candidates(data, "potion", "", vorax_scan::titles::Script::Han, &[], 4).is_empty());
    }

    #[test]
    fn the_screen_opened_again_mid_game_goes_on() {
        assert!(resumes(Some(6), Some(6)));
        assert!(resumes(Some(6), Some(7)));
        assert!(resumes(Some(12), Some(12)));
    }

    #[test]
    fn a_new_game_is_a_new_game() {
        // a new game starts in round 1, whatever is in the instrument panel (a boss drop from the map)
        assert!(!resumes(Some(13), Some(1)));
        assert!(!resumes(Some(6), Some(1)));
        // nothing goes on after round 13, nor back in rounds, nor when the round isn't read for sure
        assert!(!resumes(Some(13), Some(13)));
        assert!(!resumes(Some(8), Some(5)));
        assert!(!resumes(Some(6), None));
        // the overlay's first board
        assert!(!resumes(None, Some(6)));
    }
}

#[cfg(test)]
mod still_vorax_tests {
    use super::{decode_png, still_vorax};

    fn fixture(rel: &str) -> vorax_scan::pixels::Img {
        decode_png(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures").join(rel)).unwrap()
    }

    /// Boards at the interface scale they were read at are the Vorax screen; the town (two of a player's pictures
    /// taken there, shrunk) isn't, at any scale.
    #[test]
    fn the_vorax_screen_is_told_from_the_town() {
        for (f, ui) in [("frames/board-012.png", 1.0), ("frames/protocol-018.png", 1.0), ("frames/board-1080p.png", 1.0), ("frames/windowed-2506.png", 1.0), ("frames/ui-125.png", 1.25), ("zh-frames/twin-hormone-1080p-ui125.png", 1.25), ("zh-frames/potion-kit-large-1080p-ui125.png", 1.25)] {
            assert!(still_vorax(&fixture(f), ui, 0.0), "{f} at {ui}");
        }
        for f in ["not-vorax/town-1280.png", "not-vorax/town2-1280.png"] {
            for ui in vorax_scan::layout::UI_SCALES {
                assert!(!still_vorax(&fixture(f), ui, 0.0), "{f} at {ui}");
            }
        }
    }
}
