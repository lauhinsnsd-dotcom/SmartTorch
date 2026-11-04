//! The cards' printed names (src/solver/scan/titles.ts), read by a text reader (OCR) and matched to the known
//! names. A clearly read name outranks the picture match; when two names fit about as well, the picture picks
//! between them; when nothing is read, the picture's answer stands.

use crate::cards::read_at_with;
use crate::data::ScanData;
use crate::layout::{card_face, layout_at, CardPos};
use crate::pixels::{clamp_u8, patch, Img, Rect};
use crate::reading::{FieldRead, Fields, Problem, Reading};
use std::collections::HashMap;

const MIN_Q: f64 = 6.0;
const GAP: f64 = 3.0;

fn letters(t: &str) -> Vec<char> {
    t.to_lowercase().chars().filter(|c| c.is_ascii_lowercase()).collect()
}

/// For each start of p, the fewest edits that match it somewhere inside t.
fn prefix_costs(p: &[char], t: &[char]) -> Vec<usize> {
    let mut prev = vec![0usize; t.len() + 1];
    let mut best = vec![0];
    for i in 1..=p.len() {
        let mut row = vec![i; t.len() + 1];
        for j in 1..=t.len() {
            row[j] = (prev[j] + 1).min(row[j - 1] + 1).min(prev[j - 1] + usize::from(p[i - 1] != t[j - 1]));
        }
        best.push(*row.iter().min().unwrap());
        prev = row;
    }
    best
}

/// How much of a name the text shows: its letters matched, less two per mistake, less a half per missing letter at its end.
pub fn name_fit(name: &str, text: &str) -> f64 {
    let p = letters(name);
    let t = letters(text);
    if t.is_empty() {
        return f64::NEG_INFINITY;
    }
    let c = prefix_costs(&p, &t);
    let mut q = f64::NEG_INFINITY;
    for i in p.len().min(6)..=p.len() {
        q = q.max(i as f64 - 2.0 * c[i] as f64 - 0.5 * (p.len() - i) as f64);
    }
    q
}

/// Which writing the game's card titles are in, and so how they're read and matched.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Script {
    /// an English (or other Latin-letter) client: upstream's reading, unchanged
    #[default]
    Latin,
    /// a Chinese client (Traditional or Simplified): titles matched to names_zh.json
    Han,
}

fn is_han(c: char) -> bool {
    matches!(c as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x2FA1F)
}

fn han(t: &str, fold: &std::collections::HashMap<char, char>) -> Vec<char> {
    t.chars().filter(|&c| is_han(c)).map(|c| *fold.get(&c).unwrap_or(&c)).collect()
}

/// A Chinese character tells about as much as this many letters (MIN_Q and GAP are counted in letters).
const HAN_WEIGHT: f64 = 2.5;

/// name_fit for a Chinese name: its characters matched, less one and a half per mistake (a character misread is
/// one character, where a misread letter often comes with its neighbours), less a half per missing one at its end
/// (an overlapping card hides a title's end), counted in letters' worth. Simplified and Traditional forms count
/// as the same character (`fold`); punctuation and spaces don't count.
pub fn han_fit(name: &str, text: &str, fold: &std::collections::HashMap<char, char>) -> f64 {
    let p = han(name, fold);
    let t = han(text, fold);
    if t.is_empty() || p.is_empty() {
        return f64::NEG_INFINITY;
    }
    let c = prefix_costs(&p, &t);
    let mut q = f64::NEG_INFINITY;
    for i in p.len().min(3)..=p.len() {
        q = q.max(i as f64 - 1.5 * c[i] as f64 - 0.5 * (p.len() - i) as f64);
    }
    // a title read exactly, nothing more: ahead of the names it's part of (藥劑箱（小） in 稀有藥劑箱（小）)
    q * HAN_WEIGHT + if p == t { EXACT_BONUS } else { 0.0 }
}

/// What a Chinese title read exactly gains over a longer name holding it.
const EXACT_BONUS: f64 = 3.0;

/// Whether a read has any Chinese characters in it.
pub fn has_han(t: &str) -> bool {
    t.chars().any(is_han)
}

/// The longest run of a name's characters found in order in a read (not necessarily together).
fn lcs(p: &[char], t: &[char]) -> usize {
    let mut prev = vec![0usize; t.len() + 1];
    for &a in p {
        let mut row = vec![0usize; t.len() + 1];
        for (j, &b) in t.iter().enumerate() {
            row[j + 1] = if a == b { prev[j] + 1 } else { row[j].max(prev[j + 1]) };
        }
        prev = row;
    }
    prev[t.len()]
}

