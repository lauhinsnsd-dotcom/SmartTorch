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

/// Not in upstream: the Simplified client's title dash ("孪生激素-骨卫兵"), which a Traditional Chinese text reader
/// returns as the character 一 ("孪生激素一骨卫"), counted as a mistake and leaving the Bone Sentry version short of a
/// pick. Left out of names and reads alike, like punctuation (the one name with 一, 第一儀式聖水：告解, stays apart).
const DASH: char = '一';

fn han(t: &str, fold: &std::collections::HashMap<char, char>) -> Vec<char> {
    t.chars().filter(|&c| is_han(c) && c != DASH).map(|c| *fold.get(&c).unwrap_or(&c)).collect()
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
    // a longer name holding the leader and more of the read: "異種萬靈藥：育" is 異種萬靈藥：育雛, not 異種萬靈藥 (all of
    // whose characters show too). Only when that longer name alone matches the most characters
    let most = scored.iter().map(|s| s.1).max().unwrap_or(0);
    let longest: Vec<&(String, usize, f64)> = scored.iter().filter(|s| s.1 == most).collect();
    if let [(longer, m, r)] = longest.as_slice() {
        let zh_of = |en: &str| zh_names_for(data, phase).get(en).map(|z| z.iter().map(|x| han(x, fold)).collect::<Vec<_>>()).unwrap_or_default();
        let holds = zh_of(&name).iter().any(|a| zh_of(longer).iter().any(|b| lcs(a, b) == a.len()));
        if *longer != name && *m > matched && *r >= 0.5 && holds {
            return Some(longer.clone());
        }
    }
    // not in upstream: three or more characters, in order, from a third of a name on, that no other card of any kind
    // has as many of: a family's version told by its own character short of half the name (1920x1080: "孿 生 - 骨" for
    // 孿生激素-骨衛兵, 3/7; its Pest version has 2)
    if let [(most_name, m, r)] = longest.as_slice() {
        let others_fewer = PHASES.iter().all(|p| zh_names_for(data, p).iter().filter(|(en, _)| *en != most_name).all(|(_, zh)| zh.iter().all(|z| lcs(&han(z, fold), &t) < *m)));
        if *m >= 3 && *r >= 1.0 / 3.0 - 1e-9 && others_fewer {
            return Some(most_name.clone());
        }
    }
    // (a hair under a fifth: 3/5 - 2/5 is 0.19999999999999996 in floating point, and "腦溶液" was left unread)
    // not in upstream: two or more characters, in order, that no other card of this kind has any of and no card of any
    // kind has all of, name the card even short of half of it ("疫 像" for 疫區聖母像, its statue over the middle of
    // the title), from a third on (not "膽 汁" for 黑膽汁療法: the potions 青膽汁溶液 and 黏稠膽汁溶液 have it too)
    let alone = matched >= 2
        && ratio >= 1.0 / 3.0 - 1e-9
        && second == 0.0
        && PHASES.iter().all(|p| {
            zh_names_for(data, p).iter().filter(|(en, _)| **en != name).all(|(_, zh)| zh.iter().all(|z| lcs(&han(z, fold), &t) < matched))
        });
    // not in upstream: three or more characters and half the name, a little closer to the next card (1920x1080:
    // "麻 、 異 魔" for 麻藥酊劑-異魔, 3/6 over 脊髓溶液-異魔's 2/6)
    let gap = if matched >= 3 { 0.15 } else { 0.2 };
    (alone || (matched >= 2 && ratio >= 0.5 && ratio - second >= gap - 1e-9)).then_some(name)
}

/// A title not read as any card that is one of the cards upstream's data doesn't have (names_zh.json "unsupported"):
/// that card as printed, when at least three of its characters and half of them show in order.
/// Whether a Chinese title read names a card of one of these kinds (settled, or picked by han_pick), or one of the
/// cards the data doesn't have: reading it again wouldn't tell more.
fn names_a_card(data: &ScanData, phases: &[&str], text: &str, script: Script) -> bool {
    han_unsupported(data, text).is_some() || phases.iter().any(|p| settled(&ranked_in(data, p, text, script)) || han_pick(data, p, text).is_some())
}

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
    stretched(img, c, k, STRIP_AREA, 0)
}

/// The title line alone, for a second try when title_strip's read named no card: the line's band where it sits
/// (`lift` 0) or on the card under the mouse (`lift` -30), with a white margin. Windows' reader can miss a line at
/// the strip's edge under busy art (人蛹標本 read as nothing six times over, then fine once the card was raised).
const LINE_AREA: Rect = Rect::new(-112.0, -60.0, 224.0, 42.0);
pub const LINE_LIFTS: [f64; 2] = [0.0, -30.0];
/// not in upstream: the title line read once more at this size when no read of it named a card
const LINE_SMALL: f64 = 0.6;

