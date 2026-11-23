//! Tray icon that shows Windows notifications (Shell_NotifyIcon balloons; Windows 10/11 show them as toasts).
//!
//! A dedicated thread owns a message-only window and the tray icon. `notify` calls
//! `Shell_NotifyIconW(NIM_MODIFY, NIF_INFO)` directly, which the shell accepts from any thread.

use std::{
    ffi::OsStr,
    os::windows::ffi::OsStrExt,
    panic::{self, AssertUnwindSafe},
    ptr::{null, null_mut},
    sync::{
        atomic::{AtomicU32, Ordering},
        mpsc,
    },
    thread::JoinHandle,
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Shell::{
            ExtractIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_ERROR, NIIF_INFO,
            NIIF_RESPECT_QUIET_TIME, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION,
            NIN_BALLOONUSERCLICK, NIN_SELECT, NOTIFYICON_VERSION_4, NOTIFYICONDATAW,
            Shell_NotifyIconW,
        },
        WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyWindow, DispatchMessageW,
            GWLP_USERDATA, GetMessageW, GetWindowLongPtrW, HICON, HWND_MESSAGE, IDI_APPLICATION,
            LoadIconW, MSG, PostMessageW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW,
            SetWindowLongPtrW, UnregisterClassW, WM_APP, WM_CLOSE, WM_DESTROY, WM_LBUTTONUP,
            WNDCLASSW,
        },
    },
};

/// Callback message of the tray icon (WM_APP+1 per the plan).
const WM_TRAY: u32 = WM_APP + 1;
const TRAY_ID: u32 = 1;
const TIP: &str = "PS Package Pusher";
/// Unique window class per `Tray`, so two instances never share a class.
static NEXT_CLASS: AtomicU32 = AtomicU32::new(0);
/// Explorer broadcasts this when it restarts; the icon has to be added again then.
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

/// Tray icon that shows Windows notifications (Shell_NotifyIcon balloons; Windows 10/11 show them as toasts).
pub struct Tray {
    /// `HWND` stored as an integer so `Tray` stays `Send`.
    hwnd: isize,
    thread: Option<JoinHandle<()>>,
}

impl Tray {
    /// Adds the tray icon (app icon from the exe, tooltip "PS Package Pusher").
    /// `on_click` is called (from the tray thread) when the user clicks the notification or the tray icon.
    /// It must not drop the `Tray` itself, since that would wait for its own thread.
    /// Returns None if anything fails (the app must keep working without notifications).
    pub fn new(on_click: impl Fn() + Send + 'static) -> Option<Tray> {
        let (tx, rx) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("tray".into())
            .spawn(move || run(Box::new(on_click), tx))
            .ok()?;
        match rx.recv() {
            Ok(Some(hwnd)) => Some(Tray {
                hwnd,
                thread: Some(thread),
            }),
            _ => {
                let _ = thread.join();
                None
            }
        }
    }

