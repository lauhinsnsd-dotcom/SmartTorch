//! The seals (封印秘語): every one the game has (data/seals.tsv, from the user's list), and the one a card's text
//! read from the screen is, however roughly it was read.

use std::sync::OnceLock;

/// One seal of the table.
#[derive(Debug, Clone, PartialEq)]
pub struct Seal {
    /// 財富, 軍械 or 奇珍
    pub category: String,
    /// as the game writes it
    pub text: String,
    /// the text with each number as '#', normalised: what a reading is matched to
    pub skeleton: String,
    pub numbers: Vec<u32>,
    /// 1 white, 2 blue, 3 purple, 4 gold
    pub rarity: u8,
    /// the "效果已翻倍" version
    pub doubled: bool,
}

/// Every seal: in each category, each wording comes 8 times, white to gold, then the same doubled.
pub fn table() -> &'static [Seal] {
    static TABLE: OnceLock<Vec<Seal>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut out: Vec<Seal> = Vec::new();
        for line in include_str!("../data/seals.tsv").lines() {
            if line.starts_with('#') {
                continue;
            }
            let mut cols = line.split('\t');
            let (Some(category), Some(text)) = (cols.next(), cols.next()) else { continue };
            let skeleton = skeleton(&normalise(text));
            let seen = out.iter().filter(|s| s.category == category && s.skeleton == skeleton).count();
            // the rarity given ("3", "4*" doubled), else by its place among its wording's 8
            let (rarity, doubled) = match cols.next().map(str::trim) {
                Some(r) if !r.is_empty() => (r.trim_end_matches('*').parse().unwrap_or(1), r.ends_with('*')),
                _ => ((seen % 4) as u8 + 1, seen >= 4),
            };
            out.push(Seal { category: category.to_string(), text: text.to_string(), skeleton, numbers: numbers(text), rarity, doubled });
        }
        out
    })
}

/// Simplified characters of the seals' words, as Traditional: a Simplified client's cards read as the table's.
const SIMPLIFIED: [(char, char); 27] = [
    ('虚', '虛'), ('宝', '寶'), ('础', '礎'), ('数', '數'), ('额', '額'), ('机', '機'), ('忆', '憶'), ('游', '遊'),
    ('丝', '絲'), ('会', '會'), ('个', '個'), ('军', '軍'), ('变', '變'), ('财', '財'), ('语', '語'), ('稀', '稀'),
    ('确', '確'), ('认', '認'), ('请', '請'), ('选', '選'), ('择', '擇'), ('条', '條'), ('效', '效'), ('翻', '翻'),
    ('杂', '雜'), ('枚', '枚'), ('领', '領'),
];

/// The text as matched: Traditional characters, no spaces, half-width symbols.
pub fn normalise(s: &str) -> String {
    // "(效果已翻倍)" and the like: no part of a seal's wording
    let mut kept = String::new();
    let mut depth = 0;
    for c in s.chars() {
        match c {
            '(' | '（' => depth += 1,
            ')' | '）' if depth > 0 => depth -= 1,
            ')' | '）' => {}
            _ if depth == 0 => kept.push(c),
            _ => {}
        }
    }
    // the marker with a bracket unread ("效果已翻倍）"): up to its 倍
    let head: Vec<char> = kept.chars().take(8).collect();
    let s = match (head.iter().position(|c| *c == '效'), head.iter().position(|c| *c == '倍')) {
        (Some(a), Some(b)) if a < b => kept.chars().skip(b + 1).collect(),
        _ => kept,
    };
    s.chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '•' | '·' | '・' | '.' | '。' | '"' | '\'' | '`'))
        .map(|c| match c {
            '％' => '%',
            '＋' => '+',
            '，' | ',' | '、' => '，',
            '（' => '(',
            '）' => ')',
            '０'..='９' => char::from_digit(c as u32 - '０' as u32, 10).unwrap(),
            _ => SIMPLIFIED.iter().find(|(s, _)| *s == c).map_or(c, |(_, t)| *t),
        })
        .collect()
}

/// Each run of digits as one '#'.
pub fn skeleton(s: &str) -> String {
    let mut out = String::new();
    let mut digit = false;
    for c in s.chars() {
        if c.is_ascii_digit() {
            if !digit {
                out.push('#');
            }
            digit = true;
        } else {
            out.push(c);
            digit = false;
        }
    }
    out
}

