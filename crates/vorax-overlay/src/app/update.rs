//! Updating SmartTorch from its own GitHub releases (not in upstream: the Chinese edition's, now SmartTorch,
//! lauhinsnsd-dotcom/SmartTorch, versions v0.2.2-zh.1 and on; SmartTorch.exe, vorax-overlay.exe before v1.0.0): at start it asks for the latest release, and when it's
//! newer the tray menu offers it with its notes. The new overlay.exe is downloaded next to this one, checked against
//! the SHA-256 GitHub gives for it, and swapped in: a running exe can't be overwritten but can be renamed, so this
//! one moves aside (deleted on the next start) and the new one starts in its place.

use super::sys::note;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use windows::core::HSTRING;
use windows::Foundation::{AsyncStatus, Uri};
use windows::Storage::Streams::{Buffer, DataReader, InputStreamOptions};
use windows::Web::Http::{HttpClient, HttpCompletionOption, HttpMethod, HttpRequestMessage};

pub const REPO: &str = "lauhinsnsd-dotcom/SmartTorch";
/// The asset of a release that is the overlay.
const ASSET: &str = "SmartTorch.exe";

/// This build's version. A release's build is given its tag's (VORAX_VERSION at build time, "0.2.2-zh.1"); others
/// are upstream's (0.2.2), offered any release of the Chinese edition. A test build can claim an older one.
pub fn current() -> &'static str {
    option_env!("VORAX_VERSION").map(|v| v.trim_start_matches('v')).filter(|v| !v.is_empty()).unwrap_or(env!("CARGO_PKG_VERSION"))
}

#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    /// what's new, as the release notes say it (without the download instructions and the file table)
    pub notes: String,
    pub page: String,
    url: String,
    size: u64,
    sha256: String,
}

/// A version as numbers to compare: upstream's three, then the Chinese edition's number ("0.2.2-zh.3": 0, 2, 2, 3;
/// "0.2.2": 0, 2, 2, 0).
fn version_of(s: &str) -> Option<(u32, u32, u32, u32)> {
    let s = s.trim().trim_start_matches('v');
    let (base, zh) = match s.split_once("-zh.") {
        Some((b, z)) => (b, z.parse::<u32>().ok()?),
        None => (s, 0),
    };
    let mut it = base.split('.').map(|p| p.parse::<u32>().ok());
    Some((it.next()??, it.next()??, it.next().flatten().unwrap_or(0), zh))
}

fn client() -> Result<HttpClient, String> {
    let c = HttpClient::new().map_err(|e| e.to_string())?;
    if let Ok(h) = c.DefaultRequestHeaders() {
        let _ = h.UserAgent().and_then(|ua| ua.TryParseAdd(&HSTRING::from(format!("vorax-overlay/{}", current()))));
    }
    Ok(c)
}

/// The release notes as plain lines for the card: what's new only, markdown marks dropped.
fn plain_notes(body: &str) -> String {
    let mut out = Vec::new();
    for line in body.lines() {
        let l = line.trim();
        if l.starts_with("Download `") || l.starts_with("Built from") || l.starts_with("| File") || l.starts_with("SHA-256") || l.starts_with("第一次執行") || l.starts_with("SmartTorch：") {
            break;
        }
        if l.starts_with('>') {
            continue;
        }
        let mut t = l.replace("**", "").replace('`', "");
        // [text](link) → text
        while let (Some(a), Some(b)) = (t.find("]("), t.find('[')) {
            let Some(c) = t[a..].find(')').map(|c| a + c) else { break };
            if b > a {
                break;
            }
            t = format!("{}{}{}", &t[..b], &t[b + 1..a], &t[c + 1..]);
        }
        out.push(t);
    }
    out.join("\n").trim().to_string()
}

/// The latest release when it's newer than this build (None when this is the latest).
pub fn check() -> Result<Option<Release>, String> {
    let v = get_json(&format!("https://api.github.com/repos/{REPO}/releases/latest"))?;
    let tag = v["tag_name"].as_str().ok_or("GitHub: no release")?;
    let (Some(latest), Some(mine)) = (version_of(tag), version_of(current())) else { return Err(format!("versions {tag} / {}", current())) };
    if latest <= mine {
        return Ok(None);
    }
    let asset = v["assets"].as_array().and_then(|a| a.iter().find(|x| x["name"] == ASSET)).ok_or(format!("{tag} has no {ASSET}"))?;
    let sha256 = asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).ok_or(format!("{tag}: GitHub gives no SHA-256 for {ASSET}"))?;
    Ok(Some(Release {
        version: tag.trim_start_matches('v').to_string(),
        notes: plain_notes(v["body"].as_str().unwrap_or_default()),
        page: v["html_url"].as_str().unwrap_or_default().to_string(),
        url: asset["browser_download_url"].as_str().ok_or("no download link")?.to_string(),
        size: asset["size"].as_u64().unwrap_or(0),
        sha256: sha256.to_ascii_lowercase(),
    }))
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn aside(exe: &Path, ext: &str) -> PathBuf {
    let mut s = exe.as_os_str().to_owned();
    s.push(ext);
    PathBuf::from(s)
}

