//! What the overlay draws, and drawing it: an outline on the card (or reroll) to take, a glowing outline on the
//! glass of each tank to target, and one small pill with the move and how often it reaches 840k. Pure drawing into an RGBA
//! picture, so it can be previewed on a saved frame without the game.

use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};
use vorax::advisor::{Advice, Best, ScreenPhase, ScreenState};
use vorax_scan::layout::{CardPos, Layout};
use vorax_scan::pixels::Rect;
use vorax_scan::reading::{Problem, Reading};

/// A card face in reference pixels (layout.ts CARD_FACE).
const CARD_W: f64 = 230.0;
const CARD_H: f64 = 340.0;

#[derive(Clone, Debug, Default)]
pub struct Pill {
    /// the move: a card's name, with its tanks
    pub main: String,
    /// how it ends: "64% reach 840k"
    pub sub: String,
    /// the board is being read or worked out: the last advice, dimmed
    pub dim: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Plan {
    pub pill: Option<Pill>,
    /// the card to take: its centre, tilt and size in screen pixels
    pub card: Option<(CardPos, f64, f64)>,
    /// the reroll button
    pub button: Option<Rect>,
    /// the glass of each tank to target
    pub tanks: Vec<Rect>,
    /// not in upstream: what was read, for the player to check against the game (Ctrl+Alt+D hides it): a line for
    /// the cards under the pill, and a mark under each tank's label
    pub read: Vec<String>,
    pub read_tanks: Vec<TankMark>,
    /// where the cards read go: over the cards' middle, right above the "pick a potion" banner (its top), in
    /// screen pixels (else under the pill)
    pub read_at: Option<(f64, f64)>,
}

/// What was read of one tank, shown under its label: "骨 15×12" in its rarity's colour (not in upstream).
#[derive(Clone, Debug)]
pub struct TankMark {
    /// the tank's Activity × Quantity line, in screen pixels: the mark goes under it
    pub under: Rect,
    pub text: String,
    pub colour: (u8, u8, u8),
}

impl Plan {
    pub fn dimmed(&self) -> Plan {
        let mut p = self.clone();
        if let Some(pill) = &mut p.pill {
            pill.dim = true;
        }
        p.card = None;
        p.button = None;
        p.tanks.clear();
        p
    }
}

fn pct(x: f64) -> String {
    format!("{:.0}% reach {}k", x * 100.0, vorax::value::TARGET / 1000)
}

/// The hand positions the reading's names came from: unknown cards are left out of its hand.
fn slots(reading: &Reading, size: usize) -> Vec<usize> {
    let unknown: Vec<usize> = reading
        .problems
        .iter()
        .filter_map(|p| match p {
            Problem::UnknownArt { field, .. } => field.strip_prefix("hand").and_then(|j| j.parse().ok()),
            _ => None,
        })
        .collect();
    (0..size).filter(|j| !unknown.contains(j)).collect()
}

/// The hint's words for a Chinese client (not in upstream). The font is cut to these and the cards' Chinese names
/// (dev/make_font_zh.py), so a word added here needs the font made again (a test says so).
const ZH_REROLL: &str = "刷新";
const ZH_ALWAYS: &str = "出現就選";
/// a Dissection the data has no Chinese name for
const ZH_DISSECTION: &str = "解剖卡";
const ZH_REACH: &str = "達";
const ZH_TANK: &str = "第 {} 格";
const ZH_CARD: &str = "第 {} 張卡";
const ZH_NOT_READ: &str = "未讀到：";
const ZH_NOT_FULLY: &str = "畫面未完全讀到";
const ZH_NOT_IN_DATA: &str = "不在資料內，請自行判斷";
const ZH_READ_CARDS: &str = "卡：";
const ZH_EMPTY: &str = "空";
const ZH_UNKNOWN: &str = "？";
const ZH_SEP: &str = "｜";
const ZH_MEAN: &str = "平均";
const ZH_TEN_THOUSAND: &str = "萬";
const ZH_PEST: &str = "蟲";
const ZH_AEMBERON: &str = "異";
const ZH_BONE_SENTRY: &str = "骨";
const ZH_AGITO: &str = "覺";

/// The marks' colours: a rarity's as the game colours its names, and an empty tank's and an unread one's.
const NORMAL: (u8, u8, u8) = (236, 232, 228);
const MAGIC: (u8, u8, u8) = (110, 170, 255);
const RARE: (u8, u8, u8) = (255, 200, 90);
const BOSS: (u8, u8, u8) = (255, 95, 85);
const EMPTY: (u8, u8, u8) = (150, 144, 144);
const UNREAD: (u8, u8, u8) = (255, 150, 60);

/// A monster as its mark shows it: its kind in one word ("骨") and Activity × Quantity, in its rarity's colour.
fn monster_mark(m: &vorax::advisor::ScreenMonster, zh: bool) -> (String, (u8, u8, u8)) {
    use vorax::types::{Archetype, Rarity};
    let kind = match (m.archetype, zh) {
        (Archetype::Pest, true) => ZH_PEST,
        (Archetype::Aemberon, true) => ZH_AEMBERON,
        (Archetype::BoneSentry, true) => ZH_BONE_SENTRY,
        (Archetype::Agito, true) => ZH_AGITO,
        (Archetype::Pest, false) => "Pest",
        (Archetype::Aemberon, false) => "Aemb",
        (Archetype::BoneSentry, false) => "Bone",
        (Archetype::Agito, false) => "Agito",
    };
    let colour = match m.rarity {
        Rarity::Normal => NORMAL,
        Rarity::Magic => MAGIC,
        Rarity::Rare => RARE,
        Rarity::Boss => BOSS,
    };
    (format!("{kind} {}×{}", m.activity, m.quantity), colour)
}

/// The cards read, as a line: the hand from the left ("?" for one not read).
fn read_cards(screen: &ScreenState, size: usize, at: &[usize], missed: &Missed, zh: bool) -> Vec<String> {
    let unknown = if zh { ZH_UNKNOWN } else { "?" };
    let sep = if zh { ZH_SEP } else { " | " };
    let mut cards: Vec<String> = vec![unknown.to_string(); size];
    for (k, &slot) in at.iter().enumerate() {
        if let (Some(name), Some(c)) = (screen.hand.get(k), cards.get_mut(slot)) {
            *c = if zh { zh_name(screen.phase, name) } else { name.clone() };
        }
    }
    for (j, name) in &missed.unsupported {
        if let Some(c) = cards.get_mut(*j) {
            *c = name.clone();
        }
    }
    if size == 0 {
        return vec![];
    }
    let line = cards.iter().enumerate().map(|(i, t)| format!("{} {t}", i + 1)).collect::<Vec<_>>().join(sep);
    vec![format!("{}{line}", if zh { ZH_READ_CARDS } else { "cards: " })]
}

/// Not in upstream: the refreshes left as read, beside the refresh button ("刷新 2"; "刷新 ？" when not read sure,
/// which upstream counts as none left without a word: an instrument advised where a refresh was the better pick).
fn read_refresh(screen: &ScreenState, sure: bool, l: &Layout, zh: bool) -> Option<TankMark> {
    let left = match screen.phase {
        ScreenPhase::Potion => screen.potion_refreshes,
        ScreenPhase::Instrument => screen.instrument_refreshes,
        ScreenPhase::Protocol => return None,
    };
    let word = if zh { ZH_REROLL } else { "Rerolls" };
    let (text, colour) = match left {
        Some(n) if sure => (format!("{word} {n}"), NORMAL),
        _ => (format!("{word} {}", if zh { ZH_UNKNOWN } else { "?" }), UNREAD),
    };
    let (r, k) = (l.refresh, l.k);
    // right of the button, clear of the frame drawn round it when a refresh is advised (0.35 of its width out), about
    // level with its middle (a mark sits 6px under its rect, and is 28px tall)
    Some(TankMark { under: Rect::new(r.x + 1.35 * r.w + 12.0 * k, r.y + r.h / 2.0 - 20.0 * k, 120.0 * k, 0.0), text, colour })
}

/// The six tanks read, each as a mark under its label.
fn read_tanks(screen: &ScreenState, l: &Layout, missed: &Missed, zh: bool) -> Vec<TankMark> {
    (0..6)
        .map(|i| {
            let (text, colour) = if missed.tanks.contains(&i) {
                ((if zh { ZH_UNKNOWN } else { "?" }).to_string(), UNREAD)
            } else {
                match screen.tanks.get(i).and_then(|t| t.as_ref()) {
                    Some(m) => monster_mark(m, zh),
                    None => ((if zh { ZH_EMPTY } else { "empty" }).to_string(), EMPTY),
                }
            };
            TankMark { under: l.aq_band[i], text, colour }
        })
        .collect()
}

fn pct_in(x: f64, zh: bool) -> String {
    if zh {
        format!("{:.0}% {ZH_REACH} {}k", x * 100.0, vorax::value::TARGET / 1000)
    } else {
        pct(x)
    }
}

/// An option's end as the pill's second line: its chance of 840k, and in the aggressive mode (not in upstream) the
/// average final score it plays to too ("64% 達 840k · 平均 95 萬"), what that mode picks by.
fn end_in(e: &vorax::advisor::End, zh: bool) -> String {
    let p = pct_in(e.reached, zh);
    if !vorax::bot::mean_goal() {
        return p;
    }
    if zh {
        format!("{p} · {ZH_MEAN} {:.0} {ZH_TEN_THOUSAND}", e.activity / 10_000.0)
    } else {
        format!("{p} · mean {:.0}k", e.activity / 1000.0)
    }
}

/// A card's name as a Chinese client prints it (the first of its forms in names_zh.json), else as the data has it.
pub(crate) fn zh_name(phase: ScreenPhase, name: &str) -> String {
    let z = &vorax_scan::scan_data().names_zh;
    let names = match phase {
        ScreenPhase::Potion => &z.potion,
        ScreenPhase::Instrument => &z.instrument,
        ScreenPhase::Protocol => &z.protocol,
    };
    names.get(name).and_then(|f| f.first()).cloned().unwrap_or_else(|| {
        // a Dissection named by its title as read (`pipeline::dissection`): its characters may not be in the fonts
        if name.starts_with("Dissection: ") { ZH_DISSECTION.to_string() } else { name.to_string() }
    })
}

/// The marks for this advice on this board. `l`: the layout of the screen the overlay covers. `zh`: the hint in
/// Chinese, with the cards' Chinese names (a Chinese client; not in upstream).
pub fn plan_for(advice: &Advice, screen: &ScreenState, reading: &Reading, l: &Layout, zh: bool) -> Plan {
    let mut plan = Plan::default();
    let size = reading.hand_ranks.as_ref().map(|r| r.len()).filter(|&n| n > 0).or(reading.hand_size).unwrap_or(3);
    let at = slots(reading, size);
    let pos = l.hand_at(size);
    let tank = |t: i32| if zh { ZH_TANK.replace("{}", &(t + 1).to_string()) } else { format!("tank {}", t + 1) };
    match &advice.best {
        Best::Nothing => {}
        Best::Refresh => {
            let r = l.refresh;
            plan.button = Some(Rect::new(r.x - 0.35 * r.w, r.y - 0.45 * r.h, r.w * 1.7, r.h * 1.9));
            let sub = advice.refresh_end.map(|e| end_in(&e, zh)).unwrap_or_default();
            plan.pill = Some(Pill { main: if zh { ZH_REROLL.into() } else { "Reroll".into() }, sub, dim: false });
        }
        Best::Card(r) => {
            let name = match screen.hand.get(r.index) {
                Some(n) if zh => zh_name(screen.phase, n),
                Some(n) => n.clone(),
                None if zh => ZH_CARD.replace("{}", &(r.index + 1).to_string()),
                None => format!("Card {}", r.index + 1),
            };
            if let Some(&slot) = at.get(r.index) {
                if let Some(c) = pos.get(slot) {
                    plan.card = Some((*c, CARD_W * l.k, CARD_H * l.k));
                }
            }
            plan.tanks.extend(r.targets.iter().filter(|&&t| t < 6).map(|&t| l.tank_glass[t as usize]));
            let main = match r.targets.as_slice() {
                [] => name,
                [a] => format!("{name}  →  {}", tank(*a as i32)),
                ts if zh => format!("{name}  →  {}", ZH_TANK.replace("{}", &ts.iter().map(|t| (t + 1).to_string()).collect::<Vec<_>>().join(" + "))),
                ts => format!("{name}  →  tanks {}", ts.iter().map(|t| (t + 1).to_string()).collect::<Vec<_>>().join(" + ")),
            };
            let sub = match (&advice.note, r.end) {
                (Some(_), _) if zh => ZH_ALWAYS.into(),
                (Some(_), _) => "always take it".into(),
                (None, Some(e)) => end_in(&e, zh),
                (None, None) => String::new(),
            };
            plan.pill = Some(Pill { main, sub, dim: false });
        }
    }
    // not in upstream: what the reader couldn't make out, so the player knows to check it (read again by itself
    // a second later, up to twice; Ctrl+Alt+X reads again after that), and a card the data doesn't have (the advice
    // leaves it out: the player decides)
    let missed = not_read(reading);
    let mut notes: Vec<String> = Vec::new();
    for (j, name) in &missed.unsupported {
        notes.push(if zh { format!("{}「{name}」{ZH_NOT_IN_DATA}", ZH_CARD.replace("{}", &(j + 1).to_string())) } else { format!("card {} ({name}) not in the data: your call", j + 1) });
    }
    if !missed.is_empty() {
        let mut what: Vec<String> = Vec::new();
        what.extend(missed.cards.iter().map(|j| if zh { ZH_CARD.replace("{}", &(j + 1).to_string()) } else { format!("card {}", j + 1) }));
        what.extend(missed.tanks.iter().map(|i| tank(*i as i32)));
        notes.push(if zh { format!("{ZH_NOT_READ}{}（Ctrl+Alt+X）", what.join("、")) } else { format!("not read: {} (Ctrl+Alt+X)", what.join(", ")) });
    }
    if !notes.is_empty() {
        let warn = notes.join(" · ");
        match &mut plan.pill {
            Some(p) => p.sub = if p.sub.is_empty() { warn } else { format!("{} · {warn}", p.sub) },
            None => plan.pill = Some(Pill { main: if zh { ZH_NOT_FULLY.into() } else { "Board not fully read".into() }, sub: warn, dim: false }),
        }
    }
    plan.read = read_cards(screen, size, &at, &missed, zh);
    // over the middle of the cards (the banner's words sit there), right above the banner
    let mid = if pos.is_empty() { l.header.x + l.header.w / 2.0 } else { pos.iter().map(|c| c.cx).sum::<f64>() / pos.len() as f64 };
    plan.read_at = Some((mid, l.header.y));
    plan.read_tanks = read_tanks(screen, l, &missed, zh);
    plan.read_tanks.extend(read_refresh(screen, reading.fields.get("refreshes").is_some_and(|f| f.sure), l, zh));
    if screen.phase == ScreenPhase::Protocol {
        plan = Plan::default();
    }
    plan
}

/// What the reader couldn't make out on a board, as the player counts them (from 0): cards and tanks not read, and
/// cards read as one upstream's data doesn't have (names_zh.json "unsupported"), with the name as printed.
#[derive(Debug, Default)]
pub struct Missed {
    pub cards: Vec<usize>,
    pub tanks: Vec<usize>,
    pub unsupported: Vec<(usize, String)>,
}

impl Missed {
    /// Nothing that reading again could make out (a card not in the data stays so).
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty() && self.tanks.is_empty()
    }
}