pub fn numbers(s: &str) -> Vec<u32> {
    let mut out = Vec::new();
    let mut cur: Option<u32> = None;
    for c in s.chars() {
        if let Some(d) = c.to_digit(10) {
            cur = Some(cur.unwrap_or(0).saturating_mul(10).saturating_add(d));
        } else if let Some(n) = cur.take() {
            out.push(n);
        }
    }
    out.extend(cur);
    out
}

/// A wording without the "虛海寶藏" (and "的") every seal starts with: a line's start is where reading fails
/// most (missed, or 賣藏 for 寶藏).
fn core(s: &str) -> String {
    // up to 寶藏's 藏 (its start misread: 虍藏, 賣藏)
    if let Some(i) = s.chars().take(5).position(|c| c == '藏') {
        let t: String = s.chars().skip(i + 1).collect();
        return t.trim_start_matches('的').to_string();
    }
    let mut chars: Vec<char> = s.chars().collect();
    let head: Vec<char> = "虛海寶藏".chars().collect();
    let n = chars.len().min(4);
    if (0..n).filter(|i| chars[*i] == head[*i]).count() >= 2 {
        chars.drain(..n);
    }
    let t: String = chars.into_iter().collect();
    t.trim_start_matches('的').to_string()
}

/// How far a reading's wording is from a seal's: with its start or without it, whichever is nearer.
fn wording_distance(read: &str, seal: &str) -> usize {
    distance(read, seal).min(distance(&core(read), &core(seal)))
}

/// Edit distance between two texts, by characters.
fn distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + usize::from(a[i - 1] != b[j - 1]));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// The seals' categories (首領: seen 2026-11-08, its values only partly known).
pub const CATEGORIES: [&str; 4] = ["財富", "軍械", "奇珍", "首領"];

/// Categories whose table is partial: a reading's numbers are shown as read when they're none of the table's.
const PARTIAL: [&str; 1] = ["首領"];

/// A seal not in the table, as read (a 首領 seal of a rarity not seen yet): made once per text.
fn as_read(like: &Seal, numbers: &[u32], rarity: Option<u8>, doubled: bool) -> &'static Seal {
    use std::collections::HashMap;
    use std::sync::Mutex;
    static MADE: OnceLock<Mutex<HashMap<String, &'static Seal>>> = OnceLock::new();
    // the template's numbers replaced, in order, by the ones read
    let mut text = String::new();
    let mut nums = numbers.iter();
    let mut in_number = false;
    for c in like.text.chars() {
        if c.is_ascii_digit() {
            if !in_number {
                text.push_str(&nums.next().map_or(String::new(), |n| n.to_string()));
            }
            in_number = true;
        } else {
            text.push(c);
            in_number = false;
        }
    }
    let mut made = MADE.get_or_init(Default::default).lock().unwrap();
    made.entry(text.clone()).or_insert_with(|| Box::leak(Box::new(Seal { category: like.category.clone(), text: text.clone(), skeleton: like.skeleton.clone(), numbers: numbers.to_vec(), rarity: rarity.unwrap_or(0), doubled })))
}

/// The category of a seal whose wording only one category has (the base seal's is in all three: None).
pub fn only_category(seal: &Seal) -> Option<&'static str> {
    let mut cats = table().iter().filter(|s| s.skeleton == seal.skeleton).map(|s| s.category.as_str());
    let first = cats.next()?;
    if cats.all(|c| c == first) {
        CATEGORIES.into_iter().find(|n| *n == first)
    } else {
        None
    }
}

/// The category a card's title reads as (財富秘語 → 財富), if near enough one.
pub fn category(title: &str) -> Option<&'static str> {
    let t = normalise(title);
    CATEGORIES.into_iter().map(|c| (c, distance(&t, &format!("{c}秘語")))).filter(|(_, d)| *d <= 2).min_by_key(|(_, d)| *d).map(|(c, _)| c)
}

/// The seal a card's text reads as: the wording nearest it (within a third of its length), then of that wording's
/// 8 the one with the numbers read; when they weren't read (or aren't one of them), the one of the rarity picked
/// (`rarity`, from the log) and whether "效果已翻倍" was read.
pub fn identify(category: Option<&str>, text: &str, rarity: Option<u8>, doubled: bool) -> Option<&'static Seal> {
    // a title misread as another category's (奇珍秘語 as 財富, the seventh test): the text alone
    identify_in(category, text, rarity, doubled).or_else(|| category.and_then(|_| identify_in(None, text, rarity, doubled)))
}

