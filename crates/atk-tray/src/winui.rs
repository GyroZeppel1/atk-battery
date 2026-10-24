//! Win32 bindings for the tray UI: windowing, GDI drawing, shell notifications.
//!
//! Same policy as `atk-core`: hand-written declarations, no GUI framework, no
//! third-party crate. Only the calls this UI makes are bound.

#![allow(
    non_snake_case,
    non_camel_case_types,
    dead_code,
    clippy::upper_case_acronyms
)]

use atk_core::win::{wide, BOOL, DWORD, GUID, HANDLE, WCHAR};
use std::ffi::c_void;

pub const WM_DESTROY: u32 = 0x0002;
pub const WM_CLOSE: u32 = 0x0010;
pub const WM_PAINT: u32 = 0x000F;
pub const WM_ERASEBKGND: u32 = 0x0014;
pub const WM_COMMAND: u32 = 0x0111;
pub const WM_LBUTTONUP: u32 = 0x0202;
pub const WM_RBUTTONUP: u32 = 0x0205;
pub const WM_APP: u32 = 0x8000;

pub const WM_APP_DATA: u32 = WM_APP + 1;
pub const WM_APP_TRAY: u32 = WM_APP + 2;

pub const WS_OVERLAPPED: u32 = 0x0000_0000;
pub const WS_CAPTION: u32 = 0x00C0_0000;
pub const WS_SYSMENU: u32 = 0x0008_0000;
pub const WS_MINIMIZEBOX: u32 = 0x0002_0000;

pub const CS_HREDRAW: u32 = 0x0002;
pub const CS_VREDRAW: u32 = 0x0001;

pub const SW_SHOW: i32 = 5;
pub const SW_HIDE: i32 = 0;

pub const NIM_ADD: u32 = 0;
pub const NIM_MODIFY: u32 = 1;
pub const NIM_DELETE: u32 = 2;
pub const NIF_MESSAGE: u32 = 1;
pub const NIF_ICON: u32 = 2;
pub const NIF_TIP: u32 = 4;

pub const TPM_RIGHTBUTTON: u32 = 0x0002;
pub const TPM_RETURNCMD: u32 = 0x0100;
pub const TPM_LEFTALIGN: u32 = 0x0000;
pub const TPM_BOTTOMALIGN: u32 = 0x0020;

pub const MF_STRING: u32 = 0x0000;
pub const MF_SEPARATOR: u32 = 0x0800;

pub const DT_LEFT: u32 = 0x0000;
pub const DT_CENTER: u32 = 0x0001;
pub const DT_RIGHT: u32 = 0x0002;
pub const DT_VCENTER: u32 = 0x0004;
pub const DT_SINGLELINE: u32 = 0x0020;
pub const DT_CALCRECT: u32 = 0x0400;

pub const TRANSPARENT: i32 = 1;
pub const DIB_RGB_COLORS: u32 = 0;
pub const BI_RGB: u32 = 0;

pub const FW_BOLD: i32 = 700;
pub const DEFAULT_CHARSET: u32 = 1;
pub const CLEARTYPE_QUALITY: u32 = 5;

pub const IDI_APPLICATION: usize = 32512;

