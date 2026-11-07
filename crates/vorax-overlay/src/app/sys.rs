//! Windows itself: the game's window (by its title), where its picture is on screen, whether it's in front, and
//! where the game writes UE_game.log. Plus the overlay's own debug log and settings.

use std::path::{Path, PathBuf};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::Registry::{RegGetValueW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetClientRect, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible};

/// The game's window title.
const GAME_TITLE: &str = "Torchlight: Infinite";
/// Torchlight: Infinite on Steam.
const STEAM_APP: &str = "1974050";

/// A window handle that can cross threads (it's only a number to Windows).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hwnd(pub isize);

impl Hwnd {
    pub fn get(self) -> HWND {
        HWND(self.0 as *mut core::ffi::c_void)
    }
    pub fn of(h: HWND) -> Hwnd {
        Hwnd(h.0 as isize)
    }
}

unsafe extern "system" fn each_window(h: HWND, found: LPARAM) -> BOOL {
    let found = &mut *(found.0 as *mut Option<Hwnd>);
    if !IsWindowVisible(h).as_bool() {
        return true.into();
    }
    let mut buf = [0u16; 256];
    let n = GetWindowTextW(h, &mut buf);
    if n <= 0 {
        return true.into();
    }
    let title = String::from_utf16_lossy(&buf[..n as usize]);
    if title.trim_end() != GAME_TITLE {
        return true.into();
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(h, Some(&mut pid));
    if pid == std::process::id() {
        return true.into();
    }
    *found = Some(Hwnd::of(h));
    false.into()
}

/// The game's shown top-level window, by its title.
pub fn find_game_window() -> Option<Hwnd> {
    let mut found: Option<Hwnd> = None;
    unsafe {
        let _ = EnumWindows(Some(each_window), LPARAM(&mut found as *mut _ as isize));
    }
    found
}

/// The game's picture on screen (its client area): x, y, width, height in physical pixels.
pub fn client_rect(h: Hwnd) -> Option<(i32, i32, i32, i32)> {
    unsafe {
        if IsIconic(h.get()).as_bool() {
            return None;
        }
        let mut r = RECT::default();
        GetClientRect(h.get(), &mut r).ok()?;
        let mut p = POINT { x: 0, y: 0 };
        if !ClientToScreen(h.get(), &mut p).as_bool() {
            return None;
        }
        let (w, hh) = (r.right - r.left, r.bottom - r.top);
        (w > 0 && hh > 0).then_some((p.x, p.y, w, hh))
    }
}

pub fn foreground() -> Hwnd {
    Hwnd::of(unsafe { GetForegroundWindow() })
}

fn reg_string(root: HKEY, key: PCWSTR, value: PCWSTR) -> Option<String> {
    let mut buf = [0u16; 1024];
    let mut len = (buf.len() * 2) as u32;
    let err = unsafe { RegGetValueW(root, key, value, RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr() as *mut _), Some(&mut len)) };
    if err.is_err() {
        return None;
    }
    let n = (len as usize / 2).saturating_sub(1);
    Some(String::from_utf16_lossy(&buf[..n]))
}

fn quoted_values<'a>(text: &'a str, key: &str) -> Vec<&'a str> {
    let pat = format!("\"{key}\"");
    text.lines()
        .filter_map(|l| {
            let l = l.trim();
            let rest = l.strip_prefix(&pat)?.trim();
            rest.strip_prefix('"')?.strip_suffix('"')
        })
        .collect()
}

const LOG_IN_GAME: [&str; 5] = ["UE_game", "TorchLight", "Saved", "Logs", "UE_game.log"];

/// Not in upstream: the program behind the game's window, where it's installed (any launcher, any drive), when
/// the game is open: "X:\...\Game\UE_game\Binaries\Win64\<exe>".
fn game_exe() -> Option<PathBuf> {
    let h = find_game_window()?;
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(h.get(), Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let p = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut n = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(p, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut n).is_ok();
        let _ = CloseHandle(p);
        ok.then(|| PathBuf::from(String::from_utf16_lossy(&buf[..n as usize])))
    }
}

/// Not in upstream: where a game installed under `exe`'s folders writes its log: from each folder up, the log's
/// path from "UE_game" on (the "Game" folder) or from under it (the "UE_game" folder itself).
fn logs_above(exe: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in exe.ancestors().skip(1) {
        out.push(LOG_IN_GAME.iter().fold(dir.to_path_buf(), |p, s| p.join(s)));
        out.push(LOG_IN_GAME[1..].iter().fold(dir.to_path_buf(), |p, s| p.join(s)));
    }
    out
}