fn identify_in(category: Option<&str>, text: &str, rarity: Option<u8>, doubled: bool) -> Option<&'static Seal> {
    let read = normalise(text);
    let shape = skeleton(&read);
    if shape.chars().count() < 4 {
        return None;
    }
    let mut best: Option<(&str, usize)> = None;
    for s in table().iter().filter(|s| category.map_or(true, |c| s.category == c)) {
        let d = wording_distance(&shape, &s.skeleton);
        if best.map_or(true, |(_, b)| d < b) {
            best = Some((&s.skeleton, d));
        }
    }
    let (mut wording, d) = best?;
    if d * 3 > core(wording).chars().count() {
        // the start of a wording, the rest unread (small text): the one wording it can be the start of
        wording = start_of(category, &shape)?;
    }
    let same: Vec<&'static Seal> = table().iter().filter(|s| s.skeleton == wording && category.map_or(true, |c| s.category == c)).collect();
    let got = numbers(&read);
    let by_numbers: Vec<&'static Seal> = same.iter().copied().filter(|s| !got.is_empty() && s.numbers == got).collect();
    if by_numbers.is_empty() {
        if let Some(first) = same.first().filter(|s| PARTIAL.contains(&s.category.as_str()) && got.len() == s.numbers.len()) {
            return Some(as_read(first, &got, rarity, doubled));
        }
    }
    pick(if by_numbers.is_empty() { &same } else { &by_numbers }, rarity, doubled)
}

/// The only wording (of `category`'s) that a reading of at least 6 characters is the start of, nearly (a
/// character wrong in four).
fn start_of(category: Option<&str>, shape: &str) -> Option<&'static str> {
    let read = core(shape);
    let n = read.chars().count();
    if n < 6 {
        return None;
    }
    let mut found: Option<&'static str> = None;
    for s in table().iter().filter(|s| category.map_or(true, |c| s.category == c)) {
        let head: String = core(&s.skeleton).chars().take(n).collect();
        if head.chars().count() == n && distance(&read, &head) * 4 <= n {
            match found {
                Some(f) if f != s.skeleton => return None,
                _ => found = Some(&s.skeleton),
            }
        }
    }
    found
}

