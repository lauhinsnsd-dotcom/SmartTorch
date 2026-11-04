//! One tank from its label (src/solver/scan/tanks.ts): empty or not, rarity from the name's colour, archetype
//! from the icon above it, and Activity × Quantity, which must multiply to the total shown above it.

use crate::data::ScanData;
use indexmap::IndexMap;
use crate::layout::Layout;
use crate::pixels::{hsv, Img, Rect};
use crate::reading::{FieldRead, Fields, TankDebug};
use crate::text::{brightest, read_text, TextKind, TextRead};
use vorax::advisor::ScreenMonster;
use vorax::types::{Archetype, Rarity};

const NAME_MIN: f64 = 105.0;
const DIGIT_MIN: f64 = 0.75;
/// not in upstream: the least a read may score when both its lines, as read, multiply up to a total of three digits
/// or more (exact products of that size don't come out of noise)
const DIGIT_MIN_EXACT: f64 = 0.70;
const MAX_STAT: f64 = 1_000_000.0;
/// Where else a label's lines are looked for (reference pixels), nearest first.
const LABEL_SHIFTS: [f64; 10] = [3.0, -3.0, 6.0, -6.0, 9.0, -9.0, 12.0, -12.0, 15.0, -15.0];

/// JavaScript's Number() of a string of digits: "" is 0, anything else not a number is NaN.
pub(crate) fn js_number(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
    }
    if t.bytes().all(|b| b.is_ascii_digit()) {
        return t.parse::<f64>().unwrap_or(f64::NAN);
    }
    f64::NAN
}

/// Activity × Quantity from their line, kept within the game, and whether they multiply to the total.
pub fn stats(total: Option<&str>, aq: Option<&str>) -> (f64, f64, bool) {
    let parts: Vec<&str> = aq.unwrap_or("").split('x').collect();
    let (a, q) = if parts.len() == 2 { (js_number(parts[0]), js_number(parts[1])) } else { (f64::NAN, f64::NAN) };
    let ok = |n: f64| n.fract() == 0.0 && (1.0..=MAX_STAT).contains(&n);
    if !ok(a) || !ok(q) {
        return (1.0, 1.0, false);
    }
    let t = total.map_or(f64::NAN, js_number);
    (a, q, a * q == t)
}

/// An Activity × Quantity line read without its "x": the one place an "x" could stand that makes the total.
pub fn x_misread(total: &str, aq: &str) -> Option<String> {
    if aq.contains('x') || aq.len() < 3 || !aq.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let fits: Vec<String> = (1..aq.len() - 1).map(|p| format!("{}x{}", &aq[..p], &aq[p + 1..])).filter(|line| stats(Some(total), Some(line)).2).collect();
    (fits.len() == 1).then(|| fits[0].clone())
}

/// Archetype from the little icon above a tank's name, by colour.
/// Pixels of each icon colour in a rect: Bone Sentry's gold, Pest's purple, Aemberon's blue.
fn archetype_counts(img: &Img, r: &Rect) -> [(Archetype, usize); 3] {
    let mut n = [(Archetype::BoneSentry, 0usize), (Archetype::Pest, 0), (Archetype::Aemberon, 0)];
    let x0 = r.x.floor().max(0.0) as usize;
    let y0 = r.y.floor().max(0.0) as usize;
    let mut y = y0;
    while (y as f64) < (img.height as f64).min(r.y + r.h) {
        let mut x = x0;
        while (x as f64) < (img.width as f64).min(r.x + r.w) {
            let (rr, gg, bb) = img.px(x, y);
            let (h, s, v) = hsv(rr, gg, bb);
            if v >= 0.45 {
                if (35.0..=65.0).contains(&h) && s >= 0.5 {
                    n[0].1 += 1;
                } else if (255.0..=305.0).contains(&h) && s >= 0.5 {
                    n[1].1 += 1;
                } else if (190.0..=240.0).contains(&h) && s >= 0.25 {
                    n[2].1 += 1;
                }
            }
            x += 1;
        }
        y += 1;
    }
    n
}

pub fn archetype_of(img: &Img, r: &Rect) -> (Archetype, bool) {
    let n = archetype_counts(img, r);
    let mut sorted = n;
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    let (best, count) = sorted[0];
    if count >= 4 {
        (best, true)
    } else if count >= 2 {
        (best, false)
    } else {
        (Archetype::Agito, true)
    }
}