pub fn not_read(reading: &Reading) -> Missed {
    let mut m = Missed::default();
    for p in &reading.problems {
        match p {
            Problem::UnknownArt { field, text, .. } => {
                if let Some(j) = field.strip_prefix("hand").and_then(|j| j.parse::<usize>().ok()) {
                    match text.as_deref().and_then(|t| vorax_scan::titles::han_unsupported(vorax_scan::scan_data(), t)) {
                        Some(name) if !m.unsupported.iter().any(|(k, _)| *k == j) => m.unsupported.push((j, name)),
                        Some(_) => {}
                        None if !m.cards.contains(&j) => m.cards.push(j),
                        None => {}
                    }
                }
            }
            Problem::Unreadable { field } => {
                if let Some(i) = field.strip_prefix("tank").and_then(|r| r.split('.').next()).and_then(|i| i.parse::<usize>().ok()) {
                    if !m.tanks.contains(&i) {
                        m.tanks.push(i);
                    }
                }
            }
            _ => {}
        }
    }
    m.cards.sort_unstable();
    m.tanks.sort_unstable();
    m.unsupported.sort();
    m
}

/// A typeface with fallbacks (not in upstream): each character drawn in the first of its fonts that has it. The
/// Chinese edition's: Segoe UI from Windows, then the built-in Chinese fonts (Windows' Segoe UI has no Chinese).
pub struct Face {
    fonts: Vec<fontdue::Font>,
}

