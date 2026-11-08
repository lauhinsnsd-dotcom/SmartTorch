//! 黑帆's small box over the game listing this run's seals (from BlacksailRecorder's box): always on top, never
//! takes the keyboard, dragged anywhere with the mouse, made bigger or smaller by dragging its lower right corner
//! or its right or lower edge (the words grow with it). Where it's left and its size are kept in
//! %APPDATA%\VoraxRs\blacksail.txt. Its own thread, from `run` until `close`.

use std::sync::Mutex;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::*;

/// A line of the box: its text and colour (red, green, blue).
#[derive(Clone)]
pub struct Row {
    pub text: String,
    pub color: (u8, u8, u8),
}

/// What the box shows (written by the reader, drawn here).
static ROWS: Mutex<Vec<Row>> = Mutex::new(Vec::new());
/// The box's window, for the reader to ask for a redraw or to close it (0 while there's none).
static WINDOW: Mutex<isize> = Mutex::new(0);
/// The box's size against its natural one (dragged bigger or smaller).
static ZOOM: Mutex<f32> = Mutex::new(1.0);

const WM_REDRAW: u32 = WM_APP + 1;
const MIN_ZOOM: f32 = 0.6;
const MAX_ZOOM: f32 = 3.0;
/// How near its right or lower edge (pixels at 100%) a drag resizes it rather than moves it.
const GRIP: f32 = 10.0;

fn post(msg: u32) {
    let h = *WINDOW.lock().unwrap();
    if h != 0 {
        unsafe {
            let _ = PostMessageW(HWND(h as *mut core::ffi::c_void), msg, WPARAM(0), LPARAM(0));
        }
    }
}

/// Asks the box to draw `rows` (from any thread).
pub fn show(rows: Vec<Row>) {
    *ROWS.lock().unwrap() = rows;
    post(WM_REDRAW);
}

/// Closes the box (from any thread); its thread ends.
pub fn close() {
    post(WM_CLOSE);
}

fn place_file() -> std::path::PathBuf {
    super::super::sys::data_dir().join("blacksail.txt")
}

/// Where it was left and its zoom ("x y zoom").
fn saved_place() -> (i32, i32, f32) {
    let t = std::fs::read_to_string(place_file()).unwrap_or_default();
    let n: Vec<&str> = t.split_whitespace().collect();
    let x = n.first().and_then(|v| v.parse().ok()).unwrap_or(40);
    let y = n.get(1).and_then(|v| v.parse().ok()).unwrap_or(240);
    let z = n.get(2).and_then(|v| v.parse::<f32>().ok()).filter(|z| z.is_finite()).unwrap_or(1.0).clamp(MIN_ZOOM, MAX_ZOOM);
    (x, y, z)
}

fn save_place(hwnd: HWND) {
    let mut r = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut r) }.is_ok() {
        let _ = std::fs::write(place_file(), format!("{} {} {:.3}", r.left, r.top, *ZOOM.lock().unwrap()));
    }
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

/// The box's size for its rows at a zoom, and draws them when `draw`.
fn layout(hwnd: HWND, hdc: HDC, zoom: f32, draw: bool) -> (i32, i32) {
    let s = dpi_scale(hwnd) * zoom;
    let pad = (10.0 * s) as i32;
    let line = (24.0 * s) as i32;
    let rows = ROWS.lock().unwrap().clone();
    let f = font((15.0 * s) as i32, false);
    let fb = font((15.0 * s) as i32, true);
    let (mut width, mut y) = ((140.0 * s) as i32, pad);
    unsafe {
        let old = SelectObject(hdc, f);
        if draw {
            let _ = SetBkMode(hdc, TRANSPARENT);
        }
        for (i, row) in rows.iter().enumerate() {
            SelectObject(hdc, if i == 0 { fb } else { f });
            let text = wide(&row.text);
            let mut size = SIZE::default();
            let _ = GetTextExtentPoint32W(hdc, &text, &mut size);
            width = width.max(size.cx + 2 * pad);
            if draw {
                SetTextColor(hdc, COLORREF(row.color.0 as u32 | (row.color.1 as u32) << 8 | (row.color.2 as u32) << 16));
                let _ = TextOutW(hdc, pad, y, &text);
            }
            y += line;
        }
        SelectObject(hdc, old);
        let _ = DeleteObject(f);
        let _ = DeleteObject(fb);
    }
    (width, y + pad - (line - (18.0 * s) as i32))
}

