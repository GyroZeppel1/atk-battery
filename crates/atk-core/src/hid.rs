//! Device discovery and overlapped HID transactions.

use crate::win::*;
use std::ffi::c_void;
use std::ptr;

/// One HID collection as exposed by the driver.
#[derive(Debug, Clone)]
pub struct Collection {
    pub path: String,
    pub vid: u16,
    pub pid: u16,
    pub version: u16,
    pub usage_page: u16,
    pub usage: u16,
    pub input_len: u16,
    pub output_len: u16,
    pub feature_len: u16,
}

unsafe fn read_utf16(p: *const u16) -> String {
    let mut n = 0usize;
    while n < 4096 && *p.add(n) != 0 {
        n += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(p, n))
}

/// Enumerate every present HID collection. Opened with zero access so that
/// keyboard/mouse top-level collections can be inspected without claiming
/// input the system already owns.
pub fn enumerate() -> Vec<Collection> {
    let mut out = Vec::new();
    unsafe {
        let hdev = SetupDiGetClassDevsW(
            &GUID_DEVINTERFACE_HID,
            ptr::null(),
            ptr::null_mut(),
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        );
        if hdev.is_null() || hdev == invalid_handle() {
            return out;
        }

        let mut idx = 0u32;
        loop {
            let mut did: SP_DEVICE_INTERFACE_DATA = std::mem::zeroed();
            did.cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;
            if SetupDiEnumDeviceInterfaces(hdev, ptr::null(), &GUID_DEVINTERFACE_HID, idx, &mut did)
                == 0
            {
                break;
            }
            idx += 1;
            if let Some(c) = inspect(hdev, &mut did) {
                out.push(c);
            }
        }
        SetupDiDestroyDeviceInfoList(hdev);
    }
    out
}

unsafe fn inspect(hdev: HANDLE, did: &mut SP_DEVICE_INTERFACE_DATA) -> Option<Collection> {
    let mut need = 0u32;
    SetupDiGetDeviceInterfaceDetailW(hdev, did, ptr::null_mut(), 0, &mut need, ptr::null_mut());
    if need == 0 {
        return None;
    }

    let mut buf = vec![0u8; need as usize];
    let struct_size: u32 = if std::mem::size_of::<usize>() == 8 {
        8
    } else {
        6
    };
    *(buf.as_mut_ptr() as *mut u32) = struct_size;
    let ok = SetupDiGetDeviceInterfaceDetailW(
        hdev,
        did,
        buf.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W,
        need,
        &mut need,
        ptr::null_mut(),
    );
    if ok == 0 {
        return None;
    }
    // Path lives at offset 4, not at the struct size.
    let path = read_utf16(buf.as_ptr().add(4) as *const u16);

    let w = wide(&path);
    let h = CreateFileW(
        w.as_ptr(),
        0,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        ptr::null(),
        OPEN_EXISTING,
        0,
        ptr::null_mut(),
    );
    if h.is_null() || h == invalid_handle() {
        return None;
    }

    let api = match crate::hidapi::get() {
        Some(a) => a,
        None => {
            CloseHandle(h);
            return None;
        }
    };

    let mut attrs: HIDD_ATTRIBUTES = std::mem::zeroed();
    attrs.Size = std::mem::size_of::<HIDD_ATTRIBUTES>() as u32;
    let attr_ok = (api.get_attributes)(h, &mut attrs);

    let mut pp: *mut c_void = ptr::null_mut();
    let mut caps: HIDP_CAPS = std::mem::zeroed();
    let caps_ok = (api.get_preparsed)(h, &mut pp);
    if caps_ok != 0 {
        (api.get_caps)(pp, &mut caps);
        (api.free_preparsed)(pp);
    }
    CloseHandle(h);

    if attr_ok == 0 || caps_ok == 0 {
        return None;
    }

    Some(Collection {
        path,
        vid: attrs.VendorID,
        pid: attrs.ProductID,
        version: attrs.VersionNumber,
        usage_page: caps.UsagePage,
        usage: caps.Usage,
        input_len: caps.InputReportByteLength,
        output_len: caps.OutputReportByteLength,
        feature_len: caps.FeatureReportByteLength,
    })
}

