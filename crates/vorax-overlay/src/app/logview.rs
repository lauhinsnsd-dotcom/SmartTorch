//! Your runs, read from the debug log and shown in the tray menu's style: one card per run (when, how far, the
//! instruments, how the advice's odds went), opened board by board with a click. Scrolls with the wheel, moves by
//! its title, closes with Esc or ×; "Raw log" opens the file itself (attach that when reporting a misread board).
//! Not in upstream: resized by dragging its right or bottom edge or its corner, zoomed with Ctrl + wheel (Chinese
//! lines run longer than the English ones it was laid out for), the size kept for next time (runs-view.txt).

use super::menu::put;
use super::sys::data_dir;
use crate::draw::{measure, paint, rgba, rounded, text, Fonts, GOLD, GOLD_DIM};
use serde_json::Value;
use std::cell::RefCell;
use std::rc::Rc;
use tiny_skia::{FillRule, PathBuilder, Pixmap, PixmapPaint, Stroke, Transform};
use windows::core::w;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, VK_ESCAPE};
use windows::Win32::UI::WindowsAndMessaging::*;

/// One board the overlay read and advised on.
struct Board {
    round: i64,
    phase: String,
    /// the advised move, and its odds of reaching 840k
    advice: String,
    pct: Option<u32>,
    instruments: Vec<String>,
    tanks: String,
    score: i64,
}

/// A run: the boards from a game's start to its screen closing.
struct Run {
    at: f64,
    boards: Vec<Board>,
}

impl Run {
    fn last(&self) -> &Board {
        self.boards.last().unwrap()
    }
    /// The advice's odds over the run: (round, %) where they changed.
    fn odds(&self) -> Vec<(i64, u32)> {
        let mut out: Vec<(i64, u32)> = Vec::new();
        for b in &self.boards {
            if let Some(p) = b.pct {
                if out.last().is_none_or(|l| l.1 != p) {
                    out.push((b.round, p));
                }
            }
        }
        out
    }
}

fn short(a: &str) -> &str {
    match a {
        "boneSentry" => "骨",
        "aemberon" => "異",
        "agito" => "覺",
        "pest" => "蟲",
        x => x,
    }
}

/// A rarity as one word (not in upstream: the Chinese edition's).
fn rarity_word(r: &str) -> &str {
    match r {
        "normal" => "普通",
        "magic" => "魔法",
        "rare" => "稀有",
        "boss" => "首領",
        x => x,
    }
}

/// A card's name as the Chinese client prints it (names_zh.json's first form), else as the log has it.
fn zh_card(name: &str) -> String {
    let z = &vorax_scan::scan_data().names_zh;
    [&z.instrument, &z.potion, &z.protocol].iter().find_map(|m| m.get(name).and_then(|f| f.first().cloned())).unwrap_or_else(|| name.to_string())
}

fn parse(text: &str) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    let mut open = false;
    // the run reached the protocols: a board after them with no cards in hand is the game closing (older logs read
    // it as a board: six 1×1 monsters in "round 1")
    let mut protocols = false;
    for line in text.lines() {
        let Some((t, msg)) = line.split_once(' ') else { continue };
        let at: f64 = t.parse().unwrap_or(0.0);
        if msg.contains("Vorax screen closed") || msg.starts_with("Vorax overlay") {
            open = false;
            protocols = false;
            continue;
        }
        let Some((head, state)) = msg.split_once("; board ") else { continue };
        let Ok(s) = serde_json::from_str::<Value>(state) else { continue };
        let mut round = s["round"].as_i64().unwrap_or(0);
        // a protocol board is round 11-13: one read lower (older logs: round 12 read as 2 while the screen faded in
        // again) takes the round before it
        if s["phase"] == "protocol" && round < 11 {
            round = runs.last().filter(|_| open).map_or(11, |r| r.last().round.max(11));
        }
        if head.starts_with("screen") {
            protocols = false;
        }
        let protocol = s["phase"] == "protocol";
        if protocols && !protocol && s["hand"].as_array().is_none_or(|h| h.is_empty()) {
            continue;
        }
        protocols |= protocol;
        let advice = head.split_once(" → ").map_or("", |x| x.1).trim();
        // the move, and its odds: the part with a "%" (a "not read" note can come after it)
        let parts: Vec<&str> = advice.split(" · ").collect();
        let odds_at = parts.iter().rposition(|p| p.contains('%') && p.split('%').next().is_some_and(|n| n.trim().parse::<u32>().is_ok()));
        let (mv, pct) = match odds_at {
            Some(i) => (parts[..i].join(" · ").trim().to_string(), parts[i].split('%').next().and_then(|n| n.trim().parse().ok())),
            None => (advice.to_string(), None),
        };
        let tanks: Vec<&Value> = s["tanks"].as_array().map(|a| a.iter().filter(|t| !t.is_null()).collect()).unwrap_or_default();
        let score = tanks.iter().map(|t| t["activity"].as_i64().unwrap_or(0) * t["quantity"].as_i64().unwrap_or(0)).sum();
        let board = Board {
            round,
            phase: s["phase"].as_str().unwrap_or("").to_string(),
            advice: mv,
            pct,
            instruments: s["instruments"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(zh_card)).collect()).unwrap_or_default(),
            tanks: tanks
                .iter()
                .map(|t| {
                    let r = rarity_word(t["rarity"].as_str().unwrap_or("?"));
                    format!("{r}{} {}×{}", short(t["archetype"].as_str().unwrap_or("?")), t["activity"], t["quantity"])
                })
                .collect::<Vec<_>>()
                .join("  ·  "),
            score,
        };
        // a new run: when the round goes back (a new game), or after the screen closed when it opens on a new game
        // (round 1, whatever is in the instrument panel: a new game can start with a boss drop in it); opened again on
        // the game under way (Escape and back, to skip long animations), the run goes on
        let fresh = round <= 1;
        let new = runs.last().is_none_or(|r| round < r.last().round) || (!open && fresh);
        if new {
            runs.push(Run { at, boards: Vec::new() });
            open = true;
        }
        runs.last_mut().unwrap().boards.push(board);
    }
    runs.retain(|r| !r.boards.is_empty());
    runs.reverse();
    runs
}

