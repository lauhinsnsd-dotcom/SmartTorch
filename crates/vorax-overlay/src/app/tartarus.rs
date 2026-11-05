//! The toolbox's 囚獸助手: glcoder's Tartarus Solver (MIT, github.com/glcoder/TartarusSolver), used as it is. The
//! toolbox downloads it from its releases, starts it, keeps its window hidden (it has no tray icon of its own, and
//! its window's ✕ quits it) until 設定 shows it, starts it again if it closes while it should run, and reads its
//! statistics. Its own code is never changed.

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

/// Where the toolbox installs it: %LOCALAPPDATA%\FireToolbox\TartarusSolver.
pub fn home() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    base.join("FireToolbox")
}

/// Its own settings and statistics: %LOCALAPPDATA%\TartarusSolver.
fn its_folder() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    base.join("TartarusSolver")
}

/// The installed program: the one chosen, else the toolbox's own install.
pub fn installed(chosen: Option<&Path>) -> Option<PathBuf> {
    chosen.filter(|p| p.is_file()).map(Path::to_path_buf).or_else(|| Some(home().join("TartarusSolver").join(EXE)).filter(|p| p.is_file()))
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

/// .NET 9's desktop runtime is installed (the smaller download needs it; else the "-full" one, .NET included).
fn has_dotnet9() -> bool {
    let pf = std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".into());
    let dir = PathBuf::from(pf).join("dotnet").join("shared").join("Microsoft.WindowsDesktop.App");
    std::fs::read_dir(dir).map(|r| r.filter_map(|e| e.ok()).any(|e| e.file_name().to_string_lossy().starts_with("9."))).unwrap_or(false)
}

/// Its latest release: version, and the zip for this PC (url, size, SHA-256).
#[derive(Clone, Debug)]
pub struct Latest {
    pub version: String,
    url: String,
    size: u64,
    sha256: String,
}

pub fn latest() -> Result<Latest, String> {
    let v = super::update::get_json(&format!("https://api.github.com/repos/{REPO}/releases/latest"))?;
    let tag = v["tag_name"].as_str().ok_or("GitHub: no release")?;
    let version = tag.trim_start_matches('v').to_string();
    let full = !has_dotnet9();
    let want = format!("TartarusSolver-{version}{}.zip", if full { "-full" } else { "" });
    let asset = v["assets"].as_array().and_then(|a| a.iter().find(|x| x["name"] == want.as_str())).ok_or(format!("{tag} has no {want}"))?;
    let sha256 = asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).ok_or(format!("{want}: GitHub gives no SHA-256"))?;
    Ok(Latest { version, url: asset["browser_download_url"].as_str().ok_or("no download link")?.to_string(), size: asset["size"].as_u64().unwrap_or(0), sha256: sha256.to_ascii_lowercase() })
}

/// Copies every file under `from` into `to` (files `to` has that `from` doesn't stay), as its own updater does.
fn copy_over(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let dest = to.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_over(&e.path(), &dest)?;
        } else {
            // an antivirus may hold a fresh file a moment
            let mut tries = 0;
            loop {
                match std::fs::copy(e.path(), &dest) {
                    Ok(_) => break,
                    Err(err) if tries < 20 => {
                        tries += 1;
                        let _ = err;
                        std::thread::sleep(Duration::from_millis(500));
                    }
                    Err(err) => return Err(err),
                }
            }
        }
    }
    Ok(())
}

fn find_exe(dir: &Path) -> Option<PathBuf> {
    for e in std::fs::read_dir(dir).ok()?.filter_map(|e| e.ok()) {
        let p = e.path();
        if p.is_dir() {
            if let Some(x) = find_exe(&p) {
                return Some(x);
            }
        } else if p.file_name().is_some_and(|n| n.eq_ignore_ascii_case(EXE)) {
            return Some(p);
        }
    }
    None
}

/// Downloads the latest release and puts it in `target` (a folder; made if new). Its interface language set to
/// the toolbox's words on a first install (`ui`: "zhTw" or "zh"). The installed program.
pub fn install(target: &Path, ui: Option<&str>, progress: impl Fn(f32) + Send + Sync + 'static) -> Result<PathBuf, String> {
    let l = latest()?;
    note(&format!("Tartarus Solver: downloading v{} ({} bytes)", l.version, l.size));
    let bytes = super::update::download(&l.url, l.size, &l.sha256, progress)?;
    let work = home().join("download");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let zip = work.join("TartarusSolver.zip");
    std::fs::write(&zip, &bytes).map_err(|e| format!("{}: {e}", zip.display()))?;
    drop(bytes);
    let unpacked = work.join("unpacked");
    std::fs::create_dir_all(&unpacked).map_err(|e| e.to_string())?;
    // Windows' own tar (Windows 10 1803 and later) unpacks zips
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("tar").arg("-xf").arg(&zip).arg("-C").arg(&unpacked).creation_flags(0x0800_0000).output().map_err(|e| format!("tar: {e}"))?;
    if !out.status.success() {
        return Err(format!("unpacking: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let exe = find_exe(&unpacked).ok_or(format!("no {EXE} in the zip"))?;
    let from = exe.parent().unwrap().to_path_buf();
    copy_over(&from, target).map_err(|e| format!("copying into {}: {e}", target.display()))?;
    let _ = std::fs::remove_dir_all(&work);
    if let Some(lang) = ui {
        set_language(lang);
    }
    note(&format!("Tartarus Solver v{} installed in {}", l.version, target.display()));
    Ok(target.join(EXE))
}

/// Its interface in the toolbox's words, on a first install: written into its settings before it first starts
/// (left alone when the player has chosen a language there).
fn set_language(lang: &str) {
    let dir = its_folder();
    let path = dir.join("settings.json");
    let mut v: serde_json::Value = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok()).unwrap_or_else(|| serde_json::json!({}));
    let now = v["UiLanguage"].as_str().unwrap_or("auto");
    if now.eq_ignore_ascii_case("auto") {
        v["UiLanguage"] = serde_json::Value::String(lang.to_string());
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(&path, serde_json::to_string_pretty(&v).unwrap_or_default());
    }
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
    /// an install or update under way (0-1), and the last one's failure
    pub busy: Option<f32>,
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
            self.error = Some("囚獸助手接連關閉了三次，已停止自動重新開啟。".into());
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
                self.error = Some(format!("無法啟動：{e}"));
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
        if self.busy.is_some() {
            return false;
        }
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