/// Not in upstream: the icon found over the monster's name, for window shapes the scene wasn't measured on (the
/// tanks there sit a little off the blend of the 16:9 and 21:9 measurements, enough to miss a small icon). The
/// name's middle is taken from its bright pixels; the icon is looked for above it, a little lower first.
fn icon_over_name(img: &Img, l: &Layout, i: usize) -> Option<Rect> {
    let nb = l.name_band[i];
    let (x0, x1) = (nb.x.max(0.0) as usize, ((nb.x + nb.w) as usize).min(img.width));
    let (y0, y1) = (nb.y.max(0.0) as usize, ((nb.y + nb.h) as usize).min(img.height));
    let mid = (nb.x + nb.w / 2.0) as isize;
    let cols: Vec<usize> = (x0..x1)
        .filter(|&x| {
            (y0..y1)
                .filter(|&y| {
                    let (r, g, b) = img.px(x, y);
                    0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64 >= 95.0
                })
                .count()
                >= 2
        })
        .collect();
    // the run of name columns nearest the band's middle (gaps between characters bridged)
    let gap = (12.0 * l.k) as usize;
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for &x in &cols {
        match runs.last_mut() {
            Some(r) if x <= r.1 + gap => r.1 = x,
            _ => runs.push((x, x)),
        }
    }
    let near = runs.into_iter().min_by_key(|r| ((r.0 + r.1) as isize / 2 - mid).abs())?;
    if ((near.1 - near.0) as f64) < 20.0 * l.k {
        return None;
    }
    let cx = (near.0 + near.1) as f64 / 2.0;
    let icon = l.icon[i];
    let size = icon.w;
    let mut best: Option<(Rect, usize)> = None;
    for dy in [12.0, 8.0, 16.0, 4.0, 0.0] {
        let r = Rect::new(cx - size / 2.0, icon.y + dy * l.k, size, size * 0.85);
        let n = archetype_counts(img, &r).iter().map(|c| c.1).max().unwrap_or(0);
        if best.as_ref().is_none_or(|b| n > b.1) {
            best = Some((r, n));
        }
    }
    best.map(|b| b.0)
}

/// Whether a rect holds an icon: one of the families' colours (as archetype_of counts them) or Agito's red.
fn has_icon(img: &Img, r: &Rect) -> bool {
    let coloured: usize = archetype_counts(img, r).iter().map(|c| c.1).sum();
    let mut red = 0usize;
    let (x0, y0) = (r.x.max(0.0) as usize, r.y.max(0.0) as usize);
    let (x1, y1) = (((r.x + r.w) as usize).min(img.width), ((r.y + r.h) as usize).min(img.height));
    for y in y0..y1 {
        for x in x0..x1 {
            let (rr, gg, bb) = img.px(x, y);
            let (h, s, v) = hsv(rr, gg, bb);
            if v >= 0.45 && s >= 0.5 && (h < 15.0 || h > 340.0) {
                red += 1;
            }
        }
    }
    coloured >= 4 || red >= 4
}

