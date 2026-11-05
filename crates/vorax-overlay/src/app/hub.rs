//! The toolbox's main window (火炬工具箱): an ordinary window holding a WebView2 that shows ui/index.html. The page
//! and the overlay talk in JSON: the page posts commands ({"cmd": ...}), window.rs answers each with the whole
//! state (push), which the page draws. Closing the window only hides it; the tray brings it back.
//!
//! WebView2 runs everything on this (the window's) thread, its callbacks arriving through the message loop, so
//! nothing here waits: the window shows at once and the page fills in when WebView2 is ready.

use super::sys::{note, Hwnd};
use std::cell::RefCell;
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::*;
use windows::core::{w, Interface, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::WinRT::EventRegistrationToken;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::*;

/// The page, with the Traditional-to-Simplified table it converts itself with put in.
const PAGE: &str = include_str!("../../ui/index.html");
const T2S: &str = include_str!("../../ui/t2s.json");

struct Hub {
    hwnd: HWND,
    controller: Option<ICoreWebView2Controller>,
    webview: Option<ICoreWebView2>,
    /// the page has loaded and asked for the state
    ready: bool,
    /// a tab to open once the page is ready ("guide" on the first run)
    tab: Option<String>,
}

thread_local! {
    static HUB: RefCell<Option<Hub>> = const { RefCell::new(None) };
}

/// What the page asks for, handed to window.rs (set once at start).
type OnMessage = fn(&str);
thread_local! {
    static ON_MESSAGE: RefCell<Option<OnMessage>> = const { RefCell::new(None) };
}

pub fn set_handler(f: OnMessage) {
    ON_MESSAGE.with(|h| *h.borrow_mut() = Some(f));
}

fn page() -> String {
    PAGE.replace("/*T2S*/{}", T2S)
}

/// Shows the main window (made the first time), in front; `tab`: a page to open ("vorax", "settings:guide", ...).
pub fn open(tab: Option<&str>) {
    let existing = HUB.with(|h| h.borrow().as_ref().map(|x| x.hwnd));
    let hwnd = match existing {
        Some(h) => h,
        None => match create() {
            Ok(h) => h,
            Err(e) => {
                note(&format!("main window: {e}"));
                return;
            }
        },
    };
    if let Some(t) = tab {
        let ready = HUB.with(|h| {
            let mut h = h.borrow_mut();
            let x = h.as_mut().unwrap();
            if !x.ready {
                x.tab = Some(t.to_string());
            }
            x.ready
        });
        if ready {
            post(&serde_json::json!({ "type": "tab", "tab": t }).to_string());
        }
    }
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        } else {
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
        let _ = SetForegroundWindow(hwnd);
    }
}

/// Whether the main window is on screen.
pub fn shown() -> bool {
    HUB.with(|h| h.borrow().as_ref().is_some_and(|x| unsafe { IsWindowVisible(x.hwnd).as_bool() }))
}

/// Sends the page a message (JSON), once it's ready; dropped before (the page asks for the state when it is).
pub fn post(json: &str) {
    let webview = HUB.with(|h| h.borrow().as_ref().filter(|x| x.ready).and_then(|x| x.webview.clone()));
    if let Some(wv) = webview {
        unsafe {
            let _ = wv.PostWebMessageAsJson(&HSTRING::from(json));
        }
    }
}

/// Closes the window for good (the toolbox quitting).
pub fn destroy() {
    let hub = HUB.with(|h| h.borrow_mut().take());
    if let Some(x) = hub {
        unsafe {
            if let Some(c) = &x.controller {
                let _ = c.Close();
            }
            let _ = DestroyWindow(x.hwnd);
        }
    }
}

/// The main window's handle, for dialogs to sit on.
pub fn hwnd() -> Option<Hwnd> {
    HUB.with(|h| h.borrow().as_ref().map(|x| Hwnd::of(x.hwnd)))
}

