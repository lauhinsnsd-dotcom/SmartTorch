//! The toolbox's 易火 page: a launcher for 易火 ETor, another author's closed-source tool (not part of SmartTorch,
//! never shipped or downloaded by it). The player installs it; the toolbox finds it (chosen, or as it runs) and
//! opens it, by the button or as the game opens (when that's on). Its window is left as it is, and nothing closes
//! it. It asks for administrator rights, so Windows asks the player each time it's opened.

use super::sys::note;
use std::cell::Cell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM};
use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, SW_SHOWNORMAL};

/// The installed program: the one chosen (or found running), while it's still there.
pub fn installed(chosen: Option<&Path>) -> Option<PathBuf> {
    chosen.filter(|p| p.is_file()).map(Path::to_path_buf)
}

/// Whether a program is 易火: the one chosen, else any "…ETor.exe" (it ships as 易火-ETor.exe).
fn is_etor(exe: &str, chosen: Option<&Path>) -> bool {
    let p = Path::new(exe);
    match chosen {
        Some(c) => p.file_name().map(|n| n.to_string_lossy().to_lowercase()) == c.file_name().map(|n| n.to_string_lossy().to_lowercase()),
        None => p.file_name().is_some_and(|n| n.to_string_lossy().to_lowercase().ends_with("etor.exe")),
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

struct Search<'a> {
    chosen: Option<&'a Path>,
    seen: HashSet<u32>,
    found: Option<PathBuf>,
}

unsafe extern "system" fn each(hwnd: HWND, l: LPARAM) -> BOOL {
    let s = &mut *(l.0 as *mut Search);
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid != 0 && s.seen.insert(pid) {
        if let Some(exe) = exe_of(pid) {
            if is_etor(&exe, s.chosen) {
                s.found = Some(PathBuf::from(exe));
                return BOOL(0);
            }
        }
    }
    BOOL(1)
}

/// The program of the 易火 running now (any of its windows, shown or not), if any.
pub fn running(chosen: Option<&Path>) -> Option<PathBuf> {
    let mut s = Search { chosen, seen: HashSet::new(), found: None };
    unsafe {
        let _ = EnumWindows(Some(each), LPARAM(&mut s as *mut _ as isize));
    }
    s.found
}

thread_local! {
    static LAST_LOOK: Cell<Option<Instant>> = const { Cell::new(None) };
    static OPENED: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// Found as it runs, when not chosen yet: looked for every 10 seconds at most.
pub fn find_running() -> Option<PathBuf> {
    let due = LAST_LOOK.with(|c| c.get().is_none_or(|t| t.elapsed() >= Duration::from_secs(10)));
    if !due {
        return None;
    }
    LAST_LOOK.with(|c| c.set(Some(Instant::now())));
    running(None)
}

/// Whether it was opened in the last half minute (Windows may still be asking for the rights).
pub fn opening() -> bool {
    OPENED.with(|c| c.get().is_some_and(|t| t.elapsed() < Duration::from_secs(30)))
}

/// Opens it as Windows opens it from its shortcut (asking for administrator rights), from its own folder. On a
/// thread of its own: Windows' question waits for the player.
pub fn open(exe: &Path) {
    OPENED.with(|c| c.set(Some(Instant::now())));
    note(&format!("opening ETor: {}", exe.display()));
    let exe = exe.to_path_buf();
    std::thread::spawn(move || unsafe {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};
        use windows::Win32::UI::Shell::ShellExecuteW;
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
        let dir = exe.parent().map(|d| HSTRING::from(d.as_os_str())).unwrap_or_default();
        let r = ShellExecuteW(HWND::default(), &HSTRING::from("open"), &HSTRING::from(exe.as_os_str()), PCWSTR::null(), &dir, SW_SHOWNORMAL);
        // above 32: opened; else Windows' error (5: the player said no to the rights)
        if r.0 as isize <= 32 {
            note(&format!("ETor not opened (ShellExecute {})", r.0 as isize));
        }
    });
}