/// not in upstream: a title crop (dark text on white) in black and white only.
pub fn black_and_white(img: &Img) -> Img {
    let mut out = img.clone();
    for p in out.data.chunks_exact_mut(4) {
        let v = if p[0] < 60 { 0 } else { 255 };
        p[0] = v;
        p[1] = v;
        p[2] = v;
    }
    out
}

pub fn title_line(img: &Img, c: &CardPos, k: f64, lift: f64) -> Img {
    stretched(img, c, k, Rect::new(LINE_AREA.x, LINE_AREA.y + lift, LINE_AREA.w, LINE_AREA.h), 24)
}

/// A part of a card, straightened and enlarged, its contrast stretched and turned dark on white, with `pad` white
/// pixels around it.
fn stretched(img: &Img, c: &CardPos, k: f64, area: Rect, pad: usize) -> Img {
    let r = Rect::new(area.x * ZOOM, area.y * ZOOM, area.w * ZOOM, area.h * ZOOM);
    let p = patch(img, c.cx, c.cy, c.angle, r, k / ZOOM);
    let n = p.width * p.height;
    let g: Vec<f64> = (0..n).map(|i| 0.299 * p.data[i * 4] as f64 + 0.587 * p.data[i * 4 + 1] as f64 + 0.114 * p.data[i * 4 + 2] as f64).collect();
    let mut sorted = g.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let (lo, hi) = (sorted[n / 100], sorted[n - 1 - n / 100]);
    let span = (hi - lo).max(1.0);
    let (w, h) = (p.width + 2 * pad, p.height + 2 * pad);
    let mut out = Img::new(w, h);
    out.data.fill(255);
    for y in 0..p.height {
        for x in 0..p.width {
            let v = clamp_u8(255.0 - ((g[y * p.width + x] - lo) / span * 255.0).clamp(0.0, 255.0));
            let o = ((y + pad) * w + x + pad) * 4;
            out.data[o] = v;
            out.data[o + 1] = v;
            out.data[o + 2] = v;
        }
    }
    out
}

/// not in upstream: the one name whose words the title's pieces (its first three lines, words of four letters or
/// more) begin, or are with one letter wrong, matching five letters or more; none when two names tie.
pub fn word_pick(names: &[String], text: &str) -> Option<String> {
    let words = |t: &str| -> Vec<String> { t.to_lowercase().split(|c: char| !c.is_ascii_lowercase()).filter(|w| !w.is_empty()).map(String::from).collect() };
    let pieces: Vec<String> = text.lines().take(3).flat_map(|l| words(l)).filter(|w| w.len() >= 4).collect();
    let fits = |piece: &str, word: &str| word.starts_with(piece) || (piece.len() == word.len() && piece.len() >= 5 && piece.chars().zip(word.chars()).filter(|(a, b)| a != b).count() == 1);
    let mut best: Vec<(&String, usize)> = names
        .iter()
        .map(|n| {
            let nw = words(n);
            let letters: usize = pieces.iter().filter(|p| nw.iter().any(|w| fits(p, w))).map(|p| p.len()).sum();
            (n, letters)
        })
        .filter(|x| x.1 >= 5)
        .collect();
    best.sort_by(|a, b| b.1.cmp(&a.1));
    match best.as_slice() {
        [(n, _)] => Some((*n).clone()),
        [(n, a), (_, b), ..] if a > b => Some((*n).clone()),
        _ => None,
    }
}

