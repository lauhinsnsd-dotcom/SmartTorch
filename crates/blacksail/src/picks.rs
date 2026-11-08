//! This run's seals as the box shows them.

use crate::seals::{short, Seal, RARITY_NAMES};

/// A seal picked: its rarity (from the log), and what it is when its card was read.
#[derive(Debug, Clone)]
pub struct Pick {
    pub rarity: Option<u8>,
    pub category: Option<&'static str>,
    pub seal: Option<&'static Seal>,
    /// picked before the recorder started (from the log: its rarity only)
    pub before: bool,
}

/// The box's buttons: see-through (clicks pass through the box to the game) and close (黑帆 stopped).
pub const SEE_THROUGH: &str = "透明";
pub const CLOSE: &str = "×";

/// A run has 6 seals; the 7th pick starts the next run.
pub const PER_RUN: usize = 6;

/// The rarity of the seal confirmed, from the log's button line ("Rarity4SelectedBtn": gold).
pub fn rarity_picked(line: &str) -> Option<u8> {
    if !line.contains("UIButton@") {
        return None;
    }
    let rest = line.split("Rarity").nth(1)?;
    let (n, tail) = rest.split_at(rest.find(|c: char| !c.is_ascii_digit())?);
    tail.starts_with("SelectedBtn").then(|| n.parse().ok()).flatten().filter(|r| (1..=4).contains(r))
}

/// A rarity's colour in the box (red, green, blue).
pub fn color(rarity: Option<u8>) -> (u8, u8, u8) {
    match rarity {
        Some(1) => (225, 225, 225),
        Some(2) => (90, 165, 255),
        Some(3) => (205, 125, 255),
        Some(4) => (255, 190, 70),
        _ => (160, 160, 160),
    }
}

/// The box's lines: a heading, then each seal picked as the game words it (without the "虛海寶藏" all start
/// with, its colour word or category: the user's choice), in its rarity's colour.
pub fn rows(picks: &[Pick]) -> Vec<(String, (u8, u8, u8))> {
    let mut out = vec![(format!("黑帆備忘 {}/{PER_RUN}", picks.len()), (232, 180, 74))];
    for (i, p) in picks.iter().enumerate() {
        let what = match p.seal {
            Some(s) => short(s),
            None if p.before => "（啟動前已選，內容未知）".to_string(),
            None => "（讀不到）".to_string(),
        };
        out.push((format!("{}. {what}", i + 1), color(p.rarity)));
    }
    out
}

/// A seal picked in full, for the logs: its rarity, category and text as the game writes it.
pub fn describe(p: &Pick) -> String {
    let rarity = p.rarity.map_or("?", |r| RARITY_NAMES[(r as usize - 1).min(3)]);
    match (p.seal, p.category) {
        (Some(s), c) => format!("{rarity} {} {}", c.unwrap_or("?"), short(s)),
        (None, Some(c)) => format!("{rarity} {c} （讀不到）"),
        (None, None) if p.before => format!("{rarity} （啟動前已選，內容未知）"),
        (None, None) => format!("{rarity} （讀不到）"),
    }
}

/// The game log's time of a line ("[2026.11.08-09.28.39:217]..."), in seconds since that day began.
pub fn log_time(line: &str) -> Option<f64> {
    let t = line.strip_prefix('[')?.split(']').next()?;
    let (_, clock) = t.split_once('-')?;
    let mut parts = clock.split(['.', ':']);
    let (h, m, s, ms): (f64, f64, f64, f64) = (parts.next()?.parse().ok()?, parts.next()?.parse().ok()?, parts.next()?.parse().ok()?, parts.next()?.parse().ok()?);
    Some(h * 3600.0 + m * 60.0 + s + ms / 1000.0)
}

/// The level entered: its type and id ("LevelMgr@ LevelUid, LevelType, LevelId = 0 5 300099": the port).
pub fn level(line: &str) -> Option<(i64, i64)> {
    let rest = line.split("LevelMgr@ LevelUid, LevelType, LevelId = ").nth(1)?;
    let mut n = rest.split_whitespace().filter_map(|x| x.parse::<i64>().ok());
    let (_uid, kind, id) = (n.next()?, n.next()?, n.next()?);
    Some((kind, id))
}

/// What a log line means for this run's seals.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    /// a seal confirmed, of this rarity
    Picked(u8),
    /// the port left (the bride beaten, the chest opened): the run is over
    LeftPort,
}

