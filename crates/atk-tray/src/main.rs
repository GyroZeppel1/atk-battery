#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Tray + panel front end. Single instance, one worker thread, all UI work on
//! the message thread. Zero third-party crates.

mod icon;
mod paint;
mod settings;
mod winui;

use atk_core::win::{wide, HANDLE};
use atk_core::{scan, DeviceReading, Power, ScanOptions};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use winui::*;

const CLASS_NAME: &str = "AtkBatteryPanel";
const TITLE: &str = "ATK 电量";
const TRAY_ID: u32 = 1;
const IDC_ARROW: usize = 32512;

const ID_OPEN: usize = 1;
const ID_REFRESH: usize = 2;
const ID_QUIT: usize = 3;
const ID_STARTUP: usize = 4;
const ID_SHOW_PANEL: usize = 5;
const ID_DESKTOP: usize = 6;

struct App {
    hwnd: HANDLE,
    tray_icon: HANDLE,
    tray_added: bool,
    fonts: Fonts,
    rows: Arc<Mutex<Vec<DeviceReading>>>,
    updated: Arc<Mutex<String>>,
    interval: Arc<AtomicU64>,
    settings: settings::Settings,
    tx: Sender<()>,
}

static mut APP: *mut App = ptr::null_mut();

unsafe fn app<'a>() -> &'a mut App {
    &mut *APP
}

/// Off by default: the tool writes no files in normal use. Set ATK_TRAY_DEBUG=1
/// to trace tray registration when diagnosing a machine.
fn log(msg: &str) {
    use std::io::Write;
    if std::env::var_os("ATK_TRAY_DEBUG").is_none() {
        return;
    }
    let path = std::env::temp_dir().join("atk-tray.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{}", msg);
    }
}

fn now_hms() -> String {
    unsafe {
        let mut st: SYSTEMTIME = std::mem::zeroed();
        GetLocalTime(&mut st);
        format!("{:02}:{:02}:{:02}", st.wHour, st.wMinute, st.wSecond)
    }
}

fn make_nid(hwnd: HANDLE, icon: HANDLE, tip: &str) -> NOTIFYICONDATAW {
    let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ID;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_APP_TRAY;
    nid.hIcon = icon;
    fill_wide(&mut nid.szTip, tip);
    nid
}

impl App {
    fn new(tx: Sender<()>) -> Self {
        Self {
            hwnd: ptr::null_mut(),
            tray_icon: ptr::null_mut(),
            tray_added: false,
            fonts: Fonts::new(),
            rows: Arc::new(Mutex::new(Vec::new())),
            updated: Arc::new(Mutex::new(String::new())),
            interval: Arc::new(AtomicU64::new(60)),
            settings: settings::Settings::default(),
            tx,
        }
    }

