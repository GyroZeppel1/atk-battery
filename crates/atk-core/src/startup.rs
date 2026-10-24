//! Shortcut creation: start-with-Windows entry and desktop launcher.
//!
//! The Startup entry uses a shortcut rather than a Run key: Task Manager shows
//! it under "Startup" with a readable name and the user can disable it there.
//! The program only ever creates or removes shortcuts on explicit request - it
//! never "fixes" state behind the user's back.

use crate::win::{wide, GUID, HANDLE};
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::ptr;

#[link(name = "ole32")]
extern "system" {
    fn CoInitializeEx(pvReserved: *const c_void, dwCoInit: u32) -> i32;
    fn CoCreateInstance(
        rclsid: *const GUID,
        pUnkOuter: *mut c_void,
        dwClsContext: u32,
        riid: *const GUID,
        ppv: *mut *mut c_void,
    ) -> i32;
    fn CoUninitialize();
    fn CoTaskMemFree(pv: *mut c_void);
}

#[link(name = "shell32")]
extern "system" {
    fn SHGetKnownFolderPath(
        rfid: *const GUID,
        dwFlags: u32,
        hToken: HANDLE,
        ppszPath: *mut *mut u16,
    ) -> i32;
}

const COINIT_APARTMENTTHREADED: u32 = 0x2;
const CLSCTX_INPROC_SERVER: u32 = 0x1;

const CLSID_SHELLLINK: GUID = GUID {
    Data1: 0x0002_1401,
    Data2: 0x0000,
    Data3: 0x0000,
    Data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};
const IID_ISHELLLINKW: GUID = GUID {
    Data1: 0x0002_14F9,
    Data2: 0x0000,
    Data3: 0x0000,
    Data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};
const IID_IPERSISTFILE: GUID = GUID {
    Data1: 0x0000_010B,
    Data2: 0x0000,
    Data3: 0x0000,
    Data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};
/// {B4BFCC3A-DB2C-424C-B029-7FE99A87C641}
const FOLDERID_DESKTOP: GUID = GUID {
    Data1: 0xB4BF_CC3A,
    Data2: 0xDB2C,
    Data3: 0x424C,
    Data4: [0xB0, 0x29, 0x7F, 0xE9, 0x9A, 0x87, 0xC6, 0x41],
};

type FnStr = unsafe extern "system" fn(*mut c_void, *const u16) -> i32;
type FnIcon = unsafe extern "system" fn(*mut c_void, *const u16, i32) -> i32;
type FnGetStr = unsafe extern "system" fn(*mut c_void, *mut u16, i32) -> i32;
type FnQi = unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32;
type FnSave = unsafe extern "system" fn(*mut c_void, *const u16, i32) -> i32;
type FnRelease = unsafe extern "system" fn(*mut c_void) -> u32;

unsafe fn method<T: Copy>(obj: *mut c_void, index: usize) -> T {
    let vtbl = *(obj as *mut *const *const c_void);
    let f = *vtbl.add(index);
    std::mem::transmute_copy(&f)
}

unsafe fn read_wide(p: *const u16) -> String {
    let mut n = 0usize;
    while n < 32768 && *p.add(n) != 0 {
        n += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(p, n))
}

/// Real desktop path, following a OneDrive redirect if there is one.
pub fn desktop_dir() -> Option<PathBuf> {
    unsafe {
        let mut p: *mut u16 = ptr::null_mut();
        let hr = SHGetKnownFolderPath(&FOLDERID_DESKTOP, 0, ptr::null_mut(), &mut p);
        if hr < 0 || p.is_null() {
            return None;
        }
        let s = read_wide(p);
        CoTaskMemFree(p as *mut c_void);
        Some(PathBuf::from(s))
    }
}

pub fn startup_lnk_path() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA")?;
    let mut p = PathBuf::from(appdata);
    p.push("Microsoft");
    p.push("Windows");
    p.push("Start Menu");
    p.push("Programs");
    p.push("Startup");
    p.push("ATK 电量.lnk");
    Some(p)
}

pub fn desktop_lnk_path() -> Option<PathBuf> {
    desktop_dir().map(|d| d.join("ATK 电量.lnk"))
}

/// Reads the real filesystem state, so a shortcut the user deleted by hand
/// shows up as missing on the next menu open.
pub fn is_enabled() -> bool {
    startup_lnk_path().map(|p| p.exists()).unwrap_or(false)
}

/// Prefer the tray binary next to the caller; fall back to the caller itself.
fn target_exe() -> PathBuf {
    let cur = std::env::current_exe().unwrap_or_default();
    if let Some(dir) = cur.parent() {
        let tray = dir.join("atk-tray.exe");
        if tray.exists() {
            return tray;
        }
    }
    cur
}