/// Not in upstream: the icon looked for strictly above the monster's name. A Chinese client's names stand taller
/// than English ones, so on some window shapes the layout's icon box takes in the name's top, and a Rare name's
/// gold (or a Magic name's blue) passes for a Bone Sentry's (or an Aemberon's) icon. The name's columns and its top
/// row are found from its bright pixels; the icon is taken from the rows just above that top.
fn icon_above_name(img: &Img, l: &Layout, i: usize) -> Option<Rect> {
    let nb = l.name_band[i];
    let size = l.icon[i].w;
    let bright = |x: usize, y: usize| {
        let (r, g, b) = img.px(x, y);
        0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64 >= 95.0
    };
    let (x0, x1) = (nb.x.max(0.0) as usize, ((nb.x + nb.w) as usize).min(img.width));
    let (y0, y1) = (nb.y.max(0.0) as usize, ((nb.y + nb.h) as usize).min(img.height));
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    // the name's columns: the run of bright columns nearest the band's middle (gaps between characters bridged)
    let cols: Vec<usize> = (x0..x1).filter(|&x| (y0..y1).filter(|&y| bright(x, y)).count() >= 2).collect();
    let gap = (12.0 * l.k) as usize;
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for &x in &cols {
        match runs.last_mut() {
            Some(r) if x <= r.1 + gap => r.1 = x,
            _ => runs.push((x, x)),
        }
    }
    let mid = (nb.x + nb.w / 2.0) as isize;
    let name = runs.into_iter().min_by_key(|r| ((r.0 + r.1) as isize / 2 - mid).abs())?;
    if ((name.1 - name.0) as f64) < 20.0 * l.k {
        return None;
    }
    // the name's top row: up from the band's middle while its columns hold bright pixels (two dark rows end it)
    let lit = |y: usize| (name.0..=name.1).filter(|&x| bright(x, y)).count() >= 2;
    let lowest = (nb.y - 2.0 * size).max(0.0) as usize;
    let mut y = ((nb.y + nb.h / 2.0) as usize).min(img.height - 1);
    let mut top = y;
    let mut dark = 0;
    while y > lowest {
        y -= 1;
        if lit(y) {
            top = y;
            dark = 0;
        } else {
            dark += 1;
            if dark >= 2 {
                break;
            }
        }
    }
    let cx = (name.0 + name.1) as f64 / 2.0;
    let h = size * 1.2;
    let bottom = top as f64 - 1.0;
    (bottom - h > 0.0).then(|| Rect::new(cx - size * 0.7, bottom - h, size * 1.4, h))
}

const MAX_COMBOS: usize = 200_000;

/// Most stray glyphs dropped from one end of a line: a cursor over the label read as "21" (1x36 as 1x3621).
const MAX_STRAY: usize = 2;

fn readings(alts: &[Vec<char>]) -> Vec<(String, usize)> {
    if alts.iter().fold(1usize, |n, a| n.saturating_mul(a.len())) > MAX_COMBOS {
        return vec![];
    }
    let mut out = vec![(String::new(), 0usize)];
    for choices in alts {
        out = out.iter().flat_map(|(t, r)| choices.iter().enumerate().map(move |(i, c)| (format!("{t}{c}"), r + i))).collect();
    }
    // a number the game prints never starts with a 0: with a candle's flame read as a glyph before 5x24 ("75x24"),
    // "15x08" multiplied up to the total (120) and stood for 15x8
    out.retain(|(t, _)| !t.split('x').any(|n| n.len() > 1 && n.starts_with('0')));
    out
}

/// When Activity × Quantity doesn't make the total, one line has a misread digit: the lines that fit, or None.
pub fn reconcile(total: &[Vec<char>], aq: &[Vec<char>]) -> Option<(String, String)> {
    let first = |a: &[Vec<char>]| a.iter().map(|c| c[0]).collect::<String>();
    let best = |c: &[(String, usize)]| {
        let lowest = c.iter().map(|x| x.1).min().unwrap();
        let top: Vec<&(String, usize)> = c.iter().filter(|x| x.1 == lowest).collect();
        (top.len() == 1).then(|| top[0].0.clone())
    };
    let total_text = first(total);
    let aq_fits: Vec<(String, usize)> = readings(aq).into_iter().filter(|r| stats(Some(&total_text), Some(&r.0)).2).collect();
    if !aq_fits.is_empty() {
        return best(&aq_fits).map(|t| (total_text, t));
    }
    let aq_text = first(aq);
    let total_fits: Vec<(String, usize)> = readings(total).into_iter().filter(|r| stats(Some(&r.0), Some(&aq_text)).2).collect();
    if !total_fits.is_empty() {
        return best(&total_fits).map(|t| (t, aq_text));
    }
    // a sliver of the heart, or a cursor over the label, can pass as a glyph or two more at either end of a
    // line: dropped if the rest then fits, the fewest first
    for n in 1..=MAX_STRAY {
        let ends = |s: &str| -> Vec<String> {
            let c: Vec<char> = s.chars().collect();
            if c.len() > n {
                vec![c[n..].iter().collect(), c[..c.len() - n].iter().collect()]
            } else {
                vec![]
            }
        };
        let mut trimmed: Vec<(String, String)> = ends(&aq_text).into_iter().filter(|t| stats(Some(&total_text), Some(t)).2).map(|t| (total_text.clone(), t)).collect();
        trimmed.extend(ends(&total_text).into_iter().filter(|t| stats(Some(t), Some(&aq_text)).2).map(|t| (t, aq_text.clone())));
        if trimmed.len() == 1 {
            return trimmed.pop();
        }
        if trimmed.len() > 1 {
            return None;
        }
    }
    let parts: Vec<&str> = aq_text.split('x').collect();
    let (a, q) = (parts.first().copied().unwrap_or(""), parts.get(1).copied().unwrap_or(""));
    let t = js_number(&total_text);
    if a.is_empty() || q.is_empty() || t.fract() != 0.0 || t.is_nan() {
        return None;
    }
    let (an, qn) = (js_number(a), js_number(q));
    let one_off = |read: &str, want: f64| {
        let w = format!("{}", want as i64);
        w.len() == read.len() && read.chars().zip(w.chars()).filter(|(x, y)| x != y).count() == 1
    };
    let from_q = (qn > 0.0 && t % qn == 0.0 && one_off(a, t / qn)).then(|| format!("{}x{q}", (t / qn) as i64));
    let from_a = (an > 0.0 && t % an == 0.0 && one_off(q, t / an)).then(|| format!("{a}x{}", (t / an) as i64));
    let fixed = match (from_q, from_a) {
        (Some(_), Some(_)) => None,
        (x, y) => x.or(y),
    };
    fixed.filter(|f| stats(Some(&total_text), Some(f)).2).map(|f| (total_text, f))
}