/// not in upstream: the cards a title not made out most likely is, for the player to pick from (the toolbox's
/// picker): the names its text fits best and the pictures' best, taken by turns, `n` at most.
pub fn candidates(data: &ScanData, phase: &str, text: &str, script: Script, pictures: &[(String, f64)], n: usize) -> Vec<String> {
    let names = names_for(data, phase);
    let by_text: Vec<String> = if text.trim().is_empty() {
        Vec::new()
    } else {
        let r = if script == Script::Han { ranked_in(data, phase, text, script) } else { ranked(data, phase, text) };
        r.into_iter().filter(|(_, q)| *q > 0.0).map(|(name, _)| name).collect()
    };
    let by_picture: Vec<String> = pictures.iter().map(|(name, _)| name.clone()).filter(|name| names.contains(name)).collect();
    let mut out: Vec<String> = Vec::new();
    for k in 0..by_text.len().max(by_picture.len()) {
        for list in [&by_text, &by_picture] {
            if let Some(name) = list.get(k) {
                if out.len() < n && !out.contains(name) {
                    out.push(name.clone());
                }
            }
        }
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

/// not in upstream: how far ahead of the next of the names a title fits alike a picture's best of them must be
/// (藥劑箱（大）'s own picture 0.998 against 藥劑箱（小）'s 0.979 at 1920x1080; a family whose versions share their
/// art, 細肢藥粉, 0.996 against 0.994)
const PICTURE_TIE_GAP: f64 = 0.01;

/// not in upstream: of the names a title fits about as well as its best (a family's versions), the one the
/// card's picture clearly prefers.
fn picture_between(r: &[(String, f64)], pic: Option<&PictureRead>) -> Option<String> {
    let top = r.first()?.1;
    if top < MIN_Q {
        return None;
    }
    let close: Vec<&String> = r.iter().filter(|(_, q)| *q >= top - GAP).map(|(n, _)| n).collect();
    if close.len() < 2 {
        return None;
    }
    let mut by_picture: Vec<&(String, f64)> = pic?.ranks.iter().filter(|(n, _)| close.contains(&n)).collect();
    by_picture.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    match by_picture.as_slice() {
        [a, b, ..] if a.1 - b.1 >= PICTURE_TIE_GAP => Some(a.0.clone()),
        _ => None,
    }
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
    let mut size = if read > 0 { read } else { reading.hand_size.unwrap_or(3) };
    // not in upstream: a potion kit's five cards fanned out, on a Chinese client whose header was taken for another
    // pick's (3840x2160: the protocols'), where only a potion header looks for five: the fan's ends hold cards
    if han_client && size == 3 && crate::cards::fan_size(img, &l) == 5 {
        size = 5;
    }
    let mut texts: Vec<String> = l.hand_at(size).iter().map(|c| if han_client { read_text(&title_strip(img, c, l.k)) } else { read_text(&title_crop(img, c, l.k)) }).collect();
    // not in upstream: a Chinese title that named no card is read again from its line alone, where it sits and then
    // raised (the card under the mouse), until one names a card
    if han_client {
        for (j, c) in l.hand_at(size).iter().enumerate() {
            if names_a_card(data, &phases, &texts[j], script) {
                continue;
            }
            let mut named = false;
            for lift in LINE_LIFTS {
                let t = read_text(&title_line(img, c, l.k, lift));
                if names_a_card(data, &phases, &t, script) {
                    texts[j] = t;
                    named = true;
                    break;
                }
            }
            // not in upstream: the line once more, smaller (Windows' reader at times keeps one character of a clear
            // title, "溶" for 黏稠膽汁溶液 at 2560x1440 and 3840x2160 alike): another size reads differently
            if !named {
                let line = title_line(img, c, l.k, 0.0);
                let (w, h) = ((line.width as f64 * LINE_SMALL) as usize, (line.height as f64 * LINE_SMALL) as usize);
                let t = read_text(&crate::pixels::resize(&line, w.max(1), h.max(1)));
                if names_a_card(data, &phases, &t, script) {
                    texts[j] = t;
                    named = true;
                }
            }
            // not in upstream: the line in black and white, as it is and smaller (1920x1080: its small title,
            // enlarged, is too soft for Windows' reader, which reads other characters, "眚 觼 溶 液" for 青膽汁溶液)
            if !named {
                let line = black_and_white(&title_line(img, c, l.k, 0.0));
                let (w, h) = ((line.width as f64 * LINE_SMALL) as usize, (line.height as f64 * LINE_SMALL) as usize);
                for v in [crate::pixels::resize(&line, w.max(1), h.max(1)), line] {
                    let t = read_text(&v);
                    if names_a_card(data, &phases, &t, script) {
                        texts[j] = t;
                        break;
                    }
                }
            }
        }
    }
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
        } else if let Some(n) = han_client.then(|| picture_between(r, pic)).flatten() {
            // not in upstream: a Chinese title that fits a family's versions alike (藥劑箱 for 藥劑箱（大） and
            // 藥劑箱（小）, the size in brackets not read), told apart by the picture when it clearly prefers one
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
        } else if let Some(n) = (!han_client && img.height < 1300).then(|| word_pick(names_for(data, phase), &texts[j])).flatten() {
            // not in upstream: an English title read in pieces (1920x1080: "Claws", "Hyperp / Front / Lobe") with no
            // picture sure of it names the one card whose name words they begin (below 2560x1440 only)
            fields.insert(key, FieldRead { sure: false, guess: Some(n.clone()), score: Some(top.1), second: second.map(|s| s.1) });
            name = Some(n);
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
        // not in upstream: under a header taken for the protocols', none was read: read it now
        if header == "protocol" {
            let (left, field) = crate::scan::read_refreshes(img, &crate::pixels::gray(img), &l, data);
            if phase == "potion" {
                state.potion_refreshes = left;
            } else {
                state.instrument_refreshes = left;
            }
            fields.insert("refreshes".into(), field);
        }
    }
    Reading { state, fields, problems, hand_ranks: Some((0..pos.len()).map(|j| pics.get(&j).map(|p| p.ranks.clone()).unwrap_or_default()).collect()), hand_pending: None, ..reading }
}