fn size_at(hwnd: HWND, zoom: f32) -> (i32, i32) {
    unsafe {
        let hdc = GetDC(hwnd);
        let size = layout(hwnd, hdc, zoom, false);
        ReleaseDC(hwnd, hdc);
        size
    }
}

fn fit(hwnd: HWND) {
    let (w, h) = size_at(hwnd, *ZOOM.lock().unwrap());
    unsafe {
        let _ = SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, w, h, SWP_NOMOVE | SWP_NOACTIVATE);
        let _ = InvalidateRect(hwnd, None, true);
    }
}

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_REDRAW => {
            fit(hwnd);
            LRESULT(0)
        }
        // its right or lower edge resizes it (from its upper left corner), anywhere else drags it
        WM_NCHITTEST => {
            let mut r = RECT::default();
            let _ = GetWindowRect(hwnd, &mut r);
            let p = POINT { x: (lp.0 & 0xffff) as i16 as i32, y: ((lp.0 >> 16) & 0xffff) as i16 as i32 };
            let g = (GRIP * dpi_scale(hwnd)) as i32;
            let (right, bottom) = (p.x >= r.right - g, p.y >= r.bottom - g);
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
        // as it's dragged: the zoom its new height (or width, when only that is dragged) asks for, the box fitted
        // to its words at that zoom
        WM_SIZING => {
            let r = &mut *(lp.0 as *mut RECT);
            let zoom = *ZOOM.lock().unwrap();
            let (w0, h0) = size_at(hwnd, 1.0);
            let by_width = wp.0 as u32 == WMSZ_RIGHT;
            let want = if by_width { (r.right - r.left) as f32 / w0.max(1) as f32 } else { (r.bottom - r.top) as f32 / h0.max(1) as f32 };
            let z = if want.is_finite() { want.clamp(MIN_ZOOM, MAX_ZOOM) } else { zoom };
            *ZOOM.lock().unwrap() = z;
            let (w, h) = size_at(hwnd, z);
            r.right = r.left + w;
            r.bottom = r.top + h;
            let _ = InvalidateRect(hwnd, None, true);
            LRESULT(1)
        }
        WM_EXITSIZEMOVE => {
            save_place(hwnd);
            LRESULT(0)
        }
        WM_SETCURSOR => {
            // the arrows for resizing at its edges, the move cross elsewhere
            let hit = (lp.0 & 0xffff) as u32;
            let cursor = match hit {
                HTBOTTOMRIGHT => IDC_SIZENWSE,
                HTRIGHT => IDC_SIZEWE,
                HTBOTTOM => IDC_SIZENS,
                _ => IDC_SIZEALL,
            };
            SetCursor(LoadCursorW(None, cursor).unwrap_or_default());
            LRESULT(1)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut r = RECT::default();
            let _ = GetClientRect(hwnd, &mut r);
            let bg = CreateSolidBrush(COLORREF(0x001c1716));
            FillRect(hdc, &r, bg);
            let _ = DeleteObject(bg);
            layout(hwnd, hdc, *ZOOM.lock().unwrap(), true);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_DESTROY => {
            *WINDOW.lock().unwrap() = 0;
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

/// Makes the box and runs its window until `close` (on the calling thread, its own).
pub fn run() {
    unsafe {
        let Ok(module) = GetModuleHandleW(None) else { return };
        let class = WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: module.into(),
            lpszClassName: w!("SmartTorchBlacksailBox"),
            ..Default::default()
        };
        // made again after a stop and a start: already registered then
        RegisterClassW(&class);
        let (x, y, z) = saved_place();
        *ZOOM.lock().unwrap() = z;
        let Ok(hwnd) = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_NOACTIVATE,
            PCWSTR(class.lpszClassName.0),
            w!("Blacksail"),
            WS_POPUP,
            x,
            y,
            240,
            60,
            None,
            None,
            module,
            None,
        ) else {
            return;
        };
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 225, LWA_ALPHA);
        *WINDOW.lock().unwrap() = hwnd.0 as isize;
        fit(hwnd);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