/// Where the game writes UE_game.log: under each Steam library that has the game, or the launcher's folders.
/// The newest log wins; without one yet, where it will appear.
pub fn find_log() -> Option<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    for (root, key, value) in [
        (HKEY_CURRENT_USER, w!("Software\\Valve\\Steam"), w!("SteamPath")),
        (HKEY_LOCAL_MACHINE, w!("SOFTWARE\\WOW6432Node\\Valve\\Steam"), w!("InstallPath")),
        (HKEY_LOCAL_MACHINE, w!("SOFTWARE\\Valve\\Steam"), w!("InstallPath")),
    ] {
        if let Some(p) = reg_string(root, key, value) {
            roots.push(PathBuf::from(p.replace('/', "\\")));
        }
    }
    roots.push(PathBuf::from("C:\\Program Files (x86)\\Steam"));
    let mut libraries: Vec<PathBuf> = roots.clone();
    for r in &roots {
        if let Ok(vdf) = std::fs::read_to_string(r.join("steamapps").join("libraryfolders.vdf")) {
            for p in quoted_values(&vdf, "path") {
                libraries.push(PathBuf::from(p.replace("\\\\", "\\")));
            }
        }
    }
    let same = |a: &Path, b: &Path| a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase();
    let mut unique: Vec<PathBuf> = Vec::new();
    for l in libraries {
        if !unique.iter().any(|u| same(u, &l)) {
            unique.push(l);
        }
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    for lib in &unique {
        let steamapps = lib.join("steamapps");
        if let Ok(acf) = std::fs::read_to_string(steamapps.join(format!("appmanifest_{STEAM_APP}.acf"))) {
            if let Some(dir) = quoted_values(&acf, "installdir").first() {
                candidates.push(LOG_IN_GAME.iter().fold(steamapps.join("common").join(dir), |p, s| p.join(s)));
            }
        }
    }
    for base in ["C:\\Program Files (x86)\\Torchlight Infinite", "C:\\Program Files\\Torchlight Infinite"] {
        candidates.push(LOG_IN_GAME.iter().fold(PathBuf::from(base).join("Game"), |p, s| p.join(s)));
    }
    // not in upstream: the open game's own folders (the launcher's install on any drive), then the usual folders
    // on every drive, so the start files need no path
    if let Some(exe) = game_exe() {
        candidates.extend(logs_above(&exe));
    }
    for d in 'C'..='Z' {
        let root = PathBuf::from(format!("{d}:\\"));
        if !root.exists() {
            continue;
        }
        for sub in ["Torchlight Infinite", "Games\\Torchlight Infinite", "Program Files\\Torchlight Infinite", "Program Files (x86)\\Torchlight Infinite", "SteamLibrary\\steamapps\\common\\Torchlight Infinite", "Steam\\steamapps\\common\\Torchlight Infinite", "XDGames\\Torchlight Infinite"] {
            candidates.push(LOG_IN_GAME.iter().fold(root.join(sub).join("Game"), |p, s| p.join(s)));
        }
    }
    // a game under Program Files that can't write there has its log in Windows' VirtualStore copy
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        for c in candidates.clone() {
            let s = c.to_string_lossy().to_string();
            if s.to_lowercase().starts_with("c:\\program files") {
                candidates.push(PathBuf::from(&local).join("VirtualStore").join(&s[3..]));
            }
        }
    }
    let mut there: Vec<(PathBuf, std::time::SystemTime)> = candidates.iter().filter_map(|p| std::fs::metadata(p).ok().filter(|m| m.is_file()).map(|m| (p.clone(), m.modified().unwrap_or(std::time::UNIX_EPOCH)))).collect();
    there.sort_by(|a, b| b.1.cmp(&a.1));
    if let Some((p, _)) = there.into_iter().next() {
        return Some(p);
    }
    candidates.into_iter().find(|p| p.ancestors().nth(3).is_some_and(|d| d.exists()))
}

/// %APPDATA%\VoraxRs: settings and the debug log.
/// The calling thread below normal priority (not in upstream): the game comes first when both want the CPU.
pub fn below_normal() {
    use windows::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL};
    unsafe {
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
    }
}