impl Face {
    pub fn new(fonts: Vec<fontdue::Font>) -> Face {
        Face { fonts }
    }

    /// Not in upstream: whether one of its fonts has every character of `t` (spaces and line ends aside).
    pub fn covers(&self, t: &str) -> bool {
        t.chars().filter(|c| !c.is_whitespace()).all(|c| self.fonts.iter().any(|f| f.lookup_glyph_index(c) != 0))
    }

    /// The same, with more fonts after its own.
    fn and(&self, more: Option<fontdue::Font>) -> Face {
        Face { fonts: self.fonts.iter().cloned().chain(more).collect() }
    }

    /// The font a character is drawn in.
    fn of(&self, c: char) -> &fontdue::Font {
        self.fonts.iter().find(|f| f.lookup_glyph_index(c) != 0).unwrap_or(&self.fonts[0])
    }
}

pub struct Fonts {
    pub bold: Face,
    pub regular: Face,
}

/// Subsets of Noto Sans TC and SC (SIL OFL 1.1), cut to what the overlay can show (dev/make_font_zh.py).
static ZH_BOLD: &[u8] = include_bytes!("../../../data/fonts/zh-bold.ttf");
static ZH_REGULAR: &[u8] = include_bytes!("../../../data/fonts/zh-regular.ttf");
static SC_BOLD: &[u8] = include_bytes!("../../../data/fonts/sc-bold.ttf");
static SC_REGULAR: &[u8] = include_bytes!("../../../data/fonts/sc-regular.ttf");

