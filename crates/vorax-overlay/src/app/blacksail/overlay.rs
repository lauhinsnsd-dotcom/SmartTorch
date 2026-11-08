//! 黑帆's small box over the game listing this run's seals (from BlacksailRecorder's box), in two windows kept
//! together: its header (the title, a see-through button and a close button), always clickable, and the list
//! under it (six rows high from the start). Both stay on top and never take the keyboard; dragged anywhere by
//! either, made bigger or smaller by dragging the list's lower right corner or its right or lower edge (the words
//! grow with it). See-through: clicks pass through the list to the game (the header's buttons still take them).
//! Where it's left, its size and see-through are kept in %APPDATA%\VoraxRs\blacksail.txt. Its own thread, from
//! `run` until `close`.

use ::blacksail::picks::{CLOSE, PER_RUN, SEE_THROUGH};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::*;

/// A line of the box: its text and colour (red, green, blue). The first is the header's title.
#[derive(Clone)]
pub struct Row {
    pub text: String,
    pub color: (u8, u8, u8),
}

/// What the box shows (written by the reader, drawn here).
static ROWS: Mutex<Vec<Row>> = Mutex::new(Vec::new());
/// The header's and the list's windows (0 while there are none).
static HEAD: Mutex<isize> = Mutex::new(0);
static LIST: Mutex<isize> = Mutex::new(0);
/// The box's size against its natural one (dragged bigger or smaller).
static ZOOM: Mutex<f32> = Mutex::new(1.0);
/// Clicks pass through the list.
static SEE: AtomicBool = AtomicBool::new(false);
/// The header's upper left corner (the list sits under it).
static POS: Mutex<(i32, i32)> = Mutex::new((40, 240));
/// Set while one window is moved to follow the other (its own move then isn't followed back).
static PLACING: AtomicBool = AtomicBool::new(false);

const WM_REDRAW: u32 = WM_APP + 1;
const MIN_ZOOM: f32 = 0.6;
const MAX_ZOOM: f32 = 3.0;
/// How near the list's right or lower edge (pixels at 100%) a drag resizes it rather than moves it.
const GRIP: f32 = 10.0;
/// How opaque the header and the list are (of 255); the list lighter when see-through.
const HEAD_ALPHA: u8 = 235;
const LIST_ALPHA: u8 = 185;
const LIST_ALPHA_SEE: u8 = 140;
const BG: COLORREF = COLORREF(0x001c1716);
const GOLD: COLORREF = COLORREF(0x004ab4e8);
const BUTTON: COLORREF = COLORREF(0x00362d2a);

fn hwnd_of(m: &Mutex<isize>) -> Option<HWND> {
    let h = *m.lock().unwrap();
    (h != 0).then(|| HWND(h as *mut core::ffi::c_void))
}

/// Asks the box to draw `rows` (from any thread).
pub fn show(rows: Vec<Row>) {
    *ROWS.lock().unwrap() = rows;
    if let Some(h) = hwnd_of(&HEAD) {
        unsafe {
            let _ = PostMessageW(h, WM_REDRAW, WPARAM(0), LPARAM(0));
        }
    }
}

/// Closes the box (from any thread); its thread ends.
pub fn close() {
    if let Some(h) = hwnd_of(&HEAD) {
        unsafe {
            let _ = PostMessageW(h, WM_CLOSE, WPARAM(0), LPARAM(0));
        }
    }
}

fn place_file() -> std::path::PathBuf {
    super::super::sys::data_dir().join("blacksail.txt")
}