pub fn data_dir() -> PathBuf {
    let base = std::env::var("APPDATA").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let d = base.join("VoraxRs");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// The debug logs before the current one, newest first (not in upstream, which kept one: the runs and statistics
/// come from the logs, and the older half of them went at every 2 MB, after about a hundred runs).
pub const OLD_LOGS: [&str; 4] = ["overlay.log.old", "overlay.log.old2", "overlay.log.old3", "overlay.log.old4"];

/// Every debug log kept, oldest first, as one text (the runs and their statistics).
pub fn log_history() -> String {
    let dir = data_dir();
    let mut text = String::new();
    for name in OLD_LOGS.iter().rev().chain(std::iter::once(&"overlay.log")) {
        text.push_str(&std::fs::read_to_string(dir.join(name)).unwrap_or_default());
    }
    text
}

/// A line in the debug log (%APPDATA%\VoraxRs\overlay.log), kept under 2 MB; the ones before it in OLD_LOGS.
pub fn note(msg: &str) {
    use std::io::Write;
    let path = data_dir().join("overlay.log");
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > 2_000_000) {
        let dir = data_dir();
        for i in (1..OLD_LOGS.len()).rev() {
            let _ = std::fs::rename(dir.join(OLD_LOGS[i - 1]), dir.join(OLD_LOGS[i]));
        }
        let _ = std::fs::rename(&path, dir.join(OLD_LOGS[0]));
    }
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
    let line = format!("{secs:.3} {msg}\n");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = f.write_all(line.as_bytes());
    }
    if cfg!(debug_assertions) || std::env::var_os("VORAX_CONSOLE").is_some() {
        eprint!("{line}");
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    /// Intern Shelley level: sets the instrument rerolls a game starts with
    pub shelley: i32,
    /// the overlay shown (Ctrl+Alt+V)
    pub shown: bool,
    /// read card titles with Windows' text reader (else by their pictures)
    pub ocr: bool,
    /// the marks' colour: an index into draw::ACCENTS
    #[serde(default)]
    pub accent: usize,
    /// not in upstream: the words in Traditional or Simplified (zh::AUTO, TRADITIONAL, SIMPLIFIED)
    #[serde(default)]
    pub script: u8,
    /// not in upstream: the aggressive mode (衝分): futures judged by their plain final score (vorax::bot::MEAN_GOAL)
    #[serde(default)]
    pub aggressive: bool,
    /// the toolbox: the game's log chosen by hand (else found by itself)
    #[serde(default)]
    pub log_path: Option<PathBuf>,
    /// the toolbox: the tools started when the game opens and stopped when it closes
    #[serde(default = "yes")]
    pub auto_game: bool,
    /// the toolbox: 渴癮症助手 started with the game and stopped when it closes (unset: as auto_game, from 1.0.0)
    #[serde(default)]
    pub auto_vorax: Option<bool>,
    /// the toolbox: 囚獸助手 started with the game and stopped when it closes (unset: as auto_game, from 1.0.0)
    #[serde(default)]
    pub auto_tartarus: Option<bool>,
    /// the toolbox: 易火 (ETor) opened as the game opens (never closed by the toolbox)
    #[serde(default)]
    pub auto_etor: bool,
    /// the toolbox: 易火's program, chosen or found as it runs
    #[serde(default)]
    pub etor_exe: Option<PathBuf>,
    /// the toolbox: the runs counted in the statistics start here (log time, seconds; 0: all)
    #[serde(default)]
    pub stats_since: f64,
    /// the toolbox: the first-use steps have been shown
    #[serde(default)]
    pub first_run_done: bool,
    /// the toolbox: the "still running in the tray" note has been shown
    #[serde(default)]
    pub close_hint_shown: bool,
    /// the toolbox: 囚獸助手's program chosen by hand (else the toolbox's own install)
    #[serde(default)]
    pub tartarus_exe: Option<PathBuf>,
    /// SmartTorch's interface language: "tc", "sc" or "en" (empty: from Windows' language at the first start)
    #[serde(default)]
    pub lang: String,
    /// the picker (a card not read: the player picks it); None: on
    #[serde(default)]
    pub picker: Option<bool>,
    /// the toolbox: 囚獸助手's statistics (runs, Chromatic) when they were reset here: shown from then on (its own
    /// file is never changed)
    #[serde(default)]
    pub tartarus_base: Option<(u64, u64)>,
}