/// Create a shortcut at `path` pointing at the tray binary.
///
/// `args` is passed verbatim to the target: the Startup entry uses `--tray` so
/// login starts silently, the desktop entry passes none so a double-click
/// shows the panel.
pub fn create_shortcut(path: &Path, args: &str, desc: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("create dir: {e}"))?;
    }
    unsafe { create_lnk(path, args, desc) }
}

pub fn set_enabled(enable: bool) -> Result<PathBuf, String> {
    let path = startup_lnk_path().ok_or_else(|| "APPDATA is not set".to_string())?;
    if enable {
        create_shortcut(&path, "--tray", "ATK 鼠标电量（开机启动）")?;
    } else if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("remove shortcut: {e}"))?;
    }
    Ok(path)
}

unsafe fn create_lnk(path: &Path, args: &str, desc: &str) -> Result<(), String> {
    let hr = CoInitializeEx(ptr::null(), COINIT_APARTMENTTHREADED);
    let initialized = hr == 0 || hr == 1;
    if !initialized && (hr as u32) != 0x8001_0106 {
        return Err(format!("CoInitializeEx 0x{:08X}", hr as u32));
    }

    let mut link: *mut c_void = ptr::null_mut();
    let hr = CoCreateInstance(
        &CLSID_SHELLLINK,
        ptr::null_mut(),
        CLSCTX_INPROC_SERVER,
        &IID_ISHELLLINKW,
        &mut link,
    );
    if hr < 0 || link.is_null() {
        if initialized {
            CoUninitialize();
        }
        return Err(format!("CoCreateInstance(ShellLink) 0x{:08X}", hr as u32));
    }

    let target = target_exe();
    let target_w = wide(&target.to_string_lossy());
    let set_path: FnStr = method(link, 20);
    set_path(link, target_w.as_ptr());

    if let Some(dir) = target.parent() {
        let dir_w = wide(&dir.to_string_lossy());
        let set_dir: FnStr = method(link, 9);
        set_dir(link, dir_w.as_ptr());
    }

    let desc_w = wide(desc);
    let set_desc: FnStr = method(link, 7);
    set_desc(link, desc_w.as_ptr());

    // The exe carries no icon resource, so point the shortcut at the shipped
    // .ico when it is there - otherwise Explorer shows a blank generic icon.
    if let Some(dir) = target.parent() {
        let ico = dir.join("atk-battery.ico");
        if ico.exists() {
            let ico_w = wide(&ico.to_string_lossy());
            let set_icon: FnIcon = method(link, 17);
            let hr = set_icon(link, ico_w.as_ptr(), 0);
            if hr < 0 {
                let rel: FnRelease = method(link, 2);
                rel(link);
                if initialized {
                    CoUninitialize();
                }
                return Err(format!(
                    "SetIconLocation 0x{:08X} ({})",
                    hr as u32,
                    ico.display()
                ));
            }
        }
    }

    if !args.is_empty() {
        let args_w = wide(args);
        let set_args: FnStr = method(link, 11);
        let hr = set_args(link, args_w.as_ptr());
        if hr < 0 {
            let rel: FnRelease = method(link, 2);
            rel(link);
            if initialized {
                CoUninitialize();
            }
            return Err(format!("SetArguments 0x{:08X}", hr as u32));
        }
        // Round-trip check: a silently dropped argument would mean the panel
        // pops up at every login.
        let get_args: FnGetStr = method(link, 10);
        let mut buf = [0u16; 64];
        let hr = get_args(link, buf.as_mut_ptr(), buf.len() as i32);
        if hr < 0 || buf[0] == 0 {
            let rel: FnRelease = method(link, 2);
            rel(link);
            if initialized {
                CoUninitialize();
            }
            return Err(format!("GetArguments round-trip 0x{:08X}", hr as u32));
        }
    }

    let mut persist: *mut c_void = ptr::null_mut();
    let qi: FnQi = method(link, 0);
    let hr = qi(link, &IID_IPERSISTFILE, &mut persist);
    if hr < 0 || persist.is_null() {
        let rel: FnRelease = method(link, 2);
        rel(link);
        if initialized {
            CoUninitialize();
        }
        return Err(format!("QueryInterface(IPersistFile) 0x{:08X}", hr as u32));
    }

    let path_w = wide(&path.to_string_lossy());
    let save: FnSave = method(persist, 6);
    let hr = save(persist, path_w.as_ptr(), 1);

    let rel: FnRelease = method(persist, 2);
    rel(persist);
    let rel: FnRelease = method(link, 2);
    rel(link);
    if initialized {
        CoUninitialize();
    }

    if hr < 0 {
        Err(format!("IPersistFile::Save 0x{:08X}", hr as u32))
    } else {
        Ok(())
    }
}