/// Where it was left, its zoom and see-through ("x y zoom see"): where it was left only while on a screen,
/// else over the game's window (or near the screen's corner).
fn load_place() {
    let t = std::fs::read_to_string(place_file()).unwrap_or_default();
    let n: Vec<&str> = t.split_whitespace().collect();
    let saved = match (n.first().and_then(|v| v.parse::<i32>().ok()), n.get(1).and_then(|v| v.parse::<i32>().ok())) {
        (Some(x), Some(y)) if unsafe { !MonitorFromPoint(POINT { x: x + 20, y: y + 10 }, MONITOR_DEFAULTTONULL).is_invalid() } => Some((x, y)),
        _ => None,
    };
    let near_game = || {
        let g = super::super::sys::find_game_window()?;
        let (x, y, _, _) = super::super::sys::client_rect(g)?;
        Some((x + 60, y + 260))
    };
    *POS.lock().unwrap() = saved.or_else(near_game).unwrap_or((40, 240));
    *ZOOM.lock().unwrap() = n.get(2).and_then(|v| v.parse::<f32>().ok()).filter(|z| z.is_finite()).unwrap_or(1.0).clamp(MIN_ZOOM, MAX_ZOOM);
    SEE.store(n.get(3) == Some(&"1"), Ordering::SeqCst);
}

fn save_place() {
    let (x, y) = *POS.lock().unwrap();
    let _ = std::fs::write(place_file(), format!("{x} {y} {:.3} {}", *ZOOM.lock().unwrap(), SEE.load(Ordering::SeqCst) as u8));
}

fn dpi_scale(hwnd: HWND) -> f32 {
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi == 0 {
        1.0
    } else {
        dpi as f32 / 96.0
    }
}