fn built_in(b: &[u8]) -> Option<fontdue::Font> {
    fontdue::Font::from_bytes(b, fontdue::FontSettings::default()).ok()
}

impl Fonts {
    /// Segoe UI from Windows (Arial where it's missing), then the built-in Chinese fonts.
    pub fn system() -> Option<Fonts> {
        let dir = std::env::var("WINDIR").map(|w| format!("{w}\\Fonts")).unwrap_or_else(|_| "C:\\Windows\\Fonts".into());
        let load = |names: &[&str]| {
            names.iter().find_map(|n| std::fs::read(format!("{dir}\\{n}")).ok()).and_then(|b| fontdue::Font::from_bytes(b, fontdue::FontSettings::default()).ok())
        };
        let face = |windows: Option<fontdue::Font>, zh: &[u8], sc: &[u8]| {
            let fonts: Vec<fontdue::Font> = [windows, built_in(zh), built_in(sc)].into_iter().flatten().collect();
            (!fonts.is_empty()).then(|| Face::new(fonts))
        };
        Some(Fonts { bold: face(load(&["segoeuib.ttf", "arialbd.ttf"]), ZH_BOLD, SC_BOLD)?, regular: face(load(&["segoeui.ttf", "arial.ttf"]), ZH_REGULAR, SC_REGULAR)? })
    }

    /// Not in upstream: whether these fonts draw every character of each text.
    pub fn cover(&self, texts: &[&str]) -> bool {
        texts.iter().all(|t| self.regular.covers(t) && self.bold.covers(t))
    }

    /// Not in upstream: the same, then Windows' own Chinese font (Microsoft YaHei, else JhengHei, else SimSun) for
    /// text the built-in subsets don't hold, as a release's notes do (their characters showed as boxes). Large and
    /// slow to load: only for such text, and dropped with it.
    pub fn with_windows_chinese(&self) -> Fonts {
        let dir = std::env::var("WINDIR").map(|w| format!("{w}\\Fonts")).unwrap_or_else(|_| "C:\\Windows\\Fonts".into());
        let load = |names: &[&str]| {
            names.iter().find_map(|n| std::fs::read(format!("{dir}\\{n}")).ok()).and_then(|b| fontdue::Font::from_bytes(b, fontdue::FontSettings { collection_index: 0, ..Default::default() }).ok())
        };
        Fonts { bold: self.bold.and(load(&["msyhbd.ttc", "msjhbd.ttc", "msyh.ttc", "msjh.ttc", "simsun.ttc"])), regular: self.regular.and(load(&["msyh.ttc", "msjh.ttc", "simsun.ttc"])) }
    }
}

pub(crate) fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
    Color::from_rgba8(r, g, b, (a * 255.0) as u8)
}

pub(crate) const GOLD: (u8, u8, u8) = (255, 214, 138);
pub(crate) const GOLD_DIM: (u8, u8, u8) = (227, 178, 90);