fn yes() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Settings { shelley: 6, shown: true, ocr: true, accent: 0, script: 0, aggressive: false, log_path: None, auto_game: true, auto_vorax: None, auto_tartarus: None, auto_etor: false, etor_exe: None, stats_since: 0.0, first_run_done: false, close_hint_shown: false, tartarus_exe: None, tartarus_base: None, lang: String::new(), picker: None }
    }
}

impl Settings {
    /// 渴癮症助手 started and stopped with the game
    pub fn auto_vorax(&self) -> bool {
        self.auto_vorax.unwrap_or(self.auto_game)
    }
    /// 囚獸助手 started and stopped with the game
    pub fn auto_tartarus(&self) -> bool {
        self.auto_tartarus.unwrap_or(self.auto_game)
    }
    pub fn load() -> Settings {
        std::fs::read_to_string(data_dir().join("settings.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }
    pub fn save(&self) {
        let _ = std::fs::write(data_dir().join("settings.json"), serde_json::to_string_pretty(self).unwrap());
    }
}

/// The toolbox's entry in Windows' start-up list (HKCU\...\Run), started into the tray.
const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const RUN_NAME: PCWSTR = w!("FireToolbox");

/// Whether the toolbox starts with Windows.
pub fn starts_with_windows() -> bool {
    reg_string(HKEY_CURRENT_USER, RUN_KEY, RUN_NAME).is_some()
}

/// Starts the toolbox with Windows (into the tray), or not.
pub fn set_start_with_windows(on: bool) {
    use windows::Win32::System::Registry::{RegCloseKey, RegCreateKeyW, RegDeleteValueW, RegSetValueExW, REG_SZ};
    unsafe {
        let mut key = HKEY::default();
        if RegCreateKeyW(HKEY_CURRENT_USER, RUN_KEY, &mut key).is_err() {
            return;
        }
        if on {
            let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default();
            let cmd: Vec<u16> = format!("\"{exe}\" --tray").encode_utf16().chain(std::iter::once(0)).collect();
            let bytes = std::slice::from_raw_parts(cmd.as_ptr() as *const u8, cmd.len() * 2);
            let _ = RegSetValueExW(key, RUN_NAME, 0, REG_SZ, Some(bytes));
        } else {
            let _ = RegDeleteValueW(key, RUN_NAME);
        }
        let _ = RegCloseKey(key);
    }
}

/// Windows' file dialog for the game's log (UE_game.log); None when cancelled.
pub fn choose_log(owner: Option<Hwnd>) -> Option<PathBuf> {
    let t = crate::zh::tr("選擇遊戲日誌 UE_game.log", "Choose the game's log, UE_game.log");
    let a = crate::zh::tr("遊戲日誌 (UE_game.log)", "Game log (UE_game.log)");
    let b = crate::zh::tr("記錄檔 (*.log)", "Log files (*.log)");
    choose_file(owner, &t, &[(&a, "UE_game.log"), (&b, "*.log")])
}

/// Windows' file dialog for 囚獸助手's program (TartarusSolver.exe); None when cancelled.
pub fn choose_tartarus(owner: Option<Hwnd>) -> Option<PathBuf> {
    let t = crate::zh::tr("選擇囚獸助手 TartarusSolver.exe", "Choose Tartarus Solver, TartarusSolver.exe");
    let a = crate::zh::tr("囚獸助手 (TartarusSolver.exe)", "Tartarus Solver (TartarusSolver.exe)");
    choose_file(owner, &t, &[(&a, "TartarusSolver.exe")])
}

/// Windows' file dialog for 易火's program (易火-ETor.exe); None when cancelled.
pub fn choose_etor(owner: Option<Hwnd>) -> Option<PathBuf> {
    let t = crate::zh::tr("選擇 ETor", "Choose ETor");
    choose_file(owner, &t, &[("ETor (*.exe)", "*.exe")])
}

/// Windows' file dialog: a file to open, of these kinds (name, pattern); None when cancelled.
fn choose_file(owner: Option<Hwnd>, title: &str, kinds: &[(&str, &str)]) -> Option<PathBuf> {
    let wide = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };
    let title = wide(title);
    let kinds: Vec<(Vec<u16>, Vec<u16>)> = kinds.iter().map(|(n, p)| (wide(n), wide(p))).collect();
    use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_INPROC_SERVER};
    use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
    use windows::Win32::UI::Shell::{FileOpenDialog, IFileOpenDialog, SIGDN_FILESYSPATH};
    unsafe {
        let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        let filters: Vec<COMDLG_FILTERSPEC> = kinds.iter().map(|(n, p)| COMDLG_FILTERSPEC { pszName: PCWSTR(n.as_ptr()), pszSpec: PCWSTR(p.as_ptr()) }).collect();
        let _ = dialog.SetFileTypes(&filters);
        let _ = dialog.SetTitle(PCWSTR(title.as_ptr()));
        dialog.Show(owner.map(|h| h.get()).unwrap_or_default()).ok()?;
        let item = dialog.GetResult().ok()?;
        let name = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let path = String::from_utf16_lossy(name.as_wide());
        CoTaskMemFree(Some(name.0 as *const _));
        Some(PathBuf::from(path))
    }
}

/// A local date and time for a file name ("20261105-1430"), from seconds since 1970.
pub fn local_stamp(secs: u64) -> String {
    use windows::Win32::Foundation::{FILETIME, SYSTEMTIME};
    use windows::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};
    let ft = secs * 10_000_000 + 116_444_736_000_000_000;
    let ft = FILETIME { dwLowDateTime: ft as u32, dwHighDateTime: (ft >> 32) as u32 };
    let (mut utc, mut l) = (SYSTEMTIME::default(), SYSTEMTIME::default());
    unsafe {
        if FileTimeToSystemTime(&ft, &mut utc).is_err() || SystemTimeToTzSpecificLocalTime(None, &utc, &mut l).is_err() {
            return secs.to_string();
        }
    }
    format!("{}{:02}{:02}-{:02}{:02}{:02}", l.wYear, l.wMonth, l.wDay, l.wHour, l.wMinute, l.wSecond)
}