/// Follows the run through the log: the port is the only level of type 5 seen (300099; the others: 0 the hideout,
/// 3 maps, 21 and 22), entered whenever the player likes after the 6th seal; leaving it for any other level ends
/// the run.
#[derive(Debug, Default)]
pub struct Run {
    in_port: Option<i64>,
}

/// The port's level type.
pub const PORT_LEVEL_TYPE: i64 = 5;

impl Run {
    pub fn feed(&mut self, line: &str) -> Option<Event> {
        if let Some((kind, id)) = level(line) {
            if self.in_port.is_some_and(|p| p != id) {
                self.in_port = None;
                return Some(Event::LeftPort);
            }
            if kind == PORT_LEVEL_TYPE {
                self.in_port = Some(id);
            }
            return None;
        }
        rarity_picked(line).map(Event::Picked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seals::identify;

    #[test]
    fn rarity_from_the_log() {
        assert_eq!(rarity_picked("[2026.11.08-09.29.52:737][892]TLGame: Display: [Game] UIButton@ normal-click Rarity2SelectedBtn"), Some(2));
        assert_eq!(rarity_picked("[2026.11.08-09.30.20:551][558]TLGame: Display: [Game] UIButton@ normal-click Rarity4SelectedBtn"), Some(4));
        assert_eq!(rarity_picked("[t]TLGame: Display: [Game] UIButton@ normal-click BtnConfirm"), None);
        assert_eq!(rarity_picked("[t]TLGame: Display: [Game] UIButton@ normal-click Rarity2Btn"), None);
    }

    #[test]
    fn a_run_through_the_log() {
        // the second test's 6th seal, the port, home, the next run's first seal
        let lines = [
            "[2026.11.08-10.08.02:875][700]TLGame: Display: [Game] UIButton@ normal-click Rarity2SelectedBtn",
            "[2026.11.08-10.08.06:372][723]TLShipping: Display: [Game] LevelMgr@ LevelUid, LevelType, LevelId = 0 5 300099",
            "[2026.11.08-10.08.06:372][723]TLLua: Display: [Game] TipMsgShowMgr@DispatchPageRunChange PageName = VoidPort , PageRunState = Destory ",
            "[2026.11.08-10.08.36:180][638]TLShipping: Display: [Game] LevelMgr@ LevelUid, LevelType, LevelId = 111000 0 110",
            "[2026.11.08-10.09.17:786][724]TLShipping: Display: [Game] LevelMgr@ LevelUid, LevelType, LevelId = 1121002 3 5302",
            "[2026.11.08-10.10.03:046][700]TLGame: Display: [Game] UIButton@ normal-click Rarity3SelectedBtn",
        ];
        let mut run = Run::default();
        let got: Vec<Option<Event>> = lines.iter().map(|l| run.feed(l)).collect();
        assert_eq!(got, vec![Some(Event::Picked(2)), None, None, Some(Event::LeftPort), None, Some(Event::Picked(3))]);
        // the port entered a while later (the player's choice): the same
        let mut run = Run::default();
        let later = "[2026.11.08-10.20.06:372][723]TLShipping: Display: [Game] LevelMgr@ LevelUid, LevelType, LevelId = 0 5 300099";
        let order = [lines[0], lines[2], lines[3], later, lines[3], lines[4]];
        let got: Vec<Option<Event>> = order.iter().map(|l| run.feed(l)).collect();
        assert_eq!(got, vec![Some(Event::Picked(2)), None, None, None, Some(Event::LeftPort), None]);
        assert_eq!(log_time(lines[0]), Some(10.0 * 3600.0 + 8.0 * 60.0 + 2.875));
    }

    #[test]
    fn the_box() {
        let seal = identify(Some("財富"), "虛海寶藏基礎掉落中，追憶碎絮的掉落數量額外 + 53%", Some(2), false);
        let picks = vec![Pick { rarity: Some(2), category: Some("財富"), seal, before: false }, Pick { rarity: Some(4), category: None, seal: None, before: false }, Pick { rarity: Some(1), category: None, seal: None, before: true }];
        let r = rows(&picks);
        assert_eq!(r[0].0, "黑帆備忘 3/6");
        assert_eq!(r[3].0, "3. （啟動前已選，內容未知）");
        assert_eq!(r[1], ("1. 基礎掉落中，追憶碎絮的掉落數量額外 + 53%".to_string(), (90, 165, 255)));
        assert_eq!(r[2], ("2. （讀不到）".to_string(), (255, 190, 70)));
        assert_eq!(describe(&picks[0]), "藍 財富 基礎掉落中，追憶碎絮的掉落數量額外 + 53%");
    }
}
