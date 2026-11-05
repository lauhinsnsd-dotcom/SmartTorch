//! The overlay window: transparent, click-through, always on top, never takes the focus. It covers the game's
//! picture and is redrawn only when the advice changes (or the game window moves), so it costs nothing in between.
//! Also the tray icon and the hotkeys.

use super::sys::{client_rect, find_game_window, foreground, note, Hwnd, Settings};
use super::menu::{put, wrap, Row};
use super::update::{self, Release};
use crate::zh::{english, tr};
use crate::draw::{render, Fonts, Plan};
use std::rc::Rc;
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT};
use windows::Win32::UI::Shell::{Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW};
use windows::Win32::UI::WindowsAndMessaging::*;

/// What the worker tells the window.
pub enum ToUi {
    /// new marks, for a picture of this size
    Show(Plan, usize, usize),
    /// a board is being read: the last advice, dimmed, without its marks
    Busy,
    /// the last advice stands (the board didn't change)
    Unbusy,
    /// not the Surgical Preparation screen: nothing to show
    Clear,
    /// close the overlay (the demo's end)
    Quit,
    /// whether the game writes its moves to the log, which the overlay needs to read boards by itself
    GameLog(GameLog),
    /// a newer release on GitHub (the check at start)
    Update(Release),
    /// the update's download, 0-1
    UpdateProgress(f32),
    /// the update didn't install: why
    UpdateFailed(String),
    /// the update is in place: start it, and close this one
    Restart(PathBuf),
    /// the toolbox: the check for a newer release found none
    UpToDate,
    /// the toolbox: Windows' text reader in use (its language), or none
    Ocr(Option<String>),
    /// the toolbox: the game's log followed (None: not found), and whether it was chosen by hand
    LogFile(Option<PathBuf>, bool),
    /// the toolbox: what the last read made out, and whether something wasn't read
    Read(String, bool),
    /// the toolbox: Ctrl+Alt+B set the last picture aside (how many are now), or why not
    Marked(Result<usize, String>),
    /// the toolbox: 囚獸助手's latest release (its version)
    TartLatest(String),
}

/// The game's log as the worker sees it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameLog {
    /// no UE_game.log where the game keeps it
    Missing,
    /// the game isn't open
    NoGame,
    /// the game just opened: no game line yet, but it's early to tell
    Checking,
    /// game lines are coming: boards are read by themselves
    On,
    /// the game is open but writes no game lines: its "Enable Log" is off
    Off,
}

/// What the window asks of the worker.
pub enum ToWorker {
    /// read the board now (Ctrl+Alt+X)
    Snap,
    /// the Intern Shelley level changed: advise the board again
    Shelley(i32),
    /// the toolbox: follow this log (chosen by hand), or find it again (None)
    LogPath(Option<PathBuf>),
    /// the toolbox's Stop (true) and Start (false)
    Pause(bool),
    /// Ctrl+Alt+B: the last picture set aside for the report
    Mark,
    Quit,
}

const WM_WAKE: u32 = WM_APP + 1;
const WM_TRAY: u32 = WM_APP + 2;
const HK_TOGGLE: i32 = 1;
const HK_SNAP: i32 = 2;
const HK_QUIT: i32 = 3;
/// not in upstream: show or hide what was read (Ctrl+Alt+D)
const HK_READ: i32 = 4;
/// the toolbox: the last picture set aside for the report (Ctrl+Alt+B)
const HK_MARK: i32 = 5;
const TIMER_FOLLOW: usize = 1;
/// the toolbox: Ctrl+Alt+B's note by the tray closes by itself
const TIMER_CARD: usize = 2;
/// the toolbox: once a second, 囚獸助手 looked after and the game's opening and closing followed
const TIMER_TICK: usize = 3;
const MENU_TOGGLE: u32 = 10;
const MENU_SNAP: u32 = 11;
const MENU_LOG: u32 = 12;
const MENU_QUIT: u32 = 13;
const MENU_READ: u32 = 14;
/// not in upstream: the words in Traditional or Simplified (auto, Traditional, Simplified in turn)
const MENU_SCRIPT: u32 = 15;
/// not in upstream: the advice's mode, upstream's (保守) or aggressive (衝分)
const MENU_MODE: u32 = 18;
/// Shelley levels 1-6 are MENU_SHELLEY + level
const MENU_SHELLEY: u32 = 20;
/// the marks' colours are MENU_ACCENT + their index in draw::ACCENTS
const MENU_ACCENT: u32 = 40;
const MENU_LATER: u32 = 16;
const MENU_LOG_HELP: u32 = 17;
/// the update's "what's new" card, installing it, its page on GitHub
const MENU_UPDATE: u32 = 60;
const MENU_UPDATE_NOW: u32 = 61;
const MENU_RELEASE_PAGE: u32 = 62;
/// the toolbox's main window
const MENU_HUB: u32 = 70;
/// the main window, opened from the message loop (never from inside the state's borrow)
const WM_OPEN_HUB: u32 = WM_APP + 3;

