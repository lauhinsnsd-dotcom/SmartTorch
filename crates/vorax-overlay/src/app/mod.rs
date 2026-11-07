//! The live overlay on Windows: one window thread (the overlay, tray icon and hotkeys) and one worker thread
//! (the log, the pictures, the reading and the advice).

mod capture;
mod etor;
mod hub;
mod menu;
mod logview;
mod report;
mod tartarus;
mod ocr;
mod sys;
mod update;
mod window;
mod worker;

use crate::arg;
use std::path::PathBuf;
use windows::core::w;
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};

/// The language card titles are read in, from SmartTorch's interface language ("tc", "sc" or "en"): Chinese
/// words, a Chinese client, read with a Chinese reader even when Windows also has an English one (v1.0.0 to 1.0.2
/// read with the English one first, as the start files' --ocr-lang zh was gone), Simplified words with a
/// Simplified one first and Traditional with a Traditional one (not by the order of Windows' languages); English
/// with the English one.
pub fn ocr_lang(lang: &str) -> Option<String> {
    match lang {
        "en" => None,
        "sc" => Some("zh-Hans".to_string()),
        _ => Some("zh-Hant".to_string()),
    }
}

pub fn run(args: &[String]) {
    // Windows' shared COM area (the MTA) kept alive for the whole run, so a short-lived thread that used it can't
    // take it down under the factories the windows crate keeps cached
    unsafe {
        let _ = windows::Win32::System::Com::CoIncrementMTAUsage();
    }
    // a crash's message into the debug log (a release build has no console to show it)
    std::panic::set_hook(Box::new(|info| {
        let at = info.location().map(|l| format!(" at {}:{}", l.file(), l.line())).unwrap_or_default();
        let msg = info.payload().downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| info.payload().downcast_ref::<String>().cloned()).unwrap_or_default();
        sys::note(&format!("CRASH: {msg}{at}"));
    }));
    // a release build has no console of its own: print into the one it was started from
    if args.iter().any(|a| a == "--selftest" || a == "--console" || a == "--demo") {
        unsafe {
            let _ = windows::Win32::System::Console::AttachConsole(windows::Win32::System::Console::ATTACH_PARENT_PROCESS);
        }
    }
    if let Some(i) = args.iter().position(|a| a == "--demo") {
        return demo(args.get(i + 1).filter(|a| !a.starts_with("--")).map(PathBuf::from), arg(args, "--seconds").unwrap_or(15));
    }
    // the runs window drawn from the debug log into a picture
    if let Some(i) = args.iter().position(|a| a == "--runs-preview") {
        let Some(out) = args.get(i + 1) else { return };
        let Some(fonts) = crate::draw::Fonts::system() else { return };
        let log = std::fs::read_to_string(sys::data_dir().join("overlay.log")).unwrap_or_default();
        let pm = logview::preview(&log, std::rc::Rc::new(fonts), 1.0, 760.0);
        let mut rgba = pm.data().to_vec();
        for p in rgba.chunks_exact_mut(4) {
            let a = p[3] as f32 / 255.0;
            for c in 0..3 {
                p[c] = (p[c] as f32 + 70.0 * (1.0 - a)).round() as u8;
            }
            p[3] = 255;
        }
        let _ = crate::pipeline::encode_png(std::path::Path::new(out), pm.width(), pm.height(), &rgba);
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--menu-preview") {
        let Some(out) = args.get(i + 1) else { return };
        let Some(fonts) = crate::draw::Fonts::system() else { return };
        let rows = vec![
            menu::Row::Header { title: "Vorax 輔助器中文版".into(), status: format!("v{} · 機率資料 {}", update::current(), vorax::data().odds) },
            menu::Row::Warn("遊戲日誌未開啟：不會自動讀取畫面".into()),
            menu::Row::Action { id: 0, label: "如何開啟遊戲日誌".into(), shortcut: String::new() },
            menu::Row::Separator,
            menu::Row::Switch { id: 0, label: "顯示提示（Ctrl+Alt+V）".into(), on: true },
            menu::Row::Action { id: 0, label: "立即讀取畫面".into(), shortcut: "Ctrl+Alt+X".into() },
            menu::Row::Levels { base: 0, level: 6, hint: |l| format!("手術用具可刷新 {} 次", crate::pipeline::shelley_rerolls(l)) },
            menu::Row::Colors { base: 0, selected: 0 },
            menu::Row::Separator,
            menu::Row::Action { id: 0, label: "對局紀錄".into(), shortcut: "查看".into() },
            menu::Row::Separator,
            menu::Row::Action { id: 0, label: "結束".into(), shortcut: "Ctrl+Alt+Q".into() },
        ];
        // the menu as it usually is: the game's log on
        let plain = args.iter().any(|a| a == "--plain");
        let rows = if plain {
            let mut head = vec![
                menu::Row::Header { title: "Vorax 輔助器中文版".into(), status: format!("v{} · 機率資料 {}", update::current(), vorax::data().odds) },
                menu::Row::Text("遊戲日誌已開啟：會自動讀取畫面".into()),
            ];
            head.extend(rows.into_iter().skip(3));
            head
        } else {
            rows
        };
        // the steps shown when the game's log is off
        let rows = if args.iter().any(|a| a == "--log-card") {
            vec![
                menu::Row::Header { title: "開啟遊戲日誌".into(), status: "開啟前不會自動讀取畫面".into() },
                menu::Row::Text("1.  在遊戲中按 Esc".into()),
                menu::Row::Text("2.  開啟「帳號相關」".into()),
                menu::Row::Text("3.  點「開啟日誌」".into()),
                menu::Row::Text("4.  登出後再登入".into()),
                menu::Row::Text("在此之前，可按 Ctrl+Alt+X 讀取畫面。".into()),
                menu::Row::Buttons { primary: (0, "確定".into()), secondary: None },
            ]
        } else {
            rows
        };
        let hover = if plain { None } else { Some((6, Some(4))) };
        let pm = menu::preview(rows, std::rc::Rc::new(fonts), 1.5, hover);
        // onto a mid-grey, as a desktop would show it
        let mut rgba = pm.data().to_vec();
        for p in rgba.chunks_exact_mut(4) {
            let a = p[3] as f32 / 255.0;
            for c in 0..3 {
                p[c] = (p[c] as f32 + 70.0 * (1.0 - a)).round() as u8;
            }
            p[3] = 255;
        }
        let _ = crate::pipeline::encode_png(std::path::Path::new(out), pm.width(), pm.height(), &rgba);
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--selftest") {
        return selftest(args.get(i + 1).map(PathBuf::from).unwrap_or_else(std::env::temp_dir));
    }
    if args.iter().any(|a| a == "--console") {
        std::env::set_var("VORAX_CONSOLE", "1");
    }
    if args.iter().any(|a| a == "--version") {
        return println!("vorax-overlay {}", update::current());
    }
    unsafe {
        // one overlay at a time (after an update, the one before is still closing: wait for it)
        let waits = if args.iter().any(|a| a == "--after-update") { 50 } else { 0 };
        let mut n = 0;
        loop {
            let h = CreateMutexW(None, true, w!("Local\\VoraxRsOverlay"));
            if GetLastError() != ERROR_ALREADY_EXISTS {
                break;
            }
            if let Ok(h) = h {
                let _ = windows::Win32::Foundation::CloseHandle(h);
            }
            n += 1;
            if n > waits {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        // physical pixels everywhere, so the overlay lines up with the captured picture at any display scaling
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        // the window thread is a single-threaded COM apartment: WebView2 (the main window) and the file dialog need one
        let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_APARTMENTTHREADED);
    }
    let mut settings = sys::Settings::load();
    if let Some(s) = arg::<i32>(args, "--shelley") {
        settings.shelley = s.clamp(1, 6);
        settings.save();
    }
    if args.iter().any(|a| a == "--no-ocr") {
        settings.ocr = false;
    }
    // what earlier versions fetched from upstream and kept beside the settings: no longer used
    for old in ["data", "data.new", "sync-output.log"] {
        let p = sys::data_dir().join(old);
        let _ = if p.is_dir() { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
    }
    update::clean_up();
    sys::note(&format!("Vorax overlay v{} started (Intern Shelley level {}, odds of {})", update::current(), settings.shelley, vorax::data().odds));
    // SmartTorch's language: Traditional, Simplified or English, from Windows' language at the first start (a
    // Chinese edition updated keeps its Chinese)
    if settings.lang.is_empty() {
        settings.lang = if settings.first_run_done || settings.script != crate::zh::AUTO {
            if settings.script == crate::zh::SIMPLIFIED { "sc".into() } else { "tc".into() }
        } else {
            sys::windows_lang().into()
        };
        if settings.script == crate::zh::AUTO {
            settings.script = if settings.lang == "sc" { crate::zh::SIMPLIFIED } else { crate::zh::TRADITIONAL };
        }
        settings.save();
    }
    crate::zh::set_english(settings.lang == "en");
    crate::zh::set_mode(settings.script);
    vorax::bot::MEAN_GOAL.store(settings.aggressive, std::sync::atomic::Ordering::Relaxed);
    // the mode every start (not only the aggressive one), so the runs window can count each mode's games
    sys::note(if settings.aggressive { "mode: aggressive (衝分), futures judged by their plain final score" } else { "mode: upstream's (保守), futures counted up to 840k" });
    let Some(fonts) = crate::draw::Fonts::system() else {
        sys::note("no Segoe UI or Arial font found");
        return;
    };
    let (to_ui, from_worker) = std::sync::mpsc::channel();
    let (to_worker, from_ui) = std::sync::mpsc::channel();
    let waker = match window::create(fonts, from_worker, to_ui.clone(), to_worker, settings.clone(), args.to_vec()) {
        Ok(w) => w,
        Err(e) => {
            sys::note(&format!("overlay window: {e}"));
            return;
        }
    };
    // a log given on the command line (the old start files), else the one chosen in the main window
    let log = arg::<String>(args, "--log").map(PathBuf::from).or_else(|| settings.log_path.clone());
    let opts = worker::Options { log, shelley: settings.shelley, ocr: settings.ocr, ocr_lang: arg::<String>(args, "--ocr-lang").or_else(|| ocr_lang(&settings.lang)), frames: arg::<String>(args, "--frames").map(PathBuf::from), picker: settings.picker.unwrap_or(true) };
    waker.wake();
    // the main window at start, unless started with Windows (into the tray); the first-use steps the first time
    if !settings.first_run_done {
        window::open_hub_later(true);
    } else if !args.iter().any(|a| a == "--tray" || a == "--after-update") {
        window::open_hub_later(false);
    }
    // a newer release on GitHub: looked for at the start and every 6 hours after (SmartTorch may run for days)
    let (check_tx, check_waker) = (to_ui.clone(), waker);
    std::thread::spawn(move || {
        unsafe {
            let _ = windows::Win32::System::WinRT::RoInitialize(windows::Win32::System::WinRT::RO_INIT_MULTITHREADED);
        }
        loop {
            match update::check() {
                Ok(Some(r)) => {
                    let _ = check_tx.send(window::ToUi::Update(r));
                    check_waker.wake();
                }
                Ok(None) => sys::note(&format!("v{} is the latest release", update::current())),
                Err(e) => sys::note(&format!("couldn't check for a newer release: {e}")),
            }
            std::thread::sleep(std::time::Duration::from_secs(6 * 60 * 60));
        }
    });
    let worker = std::thread::spawn(move || worker::run(opts, to_ui, waker, from_ui));
    window::run_loop();
    let _ = worker.join();
    sys::note("Vorax overlay closed");
}

/// Checks each piece on this PC without the overlay: the log, the game window, one picture of it, the text
/// reader on a known card title, and reading the picture. Writes the picture to `out`.
fn selftest(out: PathBuf) {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let _ = windows::Win32::System::WinRT::RoInitialize(windows::Win32::System::WinRT::RO_INIT_MULTITHREADED);
    }
    // the displays' colour mode: HDR changes the colours a capture gets
    unsafe {
        use windows::core::Interface;
        use windows::Win32::Graphics::Dxgi::*;
        if let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() {
            let mut a = 0;
            while let Ok(adapter) = factory.EnumAdapters1(a) {
                let mut o = 0;
                while let Ok(output) = adapter.EnumOutputs(o) {
                    if let Ok(o6) = output.cast::<IDXGIOutput6>() {
                        if let Ok(d) = o6.GetDesc1() {
                            let name = String::from_utf16_lossy(&d.DeviceName).trim_end_matches('\0').to_string();
                            let hdr = d.ColorSpace == windows::Win32::Graphics::Dxgi::Common::DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020;
                            println!("display {name}: {}x{}, HDR {}, colour space {}, max {} nits", d.DesktopCoordinates.right - d.DesktopCoordinates.left, d.DesktopCoordinates.bottom - d.DesktopCoordinates.top, if hdr { "ON" } else { "off" }, d.ColorSpace.0, d.MaxLuminance);
                        }
                    }
                    o += 1;
                }
                a += 1;
            }
        }
    }
    println!("log: {}", sys::find_log().map_or("not found".into(), |p| p.display().to_string()));
    let game = sys::find_game_window();
    println!("game window: {}", game.map_or("not open".into(), |g| format!("{:?}, picture at {:?}, in front: {}", g, sys::client_rect(g), sys::foreground() == g)));
    match ocr::Ocr::new() {
        Ok(o) => {
            let frame = crate::pipeline::decode_png(&PathBuf::from("upstream/tests/fixtures/frames/board-1080p.png"));
            match frame {
                Ok(f) => {
                    let l = vorax_scan::layout::layout(f.width, f.height);
                    for (j, c) in l.cards.iter().enumerate() {
                        let crop = vorax_scan::titles::title_crop(&f, c, l.k);
                        let t = std::time::Instant::now();
                        let text = o.read(&crop).unwrap_or_else(|e| format!("error {e}"));
                        println!("OCR ({}) card {j}: {:?} in {} ms", o.language, text.lines().next().unwrap_or(""), t.elapsed().as_millis());
                    }
                }
                Err(e) => println!("OCR: no test frame ({e})"),
            }
        }
        Err(e) => println!("OCR: not available ({e})"),
    }
    let Some(game) = game else { return };
    let cap = capture::Capturer::new();
    let t = std::time::Instant::now();
    match cap.grab(game) {
        Ok(Some(img)) => {
            println!("capture: {}x{} in {} ms", img.width, img.height, t.elapsed().as_millis());
            let path = out.join("selftest-capture.png");
            let _ = crate::pipeline::encode_png(&path, img.width as u32, img.height as u32, &img.data);
            println!("saved {}", path.display());
            let mut p = crate::pipeline::Pipeline::new(6);
            match p.run(&img, None, None) {
                crate::pipeline::Outcome::Advice { advice, plan, read_ms, .. } => println!("read in {read_ms:.0} ms, advice in {} ms: {:?}", advice.ms, plan.pill.map(|p| p.main)),
                crate::pipeline::Outcome::Animating => println!("read: a card is moving"),
                crate::pipeline::Outcome::NotGame => println!("read: not the Surgical Preparation screen (expected outside Vorax)"),
                crate::pipeline::Outcome::Closing => println!("read: the game closing after round 13"),
            }
        }
        Ok(None) => println!("capture: no frame"),
        Err(e) => println!("capture: {e}"),
    }
}