/// Not in upstream: how long a connection may send nothing before the try is given up (a stalled connection would
/// otherwise be waited on for ever, the update card stuck at its percent), and how many tries in a row that bring
/// no bytes before the player is told. Players without a steady way to GitHub (mainland China, no accelerator) saw
/// downloads cut off and never finish; glcoder's Tartarus Solver, which goes on from where a download stopped,
/// updated for them.
const STALL: Duration = Duration::from_secs(30);
const IDLE_TRIES: u32 = 5;

/// Waits for a WinRT operation for at most STALL: its result, or an error once it failed or took too long (it's
/// cancelled then).
macro_rules! within_stall {
    ($op:expr, $what:expr) => {{
        let op = $op;
        let t0 = Instant::now();
        loop {
            match op.Status() {
                Ok(AsyncStatus::Started) if t0.elapsed() < STALL => std::thread::sleep(Duration::from_millis(20)),
                Ok(AsyncStatus::Started) => {
                    let _ = op.Cancel();
                    break Err(format!("{}: nothing for {} s", $what, STALL.as_secs()));
                }
                Ok(AsyncStatus::Completed) => break op.GetResults().map_err(|e| format!("{}: {e}", $what)),
                Ok(_) => break Err(format!("{}: {}", $what, op.ErrorCode().map(|c| windows::core::Error::from(c).to_string()).unwrap_or_else(|e| e.to_string()))),
                Err(e) => break Err(format!("{}: {e}", $what)),
            }
        }
    }};
}

/// A GitHub API answer (also the toolbox's: another project's latest release), tried three times.
pub fn get_json(url: &str) -> Result<Value, String> {
    let mut last = String::new();
    for attempt in 0..3 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_secs(2 * attempt));
        }
        let got = (|| -> Result<String, String> {
            let c = client()?;
            let uri = Uri::CreateUri(&HSTRING::from(url)).map_err(|e| e.to_string())?;
            let op = c.GetStringAsync(&uri).map_err(|e| format!("GitHub: {e}"))?;
            within_stall!(op, "GitHub").map(|t| t.to_string())
        })();
        match got {
            Ok(text) => return serde_json::from_str(&text).map_err(|e| e.to_string()),
            Err(e) => last = e,
        }
    }
    Err(last)
}

/// Downloads a file (`progress` from 0 to 1 as it comes) and checks it against GitHub's SHA-256. Not in upstream:
/// a download cut off or stalled goes on from where it stopped (a Range request), tried again while tries bring
/// bytes, given up after IDLE_TRIES in a row bring none.
pub fn download(url: &str, size: u64, sha256: &str, progress: impl Fn(f32) + Send + Sync + 'static) -> Result<Vec<u8>, String> {
    let total = size.max(1) as f32;
    let mut bytes: Vec<u8> = Vec::with_capacity(size as usize);
    let (mut idle, mut wait) = (0, Duration::from_secs(2));
    // one start over from nothing when a resumed download doesn't check out (its pieces didn't fit)
    let mut fresh_try = true;
    loop {
        let before = bytes.len();
        let mut out = fetch_from(url, &mut bytes, &|n| progress((n as f32 / total).min(1.0)));
        // the server ended the transfer early but cleanly: as cut off
        if out.is_ok() && size > 0 && (bytes.len() as u64) < size {
            out = Err(format!("ended at {} of {size} bytes", bytes.len()));
        }
        if out.is_ok() && size > 0 && bytes.len() as u64 == size && sha256_hex(&bytes) != sha256 && std::mem::replace(&mut fresh_try, false) {
            note("the download doesn't match GitHub's SHA-256: downloading it again from the start");
            bytes.clear();
            continue;
        }
        match out {
            Ok(()) => break,
            Err(e) => {
                if bytes.len() > before {
                    idle = 0;
                    wait = Duration::from_secs(2);
                } else {
                    idle += 1;
                }
                if idle >= IDLE_TRIES {
                    return Err(format!("download: {e} ({} of {size} bytes)", bytes.len()));
                }
                note(&format!("download cut off at {} of {size} bytes, trying again: {e}", bytes.len()));
                std::thread::sleep(wait);
                wait = (wait * 2).min(Duration::from_secs(30));
            }
        }
    }
    if size > 0 && bytes.len() as u64 != size {
        return Err(format!("the download has {} bytes, GitHub says {size}", bytes.len()));
    }
    let got = sha256_hex(&bytes);
    if got != sha256 {
        return Err(format!("the download doesn't match GitHub's SHA-256 ({} bytes)", bytes.len()));
    }
    Ok(bytes)
}