fn create() -> windows::core::Result<HWND> {
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None)?.into();
        let class = w!("FireToolboxMain");
        let icon = LoadImageW(instance, PCWSTR(1 as *const u16), IMAGE_ICON, 0, 0, LR_DEFAULTSIZE).map(|h| HICON(h.0)).unwrap_or_default();
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: instance,
            lpszClassName: class,
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hIcon: icon,
            hbrBackground: windows::Win32::Graphics::Gdi::CreateSolidBrush(COLORREF(0x001b1614)),
            ..Default::default()
        };
        RegisterClassW(&wc);
        // a size for 96 dpi, scaled to the screen it opens on
        let dpi = windows::Win32::UI::HiDpi::GetDpiForSystem().max(96) as i32;
        let (w, h) = (1100 * dpi / 96, 780 * dpi / 96);
        let sw = GetSystemMetrics(SM_CXSCREEN);
        let sh = GetSystemMetrics(SM_CYSCREEN);
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class,
            w!("火炬工具箱"),
            WS_OVERLAPPEDWINDOW,
            ((sw - w) / 2).max(0),
            ((sh - h) / 2).max(0),
            w.min(sw),
            h.min(sh - 40 * dpi / 96),
            None,
            None,
            instance,
            None,
        )?;
        dark_title_bar(hwnd);
        HUB.with(|x| *x.borrow_mut() = Some(Hub { hwnd, controller: None, webview: None, ready: false, tab: None }));
        start_webview(hwnd);
        Ok(hwnd)
    }
}

/// A dark title bar to match the page (Windows 10 2004 and later; older ones keep theirs).
unsafe fn dark_title_bar(hwnd: HWND) {
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};
    let on: BOOL = true.into();
    let _ = DwmSetWindowAttribute(hwnd, DWMWINDOWATTRIBUTE(20), &on as *const _ as *const _, std::mem::size_of::<BOOL>() as u32);
}

/// Where WebView2 keeps its files: beside the settings, not next to the exe (which may be read-only).
fn user_data() -> HSTRING {
    HSTRING::from(super::sys::data_dir().join("WebView2").to_string_lossy().as_ref())
}

unsafe fn start_webview(hwnd: HWND) {
    let folder = user_data();
    let handler = CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(move |result, env| {
        if let Err(e) = result {
            no_webview(hwnd, &format!("{e}"));
            return Ok(());
        }
        let Some(env) = env else { return Ok(()) };
        let done = CreateCoreWebView2ControllerCompletedHandler::create(Box::new(move |result, controller| {
            match (result, controller) {
                (Ok(()), Some(c)) => unsafe { ready_controller(hwnd, c) },
                (r, _) => no_webview(hwnd, &format!("{r:?}")),
            }
            Ok(())
        }));
        unsafe { env.CreateCoreWebView2Controller(hwnd, &done)? };
        Ok(())
    }));
    if let Err(e) = CreateCoreWebView2EnvironmentWithOptions(PCWSTR::null(), &folder, None::<&ICoreWebView2EnvironmentOptions>, &handler) {
        no_webview(hwnd, &format!("{e}"));
    }
}