/// 0x00BBGGRR
pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct POINT {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
pub struct MSG {
    pub hwnd: HANDLE,
    pub message: u32,
    pub wParam: usize,
    pub lParam: isize,
    pub time: u32,
    pub pt: POINT,
}

#[repr(C)]
pub struct PAINTSTRUCT {
    pub hdc: HANDLE,
    pub fErase: BOOL,
    pub rcPaint: RECT,
    pub fRestore: BOOL,
    pub fIncUpdate: BOOL,
    pub rgbReserved: [u8; 32],
}

pub type WNDPROC = unsafe extern "system" fn(HANDLE, u32, usize, isize) -> isize;

#[repr(C)]
pub struct WNDCLASSW {
    pub style: u32,
    pub lpfnWndProc: Option<WNDPROC>,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: HANDLE,
    pub hIcon: HANDLE,
    pub hCursor: HANDLE,
    pub hbrBackground: HANDLE,
    pub lpszMenuName: *const WCHAR,
    pub lpszClassName: *const WCHAR,
}

#[repr(C)]
pub struct NOTIFYICONDATAW {
    pub cbSize: u32,
    pub hWnd: HANDLE,
    pub uID: u32,
    pub uFlags: u32,
    pub uCallbackMessage: u32,
    pub hIcon: HANDLE,
    pub szTip: [WCHAR; 128],
    pub dwState: u32,
    pub dwStateMask: u32,
    pub szInfo: [WCHAR; 256],
    pub uTimeoutOrVersion: u32,
    pub szInfoTitle: [WCHAR; 64],
    pub dwInfoFlags: u32,
    pub guidItem: GUID,
    pub hBalloonIcon: HANDLE,
}

#[repr(C)]
pub struct ICONINFO {
    pub fIcon: BOOL,
    pub xHotspot: DWORD,
    pub yHotspot: DWORD,
    pub hbmMask: HANDLE,
    pub hbmColor: HANDLE,
}

#[repr(C)]
pub struct BITMAPINFOHEADER {
    pub biSize: u32,
    pub biWidth: i32,
    pub biHeight: i32,
    pub biPlanes: u16,
    pub biBitCount: u16,
    pub biCompression: u32,
    pub biSizeImage: u32,
    pub biXPelsPerMeter: i32,
    pub biYPelsPerMeter: i32,
    pub biClrUsed: u32,
    pub biClrImportant: u32,
}

#[repr(C)]
pub struct BITMAPINFO {
    pub bmiHeader: BITMAPINFOHEADER,
    pub bmiColors: [u32; 1],
}

#[link(name = "user32")]
extern "system" {
    pub fn RegisterClassW(lpWndClass: *const WNDCLASSW) -> u16;
    pub fn CreateWindowExW(
        dwExStyle: u32,
        lpClassName: *const WCHAR,
        lpWindowName: *const WCHAR,
        dwStyle: u32,
        x: i32,
        y: i32,
        nWidth: i32,
        nHeight: i32,
        hWndParent: HANDLE,
        hMenu: HANDLE,
        hInstance: HANDLE,
        lpParam: *mut c_void,
    ) -> HANDLE;
    pub fn DefWindowProcW(hWnd: HANDLE, Msg: u32, wParam: usize, lParam: isize) -> isize;
    pub fn DestroyWindow(hWnd: HANDLE) -> BOOL;
    pub fn ShowWindow(hWnd: HANDLE, nCmdShow: i32) -> BOOL;
    pub fn UpdateWindow(hWnd: HANDLE) -> BOOL;
    pub fn GetMessageW(
        lpMsg: *mut MSG,
        hWnd: HANDLE,
        wMsgFilterMin: u32,
        wMsgFilterMax: u32,
    ) -> BOOL;
    pub fn TranslateMessage(lpMsg: *const MSG) -> BOOL;
    pub fn DispatchMessageW(lpMsg: *const MSG) -> isize;
    pub fn PostQuitMessage(nExitCode: i32);
    pub fn PostMessageW(hWnd: HANDLE, Msg: u32, wParam: usize, lParam: isize) -> BOOL;
    pub fn SendMessageW(hWnd: HANDLE, Msg: u32, wParam: usize, lParam: isize) -> isize;
    pub fn SetTimer(
        hWnd: HANDLE,
        nIDEvent: usize,
        uElapse: u32,
        lpTimerFunc: *const c_void,
    ) -> usize;
    pub fn KillTimer(hWnd: HANDLE, uIDEvent: usize) -> BOOL;
    pub fn InvalidateRect(hWnd: HANDLE, lpRect: *const RECT, bErase: BOOL) -> BOOL;
    pub fn BeginPaint(hWnd: HANDLE, lpPaint: *mut PAINTSTRUCT) -> HANDLE;
    pub fn EndPaint(hWnd: HANDLE, lpPaint: *const PAINTSTRUCT) -> BOOL;
    pub fn GetClientRect(hWnd: HANDLE, lpRect: *mut RECT) -> BOOL;
    pub fn LoadCursorW(hInstance: HANDLE, lpCursorName: *const WCHAR) -> HANDLE;
    pub fn LoadIconW(hInstance: HANDLE, lpIconName: *const WCHAR) -> HANDLE;
    pub fn SetForegroundWindow(hWnd: HANDLE) -> BOOL;
    pub fn GetCursorPos(lpPoint: *mut POINT) -> BOOL;
    pub fn CreatePopupMenu() -> HANDLE;
    pub fn AppendMenuW(
        hMenu: HANDLE,
        uFlags: u32,
        uIDNewItem: usize,
        lpNewItem: *const WCHAR,
    ) -> BOOL;
    pub fn TrackPopupMenu(
        hMenu: HANDLE,
        uFlags: u32,
        x: i32,
        y: i32,
        nReserved: i32,
        hWnd: HANDLE,
        prcRect: *const RECT,
    ) -> BOOL;
    pub fn DestroyMenu(hMenu: HANDLE) -> BOOL;
    pub fn SetWindowTextW(hWnd: HANDLE, lpString: *const WCHAR) -> BOOL;
    pub fn SetWindowPos(
        hWnd: HANDLE,
        hWndInsertAfter: HANDLE,
        X: i32,
        Y: i32,
        cx: i32,
        cy: i32,
        uFlags: u32,
    ) -> BOOL;
    pub fn FillRect(hDC: HANDLE, lprc: *const RECT, hbr: HANDLE) -> i32;
    pub fn DrawTextW(
        hdc: HANDLE,
        lpchText: *const WCHAR,
        cchText: i32,
        lprc: *mut RECT,
        format: u32,
    ) -> i32;
    pub fn CreateIconIndirect(piconinfo: *const ICONINFO) -> HANDLE;
    pub fn DestroyIcon(hIcon: HANDLE) -> BOOL;
    pub fn GetDC(hWnd: HANDLE) -> HANDLE;
    pub fn ReleaseDC(hWnd: HANDLE, hDC: HANDLE) -> i32;
    pub fn GetSystemMetrics(nIndex: i32) -> i32;
    pub fn MessageBeep(uType: u32) -> BOOL;
    pub fn IsWindowVisible(hWnd: HANDLE) -> BOOL;
    pub fn FindWindowW(lpClassName: *const WCHAR, lpWindowName: *const WCHAR) -> HANDLE;
    pub fn MessageBoxW(
        hWnd: HANDLE,
        lpText: *const WCHAR,
        lpCaption: *const WCHAR,
        uType: u32,
    ) -> i32;
}

pub const MB_ICONERROR: u32 = 0x0000_0010;
pub const MF_CHECKED: u32 = 0x0000_0008;

#[link(name = "gdi32")]
extern "system" {
    pub fn CreateSolidBrush(color: u32) -> HANDLE;
    pub fn DeleteObject(ho: HANDLE) -> BOOL;
    pub fn SelectObject(hdc: HANDLE, h: HANDLE) -> HANDLE;
    pub fn CreateFontW(
        cHeight: i32,
        cWidth: i32,
        cEscapement: i32,
        cOrientation: i32,
        cWeight: i32,
        bItalic: DWORD,
        bUnderline: DWORD,
        bStrikeOut: DWORD,
        iCharSet: DWORD,
        iOutPrecision: DWORD,
        iClipPrecision: DWORD,
        iQuality: DWORD,
        iPitchAndFamily: DWORD,
        pszFaceName: *const WCHAR,
    ) -> HANDLE;
    pub fn SetTextColor(hdc: HANDLE, color: u32) -> u32;
    pub fn SetBkMode(hdc: HANDLE, mode: i32) -> i32;
    pub fn RoundRect(hdc: HANDLE, l: i32, t: i32, r: i32, b: i32, w: i32, h: i32) -> BOOL;
    pub fn Rectangle(hdc: HANDLE, l: i32, t: i32, r: i32, b: i32) -> BOOL;
    pub fn MoveToEx(hdc: HANDLE, x: i32, y: i32, p: *mut POINT) -> BOOL;
    pub fn LineTo(hdc: HANDLE, x: i32, y: i32) -> BOOL;
    pub fn CreateCompatibleDC(hdc: HANDLE) -> HANDLE;
    pub fn DeleteDC(hdc: HANDLE) -> BOOL;
    pub fn CreateDIBSection(
        hdc: HANDLE,
        pbmi: *const BITMAPINFO,
        usage: u32,
        ppvBits: *mut *mut c_void,
        hSection: HANDLE,
        offset: u32,
    ) -> HANDLE;
    pub fn CreateBitmap(
        nWidth: i32,
        nHeight: i32,
        nPlanes: u32,
        nBitCount: u32,
        lpBits: *const c_void,
    ) -> HANDLE;
    pub fn GetStockObject(i: i32) -> HANDLE;
}

pub const NULL_PEN: i32 = 8;
pub const ERROR_ALREADY_EXISTS: u32 = 183;

#[link(name = "shell32")]
extern "system" {
    pub fn Shell_NotifyIconW(dwMessage: u32, lpData: *const NOTIFYICONDATAW) -> BOOL;
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SYSTEMTIME {
    pub wYear: u16,
    pub wMonth: u16,
    pub wDayOfWeek: u16,
    pub wDay: u16,
    pub wHour: u16,
    pub wMinute: u16,
    pub wSecond: u16,
    pub wMilliseconds: u16,
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(lpModuleName: *const WCHAR) -> HANDLE;
    pub fn CreateMutexW(
        lpMutexAttributes: *const c_void,
        bInitialOwner: BOOL,
        lpName: *const WCHAR,
    ) -> HANDLE;
    pub fn GetLastError() -> u32;
    pub fn SetLastError(dwErrCode: u32);
    pub fn GetLocalTime(lpSystemTime: *mut SYSTEMTIME);
}

/// Copy a Rust string into a fixed-size UTF-16 field, NUL terminated.
pub fn fill_wide(dst: &mut [u16], src: &str) {
    let v: Vec<u16> = src.encode_utf16().collect();
    let n = v.len().min(dst.len().saturating_sub(1));
    dst[..n].copy_from_slice(&v[..n]);
    dst[n] = 0;
}

pub struct Fonts {
    pub big: HANDLE,
    pub mid: HANDLE,
    pub small: HANDLE,
}

impl Fonts {
    pub fn new() -> Self {
        unsafe {
            Self {
                big: CreateFontW(
                    -52,
                    0,
                    0,
                    0,
                    FW_BOLD,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET,
                    0,
                    0,
                    CLEARTYPE_QUALITY,
                    0,
                    wide("Segoe UI").as_ptr(),
                ),
                mid: CreateFontW(
                    -17,
                    0,
                    0,
                    0,
                    400,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET,
                    0,
                    0,
                    CLEARTYPE_QUALITY,
                    0,
                    wide("Microsoft YaHei UI").as_ptr(),
                ),
                small: CreateFontW(
                    -13,
                    0,
                    0,
                    0,
                    400,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET,
                    0,
                    0,
                    CLEARTYPE_QUALITY,
                    0,
                    wide("Microsoft YaHei UI").as_ptr(),
                ),
            }
        }
    }
}

impl Drop for Fonts {
    fn drop(&mut self) {
        unsafe {
            DeleteObject(self.big);
            DeleteObject(self.mid);
            DeleteObject(self.small);
        }
    }
}