    /// Shows one notification. `error` selects the error icon (NIIF_ERROR) instead of info (NIIF_INFO).
    /// Text longer than the Win32 buffers (title 64, body 256 UTF-16 units incl. NUL) is truncated safely.
    pub fn notify(&self, title: &str, body: &str, error: bool) {
        let mut nid = notify_data(self.hwnd as HWND);
        nid.uFlags = NIF_INFO;
        nid.szInfoTitle = to_wide_fixed::<64>(title);
        nid.szInfo = to_wide_fixed::<256>(body);
        let kind = if error { NIIF_ERROR } else { NIIF_INFO };
        nid.dwInfoFlags = kind | NIIF_RESPECT_QUIET_TIME;
        // SAFETY: hWnd/uID name our own icon; Shell_NotifyIconW may be called from any thread.
        unsafe { Shell_NotifyIconW(NIM_MODIFY, &nid) };
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        // SAFETY: hWnd/uID name our own icon; removing it from any thread is allowed.
        unsafe { Shell_NotifyIconW(NIM_DELETE, &notify_data(self.hwnd as HWND)) };
        // SAFETY: posting to our own window is thread-safe; WM_CLOSE makes the tray thread destroy it.
        unsafe { PostMessageW(self.hwnd as HWND, WM_CLOSE, 0, 0) };
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Tray thread: window, icon, message loop. Sends the window handle (or None) back to `new` first.
fn run(on_click: Box<dyn Fn() + Send>, ready: mpsc::Sender<Option<isize>>) {
    // SAFETY: plain registration of a message string, no pointers kept.
    let taskbar = unsafe { RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()) };
    TASKBAR_CREATED.store(taskbar, Ordering::Relaxed);
    let class = wide(&format!(
        "pspp-tray-{}",
        NEXT_CLASS.fetch_add(1, Ordering::Relaxed)
    ));
    let Some(hwnd) = create_window(&class) else {
        let _ = ready.send(None);
        return;
    };
    let (icon, owned) = load_icon();
    // Boxed so its address stays fixed while the window holds a pointer to it.
    let state = Box::new(State { on_click, icon });
    // SAFETY: `state` outlives the window: it is dropped only after the window is destroyed below.
    unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, &*state as *const State as isize) };
    let added = add_icon(hwnd, icon);
    let _ = ready.send(added.then_some(hwnd as isize));
    if added {
        // SAFETY: message loop on this thread; WM_QUIT (posted from WM_DESTROY) ends it.
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                DispatchMessageW(&msg);
            }
        }
    } else {
        // SAFETY: the window is still alive on this thread.
        unsafe { DestroyWindow(hwnd) };
    }
    // SAFETY: the window is gone, so the class can be unregistered; only an extracted icon is ours to free.
    unsafe {
        UnregisterClassW(class.as_ptr(), null_mut());
        if owned {
            DestroyIcon(icon);
        }
    }
}

struct State {
    on_click: Box<dyn Fn() + Send>,
    icon: HICON,
}

/// Message-only window of its own class (not visible, not in Alt+Tab).
fn create_window(class: &[u16]) -> Option<HWND> {
    // SAFETY: zeroed WNDCLASSW is a valid "empty" struct; the class name outlives the call.
    let mut wc: WNDCLASSW = unsafe { std::mem::zeroed() };
    wc.lpfnWndProc = Some(wnd_proc);
    wc.lpszClassName = class.as_ptr();
    // SAFETY: wc is fully initialised. hInstance is NULL here and in every later call for this class.
    if unsafe { RegisterClassW(&wc) } == 0 {
        return None;
    }
    // SAFETY: class is registered above; HWND_MESSAGE makes the window message-only.
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            null(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            null_mut(),
            null_mut(),
            null(),
        )
    };
    if hwnd.is_null() {
        // SAFETY: no window uses the class now.
        unsafe { UnregisterClassW(class.as_ptr(), null_mut()) };
        return None;
    }
    Some(hwnd)
}

/// The exe's own icon (winresource embeds it as the first icon group); falls back to the stock app icon.
/// Returns (icon, owned): only an extracted icon must be destroyed.
fn load_icon() -> (HICON, bool) {
    if let Some(exe) = std::env::current_exe().ok().map(|p| wide_os(p.as_os_str())) {
        // SAFETY: `exe` is NUL-terminated and outlives the call; hInst NULL is valid for a file path.
        let icon = unsafe { ExtractIconW(null_mut(), exe.as_ptr(), 0) };
        // 0 (no icon) and 1 (not an icon file) are not handles.
        if icon as isize > 1 {
            return (icon, true);
        }
    }
    // SAFETY: stock icon id with NULL instance; the shared icon must not be destroyed.
    (unsafe { LoadIconW(null_mut(), IDI_APPLICATION) }, false)
}

/// Adds the icon and selects version 4, so clicks arrive as NIN_SELECT in LOWORD(lParam).
fn add_icon(hwnd: HWND, icon: HICON) -> bool {
    let mut nid = notify_data(hwnd);
    nid.uFlags = NIF_ICON | NIF_TIP | NIF_MESSAGE;
    nid.uCallbackMessage = WM_TRAY;
    nid.hIcon = icon;
    nid.szTip = to_wide_fixed::<128>(TIP);
    // SAFETY: nid is fully initialised and lives across the call.
    if unsafe { Shell_NotifyIconW(NIM_ADD, &nid) } == 0 {
        return false;
    }
    nid.Anonymous.uVersion = NOTIFYICON_VERSION_4;
    // SAFETY: as above. Failure only changes the click message format, so it is ignored.
    unsafe { Shell_NotifyIconW(NIM_SETVERSION, &nid) };
    true
}