/// One try: the rest of the file after `bytes` (all of it when the server sends it whole), appended as it comes.
/// Ok once the server has sent everything.
fn fetch_from(url: &str, bytes: &mut Vec<u8>, progress: &dyn Fn(usize)) -> Result<(), String> {
    let c = client()?;
    let uri = Uri::CreateUri(&HSTRING::from(url)).map_err(|e| e.to_string())?;
    let req = HttpRequestMessage::Create(&HttpMethod::Get().map_err(|e| e.to_string())?, &uri).map_err(|e| e.to_string())?;
    if !bytes.is_empty() {
        let headers = req.Headers().map_err(|e| e.to_string())?;
        let _ = headers.TryAppendWithoutValidation(&HSTRING::from("Range"), &HSTRING::from(format!("bytes={}-", bytes.len())));
    }
    let op = c.SendRequestWithOptionAsync(&req, HttpCompletionOption::ResponseHeadersRead).map_err(|e| e.to_string())?;
    let resp = within_stall!(op, "connecting")?;
    let status = resp.StatusCode().map_err(|e| e.to_string())?.0;
    match status {
        206 => {}
        // the whole file again (the server took no Range, or this is the first try)
        200 => bytes.clear(),
        _ => return Err(format!("GitHub answered {status}")),
    }
    let content = resp.Content().map_err(|e| e.to_string())?;
    let op = content.ReadAsInputStreamAsync().map_err(|e| e.to_string())?;
    let stream = within_stall!(op, "opening the download")?;
    const CHUNK: u32 = 64 * 1024;
    loop {
        let buf = Buffer::Create(CHUNK).map_err(|e| e.to_string())?;
        let op = stream.ReadAsync(&buf, CHUNK, InputStreamOptions::Partial).map_err(|e| e.to_string())?;
        let got = within_stall!(op, "downloading")?;
        let n = got.Length().map_err(|e| e.to_string())? as usize;
        if n == 0 {
            return Ok(());
        }
        let reader = DataReader::FromBuffer(&got).map_err(|e| e.to_string())?;
        let at = bytes.len();
        bytes.resize(at + n, 0);
        reader.ReadBytes(&mut bytes[at..]).map_err(|e| e.to_string())?;
        progress(bytes.len());
    }
}

/// Downloads the release's overlay (`progress` from 0 to 1 as it comes), checks it, and swaps it in for this exe.
/// Returns the exe to start.
pub fn install(r: &Release, progress: impl Fn(f32) + Send + Sync + 'static) -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let bytes = download(&r.url, r.size, &r.sha256, progress).map_err(|e| format!("{e}: not installed"))?;
    let new = aside(&exe, ".new");
    std::fs::write(&new, &bytes).map_err(|e| format!("{}: {e}", new.display()))?;
    let old = aside(&exe, ".old");
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&exe, &old).map_err(|e| format!("moving this overlay aside: {e}"))?;
    if let Err(e) = std::fs::rename(&new, &exe) {
        let _ = std::fs::rename(&old, &exe);
        return Err(format!("putting the new overlay in place: {e}"));
    }
    note(&format!("updated to v{} ({} bytes, SHA-256 checked)", r.version, bytes.len()));
    Ok(exe)
}

/// What an update left behind: the overlay it replaced (once that one has closed).
pub fn clean_up() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::fs::remove_file(aside(&exe, ".old"));
        let _ = std::fs::remove_file(aside(&exe, ".new"));
    }
}

#[cfg(test)]
mod tests {
    use super::version_of;

    #[test]
    fn the_chinese_editions_number_counts_after_upstreams() {
        assert!(version_of("v0.2.2-zh.2") > version_of("0.2.2-zh.1"));
        assert!(version_of("v0.2.2-zh.10") > version_of("v0.2.2-zh.9"));
        assert!(version_of("v0.2.2-zh.1") > version_of("0.2.2"));
        assert!(version_of("v0.2.3-zh.1") > version_of("0.2.2-zh.5"));
        assert_eq!(version_of("v0.2.2-zh.x"), None);
    }
}