    /// Reads happen off the UI thread; the result is posted back so painting
    /// never blocks on a HID transaction (which can take 260 ms when asleep).
    fn start_worker(&self, rx: Receiver<()>) {
        let rows = Arc::clone(&self.rows);
        let updated = Arc::clone(&self.updated);
        let interval = Arc::clone(&self.interval);
        let hwnd = self.hwnd as isize;
        std::thread::spawn(move || loop {
            let result = scan(&ScanOptions::default());
            let ok = result.iter().any(|r| r.result.is_ok());
            *rows.lock().unwrap() = result;
            *updated.lock().unwrap() = now_hms();
            unsafe {
                PostMessageW(hwnd as HANDLE, WM_APP_DATA, 0, 0);
            }
            // A sleeping mouse answers nothing. Retrying on the normal 60s
            // cadence would leave the tray stale for up to a minute after the
            // user comes back, so poll quickly until the device answers again.
            let secs = if ok {
                interval.load(Ordering::Relaxed)
            } else {
                5
            };
            match rx.recv_timeout(Duration::from_secs(secs.clamp(3, 3600))) {
                Ok(_) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
        });
    }

    fn add_tray(&mut self) {
        unsafe {
            let i = icon::make_idle_icon();
            if i.is_null() {
                return;
            }
            self.tray_icon = i;
            let nid = make_nid(self.hwnd, i, "ATK 电量 · 读取中");
            let ok = Shell_NotifyIconW(NIM_ADD, &nid);
            log(&format!("add_tray icon={:?} shell={} cbSize={}", i, ok, nid.cbSize));
            self.tray_added = true;
        }
    }

    fn refresh_tray(&mut self) {
        let info: Option<(u8, bool, String)> = {
            let rows = self.rows.lock().unwrap();
            let updated = self.updated.lock().unwrap().clone();
            match rows.iter().find(|r| r.result.is_ok()) {
                Some(d) => match &d.result {
                    Ok(v) => Some((
                        v.percent,
                        matches!(v.power, Power::Charging | Power::Full),
                        format!("{} · {}%  {}", d.name, v.percent, updated),
                    )),
                    Err(_) => None,
                },
                None => None,
            }
        };
        unsafe {
            let (new_icon, tip) = match info {
                Some((pct, charging, tip)) => (icon::make_icon(pct, charging), tip),
                None => (icon::make_idle_icon(), "ATK 电量 · 未检测到设备".to_string()),
            };
            if new_icon.is_null() {
                return;
            }
            if !self.tray_icon.is_null() {
                DestroyIcon(self.tray_icon);
            }
            self.tray_icon = new_icon;
            let nid = make_nid(self.hwnd, new_icon, &tip);
            let ok = if self.tray_added {
                Shell_NotifyIconW(NIM_MODIFY, &nid)
            } else {
                self.tray_added = true;
                Shell_NotifyIconW(NIM_ADD, &nid)
            };
            log(&format!("refresh_tray icon={:?} shell={} tip={}", new_icon, ok, tip));
        }
    }

    fn remove_tray(&mut self) {
        unsafe {
            if self.tray_added {
                let nid = make_nid(self.hwnd, ptr::null_mut(), "");
                Shell_NotifyIconW(NIM_DELETE, &nid);
                self.tray_added = false;
            }
            if !self.tray_icon.is_null() {
                DestroyIcon(self.tray_icon);
                self.tray_icon = ptr::null_mut();
            }
        }
    }

    fn toggle_window(&self) {
        unsafe {
            if IsWindowVisible(self.hwnd) != 0 {
                ShowWindow(self.hwnd, SW_HIDE);
            } else {
                ShowWindow(self.hwnd, SW_SHOW);
                SetForegroundWindow(self.hwnd);
            }
        }
    }

    fn show_menu(&self) {
        unsafe {
            let menu = CreatePopupMenu();
            AppendMenuW(menu, MF_STRING, ID_OPEN, wide("打开面板").as_ptr());
            AppendMenuW(menu, MF_STRING, ID_REFRESH, wide("立即刷新").as_ptr());
            // State is read from the filesystem every time the menu opens, so a
            // shortcut the user removed by hand shows up as unchecked.
            let auto = if atk_core::startup::is_enabled() {
                MF_CHECKED
            } else {
                0
            };
            AppendMenuW(menu, MF_STRING | auto, ID_STARTUP, wide("开机自启").as_ptr());
            let show = if self.settings.show_panel {
                MF_CHECKED
            } else {
                0
            };
            AppendMenuW(menu, MF_STRING | show, ID_SHOW_PANEL, wide("启动时打开面板").as_ptr());
            AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
            AppendMenuW(menu, MF_STRING, ID_DESKTOP, wide("创建桌面快捷方式").as_ptr());
            AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
            AppendMenuW(menu, MF_STRING, ID_QUIT, wide("退出").as_ptr());

            let mut pt = POINT { x: 0, y: 0 };
            GetCursorPos(&mut pt);
            // Required, otherwise the menu stays on screen after the click.
            SetForegroundWindow(self.hwnd);
            let cmd = TrackPopupMenu(
                menu,
                TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_LEFTALIGN,
                pt.x,
                pt.y,
                0,
                self.hwnd,
                ptr::null(),
            );
            DestroyMenu(menu);
            if cmd != 0 {
                SendMessageW(self.hwnd, WM_COMMAND, cmd as usize, 0);
            }
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HANDLE, msg: u32, wp: usize, lp: isize) -> isize {
    if APP.is_null() {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut rc: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut rc);
            let a = &*APP;
            let rows = { let g = a.rows.lock().unwrap(); (*g).clone() };
            let updated = { let g = a.updated.lock().unwrap(); (*g).clone() };
            paint::paint(hdc, &rows, &a.fonts, &updated, rc.right, rc.bottom);
            EndPaint(hwnd, &ps);
            0
        }
        WM_ERASEBKGND => 1,
        WM_APP_DATA => {
            app().refresh_tray();
            InvalidateRect(hwnd, ptr::null(), 0);
            0
        }
        WM_APP_TRAY => {
            let ev = (lp as u32) & 0xFFFF;
            if ev == WM_LBUTTONUP {
                app().toggle_window();
            } else if ev == WM_RBUTTONUP {
                app().show_menu();
            }
            0
        }
        WM_COMMAND => {
            match wp & 0xFFFF {
                ID_OPEN => {
                    ShowWindow(hwnd, SW_SHOW);
                    SetForegroundWindow(hwnd);
                }
                ID_REFRESH => {
                    let _ = app().tx.send(());
                }
                ID_STARTUP => {
                    let on = atk_core::startup::is_enabled();
                    if let Err(e) = atk_core::startup::set_enabled(!on) {
                        let text = wide(&format!("设置开机自启失败：{e}"));
                        let cap = wide("ATK 电量");
                        MessageBoxW(hwnd, text.as_ptr(), cap.as_ptr(), MB_ICONERROR);
                    }
                }
                ID_SHOW_PANEL => {
                    let a = app();
                    a.settings.show_panel = !a.settings.show_panel;
                    settings::save(&a.settings);
                }
                ID_DESKTOP => {
                    if let Some(p) = atk_core::startup::desktop_lnk_path() {
                        if let Err(e) = atk_core::startup::create_shortcut(&p, "", "ATK 鼠标电量") {
                            let text = wide(&format!("创建桌面快捷方式失败：{e}"));
                            let cap = wide("ATK 电量");
                            MessageBoxW(hwnd, text.as_ptr(), cap.as_ptr(), MB_ICONERROR);
                        }
                    }
                }
                ID_QUIT => {
                    DestroyWindow(hwnd);
                }
                _ => {}
            }
            0
        }
        WM_CLOSE => {
            // Closing the panel keeps the tray running.
            ShowWindow(hwnd, SW_HIDE);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

fn main() {
    log("=== start ===");
    unsafe {
        // CreateMutexW does not clear the last error on success, so a stale
        // 183 from an earlier call would look like "already running" and make
        // the process exit instantly.
        SetLastError(0);
        let _mutex = CreateMutexW(
            ptr::null(),
            0,
            wide("Local\\atk-battery-single-instance").as_ptr(),
        );
        if GetLastError() == ERROR_ALREADY_EXISTS {
            // A second launch should surface the running instance instead of
            // doing nothing, which otherwise looks like a broken shortcut.
            let existing = FindWindowW(wide(CLASS_NAME).as_ptr(), ptr::null());
            if !existing.is_null() {
                ShowWindow(existing, SW_SHOW);
                SetForegroundWindow(existing);
            }
            return;
        }

        let (tx, rx) = channel::<()>();
        APP = Box::into_raw(Box::new(App::new(tx)));

        let hinst = GetModuleHandleW(ptr::null());
        let cls = wide(CLASS_NAME);
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: LoadIconW(ptr::null_mut(), IDI_APPLICATION as *const u16),
            hCursor: LoadCursorW(ptr::null_mut(), IDC_ARROW as *const u16),
            hbrBackground: ptr::null_mut(),
            lpszMenuName: ptr::null(),
            lpszClassName: cls.as_ptr(),
        };
        RegisterClassW(&wc);

        let (sw, sh) = (GetSystemMetrics(0), GetSystemMetrics(1));
        let (ww, wh) = (404, 240);
        let hwnd = CreateWindowExW(
            0,
            cls.as_ptr(),
            wide(TITLE).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            (sw - ww) / 2,
            (sh - wh) / 2,
            ww,
            wh,
            ptr::null_mut(),
            ptr::null_mut(),
            hinst,
            ptr::null_mut(),
        );
        if hwnd.is_null() {
            return;
        }

        let a = app();
        a.hwnd = hwnd;
        a.settings = settings::load();
        a.add_tray();
        a.start_worker(rx);

        // `--tray` is what the Startup shortcut passes: login always stays in
        // the tray. A manual launch follows the user's preference.
        let from_login = std::env::args().any(|arg| arg == "--tray");
        if !from_login && a.settings.show_panel {
            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);
        }

        log("window created, entering message loop");
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        app().remove_tray();
    }
}