struct State {
    hwnd: HWND,
    fonts: Rc<Fonts>,
    rx: Receiver<ToUi>,
    tx: Sender<ToWorker>,
    plan: Option<Plan>,
    busy: bool,
    /// the size of the picture the marks were laid out on
    frame: (usize, usize),
    settings: Settings,
    /// where the overlay last was, and whether it was showing
    placed: Option<(i32, i32, i32, i32)>,
    visible: bool,
    /// the game's log, once the worker has looked; whether its help card has shown by itself this run
    game_log: Option<GameLog>,
    log_help_shown: bool,
    /// not in upstream: what was read shown under the pill (Ctrl+Alt+D), on at each start
    show_read: bool,
    /// to send itself what the update's download thread finds
    to_self: Sender<ToUi>,
    /// a newer release, and how far its download is
    update: Option<Release>,
    updating: Option<f32>,
    /// the arguments to start the updated overlay with
    args: Vec<String>,
    /// the toolbox: Windows' text reader (once the worker has tried it), the log followed and how it was found,
    /// the last read, whether the tool runs (its Stop), whether a check found no newer release
    ocr: Option<Option<String>>,
    log_file: Option<PathBuf>,
    log_manual: bool,
    last_read: Option<(String, bool, std::time::Instant)>,
    running: bool,
    up_to_date: bool,
    /// the statistics, as last counted from the log (counting reads a few MB: not at every refresh)
    stats: Option<((usize, usize, f64), std::time::Instant)>,
    /// pictures set aside for the report (Ctrl+Alt+B), and the last report's zip
    marked: usize,
    report: Option<PathBuf>,
    /// 囚獸助手: the module, its latest release, the installed version (looked up once per program)
    tart: super::tartarus::Tart,
    tart_latest: Option<String>,
    tart_version: Option<(PathBuf, Option<String>)>,
    /// the game's window was open at the last tick (its opening and closing start and stop the tools)
    game_open: bool,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// Wakes the window's thread from the worker, with its handle.
#[derive(Clone, Copy)]
pub struct Waker(Hwnd);
unsafe impl Send for Waker {}
impl Waker {
    pub fn wake(&self) {
        unsafe {
            let _ = PostMessageW(self.0.get(), WM_WAKE, WPARAM(0), LPARAM(0));
        }
    }
}

pub fn create(fonts: Fonts, rx: Receiver<ToUi>, to_self: Sender<ToUi>, tx: Sender<ToWorker>, settings: Settings, args: Vec<String>) -> windows::core::Result<Waker> {
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None)?.into();
        let class = w!("VoraxRsOverlay");
        let wc = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: instance, lpszClassName: class, hCursor: LoadCursorW(None, IDC_ARROW)?, ..Default::default() };
        RegisterClassW(&wc);
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class,
            w!("Vorax overlay"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            instance,
            None,
        )?;
        for (id, vk) in [(HK_TOGGLE, 'V'), (HK_SNAP, 'X'), (HK_QUIT, 'Q'), (HK_READ, 'D'), (HK_MARK, 'B')] {
            if RegisterHotKey(hwnd, id, HOT_KEY_MODIFIERS(MOD_CONTROL.0 | MOD_ALT.0 | MOD_NOREPEAT.0), vk as u32).is_err() {
                note(&format!("hotkey Ctrl+Alt+{vk} is taken by another program"));
            }
        }
        let mut nid = NOTIFYICONDATAW { cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32, hWnd: hwnd, uID: 1, uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP, uCallbackMessage: WM_TRAY, hIcon: app_icon(instance), ..Default::default() };
        let tip: Vec<u16> = "SmartTorch".encode_utf16().collect();
        nid.szTip[..tip.len()].copy_from_slice(&tip);
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        SetTimer(hwnd, TIMER_FOLLOW, 250, None);
        SetTimer(hwnd, TIMER_TICK, 1000, None);
        super::hub::set_handler(from_hub);
        super::hub::set_close_handler(hub_closed);
        STATE.with(|s| {
            *s.borrow_mut() = Some(State { hwnd, fonts: Rc::new(fonts), rx, tx, plan: None, busy: false, frame: (0, 0), settings, placed: None, visible: false, game_log: None, log_help_shown: false, show_read: true, to_self, update: None, updating: None, args, ocr: None, log_file: None, log_manual: false, last_read: None, running: true, up_to_date: false, stats: None, marked: super::report::marked(), report: None, tart: super::tartarus::Tart::default(), tart_latest: None, tart_version: None, game_open: find_game_window().is_some() });
        });
        // a 囚獸助手 left running hidden by a toolbox before this one (an update's restart): looked after again
        with_state(|s| s.tart.adopt());
        Ok(Waker(Hwnd::of(hwnd)))
    }
}

/// The app's icon (assets/vorax.ico, in the .exe) at the tray's size for this screen; Windows' own without it.
unsafe fn app_icon(instance: HINSTANCE) -> HICON {
    let size = windows::Win32::UI::HiDpi::GetSystemMetricsForDpi(SM_CXSMICON, windows::Win32::UI::HiDpi::GetDpiForSystem());
    match LoadImageW(instance, PCWSTR(1 as *const u16), IMAGE_ICON, size, size, LR_DEFAULTCOLOR) {
        Ok(h) if !h.is_invalid() => HICON(h.0),
        _ => LoadIconW(None, IDI_APPLICATION).unwrap_or_default(),
    }
}