/// A Chinese title read with a character or two dropped or misread ("人示本" for 人蛹標本): the card most of whose
/// characters show, in order, when no other comes close. At least two characters and half the name, and a fifth of
/// a name ahead of the next card; a family's two versions with the family not read stay apart (細肢藥粉 fits the
/// Pest and the Bone Sentry versions about as well).
pub fn han_pick(data: &ScanData, phase: &str, text: &str) -> Option<String> {
    let fold = &data.names_zh.fold;
    let t = han(text, fold);
    if t.len() < 2 {
        return None;
    }
    let mut scored: Vec<(String, usize, f64)> = zh_names_for(data, phase)
        .iter()
        .map(|(en, zh)| {
            let best = zh.iter().map(|z| {
                let p = han(z, fold);
                let m = lcs(&p, &t);
                (m, if p.is_empty() { 0.0 } else { m as f64 / p.len() as f64 })
            });
            let (m, r) = best.fold((0, 0.0), |a, b| if b.1 > a.1 { b } else { a });
            (en.clone(), m, r)
        })
        .collect();
    scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    let (name, matched, ratio) = scored.first()?.clone();
    let second = scored.get(1).map_or(0.0, |s| s.2);
    // (a hair under a fifth: 3/5 - 2/5 is 0.19999999999999996 in floating point, and "腦溶液" was left unread)
    (matched >= 2 && ratio >= 0.5 && ratio - second >= 0.2 - 1e-9).then_some(name)
}

/// A title not read as any card that is one of the cards upstream's data doesn't have (names_zh.json "unsupported"):
/// that card as printed, when at least three of its characters and half of them show in order.
pub fn han_unsupported(data: &ScanData, text: &str) -> Option<String> {
    let fold = &data.names_zh.fold;
    let t = han(text, fold);
    if t.len() < 3 {
        return None;
    }
    data.names_zh.unsupported.iter().find(|z| {
        let p = han(z, fold);
        let m = lcs(&p, &t);
        m >= 3 && 2 * m >= p.len()
    }).cloned()
}

const TITLE_AREA: Rect = Rect::new(-140.0, -110.0, 280.0, 150.0);
const ZOOM: f64 = 2.0;
const BLUR: usize = 8;

/// A card's title area, straightened and enlarged, as dark text on white.
pub fn title_crop(img: &Img, c: &CardPos, k: f64) -> Img {
    let r = Rect::new(TITLE_AREA.x * ZOOM, TITLE_AREA.y * ZOOM, TITLE_AREA.w * ZOOM, TITLE_AREA.h * ZOOM);
    let p = patch(img, c.cx, c.cy, c.angle, r, k / ZOOM);
    let (w, h) = (p.width, p.height);
    let g: Vec<f32> = (0..w * h).map(|i| p.data[i * 4].max(p.data[i * 4 + 1]).max(p.data[i * 4 + 2]) as f32).collect();
    let mut sum = vec![0f64; (w + 1) * (h + 1)];
    for y in 0..h {
        for x in 0..w {
            sum[(y + 1) * (w + 1) + x + 1] = g[y * w + x] as f64 + sum[y * (w + 1) + x + 1] + sum[(y + 1) * (w + 1) + x] - sum[y * (w + 1) + x];
        }
    }
    let mut lift = vec![0f32; w * h];
    let mut top = 1.0f64;
    for y in 0..h {
        for x in 0..w {
            let (x0, x1, y0, y1) = (x.saturating_sub(BLUR), w.min(x + BLUR + 1), y.saturating_sub(BLUR), h.min(y + BLUR + 1));
            let mean = (sum[y1 * (w + 1) + x1] - sum[y0 * (w + 1) + x1] - sum[y1 * (w + 1) + x0] + sum[y0 * (w + 1) + x0]) / ((x1 - x0) * (y1 - y0)) as f64;
            lift[y * w + x] = (g[y * w + x] as f64 - mean).max(0.0) as f32;
            top = top.max(lift[y * w + x] as f64);
        }
    }
    let mut out = Img::new(w, h);
    for i in 0..w * h {
        let v = clamp_u8(255.0 - (lift[i] as f64 / top * 400.0).min(255.0));
        out.data[i * 4] = v;
        out.data[i * 4 + 1] = v;
        out.data[i * 4 + 2] = v;
        out.data[i * 4 + 3] = 255;
    }
    out
}