pub(crate) fn paint(c: Color) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    p
}

pub(crate) fn rounded(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let r = r.min(w / 2.0).min(h / 2.0);
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}

/// The marks' colours to choose from (the tray menu): bright ones that stand out on the game's red and gold.
pub const ACCENTS: [(&str, (u8, u8, u8)); 5] = [
    ("青色", (64, 232, 255)),
    ("綠色", (90, 255, 140)),
    ("洋紅", (255, 92, 214)),
    ("白色", (255, 255, 255)),
    ("金色", (255, 214, 138)),
];

pub fn accent(i: usize) -> (u8, u8, u8) {
    ACCENTS.get(i).unwrap_or(&ACCENTS[0]).1
}

/// An outline that reads on any background: a dark edge under it, a soft glow and a bright line in the accent.
fn glow(pm: &mut Pixmap, path: &tiny_skia::Path, t: Transform, s: f32, (r, g, b): (u8, u8, u8)) {
    for (w, a) in [(22.0, 0.08), (13.0, 0.16)] {
        pm.stroke_path(path, &paint(rgba(r, g, b, a)), &Stroke { width: w * s, ..Default::default() }, t, None);
    }
    pm.stroke_path(path, &paint(rgba(8, 6, 8, 0.85)), &Stroke { width: 8.0 * s, ..Default::default() }, t, None);
    pm.stroke_path(path, &paint(rgba(r, g, b, 1.0)), &Stroke { width: 4.0 * s, ..Default::default() }, t, None);
}


fn diamond_path(x: f32, y: f32, r: f32) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    pb.move_to(x, y - r);
    pb.line_to(x + r * 0.8, y);
    pb.line_to(x, y + r);
    pb.line_to(x - r * 0.8, y);
    pb.close();
    pb.finish()
}

/// Text width at a size, with kerning (in Simplified when the overlay shows Simplified: zh::shown).
pub(crate) fn measure(font: &Face, text: &str, px: f32) -> f32 {
    let text = crate::zh::shown(text);
    let mut w = 0.0;
    let mut prev: Option<(char, *const fontdue::Font)> = None;
    for ch in text.chars() {
        let f = font.of(ch);
        if let Some((p, _)) = prev.filter(|(_, pf)| std::ptr::eq(*pf, f)) {
            w += f.horizontal_kern(p, ch, px).unwrap_or(0.0);
        }
        w += f.metrics(ch, px).advance_width;
        prev = Some((ch, f as *const _));
    }
    w
}

/// Draws text with its baseline at y, alpha-blended into the premultiplied picture (in Simplified when the
/// overlay shows Simplified: zh::shown).
pub(crate) fn text(pm: &mut Pixmap, font: &Face, s: &str, px: f32, x: f32, baseline: f32, (r, g, b): (u8, u8, u8), alpha: f32) {
    let s = crate::zh::shown(s);
    let (w, h) = (pm.width() as i32, pm.height() as i32);
    let data = pm.data_mut();
    let mut pen = x;
    let mut prev: Option<(char, *const fontdue::Font)> = None;
    for ch in s.chars() {
        let f = font.of(ch);
        if let Some((p, _)) = prev.filter(|(_, pf)| std::ptr::eq(*pf, f)) {
            pen += f.horizontal_kern(p, ch, px).unwrap_or(0.0);
        }
        let (m, cov) = f.rasterize(ch, px);
        let gx = (pen + m.xmin as f32).round() as i32;
        let gy = (baseline - m.height as f32 - m.ymin as f32).round() as i32;
        for yy in 0..m.height as i32 {
            for xx in 0..m.width as i32 {
                let (px_x, px_y) = (gx + xx, gy + yy);
                if px_x < 0 || px_y < 0 || px_x >= w || px_y >= h {
                    continue;
                }
                let a = cov[(yy * m.width as i32 + xx) as usize] as f32 / 255.0 * alpha;
                if a <= 0.0 {
                    continue;
                }
                let i = ((px_y * w + px_x) * 4) as usize;
                for (k, c) in [r, g, b].iter().enumerate() {
                    data[i + k] = (*c as f32 * a + data[i + k] as f32 * (1.0 - a)).round() as u8;
                }
                data[i + 3] = (255.0 * a + data[i + 3] as f32 * (1.0 - a)).round() as u8;
            }
        }
        pen += m.advance_width;
        prev = Some((ch, f as *const _));
    }
}