pub fn run_loop() {
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|s| s.borrow_mut().as_mut().map(f))
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_WAKE => {
            with_state(|s| {
                let mut changed = false;
                while let Ok(m) = s.rx.try_recv() {
                    changed = true;
                    match m {
                        ToUi::Show(p, w, h) => {
                            s.plan = Some(p);
                            s.frame = (w, h);
                            s.busy = false;
                        }
                        ToUi::Busy => s.busy = true,
                        ToUi::Unbusy => s.busy = false,
                        ToUi::Clear => {
                            s.plan = None;
                            s.busy = false;
                        }
                        ToUi::Quit => unsafe {
                            let _ = DestroyWindow(s.hwnd);
                            return;
                        },
                        ToUi::GameLog(g) => {
                            s.game_log = Some(g);
                            // the steps by the tray, once a run, the first time the log turns out off
                            if g == GameLog::Off && !std::mem::replace(&mut s.log_help_shown, true) {
                                log_card(s, true);
                            }
                        }
                        ToUi::Update(r) => {
                            s.update = Some(r);
                            s.up_to_date = false;
                            if !super::hub::shown() {
                                update_offer(s);
                            }
                        }
                        ToUi::UpToDate => s.up_to_date = true,
                        ToUi::Ocr(o) => s.ocr = Some(o),
                        ToUi::LogFile(p, manual) => {
                            s.log_file = p;
                            s.log_manual = manual;
                        }
                        ToUi::Marked(r) => {
                            let rows = match &r {
                                Ok(n) => {
                                    s.marked = *n;
                                    vec![Row::Header { title: tr("已記下這個畫面", "Picture set aside"), status: if english() { format!("{n} so far · press Report a problem in the main window later") } else { format!("共 {n} 個 · 之後在主視窗按「回報問題」") } }]
                                }
                                Err(e) => vec![Row::Header { title: tr("沒有可記下的畫面", "No picture to set aside"), status: e.clone() }],
                            };
                            super::menu::notify(s.hwnd, rows, s.fonts.clone());
                            unsafe {
                                SetTimer(s.hwnd, TIMER_CARD, 2500, None);
                            }
                        }
                        ToUi::TartLatest(v) => s.tart_latest = Some(v),
                        ToUi::Read(text, warn) => {
                            // a run may have just ended: count again
                            if text.starts_with("手術方案") || text.starts_with("Protocol") {
                                s.stats = None;
                            }
                            s.last_read = Some((text, warn, std::time::Instant::now()));
                        }
                        ToUi::UpdateProgress(p) => {
                            s.updating = Some(p);
                            super::menu::refresh(download_rows(s));
                        }
                        ToUi::UpdateFailed(e) => {
                            s.updating = None;
                            note(&format!("update failed: {e}"));
                            let mut rows = vec![Row::Header { title: tr("更新失敗", "The update failed"), status: if english() { format!("still v{}", update::current()) } else { format!("繼續使用 v{}", update::current()) } }];
                            rows.push(Row::Lines(wrap(&s.fonts, &e, 6)));
                            rows.push(Row::Buttons { primary: (MENU_LATER, tr("確定", "OK")), secondary: None });
                            if !super::menu::refresh(rows.clone()) {
                                super::menu::notify(s.hwnd, rows, s.fonts.clone());
                            }
                        }
                        ToUi::Restart(exe) => {
                            let mut args: Vec<String> = s.args.iter().filter(|a| *a != "--after-update").cloned().collect();
                            args.push("--after-update".into());
                            if std::process::Command::new(&exe).args(args).spawn().is_ok() {
                                note("restarting into the update");
                                super::menu::close();
                                let _ = s.tx.send(ToWorker::Quit);
                                unsafe {
                                    let _ = DestroyWindow(s.hwnd);
                                }
                                return;
                            }
                            let _ = s.to_self.send(ToUi::UpdateFailed(format!("the update is in place but didn't start: start {} yourself", exe.display())));
                        }
                    }
                }
                if changed {
                    s.placed = None;
                    follow(s);
                }
            });
            push_state();
            LRESULT(0)
        }
        WM_OPEN_HUB => {
            let tab = if wp.0 == 1 { Some("settings:guide") } else { None };
            super::hub::open(tab);
            LRESULT(0)
        }
        WM_TIMER if wp.0 == TIMER_CARD => {
            let _ = KillTimer(hwnd, TIMER_CARD);
            super::menu::close();
            LRESULT(0)
        }
        WM_TIMER if wp.0 == TIMER_TICK => {
            if with_state(tick).unwrap_or(false) {
                push_state();
            }
            LRESULT(0)
        }
        WM_TIMER if wp.0 == TIMER_FOLLOW => {
            with_state(follow);
            LRESULT(0)
        }
        WM_HOTKEY => {
            match wp.0 as i32 {
                HK_TOGGLE => {
                    with_state(toggle);
                }
                HK_SNAP => {
                    with_state(|s| s.tx.send(ToWorker::Snap).ok());
                }
                HK_QUIT => quit(hwnd),
                HK_READ => {
                    with_state(toggle_read);
                }
                HK_MARK => {
                    with_state(|s| s.tx.send(ToWorker::Mark).ok());
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_TRAY => {
            let event = (lp.0 & 0xffff) as u32;
            if event == WM_LBUTTONUP {
                super::hub::open(None);
            } else if event == WM_RBUTTONUP {
                tray_menu(hwnd);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            command(hwnd, (wp.0 & 0xffff) as u32);
            LRESULT(0)
        }
        WM_DESTROY => {
            super::hub::destroy();
            let nid = NOTIFYICONDATAW { cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32, hWnd: hwnd, uID: 1, ..Default::default() };
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

fn toggle(s: &mut State) {
    s.settings.shown = !s.settings.shown;
    s.settings.save();
    s.placed = None;
    follow(s);
}


/// Auto, Traditional, Simplified in turn (not in upstream), kept in the settings.
fn next_script(s: &mut State) {
    crate::zh::set_mode((crate::zh::mode() + 1) % 3);
    s.settings.script = crate::zh::mode();
    s.settings.save();
    s.placed = None;
    follow(s);
}


/// Upstream's mode and the aggressive one in turn (not in upstream), kept in the settings; the board read again so
/// the advice shown is the new mode's.
fn next_mode(s: &mut State) {
    let on = !vorax::bot::mean_goal();
    vorax::bot::MEAN_GOAL.store(on, std::sync::atomic::Ordering::Relaxed);
    s.settings.aggressive = on;
    s.settings.save();
    note(if on { "mode: aggressive (衝分), futures judged by their plain final score" } else { "mode: upstream's (保守), futures counted up to 840k" });
    let _ = s.tx.send(ToWorker::Snap);
}

/// Shows or hides what was read (not in upstream).
fn toggle_read(s: &mut State) {
    s.show_read = !s.show_read;
    s.placed = None;
    follow(s);
}

fn set_shelley(s: &mut State, level: i32) {
    s.settings.shelley = level;
    s.settings.save();
    note(&format!("Intern Shelley level {level}"));
    let _ = s.tx.send(ToWorker::Shelley(level));
}

unsafe fn quit(hwnd: HWND) {
    with_state(|s| {
        // the toolbox closing closes the tools it started
        if s.tart.want {
            s.tart.stop();
        }
        s.tx.send(ToWorker::Quit).ok()
    });
    let _ = DestroyWindow(hwnd);
}


/// Opens the tray menu (menu.rs): the toolbox's tools, the main window, quitting. Everything else is in the
/// main window.
fn tray_menu(hwnd: HWND) {
    let Some((rows, fonts)) = with_state(|s| {
        let status = match &s.update {
            Some(r) => if english() { format!("v{} · v{} is out", update::current(), r.version) } else { format!("v{} · 有新版本：v{}", update::current(), r.version) },
            None => format!("v{}", update::current()),
        };
        let mut rows = vec![Row::Header { title: "SmartTorch".into(), status }];
        rows.push(Row::Text(if english() { format!("Vorax helper: {}", if s.running { "running" } else { "stopped" }) } else { format!("渴癮症助手：{}", if s.running { "執行中" } else { "已停止" }) }));
        if s.game_log == Some(GameLog::Off) {
            rows.push(Row::Warn(tr("遊戲日誌未開啟：不會自動讀取畫面", "The game's log is off: boards aren't read by themselves")));
        }
        rows.push(Row::Separator);
        rows.push(Row::Switch { id: MENU_TOGGLE, label: tr("顯示提示（Ctrl+Alt+V）", "Show hints (Ctrl+Alt+V)"), on: s.settings.shown });
        rows.push(Row::Action { id: MENU_SNAP, label: tr("立即讀取畫面", "Read the board now"), shortcut: "Ctrl+Alt+X".into() });
        rows.push(Row::Separator);
        rows.push(Row::Action { id: MENU_HUB, label: tr("開啟主視窗", "Open the main window"), shortcut: String::new() });
        rows.push(Row::Action { id: MENU_QUIT, label: tr("結束", "Quit"), shortcut: "Ctrl+Alt+Q".into() });
        (rows, s.fonts.clone())
    }) else {
        return;
    };
    super::menu::open(hwnd, rows, fonts);
}

/// What to do when the game's log has no moves in it: the in-game switch, step by step. `by_tray`: shown by
/// itself, in the corner by the tray, without taking the game's focus.
fn log_card(s: &State, by_tray: bool) {
    let rows = if english() {
        let (title, status, body) = if s.game_log == Some(GameLog::Missing) {
            ("Game log not found", "boards aren't read by themselves", "Start the game first: SmartTorch finds UE_game.log from where the game is installed. Make sure the game's log is on (Esc → Account Related → Enable Log, then log in again). Still not found: in the main window's Vorax page, press Choose… and pick UE_game.log. Meanwhile Ctrl+Alt+X reads the board.")
        } else {
            ("Turn on the game's log", "boards aren't read by themselves until then", "In the game press Esc, open Account Related, click Enable Log, then log out and in again. Meanwhile Ctrl+Alt+X reads the board.")
        };
        let mut rows = vec![Row::Header { title: title.into(), status: status.into() }];
        rows.push(Row::Lines(wrap(&s.fonts, body, 9)));
        rows.push(Row::Buttons { primary: (MENU_LATER, "OK".into()), secondary: None });
        rows
    } else if s.game_log == Some(GameLog::Missing) {
        vec![
            Row::Header { title: "找不到遊戲日誌".into(), status: "不會自動讀取畫面".into() },
            Row::Text("請先開啟遊戲：開啟後工具會從遊戲的".into()),
            Row::Text("安裝位置自動找到 UE_game.log。".into()),
            Row::Text("並確認遊戲中已開啟日誌（Esc →".into()),
            Row::Text("「帳號相關」→「開啟日誌」，再重新登入）。".into()),
            Row::Text("仍找不到時，在主視窗「渴癮症助手」頁".into()),
            Row::Text("按「選擇…」選擇 UE_game.log。".into()),
            Row::Text("在此之前，可按 Ctrl+Alt+X 讀取畫面。".into()),
            Row::Buttons { primary: (MENU_LATER, "確定".into()), secondary: None },
        ]
    } else {
        vec![
            Row::Header { title: "開啟遊戲日誌".into(), status: "開啟前不會自動讀取畫面".into() },
            Row::Text("1.  在遊戲中按 Esc".into()),
            Row::Text("2.  開啟「帳號相關」".into()),
            Row::Text("3.  點「開啟日誌」".into()),
            Row::Text("4.  登出後再登入".into()),
            Row::Text("在此之前，可按 Ctrl+Alt+X 讀取畫面。".into()),
            Row::Buttons { primary: (MENU_LATER, "確定".into()), secondary: None },
        ]
    };
    if by_tray {
        super::menu::notify(s.hwnd, rows, s.fonts.clone());
    } else {
        super::menu::open(s.hwnd, rows, s.fonts.clone());
    }
}

/// A newer release, offered once by the tray without taking the game's focus.
fn update_offer(s: &State) {
    let Some(r) = &s.update else { return };
    note(&format!("update available: v{} (this is v{})", r.version, update::current()));
    let rows = vec![
        Row::Header { title: tr("SmartTorch 有新版本", "A new SmartTorch is out"), status: if english() { format!("v{} is out · this is v{}", r.version, update::current()) } else { format!("v{} 已推出 · 你目前是 v{}", r.version, update::current()) } },
        Row::Buttons { primary: (MENU_UPDATE, tr("更新內容", "What's new")), secondary: Some((MENU_LATER, tr("稍後", "Later"))) },
    ];
    super::menu::notify(s.hwnd, rows, s.fonts.clone());
}

/// The release's notes, with the choice to install it now.
fn whats_new(s: &State) {
    let Some(r) = &s.update else { return };
    if s.updating.is_some() {
        return super::menu::open(s.hwnd, download_rows(s), s.fonts.clone());
    }
    let rows = vec![
        Row::Header { title: if english() { format!("What's new in v{}", r.version) } else { format!("v{} 更新內容", r.version) }, status: if english() { format!("this is v{}", update::current()) } else { format!("你目前是 v{}", update::current()) } },
        Row::Lines(wrap(&s.fonts, &r.notes, 18)),
        Row::Action { id: MENU_RELEASE_PAGE, label: tr("GitHub 上的版本頁面", "Its page on GitHub"), shortcut: tr("開啟", "Open") },
        Row::Buttons { primary: (MENU_UPDATE_NOW, tr("立即更新", "Update now")), secondary: Some((MENU_LATER, tr("稍後", "Later"))) },
    ];
    super::menu::open(s.hwnd, rows, s.fonts.clone());
}

/// The update's card while it downloads.
fn download_rows(s: &State) -> Vec<Row> {
    let (v, p) = (s.update.as_ref().map_or(String::new(), |r| r.version.clone()), s.updating.unwrap_or(0.0));
    vec![
        Row::Header { title: if english() { format!("Updating to v{v}") } else { format!("正在更新到 v{v}") }, status: if english() { format!("from github.com/{}", update::REPO) } else { format!("從 github.com/{} 下載", update::REPO) } },
        Row::Progress(p),
        Row::Text(if english() { format!("{:.0}% · checked, then restarted", p * 100.0) } else { format!("{:.0}% · 下載後會檢查檔案並重新啟動", p * 100.0) }),
    ]
}

/// Downloads and installs the update on its own thread; its progress and outcome come back as ToUi.
fn start_update(s: &mut State) {
    let Some(r) = s.update.clone() else { return };
    if s.updating.is_some() {
        if !super::hub::shown() {
            super::menu::open(s.hwnd, download_rows(s), s.fonts.clone());
        }
        return;
    }
    s.updating = Some(0.0);
    note(&format!("downloading v{} from {}", r.version, r.page));
    let (tx, waker) = (s.to_self.clone(), Waker(Hwnd::of(s.hwnd)));
    std::thread::spawn(move || {
        unsafe {
            let _ = windows::Win32::System::WinRT::RoInitialize(windows::Win32::System::WinRT::RO_INIT_MULTITHREADED);
        }
        let progress_tx = std::sync::Mutex::new(tx.clone());
        let last = std::sync::atomic::AtomicU32::new(0);
        let out = update::install(&r, move |p| {
            // a message per percent, not per packet
            let pct = (p * 100.0) as u32;
            if last.swap(pct, std::sync::atomic::Ordering::Relaxed) != pct {
                if let Ok(t) = progress_tx.lock() {
                    let _ = t.send(ToUi::UpdateProgress(p));
                }
                waker.wake();
            }
        });
        let _ = tx.send(match out {
            Ok(exe) => ToUi::Restart(exe),
            Err(e) => ToUi::UpdateFailed(e),
        });
        waker.wake();
    });
    if !super::hub::shown() {
        super::menu::open(s.hwnd, download_rows(s), s.fonts.clone());
    }
}

/// A choice from the tray menu.
unsafe fn command(hwnd: HWND, id: u32) {
    match id {
        MENU_TOGGLE => {
            with_state(toggle);
        }
        MENU_READ => {
            with_state(toggle_read);
        }
        MENU_SCRIPT => {
            with_state(next_script);
        }
        MENU_MODE => {
            with_state(next_mode);
        }
        MENU_SNAP => {
            with_state(|s| s.tx.send(ToWorker::Snap).ok());
        }
        MENU_LATER => {}
        MENU_LOG_HELP => {
            with_state(|s| log_card(s, false));
        }
        MENU_LOG => {
            with_state(|s| super::logview::open(s.fonts.clone()));
        }
        MENU_UPDATE => {
            with_state(|s| whats_new(s));
        }
        MENU_UPDATE_NOW => {
            with_state(start_update);
        }
        MENU_RELEASE_PAGE => {
            with_state(|s| {
                if let Some(r) = &s.update {
                    let _ = std::process::Command::new("explorer.exe").arg(&r.page).spawn();
                }
            });
        }
        MENU_QUIT => quit(hwnd),
        MENU_HUB => super::hub::open(None),
        c if (MENU_SHELLEY + 1..=MENU_SHELLEY + 6).contains(&c) => {
            with_state(|s| set_shelley(s, (c - MENU_SHELLEY) as i32));
        }
        c if (MENU_ACCENT..MENU_ACCENT + crate::draw::ACCENTS.len() as u32).contains(&c) => {
            with_state(|s| {
                s.settings.accent = (c - MENU_ACCENT) as usize;
                s.settings.save();
                s.placed = None;
                follow(s);
            });
        }
        _ => {}
    }
}

/// Keeps the overlay over the game's picture, shown only while the game is in front and there's advice.
fn follow(s: &mut State) {
    let game = find_game_window();
    let front = game.is_some_and(|g| foreground() == g);
    let rect = game.and_then(client_rect);
    let want = s.running && s.settings.shown && front && s.plan.is_some() && rect.is_some();
    unsafe {
        if !want {
            if s.visible {
                let _ = ShowWindow(s.hwnd, SW_HIDE);
                s.visible = false;
            }
            return;
        }
        let r = rect.unwrap();
        if s.placed != Some(r) {
            draw(s, r);
            s.placed = Some(r);
        }
        if !s.visible {
            let _ = ShowWindow(s.hwnd, SW_SHOWNOACTIVATE);
            s.visible = true;
        }
        let _ = SetWindowPos(s.hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
}

/// Lays the marks out on the game's picture as it is now (scaled from the picture they were read on) and puts
/// them on screen.
fn draw(s: &State, (x, y, w, h): (i32, i32, i32, i32)) {
    let Some(plan) = &s.plan else { return };
    let (fw, fh) = s.frame;
    let (sx, sy) = if fw > 0 && fh > 0 { (w as f64 / fw as f64, h as f64 / fh as f64) } else { (1.0, 1.0) };
    let mut p = if s.busy { plan.dimmed() } else { plan.clone() };
    if !s.show_read {
        p.read.clear();
        p.read_tanks.clear();
    }
    for m in p.read_tanks.iter_mut() {
        m.under.x *= sx;
        m.under.y *= sy;
        m.under.w *= sx;
        m.under.h *= sy;
    }
    if let Some((c, cw, ch)) = &mut p.card {
        c.cx *= sx;
        c.cy *= sy;
        *cw *= sx;
        *ch *= sy;
    }
    if let Some(b) = &mut p.button {
        b.x *= sx;
        b.y *= sy;
        b.w *= sx;
        b.h *= sy;
    }
    for t in p.tanks.iter_mut() {
        t.x *= sx;
        t.y *= sy;
        t.w *= sx;
        t.h *= sy;
    }
    let pm = render(&p, w as u32, h as u32, &s.fonts, crate::draw::accent(s.settings.accent));
    unsafe { put(s.hwnd, x, y, w, h, pm.data()) };
}

/// The toolbox's main window: everything it shows, as JSON for the page (ui/index.html).
fn state_json(s: &mut State) -> String {
    let game = find_game_window();
    let size = game.and_then(client_rect).map(|(_, _, w, h)| (w, h));
    let log = match s.game_log {
        Some(GameLog::On) => "on",
        Some(GameLog::Off) => "off",
        Some(GameLog::Missing) => "missing",
        Some(GameLog::Checking) => "checking",
        Some(GameLog::NoGame) | None => "nogame",
    };
    let lang = if english() {
        "en"
    } else {
        match crate::zh::mode() {
            crate::zh::SIMPLIFIED => "sc",
            crate::zh::TRADITIONAL => "tc",
            _ if super::sys::windows_simplified() => "sc",
            _ => "tc",
        }
    };
    let stats = match s.stats {
        Some((st, at)) if at.elapsed() < std::time::Duration::from_secs(30) => st,
        _ => {
            let st = super::logview::stats(vorax::bot::mean_goal(), s.settings.stats_since);
            s.stats = Some((st, std::time::Instant::now()));
            st
        }
    };
    serde_json::json!({
        "type": "state",
        "version": update::current(),
        "lang": lang,
        "update": s.update.as_ref().map(|r| serde_json::json!({ "version": r.version, "notes": r.notes, "page": r.page })),
        "updating": s.updating,
        "upToDate": s.up_to_date,
        "boot": super::sys::starts_with_windows(),
        "autoGame": s.settings.auto_game,
        "firstRunDone": s.settings.first_run_done,
        "vorax": {
            "running": s.running,
            "mode": if vorax::bot::mean_goal() { "mean" } else { "safe" },
            "shelley": s.settings.shelley,
            "rerolls": crate::pipeline::shelley_rerolls(s.settings.shelley),
            "shown": s.settings.shown,
            "showRead": s.show_read,
            "game": { "open": game.is_some(), "w": size.map(|x| x.0), "h": size.map(|x| x.1), "fullScreen": game.is_some() && super::sys::exclusive_full_screen() },
            "log": { "state": log, "path": s.log_file.as_ref().map(|p| p.display().to_string()), "manual": s.log_manual },
            "ocr": match &s.ocr { None => serde_json::Value::Null, Some(None) => serde_json::Value::Bool(false), Some(Some(l)) => serde_json::Value::String(l.clone()) },
            "last": s.last_read.as_ref().map(|(t, warn, at)| serde_json::json!({ "text": t, "warn": warn, "ago": at.elapsed().as_secs() })),
            "stats": { "games": stats.0, "rainbow": stats.1, "mean": stats.2 },
            "marked": s.marked,
            "report": s.report.as_ref().map(|p| p.display().to_string()),
        },
        "tartarus": tartarus_json(s),
    })
    .to_string()
}

/// Sends the main window the state as it is now (when it's open).
fn push_state() {
    if !super::hub::shown() {
        return;
    }
    if let Some(json) = with_state(|s| state_json(s)) {
        super::hub::post(&json);
    }
}

/// The main window's ✕: it's only hidden; the first time, a note by the tray says the tools are still running.
fn hub_closed() {
    with_state(|s| {
        if !s.settings.close_hint_shown {
            s.settings.close_hint_shown = true;
            s.settings.save();
            let rows = vec![
                Row::Header { title: tr("SmartTorch 仍在執行", "SmartTorch is still running"), status: tr("已縮到右下角", "in the tray, by the clock") },
                Row::Text(tr("點右下角的圖示可再次開啟主視窗。", "Click its icon to open the main window again.")),
                Row::Text(tr("要完全關閉：在圖示按右鍵 →「結束」。", "To close it: right-click the icon → Quit.")),
                Row::Buttons { primary: (MENU_LATER, tr("確定", "OK")), secondary: None },
            ];
            super::menu::notify(s.hwnd, rows, s.fonts.clone());
        }
    });
}

/// What the main window asks for ({"cmd": ...}); every command is answered with the state.
fn from_hub(text: &str) {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(text) else { return };
    let cmd = v["cmd"].as_str().unwrap_or("");
    match cmd {
        "ready" => {
            with_state(|s| {
                if s.tart_latest.is_none() {
                    check_tartarus(s);
                }
            });
        }
        "refresh" => {}
        "set" => {
            let value = v["value"].clone();
            with_state(|s| match v["key"].as_str().unwrap_or("") {
                "mode" => {
                    s.stats = None;
                    let want = value.as_str() == Some("mean");
                    if want != vorax::bot::mean_goal() {
                        next_mode(s);
                    }
                }
                "shelley" => {
                    if let Some(l) = value.as_i64() {
                        set_shelley(s, (l as i32).clamp(1, 6));
                    }
                }
                "shown" => {
                    if value.as_bool() != Some(s.settings.shown) {
                        toggle(s);
                    }
                }
                "showRead" => {
                    if value.as_bool() != Some(s.show_read) {
                        toggle_read(s);
                    }
                }
                "lang" => {
                    let lang = value.as_str().unwrap_or("tc");
                    crate::zh::set_english(lang == "en");
                    if lang != "en" {
                        crate::zh::set_mode(if lang == "sc" { crate::zh::SIMPLIFIED } else { crate::zh::TRADITIONAL });
                        s.settings.script = crate::zh::mode();
                    }
                    s.settings.lang = lang.to_string();
                    s.settings.save();
                    s.stats = None;
                    // the hint shown again in the new language
                    let _ = s.tx.send(ToWorker::Shelley(s.settings.shelley));
                    s.placed = None;
                    follow(s);
                }
                "boot" => super::sys::set_start_with_windows(value.as_bool().unwrap_or(false)),
                "autoGame" => {
                    s.settings.auto_game = value.as_bool().unwrap_or(true);
                    s.settings.save();
                }
                _ => {}
            });
        }
        "vorax" => {
            let on = v["on"].as_bool().unwrap_or(true);
            with_state(|s| {
                s.running = on;
                let _ = s.tx.send(ToWorker::Pause(!on));
                s.placed = None;
                follow(s);
            });
        }
        "snap" => {
            with_state(|s| s.tx.send(ToWorker::Snap).ok());
        }
        "chooseLog" => {
            // the dialog runs its own message loop: never inside the state's borrow
            if let Some(p) = super::sys::choose_log(super::hub::hwnd()) {
                with_state(|s| {
                    note(&format!("the game's log chosen by hand: {}", p.display()));
                    s.settings.log_path = Some(p.clone());
                    s.settings.save();
                    s.game_log = None;
                    let _ = s.tx.send(ToWorker::LogPath(Some(p)));
                });
            }
        }
        "autoLog" => {
            with_state(|s| {
                s.settings.log_path = None;
                s.settings.save();
                s.game_log = None;
                let _ = s.tx.send(ToWorker::LogPath(None));
            });
        }
        "runs" => {
            with_state(|s| super::logview::open(s.fonts.clone()));
        }
        "clearRecords" => {
            super::report::clear_all();
            with_state(|s| {
                s.stats = None;
                s.settings.stats_since = 0.0;
                s.settings.save();
                s.marked = 0;
                s.report = None;
                s.last_read = None;
                // the new log starts with the mode, so the runs from here count under it
                note(&format!("records cleared from the toolbox (v{})", update::current()));
                note(if vorax::bot::mean_goal() { "mode: aggressive (衝分), futures judged by their plain final score" } else { "mode: upstream's (保守), futures counted up to 840k" });
            });
        }
        "checkUpdate" => {
            let Some((tx, waker)) = with_state(|s| (s.to_self.clone(), Waker(Hwnd::of(s.hwnd)))) else { return };
            std::thread::spawn(move || {
                unsafe {
                    let _ = windows::Win32::System::WinRT::RoInitialize(windows::Win32::System::WinRT::RO_INIT_MULTITHREADED);
                }
                let _ = tx.send(match update::check() {
                    Ok(Some(r)) => ToUi::Update(r),
                    _ => ToUi::UpToDate,
                });
                waker.wake();
            });
        }
        "update" => {
            with_state(start_update);
        }
        "tartarus" => {
            let on = v["on"].as_bool().unwrap_or(true);
            with_state(|s| {
                if on {
                    if let Some(exe) = tart_exe(s) {
                        s.tart.start(&exe);
                    }
                } else {
                    s.tart.stop();
                }
            });
        }
        "tartarusShow" => {
            let on = v["on"].as_bool().unwrap_or(true);
            with_state(|s| s.tart.show(on));
        }
        "tartarusDownload" => super::hub::open_url(super::tartarus::PAGE),
        "tartarusReset" => {
            with_state(|s| {
                s.settings.tartarus_base = super::tartarus::stats();
                s.settings.save();
                note("Tartarus Solver's statistics reset in the toolbox");
            });
        }
        "tartarusChoose" => {
            if let Some(p) = super::sys::choose_tartarus(super::hub::hwnd()) {
                with_state(|s| {
                    note(&format!("Tartarus Solver chosen: {}", p.display()));
                    s.settings.tartarus_exe = Some(p);
                    s.settings.save();
                    s.tart_version = None;
                });
            }
        }
        "tartarusCheck" => {
            with_state(|s| check_tartarus(s));
        }
        "report" => {
            let version = update::current();
            match super::report::pack(&version) {
                Ok(zip) => {
                    with_state(|s| {
                        s.marked = 0;
                        s.report = Some(zip.clone());
                    });
                    // the form first, then the zip to drag onto it
                    super::hub::open_url("https://github.com/lauhinsnsd-dotcom/vorax-solver-rs-zh/issues/new?template=bug.yml");
                    super::report::show_in_folder(&zip);
                }
                Err(e) => note(&format!("report not packed: {e}")),
            }
        }
        "markedOpen" => super::report::open_marked(),
        "markedClear" => {
            super::report::clear();
            with_state(|s| {
                s.marked = 0;
                s.report = None;
            });
        }
        "firstRunDone" => {
            with_state(|s| {
                s.settings.first_run_done = true;
                s.settings.save();
            });
        }
        "open" => super::hub::open_url(v["url"].as_str().unwrap_or("")),
        _ => note(&format!("main window: unknown command {cmd}")),
    }
    push_state();
}

/// Opens the main window from anywhere (posted, so it's never made inside the state's borrow); `guide`: on the
/// first-use steps.
pub fn open_hub_later(guide: bool) {
    with_state(|s| unsafe {
        let _ = PostMessageW(s.hwnd, WM_OPEN_HUB, WPARAM(guide as usize), LPARAM(0));
    });
}

/// 囚獸助手's program, when installed.
fn tart_exe(s: &State) -> Option<PathBuf> {
    super::tartarus::installed(s.settings.tartarus_exe.as_deref())
}

/// Once a second: 囚獸助手 looked after, and the tools started as the game opens and stopped as it closes (when
/// that's on). Whether the main window should be told.
fn tick(s: &mut State) -> bool {
    // found as it runs, when not chosen yet (or moved): remembered
    if tart_exe(s).is_none() {
        if let Some(found) = super::tartarus::running_exe() {
            note(&format!("Tartarus Solver found running: {}", found.display()));
            s.settings.tartarus_exe = Some(found);
            s.settings.save();
            s.tart_version = None;
        }
    }
    let exe = tart_exe(s);
    let mut changed = s.tart.tick(exe.as_deref());
    let open = find_game_window().is_some();
    if open != s.game_open {
        s.game_open = open;
        changed = true;
        if s.settings.auto_game {
            note(if open { "the game opened: starting the tools" } else { "the game closed: stopping the tools" });
            if open {
                if !s.running {
                    s.running = true;
                    let _ = s.tx.send(ToWorker::Pause(false));
                }
                if let Some(exe) = &exe {
                    s.tart.start(exe);
                }
            } else {
                if s.running {
                    s.running = false;
                    let _ = s.tx.send(ToWorker::Pause(true));
                    s.placed = None;
                    follow(s);
                }
                if s.tart.want {
                    s.tart.stop();
                }
            }
        }
    }
    changed
}

/// Looks up 囚獸助手's latest release on its own thread.
fn check_tartarus(s: &State) {
    let (tx, waker) = (s.to_self.clone(), Waker(Hwnd::of(s.hwnd)));
    std::thread::spawn(move || {
        unsafe {
            let _ = windows::Win32::System::WinRT::RoInitialize(windows::Win32::System::WinRT::RO_INIT_MULTITHREADED);
        }
        match super::tartarus::latest() {
            Ok(v) => {
                let _ = tx.send(ToUi::TartLatest(v));
                waker.wake();
            }
            Err(e) => note(&format!("Tartarus Solver's latest release: {e}")),
        }
    });
}


/// 囚獸助手 as the main window shows it.
fn tartarus_json(s: &mut State) -> serde_json::Value {
    let exe = tart_exe(s);
    if let Some(e) = &exe {
        if s.tart_version.as_ref().is_none_or(|(p, _)| p != e) {
            s.tart_version = Some((e.clone(), super::tartarus::version(e)));
        }
    }
    let version = s.tart_version.as_ref().and_then(|(_, v)| v.clone());
    // counted from the toolbox's reset (a reset in its own window, below that, starts over)
    let stats = super::tartarus::stats().map(|(r, c)| match s.settings.tartarus_base {
        Some((br, bc)) if r >= br && c >= bc => (r - br, c - bc),
        _ => (r, c),
    });
    let newer = match (&s.tart_latest, &version) {
        (Some(l), Some(v)) => super::tartarus::newer(l, v),
        _ => false,
    };
    serde_json::json!({
        "installed": exe.is_some(),
        "exe": exe.as_ref().map(|p| p.display().to_string()),
        "version": version,
        "latest": s.tart_latest,
        "newer": newer,
        "running": s.tart.running,
        "want": s.tart.want,
        "shown": s.tart.shown,
        "error": s.tart.error,
        "stats": stats.map(|(r, c)| serde_json::json!({ "runs": r, "chromatic": c })),
    })
}