/// A Chinese client's title line (its name sits lower on the card than an English one, and its strokes are too
/// thin for title_crop's lift): the card's middle band, straightened and enlarged, its contrast stretched and
/// turned dark on white. It reaches up far enough for the card under the mouse, which stands raised.
const STRIP_AREA: Rect = Rect::new(-112.0, -112.0, 224.0, 94.0);

pub fn title_strip(img: &Img, c: &CardPos, k: f64) -> Img {
    let r = Rect::new(STRIP_AREA.x * ZOOM, STRIP_AREA.y * ZOOM, STRIP_AREA.w * ZOOM, STRIP_AREA.h * ZOOM);
    let p = patch(img, c.cx, c.cy, c.angle, r, k / ZOOM);
    let n = p.width * p.height;
    let g: Vec<f64> = (0..n).map(|i| 0.299 * p.data[i * 4] as f64 + 0.587 * p.data[i * 4 + 1] as f64 + 0.114 * p.data[i * 4 + 2] as f64).collect();
    let mut sorted = g.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let (lo, hi) = (sorted[n / 100], sorted[n - 1 - n / 100]);
    let span = (hi - lo).max(1.0);
    let mut out = Img::new(p.width, p.height);
    for i in 0..n {
        let v = clamp_u8(255.0 - ((g[i] - lo) / span * 255.0).clamp(0.0, 255.0));
        out.data[i * 4] = v;
        out.data[i * 4 + 1] = v;
        out.data[i * 4 + 2] = v;
        out.data[i * 4 + 3] = 255;
    }
    out
}

fn names_for<'a>(data: &'a ScanData, phase: &str) -> &'a [String] {
    match phase {
        "potion" => &data.names.potion,
        "instrument" => &data.names.instrument,
        _ => &data.names.protocol,
    }
}

fn zh_names_for<'a>(data: &'a ScanData, phase: &str) -> &'a indexmap::IndexMap<String, Vec<String>> {
    match phase {
        "potion" => &data.names_zh.potion,
        "instrument" => &data.names_zh.instrument,
        _ => &data.names_zh.protocol,
    }
}

/// How well a read fits a card in this script: by its English name, and on a Chinese client by its Chinese names too.
fn fit_in(data: &ScanData, phase: &str, name: &str, text: &str, script: Script) -> f64 {
    let latin = name_fit(name, text);
    match script {
        Script::Latin => latin,
        Script::Han => zh_names_for(data, phase).get(name).map_or(latin, |zh| zh.iter().map(|z| han_fit(z, text, &data.names_zh.fold)).fold(latin, f64::max)),
    }
}

fn ranked(data: &ScanData, phase: &str, text: &str) -> Vec<(String, f64)> {
    ranked_in(data, phase, text, Script::Latin)
}

fn ranked_in(data: &ScanData, phase: &str, text: &str, script: Script) -> Vec<(String, f64)> {
    let mut r: Vec<(String, f64)> = names_for(data, phase).iter().map(|n| (n.clone(), fit_in(data, phase, n, text, script))).collect();
    r.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    r
}

fn settled(r: &[(String, f64)]) -> bool {
    r[0].1 >= MIN_Q && r[0].1 - r.get(1).map_or(f64::NEG_INFINITY, |x| x.1) >= GAP
}

/// Some of a hand's cards matched by their pictures: per position, the best few cards, its field, and whether
/// no picture was close enough.
#[derive(Clone, Debug)]
pub struct PictureRead {
    pub ranks: Vec<(String, f64)>,
    pub field: Option<FieldRead>,
    pub unknown: bool,
}

/// The pictures of the given card positions, read as the reading's own would be.
/// `hand_dy`: the reading's hand_dy (0 where the layout puts the hand); `ui`: its interface scale.
pub fn pictures(img: &Img, data: &ScanData, phase: &str, size: usize, slots: &[usize], hand_dy: f64, ui: f64) -> HashMap<usize, PictureRead> {
    pictures_in(img, data, phase, size, slots, hand_dy, ui, Script::Latin)
}