/// WebView2 isn't there (Windows 10 without Edge's runtime): say so in the window, with where to get it.
fn no_webview(hwnd: HWND, why: &str) {
    note(&format!("WebView2 not available: {why}"));
    unsafe {
        let text = HSTRING::from("火炬工具箱需要 Microsoft Edge WebView2 Runtime 才能顯示主視窗。\n\n按「確定」開啟下載頁面，安裝後重新開啟工具箱。\n（提示框與快捷鍵照常可用。）");
        let r = MessageBoxW(hwnd, &text, w!("火炬工具箱"), MB_OKCANCEL | MB_ICONINFORMATION);
        if r == IDOK {
            let _ = std::process::Command::new("explorer.exe").arg("https://developer.microsoft.com/microsoft-edge/webview2/").spawn();
        }
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
}

unsafe fn ready_controller(hwnd: HWND, controller: ICoreWebView2Controller) {
    let Ok(webview) = controller.CoreWebView2() else { return };
    // the page's own dark colour while it loads, not a white flash
    if let Ok(c2) = controller.cast::<ICoreWebView2Controller2>() {
        let _ = c2.SetDefaultBackgroundColor(COREWEBVIEW2_COLOR { A: 255, R: 20, G: 22, B: 27 });
    }
    if let Ok(settings) = webview.Settings() {
        let _ = settings.SetIsStatusBarEnabled(false);
        let _ = settings.SetAreDefaultContextMenusEnabled(cfg!(debug_assertions));
        let _ = settings.SetAreDevToolsEnabled(cfg!(debug_assertions));
        let _ = settings.SetIsZoomControlEnabled(false);
    }
    let mut token = EventRegistrationToken::default();
    // the page's commands
    let _ = webview.add_WebMessageReceived(
        &WebMessageReceivedEventHandler::create(Box::new(|_wv, args| {
            if let Some(args) = args {
                let mut msg = PWSTR::null();
                if unsafe { args.WebMessageAsJson(&mut msg) }.is_ok() {
                    let text = take_pwstr(msg);
                    if text.contains("\"cmd\":\"ready\"") {
                        let tab = HUB.with(|h| {
                            let mut h = h.borrow_mut();
                            let x = h.as_mut()?;
                            x.ready = true;
                            x.tab.take()
                        });
                        if let Some(t) = tab {
                            post(&serde_json::json!({ "type": "tab", "tab": t }).to_string());
                        }
                    }
                    if let Some(f) = ON_MESSAGE.with(|h| *h.borrow()) {
                        f(&text);
                    }
                }
            }
            Ok(())
        })),
        &mut token,
    );
    // links open in the browser, never inside the toolbox
    let _ = webview.add_NewWindowRequested(
        &NewWindowRequestedEventHandler::create(Box::new(|_wv, args| {
            if let Some(args) = args {
                let mut uri = PWSTR::null();
                if unsafe { args.Uri(&mut uri) }.is_ok() {
                    open_url(&take_pwstr(uri));
                }
                let _ = unsafe { args.SetHandled(true) };
            }
            Ok(())
        })),
        &mut token,
    );
    let _ = webview.add_NavigationStarting(
        &NavigationStartingEventHandler::create(Box::new(|_wv, args| {
            if let Some(args) = args {
                let mut uri = PWSTR::null();
                if unsafe { args.Uri(&mut uri) }.is_ok() {
                    let u = take_pwstr(uri);
                    if u.starts_with("http://") || u.starts_with("https://") {
                        let _ = unsafe { args.SetCancel(true) };
                        open_url(&u);
                    }
                }
            }
            Ok(())
        })),
        &mut token,
    );
    let _ = controller.SetIsVisible(true);
    HUB.with(|h| {
        if let Some(x) = h.borrow_mut().as_mut() {
            x.controller = Some(controller.clone());
            x.webview = Some(webview.clone());
        }
    });
    fit(hwnd);
    let _ = webview.NavigateToString(&HSTRING::from(page()));
}

/// Opens a web page in the default browser (only http and https).
pub fn open_url(url: &str) {
    if url.starts_with("https://") || url.starts_with("http://") {
        let _ = std::process::Command::new("explorer.exe").arg(url).spawn();
    }
}

/// The page fills the window's client area.
fn fit(hwnd: HWND) {
    let controller = HUB.with(|h| h.borrow().as_ref().and_then(|x| x.controller.clone()));
    if let Some(c) = controller {
        unsafe {
            let mut r = RECT::default();
            let _ = GetClientRect(hwnd, &mut r);
            let _ = c.SetBounds(r);
        }
    }
}

/// What to do when the window's ✕ is pressed: window.rs shows the "still running" note the first time.
type OnClose = fn();
thread_local! {
    static ON_CLOSE: RefCell<Option<OnClose>> = const { RefCell::new(None) };
}

pub fn set_close_handler(f: OnClose) {
    ON_CLOSE.with(|h| *h.borrow_mut() = Some(f));
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_SIZE => {
            fit(hwnd);
            LRESULT(0)
        }
        WM_MOVE | WM_MOVING => {
            let controller = HUB.with(|h| h.borrow().as_ref().and_then(|x| x.controller.clone()));
            if let Some(c) = controller {
                let _ = c.NotifyParentWindowPositionChanged();
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_DPICHANGED => {
            // the size Windows suggests for the new screen's scaling
            let r = &*(lp.0 as *const RECT);
            let _ = SetWindowPos(hwnd, HWND::default(), r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER | SWP_NOACTIVATE);
            LRESULT(0)
        }
        WM_GETMINMAXINFO => {
            let dpi = GetDpiForWindow(hwnd).max(96) as i32;
            let info = &mut *(lp.0 as *mut MINMAXINFO);
            info.ptMinTrackSize = POINT { x: 720 * dpi / 96, y: 520 * dpi / 96 };
            LRESULT(0)
        }
        WM_SETFOCUS => {
            let controller = HUB.with(|h| h.borrow().as_ref().and_then(|x| x.controller.clone()));
            if let Some(c) = controller {
                let _ = c.MoveFocus(COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC);
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            // only hidden: the tools go on running, the tray brings the window back
            let _ = ShowWindow(hwnd, SW_HIDE);
            if let Some(f) = ON_CLOSE.with(|h| *h.borrow()) {
                f();
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