fn notify_data(hwnd: HWND) -> NOTIFYICONDATAW {
    // SAFETY: NOTIFYICONDATAW is plain data; all-zero is a valid "empty" value (as in firewall.rs).
    let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ID;
    nid
}

/// Window procedure. Runs on the tray thread only.
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: GWLP_USERDATA is set in `run` to the live `State` box (null before that).
    let state = unsafe { (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State).as_ref() };
    match msg {
        WM_TRAY => {
            let event = (lparam as usize & 0xFFFF) as u32;
            if let Some(st) = state
                && matches!(event, NIN_SELECT | NIN_BALLOONUSERCLICK | WM_LBUTTONUP)
            {
                // A panic must not unwind into Win32.
                let _ = panic::catch_unwind(AssertUnwindSafe(|| (st.on_click)()));
            }
            0
        }
        m if m != 0 && m == TASKBAR_CREATED.load(Ordering::Relaxed) => {
            if let Some(st) = state {
                add_icon(hwnd, st.icon);
            }
            0
        }
        WM_DESTROY => {
            // SAFETY: plain call; ends the message loop in `run`.
            unsafe { PostQuitMessage(0) };
            0
        }
        // SAFETY: default handling for every other message (WM_CLOSE destroys the window).
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn wide(s: &str) -> Vec<u16> {
    wide_os(OsStr::new(s))
}

fn wide_os(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain([0]).collect()
}

/// Copies `s` into a fixed Win32 buffer of N UTF-16 units: whole chars only, NUL-terminated.
/// A char that does not fit (a surrogate pair included) is dropped together with the rest.
fn to_wide_fixed<const N: usize>(s: &str) -> [u16; N] {
    let mut out = [0u16; N];
    let mut n = 0;
    for c in s.chars() {
        let mut buf = [0u16; 2];
        let units = c.encode_utf16(&mut buf);
        if n + units.len() > N.saturating_sub(1) {
            break;
        }
        out[n..n + units.len()].copy_from_slice(units);
        n += units.len();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::to_wide_fixed;

    /// Reads back the units up to the NUL and checks the NUL is there.
    fn text<const N: usize>(w: [u16; N]) -> String {
        let end = w.iter().position(|&c| c == 0).expect("NUL-terminated");
        String::from_utf16(&w[..end]).expect("no split surrogate")
    }

    #[test]
    fn ascii_fits_with_nul() {
        let w = to_wide_fixed::<8>("PS Pkg");
        assert_eq!(text(w), "PS Pkg");
        assert_eq!(w[6], 0);
        assert_eq!(w[7], 0);
    }

    #[test]
    fn exact_fit_and_overlong() {
        // 3 units + NUL in a 4-unit buffer
        assert_eq!(text(to_wide_fixed::<4>("abc")), "abc");
        assert_eq!(text(to_wide_fixed::<4>("abcd")), "abc");
        let long = to_wide_fixed::<64>(&"a".repeat(100));
        assert_eq!(text(long).len(), 63);
        assert_eq!(long[63], 0);
    }

    #[test]
    fn tiny_buffers() {
        assert_eq!(to_wide_fixed::<0>("x"), [] as [u16; 0]);
        assert_eq!(to_wide_fixed::<1>("x"), [0]);
    }

    #[test]
    fn emoji_is_never_split() {
        // "a" + emoji (2 units) = 3 units: fits exactly in N=4 (3 + NUL)
        assert_eq!(text(to_wide_fixed::<4>("a\u{1F600}")), "a\u{1F600}");
        // "ab" + emoji would need 4 units: the emoji is dropped whole
        assert_eq!(text(to_wide_fixed::<4>("ab\u{1F600}")), "ab");
        assert_eq!(text(to_wide_fixed::<5>("ab\u{1F600}")), "ab\u{1F600}");
        assert_eq!(text(to_wide_fixed::<3>("a\u{1F600}")), "a");
    }

    #[test]
    fn bosnian_letters() {
        let w = to_wide_fixed::<8>("čćžšđ");
        assert_eq!(text(w), "čćžšđ");
        assert_eq!(text(to_wide_fixed::<4>("čćžšđ")), "čćž");
        assert_eq!(text(to_wide_fixed::<64>("ČĆŽŠĐ čćžšđ")), "ČĆŽŠĐ čćžšđ");
    }
}