/// The overlay for a screen of `w`×`h` pixels, transparent everywhere it draws nothing (premultiplied RGBA).
/// `accent`: the marks' colour.
pub fn render(plan: &Plan, w: u32, h: u32, fonts: &Fonts, accent: (u8, u8, u8)) -> Pixmap {
    let mut pm = Pixmap::new(w.max(1), h.max(1)).unwrap();
    // sizes follow the screen's height, as the game's UI does (1440p is 1)
    let s = (h as f32 / 1440.0).max(0.6);

    if let Some((c, cw, ch)) = plan.card {
        let t = Transform::from_rotate(c.angle.to_degrees() as f32).post_translate(c.cx as f32, c.cy as f32);
        let pad = 8.0 * s;
        let (hw, hh) = (cw as f32 / 2.0 + pad, ch as f32 / 2.0 + pad);
        if let Some(p) = rounded(-hw, -hh, 2.0 * hw, 2.0 * hh, 16.0 * s) {
            glow(&mut pm, &p, t, s, accent);
        }
    }
    if let Some(b) = plan.button {
        if let Some(p) = rounded(b.x as f32, b.y as f32, b.w as f32, b.h as f32, 12.0 * s) {
            glow(&mut pm, &p, Transform::identity(), s, accent);
        }
    }
    // a tank to target: its glass tinted in the accent and outlined like the card, so it reads at a glance
    for g in &plan.tanks {
        let pad = 6.0 * s;
        if let Some(p) = rounded(g.x as f32 - pad, g.y as f32 - pad, g.w as f32 + 2.0 * pad, g.h as f32 + 2.0 * pad, 18.0 * s) {
            let (r, gg, b) = accent;
            pm.fill_path(&p, &paint(rgba(r, gg, b, 0.16)), FillRule::Winding, Transform::identity(), None);
            glow(&mut pm, &p, Transform::identity(), s, accent);
        }
    }

    if let Some(pill) = &plan.pill {
        let (big, small) = (23.0 * s, 18.0 * s);
        let gap = 16.0 * s;
        let mark = 12.0 * s;
        // "33% reach 840k": the share in the accent, the rest in light grey
        let (odds, rest) = match pill.sub.split_once(' ') {
            Some((a, b)) if a.ends_with('%') => (a.to_string(), format!(" {b}")),
            _ => (String::new(), pill.sub.clone()),
        };
        let (bold, regular) = (&fonts.bold, &fonts.regular);
        let tw = measure(bold, &pill.main, big);
        let ow = measure(bold, &odds, small);
        let rw = measure(regular, &rest, small);
        let sw = if pill.sub.is_empty() { 0.0 } else { gap + ow + rw };
        let (ph, padx) = (46.0 * s, 20.0 * s);
        let pw = padx * 2.0 + mark + 12.0 * s + tw + sw;
        let (px, py) = ((w as f32 - pw) / 2.0, 14.0 * s);
        let alpha = if pill.dim { 0.55 } else { 1.0 };
        let (ar, ag, ab) = accent;
        if let Some(p) = rounded(px, py, pw, ph, ph / 2.0) {
            pm.fill_path(&p, &paint(rgba(10, 8, 10, 0.92 * alpha)), FillRule::Winding, Transform::identity(), None);
            pm.stroke_path(&p, &paint(rgba(ar, ag, ab, 0.75 * alpha)), &Stroke { width: 1.8 * s, ..Default::default() }, Transform::identity(), None);
        }
        if let Some(p) = diamond_path(px + padx + mark / 2.0, py + ph / 2.0, mark / 2.0) {
            pm.fill_path(&p, &paint(rgba(ar, ag, ab, alpha)), FillRule::Winding, Transform::identity(), None);
        }
        let base = py + ph / 2.0 + big * 0.35;
        let mut x = px + padx + mark + 12.0 * s;
        text(&mut pm, bold, &pill.main, big, x, base, (255, 255, 255), alpha);
        x += tw + gap;
        if !pill.sub.is_empty() {
            text(&mut pm, bold, &odds, small, x, base, accent, alpha);
            text(&mut pm, regular, &rest, small, x + ow, base, (214, 206, 202), alpha);
        }
    }
    // not in upstream: what was read, just above the banner over the cards (else under the pill), in a box of its
    // own
    if !plan.read.is_empty() {
        let refs: Vec<&str> = plan.read.iter().map(String::as_str).collect();
        let regular = &fonts.regular;
        let px_ = 16.0 * s;
        let (line_h, padx, pady) = (24.0 * s, 14.0 * s, 8.0 * s);
        let bw = refs.iter().map(|t| measure(regular, t, px_)).fold(0.0f32, f32::max) + 2.0 * padx;
        let bh = line_h * refs.len() as f32 + 2.0 * pady;
        let under_pill = 14.0 * s + 46.0 * s + 8.0 * s;
        let by = plan.read_at.map_or(under_pill, |(_, y)| (y as f32 - bh - 6.0 * s).max(under_pill));
        let bx = plan.read_at.map_or((w as f32 - bw) / 2.0, |(x, _)| (x as f32 - bw / 2.0).clamp(0.0, (w as f32 - bw).max(0.0)));
        let alpha = if plan.pill.as_ref().is_some_and(|p| p.dim) { 0.55 } else { 1.0 };
        if let Some(p) = rounded(bx, by, bw, bh, 10.0 * s) {
            pm.fill_path(&p, &paint(rgba(10, 8, 10, 0.82 * alpha)), FillRule::Winding, Transform::identity(), None);
        }
        for (k, t) in refs.iter().enumerate() {
            text(&mut pm, regular, t, px_, bx + padx, by + pady + line_h * (k as f32 + 0.75), (214, 206, 202), alpha);
        }
    }
    // not in upstream: each tank as read, under its label, in its rarity's colour
    if !plan.read_tanks.is_empty() {
        let bold = &fonts.bold;
        let px_ = 19.0 * s;
        let (padx, bh) = (8.0 * s, 28.0 * s);
        let alpha = if plan.pill.as_ref().is_some_and(|p| p.dim) { 0.55 } else { 1.0 };
        for m in &plan.read_tanks {
            let tw = measure(bold, &m.text, px_);
            let bw = tw + 2.0 * padx;
            let (cx, top) = ((m.under.x + m.under.w / 2.0) as f32, (m.under.y + m.under.h) as f32 + 6.0 * s);
            if let Some(p) = rounded(cx - bw / 2.0, top, bw, bh, 7.0 * s) {
                pm.fill_path(&p, &paint(rgba(10, 8, 10, 0.85 * alpha)), FillRule::Winding, Transform::identity(), None);
            }
            text(&mut pm, bold, &m.text, px_, cx - tw / 2.0, top + bh * 0.72, m.colour, alpha);
        }
    }
    pm
}