/// Of seals of one wording, the one of the rarity and doubling given, else as near as there is.
fn pick(list: &[&'static Seal], rarity: Option<u8>, doubled: bool) -> Option<&'static Seal> {
    let find = |f: &dyn Fn(&Seal) -> bool| list.iter().copied().find(|s| f(s));
    find(&|s| Some(s.rarity) == rarity && s.doubled == doubled).or_else(|| find(&|s| Some(s.rarity) == rarity)).or_else(|| find(&|s| s.doubled == doubled)).or(list.first().copied())
}

/// How a seal is shown: its text without the "虛海寶藏" every one starts with.
pub fn short(seal: &Seal) -> String {
    let t = seal.text.trim_start_matches("虛海寶藏").trim_start_matches('的').trim();
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub const RARITY_NAMES: [&str; 4] = ["白", "藍", "紫", "金"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table() {
        let t = table();
        assert_eq!(t.len(), 160);
        assert_eq!(t.iter().filter(|s| s.category == "財富").count(), 72);
        let base: Vec<_> = t.iter().filter(|s| s.category == "軍械" && s.text.contains("基礎掉落數量 +")).collect();
        assert_eq!(base.iter().map(|s| (s.rarity, s.doubled, s.numbers[0])).collect::<Vec<_>>(), vec![(1, false, 9), (2, false, 14), (3, false, 18), (4, false, 36), (1, true, 18), (2, true, 28), (3, true, 36), (4, true, 72)]);
        let gold = t.iter().find(|s| s.text == "虛海寶藏有 20% 機率額外掉落 194 個追憶碎絮").unwrap();
        assert_eq!((gold.rarity, gold.doubled), (4, false));
        let boss: Vec<_> = t.iter().filter(|s| s.category == "首領").map(|s| (s.numbers[0], s.rarity, s.doubled)).collect();
        assert_eq!(boss, vec![(5, 1, false), (10, 2, false), (20, 3, false), (40, 4, false), (10, 1, true), (20, 2, true), (40, 3, true), (80, 4, true)]);
    }

    #[test]
    fn readings() {
        // as read in the first test's pictures (2560×1600)
        let s = identify(Some("財富"), "虛海寶藏基礎掉落中，追憶碎絮的掉落數量額外 + 53%", Some(2), false).unwrap();
        assert_eq!(s.text, "虛海寶藏基礎掉落中，追憶碎絮的掉落數量額外 + 53%");
        assert_eq!((s.rarity, s.doubled), (2, false));
        let s = identify(Some("奇珍"), "虛海寶藏掉落的追憶遊絲 +84% 機率變成 3 倍", Some(4), true).unwrap();
        assert_eq!(s.numbers, vec![84, 3]);
        // a character misread, the number lost: the rarity picked and "doubled" decide
        let s = identify(Some("軍械"), "虛海寶藏額外掉落 枚英雄追意", Some(3), true).unwrap();
        assert_eq!(s.text, "虛海寶藏額外掉落 8 枚英雄追憶");
        // Simplified
        let s = identify(Some("財富"), "虚海宝藏有 20% 机率额外掉落 49 个追忆碎絮", None, false).unwrap();
        assert_eq!(s.numbers, vec![20, 49]);
        assert_eq!(identify(None, "請選擇一條封印秘語", None, false), None);
        // 1920×1080 at the smallest scale, as Windows read it (the ninth test)
        assert_eq!(identify(Some("奇珍"), "(效已倍)寶藏的掉落稀有度98", Some(3), true).unwrap().text, "虛海寶藏的 掉落稀有度 +98%");
        assert_eq!(identify(Some("奇珍"), "虍藏的掉落數量*4%", Some(1), false).unwrap().text, "虛海寶藏的掉落數量 + 14%");
        assert_eq!(normalise("（效果已翻倍）虛海寶藏(x"), "虛海寶藏");
        assert_eq!(normalise("效果已翻倍）"), "");
        assert_eq!(normalise("掉落的追憶碎絮 +84% 機率變成 2 倍"), "掉落的追憶碎絮+84%機率變成2倍");
        // 首領 (the sixth test's): from the table, else as read
        assert_eq!(identify(Some("首領"), "虛海寶藏有40%率额外掉落1个虚海請柬", Some(4), false).unwrap().text, "虛海寶藏有 40% 機率額外掉落 1 個虛海請柬");
        let odd = identify(Some("首領"), "虛海寶藏有 15% 機率額外掉落 1 個虛海請柬", Some(2), false).unwrap();
        assert_eq!((odd.text.as_str(), odd.rarity), ("虛海寶藏有 15% 機率額外掉落 1 個虛海請柬", 2));
        assert!(std::ptr::eq(odd, identify(Some("首領"), "虛海寶藏有15%機率額外掉落1個虛海請柬", Some(2), false).unwrap()));
        // the white one doubled, its number unread (the seventh test): its rarity decides
        assert_eq!(identify(Some("首領"), "虛海寶藏有%機率額外掉落1個虛海請柬", Some(1), true).unwrap().numbers, vec![10, 1]);
        // its title misread as another category's
        assert_eq!(identify(Some("財富"), "掉落稀有度+49", Some(1), true).unwrap().text, "虛海寶藏的 掉落稀有度 +49%");
        // a 財富 seal misread stays one of the table's
        assert_eq!(identify(Some("財富"), "虛海寶藏有 20% 機率額外掉落 50 個追憶碎絮", Some(1), false).unwrap().numbers, vec![20, 49]);
        assert_eq!(category("首领秘语"), Some("首領"));
        // its end unread (the fifth test's, a white card at the smallest interface scale)
        let t = identify(None, "虚海寶藏掉落的追憶遊絲", Some(1), false).unwrap();
        assert_eq!(t.text, "虛海寶藏掉落的追憶遊絲 +21% 機率變成 2 倍");
        assert_eq!(identify(None, "虚海寶藏掉落的追憶", Some(1), false), None);
        // its start missed, its % too (the second test's)
        assert_eq!(identify(Some("奇珍"), "掉落稀有度+25", Some(1), false).unwrap().text, "虛海寶藏的 掉落稀有度 +25%");
        assert_eq!(identify(None, "虚海賣藏基掉落中，追憶碎絮的掉落數量额外+35%", Some(1), false).unwrap().text, "虛海寶藏基礎掉落中，追憶碎絮的掉落數量額外 + 35%");
        assert_eq!(identify(None, "虛海寶藏基礎掉落中•,掉落英追憶的機率提升56%", Some(3), false).unwrap().text, "虛海寶藏基礎掉落中，掉落英雄追憶的機率提升 56%");
        assert_eq!(category("財富秘語"), Some("財富"));
        assert_eq!(category("军械秘语"), Some("軍械"));
        assert_eq!(category("奇珍秘話"), Some("奇珍"));
        assert_eq!(category("請選擇一條封印秘語"), None);
        assert_eq!(short(s), "有 20% 機率額外掉落 49 個追憶碎絮");
        assert_eq!(only_category(s), Some("財富"));
        assert_eq!(only_category(identify(None, "虛海寶藏的基礎掉落數量 +18%", Some(3), false).unwrap()), None);
    }
}