/// pictures, for a client in this script: a Chinese one's cards by their art alone (their text blocks aren't the
/// English ones the references hold).
#[allow(clippy::too_many_arguments)]
pub fn pictures_in(img: &Img, data: &ScanData, phase: &str, size: usize, slots: &[usize], hand_dy: f64, ui: f64, script: Script) -> HashMap<usize, PictureRead> {
    let l = layout_at(img.width, img.height, ui).shifted_hand(hand_dy);
    let r = read_at_with(img, &l, size, phase, data, Some(slots), script == Script::Han);
    slots
        .iter()
        .map(|&j| {
            let key = format!("hand{j}");
            let unknown = r.problems.iter().any(|p| matches!(p, Problem::UnknownArt { field, .. } if *field == key));
            (j, PictureRead { ranks: r.ranks[j].clone(), field: r.fields.get(&key).cloned(), unknown })
        })
        .collect()
}

const PHASES: [&str; 3] = ["potion", "instrument", "protocol"];

/// The reading with each card's printed name added. `read_text` reads a title crop (OCR); `pictures` matches
/// the cards whose names can't be settled, when the reading left them out.
pub fn read_titles(img: &Img, reading: Reading, data: &ScanData, read_text: &mut dyn FnMut(&Img) -> String, pictures_of: Option<&dyn Fn(&str, usize, &[usize]) -> HashMap<usize, PictureRead>>) -> Reading {
    read_titles_in(img, reading, data, read_text, pictures_of, Script::Latin)
}