/// The game in exclusive full screen (the overlay can't be seen over it): Windows says a Direct3D program has the
/// whole screen.
pub fn exclusive_full_screen() -> bool {
    use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_RUNNING_D3D_FULL_SCREEN};
    unsafe { SHQueryUserNotificationState().is_ok_and(|s| s == QUNS_RUNNING_D3D_FULL_SCREEN) }
}

/// Windows' own language is Simplified Chinese (the toolbox's words start in Simplified then).
pub fn windows_simplified() -> bool {
    use windows::Win32::Globalization::GetUserDefaultUILanguage;
    // zh-CN 0x0804, zh-SG 0x1004
    matches!(unsafe { GetUserDefaultUILanguage() }, 0x0804 | 0x1004)
}

/// SmartTorch's language from Windows' own: Simplified, Traditional (any other Chinese), else English.
pub fn windows_lang() -> &'static str {
    use windows::Win32::Globalization::GetUserDefaultUILanguage;
    let id = unsafe { GetUserDefaultUILanguage() };
    if windows_simplified() {
        "sc"
    } else if id & 0x3ff == 0x04 {
        "tc"
    } else {
        "en"
    }
}

#[cfg(test)]
mod log_tests {
    use super::logs_above;
    use std::path::Path;

    /// The open game's program, wherever it's installed, gives its log's place.
    #[test]
    fn the_games_program_gives_its_logs_place() {
        let logs = logs_above(Path::new("X:\\Games\\Torchlight Infinite\\Game\\UE_game\\Binaries\\Win64\\UE_game-Win64-Shipping.exe"));
        assert!(logs.iter().any(|p| p == Path::new("X:\\Games\\Torchlight Infinite\\Game\\UE_game\\TorchLight\\Saved\\Logs\\UE_game.log")));
    }
}

/// The toolbox's 流派碼 page: the codes the player saved (name and code, in their order), in
/// %APPDATA%\\VoraxRs\\codes.json. "Clear records" leaves them.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Code {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub code: String,
}

pub fn load_codes() -> Vec<Code> {
    std::fs::read_to_string(data_dir().join("codes.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

/// Written to a new file first, then moved over the old one, so a crash never leaves half a list.
pub fn save_codes(codes: &[Code]) {
    let dir = data_dir();
    let tmp = dir.join("codes.json.new");
    if std::fs::write(&tmp, serde_json::to_string_pretty(codes).unwrap()).is_ok() {
        let _ = std::fs::rename(&tmp, dir.join("codes.json"));
    }
}

