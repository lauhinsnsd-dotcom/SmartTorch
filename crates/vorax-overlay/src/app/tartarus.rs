//! The toolbox's 囚獸助手: glcoder's Tartarus Solver (MIT, github.com/glcoder/TartarusSolver), used as it is. The
//! player downloads it from its releases page; the toolbox finds it (chosen, or as it runs), starts it, keeps its
//! window hidden (it has no tray icon of its own, and its window's ✕ quits it) until 設定 shows it, starts it again
//! if it closes while it should run, and reads its statistics. Its own code and files are never changed: it
//! updates itself (its window's Update button).

use super::sys::{note, Hwnd};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM, WPARAM};
use windows::Win32::System::Threading::{GetExitCodeProcess, OpenProcess, QueryFullProcessImageNameW, TerminateProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE};
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, SetForegroundWindow, ShowWindow, SW_HIDE, SW_RESTORE, SW_SHOW, WM_CLOSE};

const REPO: &str = "glcoder/TartarusSolver";
const EXE: &str = "TartarusSolver.exe";
/// Its control window's title ("Tartarus Solver 0.7.44").
const TITLE: &str = "Tartarus Solver";
/// How long it may take from its start to its window showing (it loads its models first).
const STARTING: Duration = Duration::from_secs(30);
/// Closed while it should run: started again after this long (its own update restarts it within it).
const GONE: Duration = Duration::from_secs(8);

/// Its own settings and statistics: %LOCALAPPDATA%\TartarusSolver.
fn its_folder() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    base.join("TartarusSolver")
}

/// The installed program: the one chosen (or found running).
pub fn installed(chosen: Option<&Path>) -> Option<PathBuf> {
    chosen.filter(|p| p.is_file()).map(Path::to_path_buf)
}

/// The program of the one running now, if any: how the toolbox finds it without being told.
pub fn running_exe() -> Option<PathBuf> {
    window().and_then(|(_, pid)| exe_of(pid)).map(PathBuf::from)
}

