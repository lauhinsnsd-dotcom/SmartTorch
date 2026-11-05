//! Not in upstream: the Chinese edition's words are written in Traditional Chinese and shown in Simplified for a
//! Simplified client. Which one: as set in the tray (auto, Traditional, Simplified), auto following the card titles
//! the game shows (`observe`). The text is turned at the one place it is drawn (draw::text and draw::measure), a
//! character at a time, by data/fonts/t2s.json (OpenCC's, for the characters the overlay can show: made with the
//! fonts by dev/make_font_zh.py).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::OnceLock;

/// The tray's setting: 0 auto, 1 Traditional, 2 Simplified.
static MODE: AtomicU8 = AtomicU8::new(0);
/// What the card titles showed: 0 nothing yet, 1 Traditional, 2 Simplified.
static SEEN: AtomicU8 = AtomicU8::new(0);

#[allow(dead_code)]
pub const AUTO: u8 = 0;
pub const TRADITIONAL: u8 = 1;
pub const SIMPLIFIED: u8 = 2;

/// SmartTorch's English interface: the overlay's hint with the cards' English names, the menus and notes in English.
static ENGLISH: AtomicBool = AtomicBool::new(false);

pub fn set_english(on: bool) {
    ENGLISH.store(on, Ordering::Relaxed);
}

pub fn english() -> bool {
    ENGLISH.load(Ordering::Relaxed)
}

/// The words in the interface's language: `zh` (Traditional, shown in Simplified when that's on) or `en`.
pub fn tr(zh: &str, en: &str) -> String {
    if english() { en.to_string() } else { zh.to_string() }
}

pub fn set_mode(m: u8) {
    MODE.store(m.min(2), Ordering::Relaxed);
}

pub fn mode() -> u8 {
    MODE.load(Ordering::Relaxed)
}

/// Whether the overlay's words are shown in Simplified now.
pub fn simplified() -> bool {
    match mode() {
        TRADITIONAL => false,
        SIMPLIFIED => true,
        _ => SEEN.load(Ordering::Relaxed) == SIMPLIFIED,
    }
}

fn t2s() -> &'static HashMap<char, char> {
    static MAP: OnceLock<HashMap<char, char>> = OnceLock::new();
    MAP.get_or_init(|| {
        let m: HashMap<String, String> = serde_json::from_str(include_str!("../../../data/fonts/t2s.json")).unwrap_or_default();
        m.into_iter().filter_map(|(t, s)| Some((t.chars().next()?, s.chars().next()?))).collect()
    })
}

/// `s` in Simplified, a character at a time.
pub fn to_simplified(s: &str) -> String {
    let m = t2s();
    s.chars().map(|c| *m.get(&c).unwrap_or(&c)).collect()
}

/// `s` as it's shown now: in Simplified when the overlay shows Simplified.
pub fn shown(s: &str) -> std::borrow::Cow<'_, str> {
    if simplified() && s.chars().any(|c| t2s().contains_key(&c)) {
        std::borrow::Cow::Owned(to_simplified(s))
    } else {
        std::borrow::Cow::Borrowed(s)
    }
}

/// Which writing card titles read are in: characters only Traditional writes (the names' fold keys) against ones
/// only Simplified writes (its values); None when they don't tell.
pub fn script_of(texts: &[String]) -> Option<u8> {
    let fold = &vorax_scan::scan_data().names_zh.fold;
    let simp: std::collections::HashSet<char> = fold.iter().filter(|(t, s)| t != s).map(|(_, s)| *s).filter(|s| !fold.contains_key(s)).collect();
    let (mut t, mut s) = (0, 0);
    for c in texts.iter().flat_map(|x| x.chars()) {
        if fold.get(&c).is_some_and(|x| *x != c) {
            t += 1;
        } else if simp.contains(&c) {
            s += 1;
        }
    }
    match (t, s) {
        (t, s) if s >= 2 && s > 2 * t => Some(SIMPLIFIED),
        (t, s) if t >= 2 && t > 2 * s => Some(TRADITIONAL),
        _ => None,
    }
}

/// The card titles of a board read: the writing they show, for auto.
pub fn observe(texts: &[String]) {
    if let Some(w) = script_of(texts) {
        SEEN.store(w, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_tell_their_writing() {
        assert_eq!(script_of(&["脊 髓 灰 質 溶 液".into(), "藥 劑 箱 ( 小 )".into()]), Some(TRADITIONAL));
        assert_eq!(script_of(&["脊髓灰质溶液".into(), "药剂箱（小）".into()]), Some(SIMPLIFIED));
        // the same in both, or nothing read: no telling
        assert_eq!(script_of(&["孵化".into(), "".into()]), None);
    }

    #[test]
    fn words_turn_simplified() {
        assert_eq!(to_simplified("畫面未完全讀到"), "画面未完全读到");
        assert_eq!(to_simplified("細肢藥粉-蠱蟲 → 第 3 格"), "细肢药粉-蛊虫 → 第 3 格");
        assert_eq!(to_simplified("Ctrl+Alt+X 64%"), "Ctrl+Alt+X 64%");
    }
}