/// A HID handle used strictly for request/response transactions.
///
/// Every operation is overlapped and serialised on one thread. Blocking
/// `ReadFile` on the same handle that also issues writes is what kills
/// processes outright on these vendor collections - never reintroduce it.
pub struct Endpoint {
    handle: HANDLE,
    event: HANDLE,
}

impl Endpoint {
    pub fn open(path: &str, write: bool) -> std::io::Result<Self> {
        let w = wide(path);
        let access = if write {
            GENERIC_READ | GENERIC_WRITE
        } else {
            0
        };
        unsafe {
            let h = CreateFileW(
                w.as_ptr(),
                access,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                ptr::null_mut(),
            );
            if h.is_null() || h == invalid_handle() {
                return Err(std::io::Error::last_os_error());
            }
            let ev = CreateEventW(ptr::null(), 1, 0, ptr::null());
            if ev.is_null() || ev == invalid_handle() {
                let e = std::io::Error::last_os_error();
                CloseHandle(h);
                return Err(e);
            }
            Ok(Self {
                handle: h,
                event: ev,
            })
        }
    }

    pub fn write(&self, data: &[u8], timeout_ms: u32) -> std::io::Result<()> {
        unsafe {
            let mut ov: OVERLAPPED = std::mem::zeroed();
            ov.hEvent = self.event;
            ResetEvent(self.event);
            let mut written = 0u32;
            let ok = WriteFile(
                self.handle,
                data.as_ptr() as *const c_void,
                data.len() as u32,
                &mut written,
                &mut ov,
            );
            if ok == 0 {
                let err = GetLastError();
                if err != ERROR_IO_PENDING {
                    return Err(std::io::Error::from_raw_os_error(err as i32));
                }
                if WaitForSingleObject(self.event, timeout_ms) != WAIT_OBJECT_0 {
                    CancelIoEx(self.handle, &mut ov);
                    WaitForSingleObject(self.event, 50);
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "write timed out",
                    ));
                }
            }
            let mut got = 0u32;
            if GetOverlappedResult(self.handle, &mut ov, &mut got, 0) == 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        }
    }

    pub fn read(&self, size: usize, timeout_ms: u32) -> std::io::Result<Vec<u8>> {
        unsafe {
            let mut ov: OVERLAPPED = std::mem::zeroed();
            ov.hEvent = self.event;
            ResetEvent(self.event);
            let mut buf = vec![0u8; size];
            let mut got = 0u32;
            let ok = ReadFile(
                self.handle,
                buf.as_mut_ptr() as *mut c_void,
                size as u32,
                &mut got,
                &mut ov,
            );
            if ok == 0 {
                let err = GetLastError();
                if err != ERROR_IO_PENDING {
                    return Err(std::io::Error::from_raw_os_error(err as i32));
                }
                if WaitForSingleObject(self.event, timeout_ms) != WAIT_OBJECT_0 {
                    CancelIoEx(self.handle, &mut ov);
                    WaitForSingleObject(self.event, 50);
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "read timed out",
                    ));
                }
            }
            if GetOverlappedResult(self.handle, &mut ov, &mut got, 0) == 0 {
                return Err(std::io::Error::last_os_error());
            }
            buf.truncate(got as usize);
            Ok(buf)
        }
    }

    /// Discard stale replies left in the vendor queue by an earlier query.
    /// Bounded: stops at the first timeout, so an idle device costs ~12 ms.
    pub fn drain(&self, size: usize, rounds: usize) {
        for _ in 0..rounds {
            if self.read(size, 12).is_err() {
                break;
            }
        }
    }
}

impl Drop for Endpoint {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle);
            if !self.event.is_null() && self.event != invalid_handle() {
                CloseHandle(self.event);
            }
        }
    }
}