#[cfg(test)]
mod zh_tests {
    use super::*;

    /// The built-in fonts alone (no Windows here), as faces: Traditional then Simplified.
    fn built_in_fonts() -> Fonts {
        let f = |b: &[u8]| fontdue::Font::from_bytes(b, fontdue::FontSettings::default()).unwrap();
        Fonts { bold: Face::new(vec![f(ZH_BOLD), f(SC_BOLD)]), regular: Face::new(vec![f(ZH_REGULAR), f(SC_REGULAR)]) }
    }

    /// Every Chinese string in the overlay's source (its words: the hint, the menus, the runs window).
    fn source_words() -> Vec<String> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut out = Vec::new();
        let mut files = vec![dir.clone()];
        while let Some(p) = files.pop() {
            if p.is_dir() {
                files.extend(std::fs::read_dir(&p).unwrap().flatten().map(|e| e.path()));
            } else if p.extension().is_some_and(|e| e == "rs") {
                let src = std::fs::read_to_string(&p).unwrap();
                for lit in src.split('"').skip(1).step_by(2) {
                    if lit.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c)) {
                        out.push(lit.to_string());
                    }
                }
            }
        }
        out
    }

    #[test]
    fn text_beyond_the_built_in_fonts_is_told() {
        // a release's notes: what the built-in subsets hold, and a character they don't (it showed as a box)
        let f = built_in_fonts();
        assert!(f.cover(&["渴癮症助手 v2.0.1", "Update now"]));
        assert!(!f.cover(&["\u{9F98}"]));
        // with no Windows fonts here, the same fonts again
        assert!(!f.with_windows_chinese().cover(&["\u{9F98}"]));
    }

    /// Every character the overlay can show is in the built-in fonts, in Traditional and in Simplified (else: run
    /// dev/make_font_zh.py again).
    #[test]
    fn the_chinese_fonts_have_every_character_shown() {
        let z = &vorax_scan::scan_data().names_zh;
        let mut texts = source_words();
        assert!(texts.iter().any(|t| t.contains("立即讀取畫面")), "the tray's words are among the source's");
        texts.push("→·×0123456789% +Ctrl+Alt+Xk".into());
        // the names shown: each card's first form, and the unsupported ones as they're recognized
        for m in [&z.potion, &z.instrument, &z.protocol] {
            texts.extend(m.values().filter_map(|f| f.first()).cloned());
        }
        texts.extend(z.unsupported.iter().filter_map(|n| vorax_scan::titles::han_unsupported(vorax_scan::scan_data(), n)));
        let fonts = built_in_fonts();
        for t in texts.iter().flat_map(|t| [t.clone(), crate::zh::to_simplified(t)]) {
            for c in t.chars().filter(|c| !c.is_whitespace() && !c.is_control()) {
                for face in [&fonts.bold, &fonts.regular] {
                    assert!(face.of(c).lookup_glyph_index(c) != 0, "{c} ({t}) is not in data/fonts: run dev/make_font_zh.py");
                }
            }
        }
    }

    #[test]
    fn a_card_is_named_as_the_chinese_client_prints_it() {
        assert_eq!(zh_name(ScreenPhase::Instrument, "Sinew Garrote"), "獸筋絞肉索");
        assert_eq!(zh_name(ScreenPhase::Potion, "Slender-Limb Powder - Pest"), "細肢藥粉-蠱蟲");
        assert_eq!(zh_name(ScreenPhase::Potion, "Not A Card"), "Not A Card");
    }

    /// VORAX_PILL_PNG=<dir>: the Chinese pills drawn there, to look at.
    #[test]
    fn chinese_pills_draw() {
        let fonts = built_in_fonts();
        let pills = [
            Pill { main: "細肢藥粉-蠱蟲  →  第 3 格".into(), sub: "64% 達 840k".into(), dim: false },
            Pill { main: "獸筋絞肉索  →  第 2 + 5 格".into(), sub: "41% 達 840k · 未讀到：第 2 張卡、第 3 格（Ctrl+Alt+X）".into(), dim: false },
            Pill { main: "第一儀式聖水：告解".into(), sub: "出現就選 · 第 3 張卡「解剖浸液：病軀」不在資料內，請自行判斷".into(), dim: false },
        ];
        for (i, p) in pills.iter().enumerate() {
            let plan = Plan { pill: Some(p.clone()), ..Default::default() };
            let pm = render(&plan, 1600, 120, &fonts, (64, 232, 255));
            assert!(pm.data().chunks(4).any(|px| px[3] > 0));
            if let Ok(dir) = std::env::var("VORAX_PILL_PNG") {
                crate::pipeline::encode_png(&std::path::Path::new(&dir).join(format!("pill{i}.png")), pm.width(), pm.height(), pm.data()).unwrap();
                // and as a Simplified client sees it
                crate::zh::set_mode(crate::zh::SIMPLIFIED);
                let pm = render(&plan, 1600, 120, &fonts, (64, 232, 255));
                crate::zh::set_mode(crate::zh::AUTO);
                crate::pipeline::encode_png(&std::path::Path::new(&dir).join(format!("pill{i}s.png")), pm.width(), pm.height(), pm.data()).unwrap();
            }
        }
    }

    #[test]
    fn cards_not_read_are_listed_to_read_again() {
        let data = vorax_scan::scan_data();
        let mut r = vorax_scan::scan(&vorax_scan::pixels::Img::new(64, 64), data, false);
        let art = |j: usize, text: Option<&str>| Problem::UnknownArt { field: format!("hand{j}"), crop: Rect::new(0.0, 0.0, 1.0, 1.0), hand: None, text: text.map(String::from) };
        r.problems = vec![art(0, Some("細肢藥粉-")), art(1, None), Problem::Unreadable { field: "tank4.total".into() }];
        let m = not_read(&r);
        assert_eq!((m.cards.clone(), m.tanks.clone(), m.unsupported.len()), (vec![0, 1], vec![4], 0));
        assert!(!m.is_empty());
    }

    #[test]
    fn a_dissection_without_a_chinese_name_is_shown_as_one() {
        assert_eq!(zh_name(ScreenPhase::Potion, "Dissection: 解剖一：病蟲"), ZH_DISSECTION);
        assert_eq!(zh_name(ScreenPhase::Potion, "Dissection Infusion: Morbid Corpse"), "解剖浸液：病軀");
    }

    #[test]
    fn what_was_read_is_the_cards_in_a_line_and_a_mark_per_tank() {
        let state: ScreenState = serde_json::from_str(r#"{"tanks":[{"archetype":"boneSentry","rarity":"rare","activity":15,"quantity":12},null,{"archetype":"aemberon","rarity":"normal","activity":1,"quantity":36},null,null,{"archetype":"pest","rarity":"boss","activity":229,"quantity":1102}],"hand":["Hollow Spinal Solution","Eggshell Powder"],"instruments":[],"phase":"potion","round":2}"#).unwrap();
        let missed = Missed { cards: vec![1], tanks: vec![3], unsupported: vec![] };
        assert_eq!(read_cards(&state, 3, &[0, 2], &missed, true), vec!["卡：1 空心脊髓溶液｜2 ？｜3 卵殼藥粉".to_string()]);
        let l = vorax_scan::layout::layout_at(2560, 1440, 1.0);
        let marks = read_tanks(&state, &l, &missed, true);
        let got: Vec<(&str, (u8, u8, u8))> = marks.iter().map(|m| (m.text.as_str(), m.colour)).collect();
        assert_eq!(got, vec![("骨 15×12", RARE), ("空", EMPTY), ("異 1×36", NORMAL), ("？", UNREAD), ("空", EMPTY), ("蟲 229×1102", BOSS)]);
        // under each tank's own label, left to right
        assert!(marks.windows(2).all(|w| w[0].under.x < w[1].under.x));
    }

    #[test]
    fn the_refreshes_read_show_beside_the_button_and_a_count_not_read_shows_as_such() {
        let mut state: ScreenState = serde_json::from_str(r#"{"tanks":[null,null,null,null,null,null],"hand":[],"instruments":[],"phase":"instrument","round":2,"potionRefreshes":3,"instrumentRefreshes":1}"#).unwrap();
        let l = vorax_scan::layout::layout_at(2560, 1440, 1.0);
        let m = read_refresh(&state, true, &l, true).unwrap();
        assert_eq!((m.text.as_str(), m.colour), ("刷新 1", NORMAL));
        assert!(m.under.x > l.refresh.x + l.refresh.w, "right of the button");
        state.phase = ScreenPhase::Potion;
        assert_eq!(read_refresh(&state, true, &l, false).unwrap().text, "Rerolls 3");
        // not read sure: upstream counts it as none left, shown as not read
        assert_eq!(read_refresh(&state, false, &l, true).map(|m| (m.text, m.colour)), Some(("刷新 ？".to_string(), UNREAD)));
        state.phase = ScreenPhase::Protocol;
        assert!(read_refresh(&state, false, &l, true).is_none());
    }

    /// VORAX_FRAME=<png> VORAX_PILL_PNG=<dir>: that frame read (cards by their pictures) and drawn over, to look at.
    #[test]
    fn a_frame_drawn_with_what_was_read() {
        let (Ok(frame), Ok(dir)) = (std::env::var("VORAX_FRAME"), std::env::var("VORAX_PILL_PNG")) else { return };
        let img = crate::pipeline::decode_png(std::path::Path::new(&frame)).unwrap();
        let mut pipe = crate::pipeline::Pipeline::new(6);
        pipe.script = vorax_scan::titles::Script::Han;
        let crate::pipeline::Outcome::Advice { plan, .. } = pipe.run(&img, Some("board"), None) else { panic!("not a board") };
        let fonts = built_in_fonts();
        let mut pm = render(&plan, img.width as u32, img.height as u32, &fonts, (64, 232, 255));
        // over the frame, to see it as on screen
        let mut out = img.data.clone();
        for (o, p) in out.chunks_mut(4).zip(pm.data_mut().chunks(4)) {
            let a = p[3] as u32;
            for c in 0..3 {
                o[c] = (p[c] as u32 + o[c] as u32 * (255 - a) / 255).min(255) as u8;
            }
        }
        crate::pipeline::encode_png(&std::path::Path::new(&dir).join("frame.png"), img.width as u32, img.height as u32, &out).unwrap();
    }
}
