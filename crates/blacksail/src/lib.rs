//! The parts that don't need Windows (tested on any system): following the game's log, which of its lines are
//! kept, which game pages are open, and pictures (likeness, saving).
//!
//! (SmartTorch's 黑帆 page: the box over the game listing this run's seals; its Windows part is in
//! crates/vorax-overlay/src/app/blacksail.)

pub mod cards;
pub mod picks;
pub mod seals;

use std::path::Path;

/// A picture: RGBA, row by row.
pub struct Img {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

impl Img {
    pub fn new(width: usize, height: usize) -> Img {
        Img { width, height, data: vec![0; width * height * 4] }
    }

    /// The part at (x, y), w × h (inside the picture).
    pub fn crop(&self, x: usize, y: usize, w: usize, h: usize) -> Img {
        let mut out = Img::new(w, h);
        for row in 0..h {
            let s = ((y + row) * self.width + x) * 4;
            out.data[row * w * 4..(row + 1) * w * 4].copy_from_slice(&self.data[s..s + w * 4]);
        }
        out
    }

    /// Whether it's all black (or nearly): what a capture gives some windows it can't see into.
    pub fn black(&self) -> bool {
        let (mut sum, mut n) = (0u64, 0u64);
        for p in self.data.chunks_exact(4).step_by(97) {
            sum += p[0] as u64 + p[1] as u64 + p[2] as u64;
            n += 1;
        }
        n == 0 || sum < n * 3 * 4
    }

    /// Saved as PNG (RGB, fast compression: a full screen every few tenths of a second).
    pub fn save_png(&self, path: &Path) -> std::io::Result<()> {
        let file = std::fs::File::create(path)?;
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), self.width as u32, self.height as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Fast);
        let mut w = enc.write_header().map_err(std::io::Error::other)?;
        let rgb: Vec<u8> = self.data.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
        w.write_image_data(&rgb).map_err(std::io::Error::other)?;
        Ok(())
    }
}

/// A 96-pixel-wide grey thumbnail, for telling whether the screen changed.
pub fn thumb(img: &Img) -> Vec<f32> {
    let w = 96usize;
    let h = ((img.height * w) as f64 / img.width.max(1) as f64).round().max(1.0) as usize;
    let mut g = vec![0f32; w * h];
    let mut n = vec![0f32; w * h];
    let (sx, sy) = (w as f64 / img.width as f64, h as f64 / img.height as f64);
    for y in (0..img.height).step_by(2) {
        let row = ((y as f64 * sy) as usize).min(h - 1) * w;
        for x in (0..img.width).step_by(2) {
            let i = (y * img.width + x) * 4;
            let t = row + ((x as f64 * sx) as usize).min(w - 1);
            g[t] += 0.299 * img.data[i] as f32 + 0.587 * img.data[i + 1] as f32 + 0.114 * img.data[i + 2] as f32;
            n[t] += 1.0;
        }
    }
    for i in 0..g.len() {
        g[i] /= n[i].max(1.0);
    }
    g
}

/// Mean absolute grey difference of two thumbnails, 0-255 (255 when their sizes differ).
pub fn thumb_diff(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 255.0;
    }
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum::<f32>() / a.len() as f32
}

/// A picture is saved when the screen moved at least this much from the last one saved (the cards dealt, one
/// picked, the switch to the base seal), not for the cards' shimmer or the pointer over them (1.2-2.9 in the first
/// test's pictures, 2560×1600).
pub const CHANGED: f32 = 3.0;

/// The button that ends a pick: the seal's (`Rarity2SelectedBtn`: the rarity of the seal picked) or the flame's
/// (`BtnConfirm`). What follows is an animation until the page closes, not worth a picture (27 of the first
/// test's 34).
pub fn ends_pick(line: &str) -> bool {
    line.contains("UIButton@") && (line.contains("SelectedBtn") || line.contains("BtnConfirm"))
}

/// Follows a growing log: each poll returns the complete lines written since the last one (from SmartTorch's
/// vorax-scan live.rs). The first poll only notes where it ends.
pub struct Follower {
    path: std::path::PathBuf,
    offset: Option<u64>,
    rest: Vec<u8>,
}