fn font(px: i32, bold: bool) -> HFONT {
    unsafe {
        CreateFontW(
            -px,
            0,
            0,
            0,
            if bold { FW_BOLD.0 as i32 } else { FW_NORMAL.0 as i32 },
            0,
            0,
            0,
            DEFAULT_CHARSET.0 as u32,
            OUT_DEFAULT_PRECIS.0 as u32,
            CLIP_DEFAULT_PRECIS.0 as u32,
            CLEARTYPE_QUALITY.0 as u32,
            (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
            w!("Microsoft JhengHei UI"),
        )
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn colour((r, g, b): (u8, u8, u8)) -> COLORREF {
    COLORREF(r as u32 | (g as u32) << 8 | (b as u32) << 16)
}

/// The box at a zoom: its width, the header's and the list's heights, and the header's buttons (see-through,
/// close), in the header's own pixels.
struct Geo {
    w: i32,
    head_h: i32,
    list_h: i32,
    pad: i32,
    line: i32,
    font_px: i32,
    see: RECT,
    close: RECT,
}

fn text_width(hdc: HDC, f: HFONT, text: &str) -> i32 {
    unsafe {
        SelectObject(hdc, f);
        let mut size = SIZE::default();
        let _ = GetTextExtentPoint32W(hdc, &wide(text), &mut size);
        size.cx
    }
}

fn geo(hwnd: HWND, zoom: f32) -> Geo {
    let s = dpi_scale(hwnd) * zoom;
    let pad = (12.0 * s) as i32;
    let line = (28.0 * s) as i32;
    let font_px = (18.0 * s) as i32;
    let rows = ROWS.lock().unwrap().clone();
    unsafe {
        let hdc = GetDC(hwnd);
        let f = font(font_px, false);
        let fb = font(font_px, true);
        let old = SelectObject(hdc, f);
        let title = rows.first().map_or(0, |r| text_width(hdc, fb, &r.text));
        let see_w = text_width(hdc, f, SEE_THROUGH) + pad;
        let close_w = text_width(hdc, fb, CLOSE) + pad;
        let list_w = rows.iter().skip(1).map(|r| text_width(hdc, f, &r.text)).max().unwrap_or(0);
        SelectObject(hdc, old);
        let _ = DeleteObject(f);
        let _ = DeleteObject(fb);
        ReleaseDC(hwnd, hdc);
        let head_h = line + pad;
        let w = ((360.0 * s) as i32).max(list_w + 2 * pad).max(title + see_w + close_w + 4 * pad);
        let (bt, bb) = (pad / 2 + line / 10, head_h - pad / 2 - line / 10);
        let close = RECT { left: w - pad - close_w, top: bt, right: w - pad, bottom: bb };
        let see = RECT { left: close.left - pad / 2 - see_w, top: bt, right: close.left - pad / 2, bottom: bb };
        Geo { w, head_h, list_h: PER_RUN as i32 * line + pad, pad, line, font_px, see, close }
    }
}

/// Both windows where POS and the zoom put them, redrawn.
fn place_all() {
    let (Some(head), Some(list)) = (hwnd_of(&HEAD), hwnd_of(&LIST)) else { return };
    let g = geo(head, *ZOOM.lock().unwrap());
    let (x, y) = *POS.lock().unwrap();
    PLACING.store(true, Ordering::SeqCst);
    unsafe {
        let _ = SetWindowPos(head, HWND_TOPMOST, x, y, g.w, g.head_h, SWP_NOACTIVATE);
        let _ = SetWindowPos(list, HWND_TOPMOST, x, y + g.head_h, g.w, g.list_h, SWP_NOACTIVATE);
        let _ = InvalidateRect(head, None, true);
        let _ = InvalidateRect(list, None, true);
    }
    PLACING.store(false, Ordering::SeqCst);
}

/// The list see-through (clicks pass through it) or not, as SEE says.
fn apply_see() {
    let Some(list) = hwnd_of(&LIST) else { return };
    let see = SEE.load(Ordering::SeqCst);
    unsafe {
        let ex = GetWindowLongPtrW(list, GWL_EXSTYLE);
        let ex = if see { ex | WS_EX_TRANSPARENT.0 as isize } else { ex & !(WS_EX_TRANSPARENT.0 as isize) };
        SetWindowLongPtrW(list, GWL_EXSTYLE, ex);
        let _ = SetLayeredWindowAttributes(list, COLORREF(0), if see { LIST_ALPHA_SEE } else { LIST_ALPHA }, LWA_ALPHA);
    }
    if let Some(head) = hwnd_of(&HEAD) {
        unsafe {
            let _ = InvalidateRect(head, None, true);
        }
    }
}

fn inside(r: &RECT, x: i32, y: i32) -> bool {
    x >= r.left && x < r.right && y >= r.top && y < r.bottom
}

/// A point of the screen in a window's own pixels.
fn local(hwnd: HWND, lp: LPARAM) -> (i32, i32) {
    let mut r = RECT::default();
    unsafe {
        let _ = GetWindowRect(hwnd, &mut r);
    }
    ((lp.0 & 0xffff) as i16 as i32 - r.left, ((lp.0 >> 16) & 0xffff) as i16 as i32 - r.top)
}

fn fill(hdc: HDC, r: &RECT, c: COLORREF) {
    unsafe {
        let b = CreateSolidBrush(c);
        FillRect(hdc, r, b);
        let _ = DeleteObject(b);
    }
}

unsafe extern "system" fn head_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_REDRAW => {
            place_all();
            LRESULT(0)
        }
        // its buttons take clicks, anywhere else drags the box
        WM_NCHITTEST => {
            let g = geo(hwnd, *ZOOM.lock().unwrap());
            let (x, y) = local(hwnd, lp);
            LRESULT(if inside(&g.see, x, y) || inside(&g.close, x, y) { HTCLIENT } else { HTCAPTION } as isize)
        }
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_LBUTTONUP => {
            let g = geo(hwnd, *ZOOM.lock().unwrap());
            let (x, y) = ((lp.0 & 0xffff) as i16 as i32, ((lp.0 >> 16) & 0xffff) as i16 as i32);
            if inside(&g.close, x, y) {
                // as the main window's Stop: the reader ends, this box closes
                super::stop();
            } else if inside(&g.see, x, y) {
                SEE.store(!SEE.load(Ordering::SeqCst), Ordering::SeqCst);
                apply_see();
                save_place();
            }
            LRESULT(0)
        }
        WM_SETCURSOR => {
            let hit = (lp.0 & 0xffff) as u32;
            SetCursor(LoadCursorW(None, if hit == HTCLIENT { IDC_HAND } else { IDC_SIZEALL }).unwrap_or_default());
            LRESULT(1)
        }
        // dragged: the list follows
        WM_MOVE => {
            if !PLACING.load(Ordering::SeqCst) {
                let mut r = RECT::default();
                if GetWindowRect(hwnd, &mut r).is_ok() {
                    *POS.lock().unwrap() = (r.left, r.top);
                    place_all();
                }
            }
            LRESULT(0)
        }
        WM_EXITSIZEMOVE => {
            save_place();
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut r = RECT::default();
            let _ = GetClientRect(hwnd, &mut r);
            let g = geo(hwnd, *ZOOM.lock().unwrap());
            fill(hdc, &r, GOLD);
            let b = (2.0 * dpi_scale(hwnd)).max(1.0) as i32;
            fill(hdc, &RECT { left: b, top: b, right: r.right - b, bottom: r.bottom }, BG);
            let rows = ROWS.lock().unwrap().clone();
            let f = font(g.font_px, false);
            let fb = font(g.font_px, true);
            let old = SelectObject(hdc, fb);
            let _ = SetBkMode(hdc, TRANSPARENT);
            if let Some(t) = rows.first() {
                SetTextColor(hdc, colour(t.color));
                let _ = TextOutW(hdc, g.pad, (g.head_h - g.font_px) / 2 - g.font_px / 8, &wide(&t.text));
            }
            let see = SEE.load(Ordering::SeqCst);
            for (rect, text, on, bold) in [(&g.see, SEE_THROUGH, see, false), (&g.close, CLOSE, false, true)] {
                fill(hdc, rect, if on { GOLD } else { BUTTON });
                SelectObject(hdc, if bold { fb } else { f });
                SetTextColor(hdc, if on { BG } else { COLORREF(0x00e6e6e6) });
                let mut tr = *rect;
                let mut text = wide(text);
                DrawTextW(hdc, &mut text, &mut tr, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
            }
            SelectObject(hdc, old);
            let _ = DeleteObject(f);
            let _ = DeleteObject(fb);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_DESTROY => {
            if let Some(list) = hwnd_of(&LIST) {
                let _ = DestroyWindow(list);
            }
            *HEAD.lock().unwrap() = 0;
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe extern "system" fn list_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        // its right or lower edge resizes the box (from the list's upper left corner), anywhere else drags it
        WM_NCHITTEST => {
            let mut r = RECT::default();
            let _ = GetWindowRect(hwnd, &mut r);
            let (x, y) = local(hwnd, lp);
            let g = (GRIP * dpi_scale(hwnd)) as i32;
            let (right, bottom) = (x >= r.right - r.left - g, y >= r.bottom - r.top - g);
            LRESULT(if right && bottom {
                HTBOTTOMRIGHT
            } else if right {
                HTRIGHT
            } else if bottom {
                HTBOTTOM
            } else {
                HTCAPTION
            } as isize)
        }
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_SETCURSOR => {
            let cursor = match (lp.0 & 0xffff) as u32 {
                HTBOTTOMRIGHT => IDC_SIZENWSE,
                HTRIGHT => IDC_SIZEWE,
                HTBOTTOM => IDC_SIZENS,
                _ => IDC_SIZEALL,
            };
            SetCursor(LoadCursorW(None, cursor).unwrap_or_default());
            LRESULT(1)
        }
        // as it's dragged bigger or smaller: the zoom its new height (or width, when only that is dragged) asks
        // for, the box fitted to its words at that zoom, the header kept on top of it
        WM_SIZING => {
            let r = &mut *(lp.0 as *mut RECT);
            let g1 = geo(hwnd, 1.0);
            let want = if wp.0 as u32 == WMSZ_RIGHT { (r.right - r.left) as f32 / g1.w.max(1) as f32 } else { (r.bottom - r.top) as f32 / g1.list_h.max(1) as f32 };
            let z = if want.is_finite() { want.clamp(MIN_ZOOM, MAX_ZOOM) } else { *ZOOM.lock().unwrap() };
            *ZOOM.lock().unwrap() = z;
            let g = geo(hwnd, z);
            r.right = r.left + g.w;
            r.bottom = r.top + g.list_h;
            *POS.lock().unwrap() = (r.left, r.top - g.head_h);
            if let Some(head) = hwnd_of(&HEAD) {
                PLACING.store(true, Ordering::SeqCst);
                let _ = SetWindowPos(head, HWND_TOPMOST, r.left, r.top - g.head_h, g.w, g.head_h, SWP_NOACTIVATE);
                let _ = InvalidateRect(head, None, true);
                PLACING.store(false, Ordering::SeqCst);
            }
            let _ = InvalidateRect(hwnd, None, true);
            LRESULT(1)
        }
        // dragged: the header follows
        WM_MOVE => {
            if !PLACING.load(Ordering::SeqCst) {
                let mut r = RECT::default();
                if GetWindowRect(hwnd, &mut r).is_ok() {
                    let g = geo(hwnd, *ZOOM.lock().unwrap());
                    *POS.lock().unwrap() = (r.left, r.top - g.head_h);
                    place_all();
                }
            }
            LRESULT(0)
        }
        WM_EXITSIZEMOVE => {
            save_place();
            place_all();
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut r = RECT::default();
            let _ = GetClientRect(hwnd, &mut r);
            fill(hdc, &r, BG);
            let g = geo(hwnd, *ZOOM.lock().unwrap());
            let rows = ROWS.lock().unwrap().clone();
            let f = font(g.font_px, false);
            let old = SelectObject(hdc, f);
            let _ = SetBkMode(hdc, TRANSPARENT);
            for (i, row) in rows.iter().skip(1).enumerate() {
                SetTextColor(hdc, colour(row.color));
                let _ = TextOutW(hdc, g.pad, g.pad / 2 + i as i32 * g.line + (g.line - g.font_px) / 2 - g.font_px / 8, &wide(&row.text));
            }
            SelectObject(hdc, old);
            let _ = DeleteObject(f);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_DESTROY => {
            *LIST.lock().unwrap() = 0;
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

/// Makes the box and runs its windows until `close` (on the calling thread, its own).
pub fn run() {
    unsafe {
        let Ok(module) = GetModuleHandleW(None) else { return };
        // made again after a stop and a start: already registered then
        let classes: [(PCWSTR, WNDPROC); 2] = [(w!("SmartTorchBlacksailHead"), Some(head_proc)), (w!("SmartTorchBlacksailList"), Some(list_proc))];
        for (name, proc) in classes {
            let class = WNDCLASSW { lpfnWndProc: proc, hInstance: module.into(), lpszClassName: name, ..Default::default() };
            RegisterClassW(&class);
        }
        load_place();
        let (x, y) = *POS.lock().unwrap();
        let make = |class: PCWSTR, title: PCWSTR| {
            CreateWindowExW(WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_NOACTIVATE, class, title, WS_POPUP, x, y, 360, 60, None, None, module, None).ok()
        };
        let Some(head) = make(w!("SmartTorchBlacksailHead"), w!("Blacksail")) else { return };
        let Some(list) = make(w!("SmartTorchBlacksailList"), w!("Blacksail list")) else {
            let _ = DestroyWindow(head);
            return;
        };
        let _ = SetLayeredWindowAttributes(head, COLORREF(0), HEAD_ALPHA, LWA_ALPHA);
        *HEAD.lock().unwrap() = head.0 as isize;
        *LIST.lock().unwrap() = list.0 as isize;
        apply_see();
        place_all();
        let _ = ShowWindow(list, SW_SHOWNOACTIVATE);
        let _ = ShowWindow(head, SW_SHOWNOACTIVATE);
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
