//! 黑帆 (Blacksail, from BlacksailRecorder 0.2.9): follows the game's log, reads the seal screen while it's open
//! (Windows' own text reader), and lists this run's seals in a small box over the game. Started and stopped from
//! the main window's 黑帆 page; two threads of its own while on (the reader, and the box's window). Only reads:
//! the seal screen's pictures and the log are neither kept nor sent anywhere.

mod ocr;
mod overlay;

use super::sys::{find_game_window, note};
use ::blacksail::cards::{cards, glowing, title_rarity, Board, Card};
use ::blacksail::picks::{describe, rows, Event, Pick, Run, PER_RUN};
use ::blacksail::{ends_pick, is_switch, Follower, Img, Pages, SEAL_PAGE};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// On (from start until stop).
static ON: AtomicBool = AtomicBool::new(false);
/// Bumped by each start: a reader left from an earlier start ends.
static GENERATION: AtomicUsize = AtomicUsize::new(0);
/// The seals listed now.
static PICKED: AtomicUsize = AtomicUsize::new(0);
/// Whether the game's log was found and followed.
static FOLLOWING: AtomicBool = AtomicBool::new(false);

/// For now (gathering pictures to tell the 虛海夜火 apart by): the page where they're put in, pictured once a
/// second after it opens, into FRAMES. To go once the beacons are read.
const BEACON_PAGE: &str = "VoidBeacon";
const BEACON_WAIT: Duration = Duration::from_secs(1);

/// %APPDATA%\VoraxRs\blacksail-frames: the beacon pages pictured.
pub fn frames_dir() -> PathBuf {
    super::sys::data_dir().join("blacksail-frames")
}

fn save_beacon_page(capturer: &super::capture::Capturer) {
    match grab(capturer) {
        Ok(img) => {
            let dir = frames_dir();
            let _ = std::fs::create_dir_all(&dir);
            let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
            let path = dir.join(format!("beacon-{}-{}x{}.png", super::sys::local_stamp(secs), img.width, img.height));
            match img.save_png(&path) {
                Ok(()) => note(&format!("Blacksail: beacon page pictured ({}x{})", img.width, img.height)),
                Err(e) => note(&format!("Blacksail: beacon picture not saved ({e})")),
            }
        }
        Err(e) => note(&format!("Blacksail: beacon page not pictured: {e}")),
    }
}

pub fn on() -> bool {
    ON.load(Ordering::SeqCst)
}

/// The seals listed now, and whether the log is followed (for the main window).
pub fn status() -> (usize, bool) {
    (PICKED.load(Ordering::SeqCst), FOLLOWING.load(Ordering::SeqCst))
}

/// Shows the box and starts reading; `log`: the game's log chosen by hand (else found by itself).
pub fn start(log: Option<PathBuf>) {
    if ON.swap(true, Ordering::SeqCst) {
        return;
    }
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    note("Blacksail: started");
    PICKED.store(0, Ordering::SeqCst);
    show(&[]);
    std::thread::spawn(overlay::run);
    std::thread::spawn(move || read(log, generation));
}

/// Closes the box and stops reading.
pub fn stop() {
    if !ON.swap(false, Ordering::SeqCst) {
        return;
    }
    GENERATION.fetch_add(1, Ordering::SeqCst);
    FOLLOWING.store(false, Ordering::SeqCst);
    overlay::close();
    note("Blacksail: stopped");
}

fn show(picks: &[Pick]) {
    PICKED.store(picks.len(), Ordering::SeqCst);
    overlay::show(rows(picks).into_iter().map(|(text, color)| overlay::Row { text, color }).collect());
}

/// The log so far: the pages open at its end, and the seals of this run picked before (their rarities).
/// Returns where it ended.
fn read_so_far(log: &Path, pages: &mut Pages, run: &mut Run, before: &mut Vec<u8>) -> u64 {
    use std::io::BufRead;
    let Ok(f) = std::fs::File::open(log) else { return 0 };
    let mut reader = std::io::BufReader::new(f);
    let (mut buf, mut read) = (Vec::new(), 0u64);
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                // a last line still being written is read again by the follower
                if buf.last() != Some(&b'\n') {
                    break;
                }
                read += n as u64;
                let line = String::from_utf8_lossy(&buf);
                let line = line.trim_end_matches(['\r', '\n']);
                pages.feed(line);
                match run.feed(line) {
                    Some(Event::Picked(r)) => before.push(r),
                    Some(Event::LeftPort) => before.clear(),
                    None => {}
                }
            }
        }
    }
    read
}

/// A reading of the seal screen in a picture: its cards, their titles' rarities, the one lit (chosen).
struct Seen {
    cards: Vec<Card>,
    rarities: Vec<Option<u8>>,
    lit: Option<usize>,
}

fn look(ocr: Option<&ocr::Ocr>, img: &Img) -> Option<Seen> {
    let lines = match ocr?.lines(img) {
        Ok(l) => l,
        Err(e) => {
            note(&format!("Blacksail: text not read ({e})"));
            return None;
        }
    };
    let cards = cards(&lines, img.width as f32, img.height as f32);
    if cards.is_empty() {
        return None;
    }
    let lit = glowing(img, &cards);
    let rarities = cards.iter().map(|c| title_rarity(img, c)).collect();
    Some(Seen { cards, rarities, lit })
}

/// The game's window now, as 黑帆's reader takes pictures.
fn grab(capturer: &super::capture::Capturer) -> Result<Img, String> {
    let h = find_game_window().ok_or("no game window")?;
    match capturer.grab(h) {
        Ok(Some(i)) => Ok(Img { width: i.width, height: i.height, data: i.data }),
        Ok(None) => Err("no picture (the game minimised?)".into()),
        Err(e) => Err(e.to_string()),
    }
}