impl Follower {
    pub fn new(path: std::path::PathBuf) -> Follower {
        Follower { path, offset: None, rest: Vec::new() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Starts from `offset` (where reading the log so far ended) instead of the log's end.
    pub fn from(path: std::path::PathBuf, offset: u64) -> Follower {
        Follower { path, offset: Some(offset), rest: Vec::new() }
    }

    pub fn poll(&mut self) -> Vec<String> {
        use std::io::{Read, Seek, SeekFrom};
        let Ok(mut f) = std::fs::File::open(&self.path) else { return vec![] };
        let Ok(size) = f.metadata().map(|m| m.len()) else { return vec![] };
        let Some(mut offset) = self.offset else {
            self.offset = Some(size);
            return vec![];
        };
        // a new log (the game restarted): from its start
        if size < offset {
            offset = 0;
            self.rest.clear();
        }
        if size == offset {
            return vec![];
        }
        // at most 64 MB a poll (the log grows a few MB an hour)
        let take = (size - offset).min(64 << 20);
        let mut buf = Vec::with_capacity(take as usize);
        if f.seek(SeekFrom::Start(offset)).is_err() || f.take(take).read_to_end(&mut buf).is_err() {
            return vec![];
        }
        self.offset = Some(offset + buf.len() as u64);
        let mut all = std::mem::take(&mut self.rest);
        all.extend_from_slice(&buf);
        let mut parts: Vec<&[u8]> = all.split(|&b| b == b'\n').collect();
        self.rest = parts.pop().unwrap_or_default().to_vec();
        parts.iter().map(|l| String::from_utf8_lossy(l.strip_suffix(b"\r").unwrap_or(l)).into_owned()).collect()
    }
}

/// The log's lines kept: the bag (what it holds at login, every change and what caused it), which pages open and
/// close, entering areas and levels, buttons pressed and drop tables rolled.
pub fn keep(line: &str) -> bool {
    const KEEP: [&str; 10] = [
        "BagMgr@:",
        "ItemChange@",
        "PageRunState",
        "_JoinFight",
        "LevelMgr@ LevelUid",
        "[LEVEL_ENTER]",
        "UIButton@",
        "dropId:",
        "AudioEnum  == S2_System",
        "OpenFlow0!",
    ];
    KEEP.iter().any(|k| line.contains(k))
}

/// Whether the line is part of what the bag held when the game logged in (kept from the whole log when the
/// recorder starts, so every later change can be counted from it).
pub fn is_bag_state(line: &str) -> bool {
    line.contains("BagMgr@:") || line.contains("ItemChange@")
}

/// Blacksail's pages, by the names the log gives them: the seal pick (VoidBeacon) and the port (VoidPort).
/// Any page whose name starts with "Void" counts, so one not seen yet is pictured too.
pub fn is_void_page(name: &str) -> bool {
    name.starts_with("Void")
}

/// The seal page's 切換 button: three cards ↔ the base seal alone. What was read before it is of the other cards.
pub fn is_switch(line: &str) -> bool {
    line.contains("UIButton@") && line.contains("BtnShowGVAC")
}

/// The page where a seal is picked (3 cards, or the base seal alone).
pub const SEAL_PAGE: &str = "VoidPort";

/// Which pages are open, from the log's "PageName = X , PageRunState = Run / Hide / Destory" lines.
#[derive(Default)]
pub struct Pages {
    open: Vec<String>,
}

impl Pages {
    /// Takes one line; the page it opened or closed, if any.
    pub fn feed(&mut self, line: &str) -> Option<(String, bool)> {
        let rest = line.split("PageName = ").nth(1)?;
        let (name, state) = rest.split_once(" , PageRunState = ")?;
        let (name, state) = (name.trim().to_string(), state.trim());
        let run = state.starts_with("Run");
        self.open.retain(|p| *p != name);
        if run {
            self.open.push(name.clone());
        }
        Some((name, run))
    }

    /// The Blacksail page open now, if any.
    pub fn void_page(&self) -> Option<&str> {
        self.open.iter().rev().find(|p| is_void_page(p)).map(|s| s.as_str())
    }
}

/// A file name's safe form of a page name.
pub fn file_safe(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUN: &str = "[2026.11.08-08.52.21:729][986]TLLua: Display: [Game] TipMsgShowMgr@DispatchPageRunChange PageName = VoidBeacon , PageRunState = Run ";
    const GONE: &str = "[2026.11.08-08.52.24:638][728]TLLua: Display: [Game] TipMsgShowMgr@DispatchPageRunChange PageName = VoidBeacon , PageRunState = Destory ";
    const FIGHT: &str = "[2026.11.08-08.52.24:639][728]TLLua: Display: [Game] TipMsgShowMgr@DispatchPageRunChange PageName = Fight , PageRunState = Run ";
    const HIDE: &str = "[2026.11.08-08.52.24:639][728]TLLua: Display: [Game] TipMsgShowMgr@DispatchPageRunChange PageName = VoidPort , PageRunState = Hide ";
    const PORT: &str = "[2026.11.08-08.52.30:880][838]TLLua: Display: [Game] TipMsgShowMgr@DispatchPageRunChange PageName = VoidPort , PageRunState = Run ";

    #[test]
    fn void_pages_open_and_close() {
        let mut p = Pages::default();
        assert_eq!(p.void_page(), None);
        assert_eq!(p.feed(RUN), Some(("VoidBeacon".into(), true)));
        assert_eq!(p.void_page(), Some("VoidBeacon"));
        p.feed(FIGHT);
        assert_eq!(p.void_page(), Some("VoidBeacon"));
        p.feed(GONE);
        assert_eq!(p.void_page(), None);
        p.feed(PORT);
        assert_eq!(p.void_page(), Some("VoidPort"));
        p.feed(HIDE);
        assert_eq!(p.void_page(), None);
        assert_eq!(p.feed("[x] something else"), None);
    }

    #[test]
    fn picks_end() {
        assert!(ends_pick("[2026.11.08-09.29.52:737][892]TLGame: Display: [Game] UIButton@ normal-click Rarity2SelectedBtn"));
        assert!(ends_pick("[2026.11.08-09.30.20:551][558]TLGame: Display: [Game] UIButton@ normal-click Rarity4SelectedBtn"));
        assert!(ends_pick("[2026.11.08-09.29.44:606][757]TLGame: Display: [Game] UIButton@ normal-click BtnConfirm"));
        assert!(!ends_pick("[2026.11.08-09.29.52:019][768]TLGame: Display: [Game] UIButton@ normal-click BtnSelect"));
        assert!(!ends_pick("[2026.11.08-09.29.52:737][892]TLLua: Display: [Game] AudioMgr@PlayAudio  AudioEnum  == S2_System_Card_Confirm"));
    }

    #[test]
    fn the_switch() {
        assert!(is_switch("[2026.11.08-10.31.51:034][1]TLGame: Display: [Game] UIButton@ normal-click BtnShowGVAC"));
        assert!(!is_switch("[2026.11.08-10.31.51:514][1]TLGame: Display: [Game] UIButton@ normal-click BtnSelect"));
    }

    #[test]
    fn lines_kept() {
        assert!(keep("[t]TLLua: Display: [Game] BagMgr@:Modfy BagItem PageId = 102 SlotId = 39 ConfigBaseId = 5140 Num = 943"));
        assert!(keep("[t]TLLua: Display: [Game] BagMgr@:InitBagData PageId = 100 SlotId = 1 ConfigBaseId = 1919 Num = 1"));
        assert!(keep("[t]TLLua: Display: [Game] ItemChange@ ProtoName=PickItems start"));
        assert!(keep("[t]TLGame: Display: [Game] Trigger Trace: Trigger node(31) execute success, dropId:10012"));
        assert!(keep("[t]TLGame: Display: [Game] UIButton@ normal-click Rarity2SelectedBtn"));
        assert!(keep(RUN));
        assert!(!keep("[t]TLLua: Display: [Game] TCP Ping Result: 79"));
        assert!(!keep("[t]TLGame: Display: [Game] UDropInstanceMgr@ AddInstance! InstanceId = 100 InstanceIndex = 0"));
        assert!(is_bag_state("[t]TLLua: Display: [Game] ItemChange@ Update Id=5140_x BagNum=943 in PageId=102 SlotId=39"));
        assert!(!is_bag_state(RUN));
    }

    #[test]
    fn follower_reads_new_lines_only() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("blacksail-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("UE_game.log");
        std::fs::write(&path, "old 1\nold 2\n").unwrap();
        let mut f = Follower::new(path.clone());
        assert!(f.poll().is_empty());
        let mut file = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"new 1\r\nnew 2 half").unwrap();
        assert_eq!(f.poll(), vec!["new 1".to_string()]);
        file.write_all(b" done\n").unwrap();
        assert_eq!(f.poll(), vec!["new 2 half done".to_string()]);
        // the game restarted: a new, shorter log from its start
        std::fs::write(&path, "fresh\n").unwrap();
        assert_eq!(f.poll(), vec!["fresh".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pictures() {
        let mut a = Img::new(200, 100);
        assert!(a.black());
        for p in a.data.chunks_exact_mut(4) {
            p.copy_from_slice(&[40, 40, 60, 255]);
        }
        assert!(!a.black());
        let ta = thumb(&a);
        assert!(thumb_diff(&ta, &ta) < 0.01);
        let mut b = Img::new(200, 100);
        b.data.copy_from_slice(&a.data);
        for y in 30..70 {
            for x in 50..150 {
                b.data[(y * 200 + x) * 4..(y * 200 + x) * 4 + 3].copy_from_slice(&[255, 200, 255]);
            }
        }
        assert!(thumb_diff(&ta, &thumb(&b)) > CHANGED);
        let c = b.crop(50, 30, 10, 10);
        assert_eq!((c.width, c.height), (10, 10));
        assert_eq!(&c.data[..4], &[255, 200, 255, 255]);
        let path = std::env::temp_dir().join(format!("blacksail-test-{}.png", std::process::id()));
        b.save_png(&path).unwrap();
        assert!(std::fs::metadata(&path).unwrap().len() > 0);
        let _ = std::fs::remove_file(&path);
        assert_eq!(file_safe("Void Port/1"), "Void_Port_1");
    }
}