/// Its statistics (runs, Chromatic Inmates), from its statistic.json.
pub fn stats() -> Option<(u64, u64)> {
    let text = std::fs::read_to_string(its_folder().join("statistic.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    Some((v["Runs"].as_u64().unwrap_or(0), v["Chromatic"].as_u64().unwrap_or(0)))
}

/// The program's version, from its file version ("0.7.44").
pub fn version(exe: &Path) -> Option<String> {
    use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW, VS_FIXEDFILEINFO};
    let name: Vec<u16> = exe.as_os_str().to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let size = GetFileVersionInfoSizeW(PCWSTR(name.as_ptr()), None);
        if size == 0 {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        GetFileVersionInfoW(PCWSTR(name.as_ptr()), 0, size, buf.as_mut_ptr() as *mut _).ok()?;
        let mut info: *mut core::ffi::c_void = std::ptr::null_mut();
        let mut len = 0u32;
        let root: Vec<u16> = "\\".encode_utf16().chain(std::iter::once(0)).collect();
        if !VerQueryValueW(buf.as_ptr() as *const _, PCWSTR(root.as_ptr()), &mut info, &mut len).as_bool() || info.is_null() {
            return None;
        }
        let f = &*(info as *const VS_FIXEDFILEINFO);
        Some(format!("{}.{}.{}", f.dwFileVersionMS >> 16, f.dwFileVersionMS & 0xffff, f.dwFileVersionLS >> 16))
    }
}

fn numbers(v: &str) -> Vec<u32> {
    v.trim_start_matches('v').split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

/// Whether `a` is a newer version than `b`.
pub fn newer(a: &str, b: &str) -> bool {
    numbers(a) > numbers(b)
}


/// Its releases page, to download it from.
pub const PAGE: &str = "https://github.com/glcoder/TartarusSolver/releases/latest";

/// Its latest release's version ("0.7.44").
pub fn latest() -> Result<String, String> {
    let v = super::update::get_json(&format!("https://api.github.com/repos/{REPO}/releases/latest"))?;
    let tag = v["tag_name"].as_str().ok_or("GitHub: no release")?;
    Ok(tag.trim_start_matches('v').to_string())
}





fn exe_of(pid: u32) -> Option<String> {
    unsafe {
        let p = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut n = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(p, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut n).is_ok();
        let _ = CloseHandle(p);
        ok.then(|| String::from_utf16_lossy(&buf[..n as usize]))
    }
}

fn alive(pid: u32) -> bool {
    unsafe {
        let Ok(p) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return false };
        let mut code = 0u32;
        let ok = GetExitCodeProcess(p, &mut code).is_ok();
        let _ = CloseHandle(p);
        ok && code == 259
    }
}

unsafe extern "system" fn each(h: HWND, found: LPARAM) -> BOOL {
    let found = &mut *(found.0 as *mut Option<(Hwnd, u32)>);
    let mut buf = [0u16; 256];
    let n = GetWindowTextW(h, &mut buf);
    if n <= 0 || !String::from_utf16_lossy(&buf[..n as usize]).starts_with(TITLE) {
        return true.into();
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(h, Some(&mut pid));
    if exe_of(pid).is_some_and(|e| e.to_ascii_lowercase().ends_with(&EXE.to_ascii_lowercase())) {
        *found = Some((Hwnd::of(h), pid));
        return false.into();
    }
    true.into()
}

/// Its control window (shown or hidden), and its process.
fn window() -> Option<(Hwnd, u32)> {
    let mut found = None;
    unsafe {
        let _ = EnumWindows(Some(each), LPARAM(&mut found as *mut _ as isize));
    }
    found
}

/// The module as the toolbox runs it.
#[derive(Default)]
pub struct Tart {
    /// it should be running (Start, or the game opening)
    pub want: bool,
    /// its window shown (設定)
    pub shown: bool,
    pub running: bool,
    pid: Option<u32>,
    started: Option<Instant>,
    gone_since: Option<Instant>,
    stop_at: Option<Instant>,
    starts: Vec<Instant>,
    /// why it isn't running (it kept closing, or didn't start)
    pub error: Option<String>,
}

impl Tart {
    /// Starts it (hidden as its window appears).
    pub fn start(&mut self, exe: &Path) {
        self.want = true;
        self.error = None;
        self.stop_at = None;
        if self.running {
            return;
        }
        let now = Instant::now();
        self.starts.retain(|t| now.duration_since(*t) < Duration::from_secs(120));
        if self.starts.len() >= 3 {
            self.want = false;
            self.error = Some(crate::zh::tr("囚獸助手接連關閉了三次，已停止自動重新開啟。", "Tartarus Solver closed three times in a row: it won't be started again by itself."));
            note("Tartarus Solver closed three times in two minutes: not started again");
            return;
        }
        match std::process::Command::new(exe).current_dir(exe.parent().unwrap_or(Path::new("."))).spawn() {
            Ok(c) => {
                note(&format!("Tartarus Solver started ({})", exe.display()));
                self.pid = Some(c.id());
                self.started = Some(now);
                self.starts.push(now);
                self.gone_since = None;
            }
            Err(e) => {
                self.want = false;
                self.error = Some(format!("{}{e}", crate::zh::tr("無法啟動：", "Couldn't start it: ")));
                note(&format!("Tartarus Solver didn't start: {e}"));
            }
        }
    }

    /// Asks it to close (as its window's ✕ would); ended by force if it hasn't after a few seconds.
    pub fn stop(&mut self) {
        self.want = false;
        self.shown = false;
        if let Some((h, _)) = window() {
            unsafe {
                let _ = PostMessageW(h.get(), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
            self.stop_at = Some(Instant::now());
            note("Tartarus Solver asked to close");
        }
    }

    /// One already running with its window hidden (left by the toolbox before an update): looked after as started.
    pub fn adopt(&mut self) {
        if let Some((h, pid)) = window() {
            if !unsafe { IsWindowVisible(h.get()).as_bool() } {
                self.want = true;
                self.running = true;
                self.pid = Some(pid);
                note("Tartarus Solver already running hidden: looked after");
            }
        }
    }

    /// Shows its window (設定), or hides it again.
    pub fn show(&mut self, on: bool) {
        self.shown = on;
        if let Some((h, _)) = window() {
            unsafe {
                if on {
                    let _ = ShowWindow(h.get(), SW_SHOW);
                    let _ = ShowWindow(h.get(), SW_RESTORE);
                    let _ = SetForegroundWindow(h.get());
                } else {
                    let _ = ShowWindow(h.get(), SW_HIDE);
                }
            }
        }
    }

    /// About once a second: keeps its window hidden unless shown, follows it through its own updates (a new
    /// process), starts it again when it closed while it should run, ends it when it didn't close when asked.
    /// `exe`: the installed program. Whether anything changed that the main window shows.
    pub fn tick(&mut self, exe: Option<&Path>) -> bool {
        let before = (self.running, self.shown);
        match window() {
            Some((h, pid)) => {
                self.running = true;
                self.pid = Some(pid);
                self.gone_since = None;
                if self.want && !self.shown && unsafe { IsWindowVisible(h.get()).as_bool() } {
                    unsafe {
                        let _ = ShowWindow(h.get(), SW_HIDE);
                    }
                }
                if !self.want && self.stop_at.is_some_and(|t| t.elapsed() > Duration::from_secs(6)) {
                    unsafe {
                        if let Ok(p) = OpenProcess(PROCESS_TERMINATE, false, pid) {
                            let _ = TerminateProcess(p, 0);
                            let _ = CloseHandle(p);
                        }
                    }
                    note("Tartarus Solver didn't close: ended");
                    self.stop_at = None;
                }
            }
            None => {
                // starting (its window not up yet) counts as running
                let starting = self.pid.is_some_and(alive) && self.started.is_some_and(|t| t.elapsed() < STARTING);
                self.running = starting;
                if !starting {
                    self.shown = false;
                    self.stop_at = None;
                }
                if self.want && !starting {
                    let since = *self.gone_since.get_or_insert_with(Instant::now);
                    if since.elapsed() > GONE {
                        if let Some(exe) = exe {
                            note("Tartarus Solver closed while it should run: starting it again");
                            self.start(exe);
                        }
                    }
                }
            }
        }
        before != (self.running, self.shown)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn versions_compare_by_their_numbers() {
        assert!(super::newer("0.7.44", "0.7.40"));
        assert!(super::newer("v0.8.0", "0.7.44"));
        assert!(!super::newer("0.7.44", "0.7.44"));
        assert!(!super::newer("0.7.9", "0.7.10"));
    }
}