/// The overlay with a saved board's advice, over the game for 15 seconds (switch to the game to see it): checks
/// that it lines up on this screen without playing a Vorax round.
fn demo(frame: Option<PathBuf>, seconds: u64) {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    let path = frame.unwrap_or_else(|| PathBuf::from("upstream/tests/fixtures/frames/board-1080p.png"));
    let img = match crate::pipeline::decode_png(&path) {
        Ok(i) => i,
        Err(e) => return println!("{e}"),
    };
    let mut p = crate::pipeline::Pipeline::new(6);
    let crate::pipeline::Outcome::Advice { plan, .. } = p.run(&img, None, None) else { return println!("{}: no advice on that picture", path.display()) };
    println!("demo: {} over the game for {seconds} s", plan.pill.as_ref().map_or(String::new(), |p| format!("{} · {}", p.main, p.sub)));
    let Some(fonts) = crate::draw::Fonts::system() else { return };
    let (to_ui, from_worker) = std::sync::mpsc::channel();
    let (to_worker, _from_ui) = std::sync::mpsc::channel();
    let Ok(waker) = window::create(fonts, from_worker, to_ui.clone(), to_worker, sys::Settings { shown: true, ..sys::Settings::default() }, vec![]) else { return };
    let _ = to_ui.send(window::ToUi::Show(plan, img.width, img.height));
    waker.wake();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(seconds));
        let _ = to_ui.send(window::ToUi::Quit);
        waker.wake();
    });
    window::run_loop();
}