/// Local date and time of a log time (seconds since 1970), and the day as a number for "today".
fn local(at: f64) -> (String, i64) {
    let ft = (at * 1e7) as u64 + 116_444_736_000_000_000;
    let ft = FILETIME { dwLowDateTime: ft as u32, dwHighDateTime: (ft >> 32) as u32 };
    let (mut utc, mut l) = Default::default();
    unsafe {
        if FileTimeToSystemTime(&ft, &mut utc).is_err() || SystemTimeToTzSpecificLocalTime(None, &utc, &mut l).is_err() {
            return (String::new(), 0);
        }
    }
    let l: windows::Win32::Foundation::SYSTEMTIME = l;
    let day = l.wYear as i64 * 400 + l.wMonth as i64 * 32 + l.wDay as i64;
    (format!("{} 月 {} 日 · {:02}:{:02}", l.wMonth, l.wDay, l.wHour, l.wMinute), day)
}

fn when(at: f64) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64());
    let (s, day) = local(at);
    let (_, today) = local(now);
    let time = s.split(" · ").nth(1).unwrap_or("").to_string();
    if day == today {
        format!("今天 · {time}")
    } else if day == local(now - 86_400.0).1 {
        format!("昨天 · {time}")
    } else {
        s
    }
}

fn thousands(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

/// `t` cut to `room` pixels at `px`, with "…".
fn fit(font: &crate::draw::Face, t: &str, px: f32, room: f32) -> String {
    if measure(font, t, px) <= room {
        return t.to_string();
    }
    let mut s: String = t.to_string();
    while !s.is_empty() && measure(font, &format!("{s}…"), px) > room {
        s.pop();
    }
    format!("{s}…")
}

const W: f32 = 640.0;
/// not in upstream: the sizes and zoom the window may be dragged to (logical pixels, before the zoom)
const W_MIN: f32 = 480.0;
const W_MAX: f32 = 1800.0;
const H_MIN: f32 = 300.0;
const ZOOM_MIN: f32 = 0.7;
const ZOOM_MAX: f32 = 2.0;
/// how near the panel's right or bottom edge (screen pixels) a press resizes it
const GRIP: f32 = 10.0;

/// Which edges a drag moves (not in upstream).
#[derive(Clone, Copy, PartialEq)]
struct Edges {
    right: bool,
    bottom: bool,
}

/// A resize under way: the edges, where the cursor started (screen pixels), and the size then.
#[derive(Clone, Copy)]
struct Drag {
    edges: Edges,
    from: (i32, i32),
    w: f32,
    h: f32,
}
const MARGIN: f32 = 12.0;
const HEAD: f32 = 64.0;
const RUN_H: f32 = 70.0;
const BOARD_H: f32 = 40.0;

#[derive(Clone, Copy, PartialEq)]
enum Hit {
    Run(usize),
    Raw,
    Close,
    Title,
}

#[derive(Clone, Copy)]
enum Item {
    Run(usize),
    Board(usize, usize),
}

struct View {
    hwnd: HWND,
    fonts: Rc<Fonts>,
    s: f32,
    /// not in upstream: the panel's width (W unless dragged), the display's scale without the zoom, the zoom
    w: f32,
    dpi: f32,
    zoom: f32,
    drag: Option<Drag>,
    h: f32,
    runs: Vec<Run>,
    open: Vec<bool>,
    scroll: f32,
    hover: Option<Hit>,
}

thread_local! {
    static VIEW: RefCell<Option<View>> = const { RefCell::new(None) };
}

impl View {
    fn items(&self) -> Vec<(Item, f32, f32)> {
        let mut y = 0.0;
        let mut out = Vec::new();
        for (i, r) in self.runs.iter().enumerate() {
            out.push((Item::Run(i), y, RUN_H));
            y += RUN_H + 6.0;
            if self.open[i] {
                for j in 0..r.boards.len() {
                    out.push((Item::Board(i, j), y, BOARD_H));
                    y += BOARD_H;
                }
                y += 10.0;
            }
        }
        out
    }
    fn viewport(&self) -> f32 {
        self.h - HEAD - 14.0
    }
    fn content(&self) -> f32 {
        self.items().last().map_or(0.0, |(_, y, h)| y + h)
    }
    fn clamp(&mut self) {
        self.scroll = self.scroll.clamp(0.0, (self.content() - self.viewport()).max(0.0));
    }
    fn buttons(&self) -> ((f32, f32), (f32, f32)) {
        let raw_w = measure(&self.fonts.bold, "原始紀錄", 12.5) + 26.0;
        ((self.w - 14.0 - 30.0 - 8.0 - raw_w, raw_w), (self.w - 14.0 - 30.0, 30.0))
    }
    /// The edges a press at (x, y) in the window would drag (not in upstream): near the panel's right or bottom edge.
    fn edges(&self, x: f32, y: f32) -> Option<Edges> {
        let (r, b) = ((MARGIN + self.w) * self.s, (MARGIN + self.h) * self.s);
        let (l, t) = (MARGIN * self.s, MARGIN * self.s);
        if x < l || y < t || x > r + GRIP || y > b + GRIP {
            return None;
        }
        let e = Edges { right: x >= r - GRIP, bottom: y >= b - GRIP };
        (e.right || e.bottom).then_some(e)
    }
    fn hit(&self, x: f32, y: f32) -> Option<Hit> {
        let (cx, cy) = (x / self.s - MARGIN, y / self.s - MARGIN);
        if !(0.0..self.w).contains(&cx) || cy < 0.0 || cy > self.h {
            return None;
        }
        if cy < HEAD {
            let ((rx, rw), (xx, xw)) = self.buttons();
            if (16.0..48.0).contains(&cy) {
                if cx >= rx && cx < rx + rw {
                    return Some(Hit::Raw);
                }
                if cx >= xx && cx < xx + xw {
                    return Some(Hit::Close);
                }
            }
            return Some(Hit::Title);
        }
        let ly = cy - HEAD + self.scroll;
        self.items().into_iter().find(|(_, t, h)| ly >= *t && ly < t + h).and_then(|(it, _, _)| match it {
            Item::Run(i) => Some(Hit::Run(i)),
            Item::Board(i, _) => Some(Hit::Run(i)),
        })
    }
}

fn odds_colour(p: u32) -> (u8, u8, u8) {
    if p >= 70 {
        (130, 210, 140)
    } else if p >= 35 {
        GOLD
    } else {
        (230, 120, 100)
    }
}

fn render(v: &View) -> Pixmap {
    let s = v.s;
    let (w, h) = ((v.w + 2.0 * MARGIN) * s, (v.h + 2.0 * MARGIN) * s);
    let mut pm = Pixmap::new(w.ceil() as u32, h.ceil() as u32).unwrap();
    let (ox, oy, cw, ch) = (MARGIN * s, MARGIN * s, v.w * s, v.h * s);
    for (grow, a) in [(10.0, 0.05), (6.0, 0.08), (3.0, 0.12)] {
        if let Some(p) = rounded(ox - grow * s, oy - grow * s + 3.0 * s, cw + 2.0 * grow * s, ch + 2.0 * grow * s, (14.0 + grow) * s) {
            pm.fill_path(&p, &paint(rgba(0, 0, 0, a)), FillRule::Winding, Transform::identity(), None);
        }
    }
    if let Some(p) = rounded(ox, oy, cw, ch, 14.0 * s) {
        pm.fill_path(&p, &paint(rgba(22, 17, 20, 0.985)), FillRule::Winding, Transform::identity(), None);
        let (r, g, b) = GOLD_DIM;
        pm.stroke_path(&p, &paint(rgba(r, g, b, 0.35)), &Stroke { width: 1.0 * s, ..Default::default() }, Transform::identity(), None);
    }
    let (light, dim, soft) = ((241, 233, 226), (150, 138, 132), (210, 200, 195));
    // the title
    let mut pb = PathBuilder::new();
    let (dx, dy, dr) = (ox + 24.0 * s, oy + 24.0 * s, 9.0 * s);
    pb.move_to(dx, dy - dr);
    pb.line_to(dx + dr * 0.8, dy);
    pb.line_to(dx, dy + dr);
    pb.line_to(dx - dr * 0.8, dy);
    pb.close();
    if let Some(p) = pb.finish() {
        pm.fill_path(&p, &paint(rgba(GOLD.0, GOLD.1, GOLD.2, 1.0)), FillRule::Winding, Transform::identity(), None);
    }
    text(&mut pm, &v.fonts.bold, "對局紀錄", 16.0 * s, ox + 40.0 * s, oy + 29.0 * s, light, 1.0);
    let status = format!("共 {} 局 · 最新的在前 · 點一局可查看每回合", v.runs.len());
    text(&mut pm, &v.fonts.regular, &status, 12.0 * s, ox + 40.0 * s, oy + 47.0 * s, dim, 1.0);
    let ((rx, rw), (xx, xw)) = v.buttons();
    for (hit, bx, bw, label) in [(Hit::Raw, rx, rw, "原始紀錄"), (Hit::Close, xx, xw, "×")] {
        let under = v.hover == Some(hit);
        if let Some(p) = rounded(ox + bx * s, oy + 16.0 * s, bw * s, 30.0 * s, 8.0 * s) {
            pm.fill_path(&p, &paint(rgba(255, 255, 255, if under { 0.16 } else { 0.08 })), FillRule::Winding, Transform::identity(), None);
        }
        let px = if label == "×" { 18.0 } else { 12.5 };
        let tw = measure(&v.fonts.bold, label, px * s);
        text(&mut pm, &v.fonts.bold, label, px * s, ox + bx * s + (bw * s - tw) / 2.0, oy + (label == "×").then_some(37.5).unwrap_or(35.5) * s, light, 1.0);
    }
    if let Some(p) = rounded(ox + 14.0 * s, oy + (HEAD - 1.0) * s, cw - 28.0 * s, 1.0 * s, 0.0) {
        pm.fill_path(&p, &paint(rgba(255, 255, 255, 0.08)), FillRule::Winding, Transform::identity(), None);
    }

    // the list, drawn on its own picture so what scrolls out is cut off cleanly
    let (lw, lh) = (cw, v.viewport() * s);
    let Some(mut list) = Pixmap::new(lw.ceil() as u32, lh.max(1.0).ceil() as u32) else { return pm };
    if v.runs.is_empty() {
        text(&mut list, &v.fonts.regular, "還沒有紀錄：工具讀過一局後就會出現在這裡。", 13.0 * s, 20.0 * s, 30.0 * s, soft, 1.0);
    }
    for (it, top, height) in v.items() {
        let y0 = (top - v.scroll) * s;
        if y0 + height * s < 0.0 || y0 > lh {
            continue;
        }
        match it {
            Item::Run(i) => {
                let r = &v.runs[i];
                let last = r.last();
                let under = v.hover == Some(Hit::Run(i));
                if let Some(p) = rounded(12.0 * s, y0, lw - 24.0 * s, height * s, 10.0 * s) {
                    let c = if under { rgba(GOLD.0, GOLD.1, GOLD.2, 0.10) } else { rgba(255, 255, 255, 0.035) };
                    list.fill_path(&p, &paint(c), FillRule::Winding, Transform::identity(), None);
                }
                // ▸ or ▾
                let (tx, ty, t) = (28.0 * s, y0 + 22.0 * s, 4.5 * s);
                let mut pb = PathBuilder::new();
                if v.open[i] {
                    pb.move_to(tx - t, ty - t * 0.6);
                    pb.line_to(tx + t, ty - t * 0.6);
                    pb.line_to(tx, ty + t * 0.8);
                } else {
                    pb.move_to(tx - t * 0.6, ty - t);
                    pb.line_to(tx + t * 0.8, ty);
                    pb.line_to(tx - t * 0.6, ty + t);
                }
                pb.close();
                if let Some(p) = pb.finish() {
                    list.fill_path(&p, &paint(rgba(GOLD.0, GOLD.1, GOLD.2, 0.9)), FillRule::Winding, Transform::identity(), None);
                }
                text(&mut list, &v.fonts.bold, &when(r.at), 14.0 * s, 42.0 * s, y0 + 27.0 * s, light, 1.0);
                let odds = r.odds();
                if let Some(&(_, p)) = odds.last() {
                    let label = format!("{p}% 達 840k");
                    let tw = measure(&v.fonts.bold, &label, 13.5 * s);
                    text(&mut list, &v.fonts.bold, &label, 13.5 * s, lw - 28.0 * s - tw, y0 + 27.0 * s, odds_colour(p), 1.0);
                }
                let path: Vec<String> = odds.iter().map(|(rd, p)| format!("R{rd} {p}%")).collect();
                let line2 = format!("第 {} 回合 · 最後讀到 {} · {} 個畫面{}", last.round, thousands(last.score), r.boards.len(), if path.is_empty() { String::new() } else { format!(" · {}", path.join(" → ")) });
                text(&mut list, &v.fonts.regular, &fit(&v.fonts.regular, &line2, 12.5 * s, lw - 70.0 * s), 12.5 * s, 42.0 * s, y0 + 46.0 * s, soft, 1.0);
                let inst = if last.instruments.is_empty() { "沒有讀到手術用具".to_string() } else { last.instruments.join("  ·  ") };
                text(&mut list, &v.fonts.regular, &fit(&v.fonts.regular, &inst, 12.0 * s, lw - 70.0 * s), 12.0 * s, 42.0 * s, y0 + 62.0 * s, dim, 1.0);
            }
            Item::Board(i, j) => {
                let b = &v.runs[i].boards[j];
                let phase = match b.phase.as_str() {
                    "instrument" => "手術用具",
                    "protocol" => "手術方案",
                    _ => "藥劑",
                };
                text(&mut list, &v.fonts.bold, &format!("R{}", b.round), 12.5 * s, 44.0 * s, y0 + 17.0 * s, GOLD, 1.0);
                text(&mut list, &v.fonts.regular, phase, 11.5 * s, 78.0 * s, y0 + 17.0 * s, dim, 1.0);
                let advice = if b.advice.is_empty() { "（沒有建議）" } else { &b.advice };
                let right = b.pct.map(|p| format!("{p}%")).unwrap_or_default();
                let rw = measure(&v.fonts.bold, &right, 12.5 * s);
                text(&mut list, &v.fonts.regular, &fit(&v.fonts.regular, advice, 13.0 * s, lw - 190.0 * s - rw), 13.0 * s, 150.0 * s, y0 + 17.0 * s, light, 1.0);
                if let Some(p) = b.pct {
                    text(&mut list, &v.fonts.bold, &right, 12.5 * s, lw - 28.0 * s - rw, y0 + 17.0 * s, odds_colour(p), 1.0);
                }
                // the instruments when they changed, else the tanks
                let prev = j.checked_sub(1).map(|k| &v.runs[i].boards[k].instruments);
                let detail = if prev != Some(&b.instruments) && !b.instruments.is_empty() { format!("手術用具：{}", b.instruments.join(" · ")) } else { b.tanks.clone() };
                text(&mut list, &v.fonts.regular, &fit(&v.fonts.regular, &detail, 11.5 * s, lw - 190.0 * s), 11.5 * s, 150.0 * s, y0 + 33.0 * s, dim, 1.0);
            }
        }
    }
    pm.draw_pixmap((ox) as i32, (oy + HEAD * s) as i32, list.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
    // where the list goes on: a scroll bar
    let (content, view) = (v.content(), v.viewport());
    if content > view {
        let (bh, track) = ((view / content * view).max(30.0), view);
        let by = v.scroll / (content - view) * (track - bh);
        if let Some(p) = rounded(ox + cw - 7.0 * s, oy + (HEAD + by) * s, 3.0 * s, bh * s, 1.5 * s) {
            pm.fill_path(&p, &paint(rgba(255, 255, 255, 0.18)), FillRule::Winding, Transform::identity(), None);
        }
    }
    // not in upstream: the corner to drag, three short diagonal lines
    let (gx, gy) = (ox + cw - 6.0 * s, oy + ch - 6.0 * s);
    for k in 1..=3 {
        let d = 4.0 * k as f32 * s;
        let mut pb = PathBuilder::new();
        pb.move_to(gx - d, gy);
        pb.line_to(gx, gy - d);
        if let Some(p) = pb.finish() {
            pm.stroke_path(&p, &paint(rgba(255, 255, 255, 0.25)), &Stroke { width: 1.2 * s, ..Default::default() }, Transform::identity(), None);
        }
    }
    pm
}

/// The size and zoom last left (not in upstream): "width height zoom" in runs-view.txt.
fn saved_view() -> Option<(f32, f32, f32)> {
    let t = std::fs::read_to_string(data_dir().join("runs-view.txt")).ok()?;
    let v: Vec<f32> = t.split_whitespace().filter_map(|x| x.parse().ok()).collect();
    match v[..] {
        [w, h, z] if w.is_finite() && h.is_finite() && z.is_finite() => Some((w.clamp(W_MIN, W_MAX), h.max(H_MIN), z.clamp(ZOOM_MIN, ZOOM_MAX))),
        _ => None,
    }
}

fn save_view(v: &View) {
    let _ = std::fs::write(data_dir().join("runs-view.txt"), format!("{} {} {}", v.w, v.h, v.zoom));
}

fn redraw(v: &View) {
    let pm = render(v);
    let mut r = RECT::default();
    unsafe {
        let _ = GetWindowRect(v.hwnd, &mut r);
        put(v.hwnd, r.left, r.top, pm.width() as i32, pm.height() as i32, pm.data());
    }
}

/// The view, unless a message came in while it's in use (a resize can send one): then nothing (not in upstream,
/// which borrowed it outright).
fn with<R>(f: impl FnOnce(&mut View) -> R) -> Option<R> {
    VIEW.with(|c| c.try_borrow_mut().ok().and_then(|mut v| v.as_mut().map(f)))
}

/// Opens the runs (or brings them to the front, read afresh), centred on the screen the cursor is on.
pub fn open(fonts: Rc<Fonts>) {
    close();
    let text = std::fs::read_to_string(data_dir().join("overlay.log")).unwrap_or_default();
    let runs = parse(&text);
    unsafe {
        let Ok(instance) = GetModuleHandleW(None) else { return };
        let class = w!("VoraxRsRuns");
        let wc = WNDCLASSW { lpfnWndProc: Some(proc), hInstance: instance.into(), lpszClassName: class, hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(), ..Default::default() };
        RegisterClassW(&wc);
        let mut cur = POINT::default();
        let _ = GetCursorPos(&mut cur);
        let mon = MonitorFromPoint(cur, MONITOR_DEFAULTTONEAREST);
        let (mut dx, mut dy) = (96u32, 96u32);
        let _ = GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
        let s = dx as f32 / 96.0;
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(mon, &mut mi);
        let work = mi.rcWork;
        let most_h = ((work.bottom - work.top) as f32 / s) * 0.95;
        let (w0, h0, zoom) = saved_view().unwrap_or((W, (((work.bottom - work.top) as f32 / s) * 0.82).min(820.0), 1.0));
        let (dpi, s) = (s, s * zoom);
        let h = h0.min(most_h / zoom).max(H_MIN);
        let n = runs.len();
        // the newest run open from the start
        let mut open = vec![false; n];
        if let Some(o) = open.first_mut() {
            *o = true;
        }
        let v = View { hwnd: HWND::default(), fonts, s, w: w0, dpi, zoom, drag: None, h, runs, open, scroll: 0.0, hover: None };
        let pm = render(&v);
        let (w, hh) = (pm.width() as i32, pm.height() as i32);
        let x = work.left + ((work.right - work.left) - w) / 2;
        let y = work.top + ((work.bottom - work.top) - hh) / 2;
        let Ok(hwnd) = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOOLWINDOW, class, w!("Your Vorax runs"), WS_POPUP, x, y, w, hh, None, None, instance, None) else { return };
        put(hwnd, x, y, w, hh, pm.data());
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
        VIEW.with(|c| *c.borrow_mut() = Some(View { hwnd, ..v }));
    }
}

pub fn close() {
    let h = VIEW.with(|c| c.borrow_mut().take().map(|v| v.hwnd));
    if let Some(h) = h {
        unsafe {
            let _ = DestroyWindow(h);
        }
    }
}

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let (x, y) = ((lp.0 & 0xffff) as i16 as f32, ((lp.0 >> 16) & 0xffff) as i16 as f32);
    match msg {
        // dragging by the title moves the window
        WM_NCHITTEST => {
            let mut r = RECT::default();
            let _ = GetWindowRect(hwnd, &mut r);
            // not in upstream: an edge to drag is the window's own, not the title's
            let on_title = with(|v| {
                let (cx, cy) = (x - r.left as f32, y - r.top as f32);
                v.edges(cx, cy).is_none() && v.hit(cx, cy) == Some(Hit::Title)
            })
            .unwrap_or(false);
            LRESULT(if on_title { HTCAPTION as isize } else { HTCLIENT as isize })
        }
        // not in upstream: the resize cursor over an edge, and pressing there starts a resize
        WM_SETCURSOR => {
            let mut cur = POINT::default();
            let _ = GetCursorPos(&mut cur);
            let mut r = RECT::default();
            let _ = GetWindowRect(hwnd, &mut r);
            let e = with(|v| v.drag.map(|d| d.edges).or_else(|| v.edges((cur.x - r.left) as f32, (cur.y - r.top) as f32))).flatten();
            match e {
                Some(e) => {
                    let id = match (e.right, e.bottom) {
                        (true, true) => IDC_SIZENWSE,
                        (true, false) => IDC_SIZEWE,
                        _ => IDC_SIZENS,
                    };
                    if let Ok(c) = LoadCursorW(None, id) {
                        SetCursor(c);
                    }
                    LRESULT(1)
                }
                None => DefWindowProcW(hwnd, msg, wp, lp),
            }
        }
        WM_LBUTTONDOWN => {
            let started = with(|v| match v.edges(x, y) {
                Some(edges) => {
                    let mut cur = POINT::default();
                    let _ = GetCursorPos(&mut cur);
                    v.drag = Some(Drag { edges, from: (cur.x, cur.y), w: v.w, h: v.h });
                    true
                }
                None => false,
            })
            .unwrap_or(false);
            if started {
                SetCapture(hwnd);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE if with(|v| v.drag.is_some()).unwrap_or(false) => {
            let mut cur = POINT::default();
            let _ = GetCursorPos(&mut cur);
            with(|v| {
                if let Some(d) = v.drag {
                    let (dx, dy) = ((cur.x - d.from.0) as f32 / v.s, (cur.y - d.from.1) as f32 / v.s);
                    if d.edges.right {
                        v.w = (d.w + dx).clamp(W_MIN, W_MAX);
                    }
                    if d.edges.bottom {
                        v.h = (d.h + dy).max(H_MIN);
                    }
                    v.clamp();
                    redraw(v);
                }
            });
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            with(|v| {
                let h = v.hit(x, y).filter(|h| *h != Hit::Title);
                if h != v.hover {
                    v.hover = h;
                    redraw(v);
                }
            });
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = ((wp.0 >> 16) & 0xffff) as i16 as f32;
            // not in upstream: Ctrl + wheel zooms (MK_CONTROL, 0x0008)
            if wp.0 & 0x0008 != 0 {
                with(|v| {
                    v.zoom = (v.zoom * if delta > 0.0 { 1.1 } else { 1.0 / 1.1 }).clamp(ZOOM_MIN, ZOOM_MAX);
                    v.s = v.dpi * v.zoom;
                    v.clamp();
                    redraw(v);
                    save_view(v);
                });
                return LRESULT(0);
            }
            with(|v| {
                v.scroll -= delta / 120.0 * 70.0;
                v.clamp();
                redraw(v);
            });
            LRESULT(0)
        }
        WM_LBUTTONUP if with(|v| v.drag.is_some()).unwrap_or(false) => {
            let _ = ReleaseCapture();
            with(|v| {
                v.drag = None;
                save_view(v);
            });
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let act = with(|v| match v.hit(x, y) {
                Some(Hit::Run(i)) => {
                    v.open[i] = !v.open[i];
                    v.clamp();
                    redraw(v);
                    None
                }
                other => other,
            })
            .flatten();
            match act {
                Some(Hit::Close) => close(),
                Some(Hit::Raw) => {
                    let _ = std::process::Command::new("notepad.exe").arg(data_dir().join("overlay.log")).spawn();
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_KEYDOWN if wp.0 == VK_ESCAPE.0 as usize => {
            close();
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

/// The runs drawn into a picture from a log's text (for checking the look without opening the window).
pub fn preview(log: &str, fonts: Rc<Fonts>, s: f32, h: f32) -> Pixmap {
    let runs = parse(log);
    let mut open = vec![false; runs.len()];
    if let Some(o) = open.first_mut() {
        *o = true;
    }
    render(&View { hwnd: HWND::default(), fonts, s, w: W, dpi: s, zoom: 1.0, drag: None, h, runs, open, scroll: 0.0, hover: None })
}

#[cfg(test)]
mod tests {
    use super::parse;

    fn board(t: u32, trigger: &str, round: u32, phase: &str, hand: &str, tank: (u32, u32)) -> String {
        let m = format!(r#"{{"archetype":"agito","rarity":"normal","activity":{},"quantity":{}}}"#, tank.0, tank.1);
        format!(r#"{t}.0 {trigger} 1920x1080: picture 400 ms, read 300 ms, advice 1 ms → (nothing to advise); board {{"tanks":[{m},{m},null,null,null,null],"hand":[{hand}],"instruments":[],"phase":"{phase}","round":{round}}}"#)
    }

    /// The game closing after the protocols, read as a board (no cards, six 1×1 monsters, "round 1"), isn't a run.
    #[test]
    fn the_game_closing_is_not_a_run() {
        let log = [
            board(1, "screen", 1, "instrument", r#""Chrysalis Specimen","Incubation Sac""#, (1, 36)),
            board(2, "board", 10, "potion", r#""Xeno-Hormone""#, (300, 500)),
            board(3, "board", 11, "protocol", r#""Laser Amputation""#, (320, 500)),
            board(4, "board", 12, "protocol", r#""Trepanation""#, (340, 500)),
            board(5, "board", 1, "potion", "", (1, 1)),
            "6.0 the game's Vorax screen closed".to_string(),
            board(7, "screen", 1, "instrument", r#""Aberrant Sprout","Prismatic Liver""#, (1, 36)),
        ]
        .join("\n");
        let runs = parse(&log);
        assert_eq!(runs.len(), 2, "{:?}", runs.iter().map(|r| r.boards.len()).collect::<Vec<_>>());
        // newest first: the new game, then the first one with its 4 real boards
        assert_eq!(runs[0].boards.len(), 1);
        assert_eq!(runs[1].boards.len(), 4);
        assert_eq!(runs[1].last().round, 12);
    }
}

#[cfg(test)]
mod reopen_tests {
    use super::parse;

    fn board(t: u32, trigger: &str, round: u32, phase: &str, instruments: &str) -> String {
        let m = r#"{"archetype":"agito","rarity":"normal","activity":30,"quantity":40}"#;
        format!(r#"{t}.0 {trigger} 1920x1080: picture 400 ms, read 300 ms, advice 1 ms → Xeno-Hormone · 40% reach 840k; board {{"tanks":[{m},null,null,null,null,null],"hand":["Xeno-Hormone"],"instruments":[{instruments}],"phase":"{phase}","round":{round}}}"#)
    }

    /// Escape and back mid-game (to skip long animations) is the same run; a new game after it is a new one.
    #[test]
    fn a_screen_opened_again_goes_on_with_the_run() {
        let log = [
            board(1, "screen", 1, "instrument", ""),
            board(2, "board", 4, "potion", r#""Chrysalis Specimen""#),
            "3.0 the game's Vorax screen closed".to_string(),
            board(4, "screen", 5, "potion", r#""Chrysalis Specimen""#),
            board(5, "board", 6, "potion", r#""Chrysalis Specimen""#),
            "6.0 the game's Vorax screen closed".to_string(),
            board(7, "screen", 1, "instrument", ""),
        ]
        .join("\n");
        let runs = parse(&log);
        assert_eq!(runs.iter().map(|r| r.boards.len()).collect::<Vec<_>>(), vec![1, 4]);
    }
}

#[cfg(test)]
mod protocol_round_tests {
    use super::parse;

    /// Round 12's protocol board read as "2" (the screen fading in again) stays in its run, as round 12.
    #[test]
    fn a_protocol_board_read_below_round_11_keeps_its_run() {
        let b = |t: u32, trigger: &str, round: u32, phase: &str| {
            format!(r#"{t}.0 {trigger} 1920x1080: picture 400 ms, read 300 ms, advice 1 ms → (nothing to advise); board {{"tanks":[{{"archetype":"pest","rarity":"magic","activity":300,"quantity":500}},null,null,null,null,null],"hand":["Trepanation"],"instruments":["Chrysalis Specimen"],"phase":"{phase}","round":{round}}}"#)
        };
        let log = [b(1, "screen", 1, "instrument"), b(2, "board", 11, "protocol"), b(3, "board", 12, "protocol"), b(4, "screen", 2, "protocol")].join("
");
        let runs = parse(&log);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].boards.iter().map(|b| b.round).collect::<Vec<_>>(), vec![1, 11, 12, 12]);
    }

    #[test]
    fn a_chinese_advice_keeps_its_odds_before_a_note() {
        let log = r#"100.0 screen 2560x1440: picture 400 ms, read 300 ms, advice 1 ms → 細肢藥粉-蠱蟲  →  第 3 格 · 64% 達 840k · 未讀到：第 2 張卡（Ctrl+Alt+X）; board {"tanks":[{"archetype":"boneSentry","rarity":"rare","activity":15,"quantity":12},null,null,null,null,null],"hand":["Eggshell Powder"],"instruments":["Chrysalis Specimen"],"phase":"potion","round":1}"#;
        let runs = parse(log);
        let b = &runs[0].boards[0];
        assert_eq!((b.advice.as_str(), b.pct), ("細肢藥粉-蠱蟲  →  第 3 格", Some(64)));
        assert_eq!(b.instruments, vec!["人蛹標本".to_string()]);
        assert_eq!(b.tanks, "稀有骨 15×12");
    }
}