/// read_titles for a client in this script. On a Chinese client the title line is read (title_strip), matched to
/// the Chinese names too, the kind of pick is the one the names fit best (the header's shape is an English
/// one's), and a picture only stands in for a name not read when it's sure.
pub fn read_titles_in(img: &Img, reading: Reading, data: &ScanData, read_text: &mut dyn FnMut(&Img) -> String, pictures_of: Option<&dyn Fn(&str, usize, &[usize]) -> HashMap<usize, PictureRead>>, script: Script) -> Reading {
    let han_client = script == Script::Han;
    let read = reading.hand_ranks.as_ref().map_or(0, |r| r.len());
    if (read == 0 && reading.hand_pending != Some(true)) || reading.has(|p| matches!(p, Problem::NotFound | Problem::Animating)) {
        return reading;
    }
    let l = layout_at(img.width, img.height, reading.ui()).shifted_hand(reading.hand_dy.unwrap_or(0.0));
    let header = match reading.state.phase {
        vorax::advisor::ScreenPhase::Potion => "potion",
        vorax::advisor::ScreenPhase::Instrument => "instrument",
        vorax::advisor::ScreenPhase::Protocol => "protocol",
    };
    let phases: Vec<&str> = if !han_client && reading.fields.get("phase").is_some_and(|f| f.sure) {
        vec![header]
    } else if han_client {
        // the header's phase first: it keeps a tie
        std::iter::once(header).chain(PHASES.into_iter().filter(|p| *p != header)).collect()
    } else {
        PHASES.to_vec()
    };
    let size = if read > 0 { read } else { reading.hand_size.unwrap_or(3) };
    let texts: Vec<String> = l.hand_at(size).iter().map(|c| if han_client { read_text(&title_strip(img, c, l.k)) } else { read_text(&title_crop(img, c, l.k)) }).collect();
    let mut best: Option<(&str, Vec<Vec<(String, f64)>>, f64)> = None;
    for phase in phases {
        let slots: Vec<Vec<(String, f64)>> = texts.iter().map(|t| if han_client { ranked_in(data, phase, t, script) } else { ranked(data, phase, t) }).collect();
        let fit = slots.iter().fold(0.0, |s, r| s + r[0].1.max(0.0));
        if best.as_ref().is_none_or(|b| fit > b.2) {
            best = Some((phase, slots, fit));
        }
    }
    let (phase, slots, _) = best.unwrap();
    let pos = l.hand_at(size);

    let need: Vec<usize> = slots.iter().enumerate().filter(|(_, r)| !settled(r)).map(|(j, _)| j).collect();
    let from_reading = read == size && phase == header;
    let pics: HashMap<usize, PictureRead> = if from_reading {
        need.iter()
            .map(|&j| {
                let key = format!("hand{j}");
                let unknown = reading.problems.iter().any(|p| matches!(p, Problem::UnknownArt { field, .. } if *field == key));
                (j, PictureRead { ranks: reading.hand_ranks.as_ref().unwrap()[j].clone(), field: reading.fields.get(&key).cloned(), unknown })
            })
            .collect()
    } else if !need.is_empty() {
        pictures_of.map(|f| f(phase, size, &need)).unwrap_or_default()
    } else {
        HashMap::new()
    };

    let mut fields: Fields = reading.fields.iter().filter(|(k, _)| !(k.len() == 5 && k.starts_with("hand") && k.as_bytes()[4].is_ascii_digit())).map(|(k, v)| (k.clone(), v.clone())).collect();
    let mut hand = Vec::new();
    let mut unknown = Vec::new();
    for (j, r) in slots.iter().enumerate() {
        let (top, second) = (&r[0], r.get(1));
        let key = format!("hand{j}");
        let pic = pics.get(&j);
        let mut name: Option<String> = None;
        if settled(r) {
            name = Some(top.0.clone());
            fields.insert(key, FieldRead { sure: true, guess: name.clone(), score: Some(top.1), second: second.map(|s| s.1) });
        } else if let Some(n) = han_client.then(|| han_pick(data, phase, &texts[j])).flatten().filter(|n| !pic.is_some_and(|p| p.field.as_ref().is_some_and(|f| f.sure) && p.ranks.first().is_some_and(|x| x.0 != *n))) {
            // a Chinese title with a character or two not read, clearly one card's (and no sure picture against it)
            fields.insert(key, FieldRead { sure: false, guess: Some(n.clone()), score: Some(top.1), second: second.map(|s| s.1) });
            name = Some(n);
        } else if han_client && top.1 >= MIN_Q && !pic.is_some_and(|p| p.field.as_ref().is_some_and(|f| f.sure) && p.ranks.first().is_some_and(|x| r.iter().any(|(n, q)| *n == x.0 && *q >= top.1 - GAP))) {
            // a Chinese title that fits more than one card about as well (a family's two versions with the family
            // not read): left unknown rather than guessed, unless the picture is sure of one of them
            fields.insert(key, FieldRead { sure: false, guess: Some(top.0.clone()), score: Some(top.1), second: second.map(|s| s.1) });
        } else if top.1 >= MIN_Q {
            let close: Vec<&String> = r.iter().filter(|(_, q)| *q >= top.1 - GAP).map(|(n, _)| n).collect();
            let n = pic.and_then(|p| p.ranks.iter().find(|(n, _)| close.contains(&n)).map(|x| x.0.clone())).unwrap_or_else(|| top.0.clone());
            let sure = pic.is_some_and(|p| p.ranks.first().is_some_and(|x| x.0 == n) && p.field.as_ref().is_some_and(|f| f.sure));
            fields.insert(key, FieldRead { sure, guess: Some(n.clone()), score: Some(top.1), second: second.map(|s| s.1) });
            name = Some(n);
        } else if let Some(p) = pic.filter(|p| !p.unknown && p.field.as_ref().is_some_and(|f| f.guess.is_some() && (!han_client || f.sure))) {
            name = p.field.as_ref().unwrap().guess.clone();
            fields.insert(key, p.field.clone().unwrap());
        } else {
            fields.insert(key, pic.and_then(|p| p.field.clone()).unwrap_or(FieldRead::sure(false)));
        }
        match name {
            Some(n) => hand.push(n),
            None => unknown.push(j),
        }
    }
    let mut problems: Vec<Problem> = reading.problems.iter().filter(|p| !matches!(p, Problem::UnknownArt { .. })).cloned().collect();
    for &j in &unknown {
        let text = texts[j].trim();
        problems.push(Problem::UnknownArt { field: format!("hand{j}"), crop: card_face(&pos[j], l.k), hand: (size == 5).then_some(5), text: (!text.is_empty()).then(|| text.to_string()) });
    }
    if phase != header {
        fields.insert("phase".into(), FieldRead { sure: unknown.is_empty(), guess: Some(phase.into()), ..Default::default() });
    }
    let mut state = reading.state.clone();
    state.phase = match phase {
        "potion" => vorax::advisor::ScreenPhase::Potion,
        "instrument" => vorax::advisor::ScreenPhase::Instrument,
        _ => vorax::advisor::ScreenPhase::Protocol,
    };
    state.hand = hand;
    // a refresh count read under the header's kind of pick belongs to the one the names show
    if phase != header && phase != "protocol" {
        let left = if header == "potion" { reading.state.potion_refreshes } else { reading.state.instrument_refreshes };
        if phase == "potion" {
            state.potion_refreshes = left;
        } else {
            state.instrument_refreshes = left;
        }
    }
    Reading { state, fields, problems, hand_ranks: Some((0..pos.len()).map(|j| pics.get(&j).map(|p| p.ranks.clone()).unwrap_or_default()).collect()), hand_pending: None, ..reading }
}