/// The red glow of an empty tank's glass, not a name: deep red but dim. Not in upstream: on its frames empty
/// tanks measure 88 or less (under NAME_MIN), but some screens light the glass brighter (measured live at about
/// 115, 50, 55), which upstream reads as a Boss with unreadable numbers. Boss names measure about 213, 93, 89.
const GLOW_MAX: f64 = 160.0;

pub fn empty_glow(rgb: [f64; 3]) -> bool {
    if rgb.iter().any(|v| v.is_nan()) {
        return false;
    }
    let (h, s, _) = hsv(rgb[0] as u8, rgb[1] as u8, rgb[2] as u8);
    let max = rgb[0].max(rgb[1]).max(rgb[2]);
    s >= 0.35 && (h < 15.0 || h > 330.0) && max < GLOW_MAX
}

/// Rarity from a name's colour.
pub fn rarity_of(rgb: [f64; 3]) -> (Rarity, bool) {
    let (h, s, _) = hsv(rgb[0] as u8, rgb[1] as u8, rgb[2] as u8);
    if s >= 0.35 {
        if (15.0..=70.0).contains(&h) {
            return (Rarity::Rare, true);
        }
        if h < 15.0 || h > 330.0 {
            return (Rarity::Boss, true);
        }
        return (Rarity::Magic, (180.0..=240.0).contains(&h));
    }
    if (180.0..=240.0).contains(&h) {
        return if s >= 0.15 { (Rarity::Magic, true) } else { (if s >= 0.12 { Rarity::Magic } else { Rarity::Normal }, false) };
    }
    (Rarity::Normal, true)
}

fn rarity_name(r: Rarity) -> &'static str {
    match r {
        Rarity::Normal => "normal",
        Rarity::Magic => "magic",
        Rarity::Rare => "rare",
        Rarity::Boss => "boss",
    }
}

pub fn archetype_name(a: Archetype) -> &'static str {
    match a {
        Archetype::Pest => "pest",
        Archetype::Aemberon => "aemberon",
        Archetype::BoneSentry => "boneSentry",
        Archetype::Agito => "agito",
    }
}

pub struct TankRead {
    pub monster: Option<ScreenMonster>,
    pub fields: Fields,
    pub debug: Option<TankDebug>,
}

fn num(x: f64) -> String {
    format!("{}", x as i64)
}