fn read(chosen_log: Option<PathBuf>, generation: usize) {
    unsafe {
        let _ = windows::Win32::System::WinRT::RoInitialize(windows::Win32::System::WinRT::RO_INIT_MULTITHREADED);
    }
    let live = || GENERATION.load(Ordering::SeqCst) == generation;
    // the log: chosen, else where the game is installed (waiting for the game)
    let log = loop {
        if !live() {
            return;
        }
        if let Some(p) = chosen_log.clone().filter(|p| p.is_file()).or_else(|| super::sys::find_log().filter(|p| p.is_file())) {
            break p;
        }
        std::thread::sleep(Duration::from_secs(3));
    };
    FOLLOWING.store(true, Ordering::SeqCst);
    let mut pages = Pages::default();
    let mut run = Run::default();
    let mut before: Vec<u8> = Vec::new();
    let end = read_so_far(&log, &mut pages, &mut run, &mut before);
    let mut follower = Follower::from(log, end);

    let capturer = super::capture::Capturer::new();
    let ocr = match ocr::Ocr::new() {
        Ok(o) => {
            note(&format!("Blacksail: reading {}", o.language));
            Some(o)
        }
        Err(e) => {
            note(&format!("Blacksail: Windows can't read Chinese text here ({e}): seals not read"));
            None
        }
    };
    let mut picks: Vec<Pick> = before.iter().map(|r| Pick { rarity: Some(*r), category: None, seal: None, before: true }).collect();
    if !picks.is_empty() {
        note(&format!("Blacksail: {} seals picked this run before it started", picks.len()));
        show(&picks);
    }
    let mut page: Option<String> = pages.void_page().filter(|p| *p == SEAL_PAGE).map(str::to_string);
    let mut last_grab = Instant::now() - Duration::from_secs(1);
    // the seal confirmed on the page open now: nothing more to read on it
    let mut picked = false;
    // the seal screen over its readings; whether the last one had a card lit
    let mut board = Board::default();
    let mut width = 2560.0f32;
    // the last capture problem noted (once until it changes)
    let mut told = String::new();
    // the beacon page: open at the last look, and when it opened (until it's pictured)
    let (mut beacon, mut beacon_at): (bool, Option<Instant>) = (false, None);
    while live() {
        let lines = follower.poll();
        let mut confirmed: Option<u8> = None;
        for line in &lines {
            pages.feed(line);
            match run.feed(line) {
                Some(Event::Picked(r)) => confirmed = confirmed.or(Some(r)),
                Some(Event::LeftPort) => {
                    picks.clear();
                    show(&picks);
                    note("Blacksail: left the port, the list cleared");
                }
                None => {}
            }
            if page.as_deref() == Some(SEAL_PAGE) && ends_pick(line) {
                picked = true;
            }
            // 切換: the other cards from now on (the base seal alone, or the three again)
            if page.as_deref() == Some(SEAL_PAGE) && is_switch(line) {
                board = Board::default();
                // the screen before it is redrawn: the next picture a moment later
                last_grab = Instant::now();
            }
        }
        if let Some(rarity) = confirmed {
            // confirmed before a lit card was read: the screen now (its card may still be lit)
            if board.lit.is_none() {
                if let Ok(img) = grab(&capturer) {
                    if let Some(s) = look(ocr.as_ref(), &img) {
                        width = img.width as f32;
                        board.add(&s.cards, &s.rarities, s.lit, width);
                    }
                }
            }
            if picks.len() >= PER_RUN {
                picks.clear();
            }
            let pick = match board.picked(rarity, width) {
                Some(k) => {
                    let seal = k.seal(Some(rarity));
                    Pick { rarity: Some(rarity), category: k.category.or(seal.and_then(::blacksail::seals::only_category)), seal, before: false }
                }
                None => Pick { rarity: Some(rarity), category: None, seal: None, before: false },
            };
            note(&format!("Blacksail: seal {}", describe(&pick)));
            picks.push(pick);
            show(&picks);
        }
        let beacon_now = pages.void_page() == Some(BEACON_PAGE);
        if beacon_now && !beacon {
            beacon_at = Some(Instant::now());
        }
        if !beacon_now {
            beacon_at = None;
        }
        beacon = beacon_now;
        if beacon_at.is_some_and(|t| t.elapsed() >= BEACON_WAIT) {
            beacon_at = None;
            save_beacon_page(&capturer);
        }
        let current = pages.void_page().filter(|p| *p == SEAL_PAGE).map(str::to_string);
        if current != page {
            page = current;
            picked = false;
            board = Board::default();
        }
        if page.is_some() && !picked {
            // every 0.15 s while the cards are up, each picture read: a card clicked and confirmed quickly is
            // still seen lit
            if last_grab.elapsed() >= Duration::from_millis(150) {
                last_grab = Instant::now();
                match grab(&capturer) {
                    Ok(img) => {
                        if let Some(s) = look(ocr.as_ref(), &img) {
                            // the lit card read last stays the choice: the confirm's own animation can put it out
                            // before its line reaches the log
                            width = img.width as f32;
                            board.add(&s.cards, &s.rarities, s.lit, width);
                        }
                    }
                    Err(e) => {
                        if e != told {
                            note(&format!("Blacksail: no picture: {e}"));
                            told = e;
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        } else {
            std::thread::sleep(Duration::from_millis(150));
        }
    }
}
