//! The toolbox's problem reports (not in upstream). Every picture read is kept for a while (the last 20, in
//! %APPDATA%\VoraxRs\recent), so nothing has to be switched on before a misread: Ctrl+Alt+B copies the last one
//! to "marked", and the main window's 回報問題 packs the marked pictures, the debug log and the settings into one
//! zip on the desktop, and opens the project's issue page.

use super::sys::{data_dir, note};
use std::path::{Path, PathBuf};

/// Pictures kept.
const KEEP: usize = 20;

fn recent_dir() -> PathBuf {
    data_dir().join("recent")
}

fn marked_dir() -> PathBuf {
    data_dir().join("marked")
}

fn pngs(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).map(|r| r.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "png")).collect()).unwrap_or_default();
    // named by the time they were read: in name order, oldest first
    v.sort();
    v
}

/// Keeps a picture just read (named by when, what started the read and what came of it), dropping the oldest
/// beyond KEEP. Its path, for Ctrl+Alt+B.
pub fn keep(name: &str, w: usize, h: usize, rgba: &[u8]) -> Option<PathBuf> {
    let dir = recent_dir();
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join(name);
    if let Err(e) = crate::pipeline::encode_png_fast(&path, w as u32, h as u32, rgba) {
        note(&format!("picture not kept: {e}"));
        return None;
    }
    let all = pngs(&dir);
    for old in all.iter().take(all.len().saturating_sub(KEEP)) {
        let _ = std::fs::remove_file(old);
    }
    Some(path)
}

/// Ctrl+Alt+B: the last picture read set aside for the report. How many are set aside now.
pub fn mark(last: Option<&Path>) -> Result<usize, String> {
    let last = last.filter(|p| p.is_file()).ok_or_else(|| crate::zh::tr("還沒有讀過畫面", "nothing has been read yet"))?;
    let dir = marked_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = last.file_name().ok_or("no name")?;
    std::fs::copy(last, dir.join(name)).map_err(|e| e.to_string())?;
    note(&format!("marked for the report: {}", name.to_string_lossy()));
    Ok(marked())
}

/// Opens the folder of the pictures set aside, in Explorer.
pub fn open_marked() {
    let dir = marked_dir();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::process::Command::new("explorer.exe").arg(&dir).spawn();
}

/// Throws away the pictures set aside (and the last ones read).
pub fn clear() {
    for p in pngs(&marked_dir()).into_iter().chain(pngs(&recent_dir())) {
        let _ = std::fs::remove_file(p);
    }
    note("pictures set aside cleared");
}

/// 清除紀錄: the debug log (and the one before it: the runs and statistics come from them) and the pictures,
/// gone. A problem reported and solved, the next report starts clean.
pub fn clear_all() {
    clear();
    let dir = data_dir();
    for name in std::iter::once("overlay.log").chain(super::sys::OLD_LOGS) {
        let _ = std::fs::remove_file(dir.join(name));
    }
}

/// How many pictures are set aside for the report.
pub fn marked() -> usize {
    pngs(&marked_dir()).len()
}

/// The desktop (where it really is: OneDrive moves it), else the user's folder.
fn desktop() -> PathBuf {
    use windows::Win32::UI::Shell::{FOLDERID_Desktop, SHGetKnownFolderPath, KF_FLAG_DEFAULT};
    unsafe {
        if let Ok(p) = SHGetKnownFolderPath(&FOLDERID_Desktop, KF_FLAG_DEFAULT, windows::Win32::Foundation::HANDLE::default()) {
            let s = String::from_utf16_lossy(p.as_wide());
            windows::Win32::System::Com::CoTaskMemFree(Some(p.0 as *const _));
            return PathBuf::from(s);
        }
    }
    std::env::var("USERPROFILE").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir())
}

/// Packs the report: the marked pictures (else the last 5 read), the debug logs kept, the
/// settings and the version. The marked pictures are cleared once packed. The zip's path.
pub fn pack(version: &str) -> Result<PathBuf, String> {
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let marked = pngs(&marked_dir());
    let pictures: Vec<PathBuf> = if marked.is_empty() {
        let all = pngs(&recent_dir());
        all.iter().rev().take(5).cloned().collect()
    } else {
        marked.clone()
    };
    for p in &pictures {
        if let (Some(name), Ok(data)) = (p.file_name(), std::fs::read(p)) {
            // ASCII names: every unzip reads them alike
            let folder = if marked.is_empty() { "recent" } else { "marked" };
            files.push((format!("{folder}/{}", name.to_string_lossy()), data));
        }
    }
    let dir = data_dir();
    // every log kept (oldest first): the runs in them are what a report is checked against
    let names: Vec<&str> = super::sys::OLD_LOGS.iter().rev().copied().chain(["overlay.log", "settings.json", "learned.json"]).collect();
    for name in names {
        if let Ok(data) = std::fs::read(dir.join(name)) {
            // the home folder left out (logs written before v2.0.0 have it; settings.json has the Tartarus path)
            let data = match String::from_utf8(data) {
                Ok(text) => crate::privacy::private(&text).into_bytes(),
                Err(e) => e.into_bytes(),
            };
            files.push((name.to_string(), data));
        }
    }
    files.push(("version.txt".to_string(), format!("SmartTorch v{version}\n").into_bytes()));
    let stamp = {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        super::sys::local_stamp(secs)
    };
    let out = desktop().join(format!("SmartTorch-report-{stamp}.zip"));
    let mut f = std::fs::File::create(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    crate::zipw::write(&mut f, &files).map_err(|e| e.to_string())?;
    for p in &marked {
        let _ = std::fs::remove_file(p);
    }
    note(&format!("report packed: {} ({} pictures)", out.display(), pictures.len()));
    Ok(out)
}

/// Shows the zip in Explorer, selected, so it can be dragged onto the issue page.
pub fn show_in_folder(p: &Path) {
    let _ = std::process::Command::new("explorer.exe").arg(format!("/select,{}", p.display())).spawn();
}