/// `glow`: an empty tank's red glow taken for empty (not in upstream).
pub fn read_tank(img: &Img, g: &[f32], l: &Layout, i: usize, data: &ScanData, glow: bool) -> TankRead {
    let mut fields = Fields::new();
    let base = &data.digits["label"];
    let zh = data.digits_with_zh("label");
    let read_with = |dt: f64, da: f64, refs: &IndexMap<String, Vec<Vec<f64>>>| {
        let down = |r: &Rect, dy: f64| Rect::new(r.x, r.y + dy * l.k, r.w, r.h);
        let total = read_text(img, g, &down(&l.total_band[i], dt), TextKind::Label, refs);
        let aq = read_text(img, g, &down(&l.aq_band[i], da), TextKind::Label, refs);
        let mut st = stats(total.as_ref().map(|t| t.text.as_str()), aq.as_ref().map(|t| t.text.as_str()));
        let fix = if !st.2 { if let (Some(t), Some(a)) = (&total, &aq) { reconcile(&t.alts, &a.alts) } else { None } } else { None };
        if let Some((t, a)) = &fix {
            st = stats(Some(t), Some(a));
        }
        let (ts, qs) = (total.as_ref().map_or(0.0, |t| t.score), aq.as_ref().map_or(0.0, |a| a.score));
        // not in upstream: both lines read as they are, multiplying up to a total of three digits or more, are
        // taken from a little lower (3840x2160 draws the digits a little unlike the references: 8436 and 38x222,
        // exactly, scored 0.744 and the tank was shown as not read on every look)
        let exact = glow && fix.is_none() && total.as_ref().is_some_and(|t| t.text.len() >= 3) && ts >= DIGIT_MIN_EXACT && qs >= DIGIT_MIN_EXACT;
        let fits = st.2 && total.is_some() && aq.is_some() && (fix.is_some() || (ts >= DIGIT_MIN && qs >= DIGIT_MIN) || exact);
        (total, aq, st, fits)
    };
    // not in upstream: the Simplified Chinese client draws the Activity x Quantity line's "1" with a longer flag,
    // which the English client's references take for a "4" (1x171 as 4x474). Its glyphs are tried when a line
    // doesn't fit as read, and kept when they make both lines fit as read. Also when upstream's read fits only
    // mended (a glyph changed or dropped): 15x12 read as 45x42, then 45x4 with the 2 dropped, fits a total of 180.
    let as_read = |r: &(Option<TextRead>, Option<TextRead>, (f64, f64, bool), bool)| match (&r.0, &r.1) {
        (Some(t), Some(a)) => stats(Some(&t.text), Some(&a.text)).2 && t.score >= DIGIT_MIN && a.score >= DIGIT_MIN,
        _ => false,
    };
    let has_zh = zh.values().map(Vec::len).sum::<usize>() > base.values().map(Vec::len).sum::<usize>();
    let read_at = |dt: f64, da: f64| {
        let r = read_with(dt, da, base);
        if !has_zh || as_read(&r) {
            return r;
        }
        let z = read_with(dt, da, &zh);
        if as_read(&z) || (z.3 && !r.3) {
            z
        } else {
            r
        }
    };
    // a read found by searching around counts only when both lines read well: reconciling weak glyphs there can
    // make a line up out of noise ("4" and "4 x 1" from a label that says 31125 and 125 x 249)
    let confident = |r: &(Option<TextRead>, Option<TextRead>, (f64, f64, bool), bool)| {
        r.3 && r.0.as_ref().is_some_and(|t| t.score >= DIGIT_MIN) && r.1.as_ref().is_some_and(|a| a.score >= DIGIT_MIN)
    };
    let is_empty = |rgb: [f64; 3]| {
        // Math.max(...rgb) < NAME_MIN, where a NaN (a band off the picture) makes it false
        let max = if rgb.iter().any(|v| v.is_nan()) { f64::NAN } else { rgb[0].max(rgb[1]).max(rgb[2]) };
        max < NAME_MIN || (glow && empty_glow(rgb))
    };
    let mut rgb = brightest(img, &l.name_band[i]);
    let mut found: Option<(Option<TextRead>, Option<TextRead>, (f64, f64, bool), bool)> = None;
    if is_empty(rgb) {
        // not in upstream: on a window shape the scene wasn't measured on, a monster's label can sit off its
        // bands and the tank pass for empty (a Bone Sentry missed, its instruments' effects then misjudged). It
        // counts as a monster only when a name shows a little higher or lower and its numbers multiply up there.
        if glow {
            // a short plain name over a grey glass reads dim (士兵, white with few strokes: 104 against NAME_MIN 105 at
            // 2732x1426, and taken for empty for nine rounds): its numbers in place, multiplying up, say it's there
            let r = read_at(0.0, 0.0);
            if confident(&r) {
                found = Some(r);
            }
        }
        if glow && found.is_none() {
            for dy in LABEL_SHIFTS {
                let nb = l.name_band[i];
                let shifted = brightest(img, &Rect::new(nb.x, nb.y + dy * l.k, nb.w, nb.h));
                if !is_empty(shifted) {
                    let r = read_at(dy, dy);
                    if confident(&r) {
                        rgb = shifted;
                        found = Some(r);
                        break;
                    }
                }
            }
        }
        if found.is_none() {
            fields.insert(format!("tank{i}.empty"), FieldRead::sure(true));
            return TankRead { monster: None, fields, debug: None };
        }
    }
    let (rarity, sure) = rarity_of(rgb);
    fields.insert(format!("tank{i}.rarity"), FieldRead { sure, guess: Some(rarity_name(rarity).into()), ..Default::default() });
    let (mut archetype, mut a_sure) = archetype_of(img, &l.icon[i]);
    // not in upstream: the icon above the name's top, where a name can't pass for an icon. It decides when an icon
    // shows there: a coloured one, or Agito's red eye (Agito is otherwise only the absence of the others' colours)
    let above = if glow { icon_above_name(img, l, i).filter(|r| has_icon(img, r)) } else { None };
    if let Some(r) = above {
        (archetype, a_sure) = archetype_of(img, &r);
    } else {
        // no icon colour where the layout puts it (upstream calls that Agito): looked for over the name
        if glow && archetype == Archetype::Agito {
            if let Some(r) = icon_over_name(img, l, i) {
                (archetype, a_sure) = archetype_of(img, &r);
            }
        }
        if glow && !a_sure {
            if let Some(r) = icon_over_name(img, l, i) {
                let found = archetype_of(img, &r);
                if found.1 {
                    (archetype, a_sure) = found;
                }
            }
        }
    }
    fields.insert(format!("tank{i}.archetype"), FieldRead { sure: a_sure, guess: Some(archetype_name(archetype).into()), ..Default::default() });

    let (mut total, mut aq, mut st, mut fits) = found.unwrap_or_else(|| read_at(0.0, 0.0));
    // not in upstream: on a window shape the scene wasn't measured on, the label can sit a few pixels off its
    // bands; looked for a little higher and lower, the nearest place where the line multiplies to the total wins
    // not in upstream: a small "x" read as a digit ("331 x 1" as "33131"): the one glyph that, taken for the "x",
    // leaves a line that multiplies to the total, if just one does
    if !fits && glow {
        if let (Some(t), Some(a)) = (&total, &aq) {
            if let Some(line) = x_misread(&t.text, &a.text) {
                st = stats(Some(&t.text), Some(&line));
                fits = st.2 && t.score >= DIGIT_MIN;
            }
        }
    }
    if !fits && glow {
        // both lines moved together first, then each on its own
        let together = LABEL_SHIFTS.iter().map(|&d| (d, d));
        let apart = std::iter::once(0.0).chain(LABEL_SHIFTS).flat_map(|dt| std::iter::once(0.0).chain(LABEL_SHIFTS).map(move |da| (dt, da))).filter(|(dt, da)| dt != da);
        for (dt, da) in together.chain(apart) {
            let mut r = read_at(dt, da);
            if !r.3 {
                if let (Some(t), Some(a)) = (&r.0, &r.1) {
                    if let Some(line) = x_misread(&t.text, &a.text) {
                        r.2 = stats(Some(&t.text), Some(&line));
                        r.3 = r.2 .2 && t.score >= DIGIT_MIN;
                    }
                }
            }
            if confident(&r) {
                (total, aq, st, fits) = r;
                break;
            }
        }
    }
    let (score, second) = (aq.as_ref().map(|a| a.score), aq.as_ref().map(|a| a.second));
    fields.insert(format!("tank{i}.activity"), FieldRead { sure: fits, guess: Some(num(st.0)), score, second });
    fields.insert(format!("tank{i}.quantity"), FieldRead { sure: fits, guess: Some(num(st.1)), score, second });
    let debug = TankDebug { rgb, total: total.as_ref().map_or(String::new(), |t| t.text.clone()), aq: aq.as_ref().map_or(String::new(), |t| t.text.clone()) };
    TankRead { monster: Some(ScreenMonster { archetype, rarity, activity: st.0 as i64, quantity: st.1 as i64 }), fields, debug: Some(debug) }
}
