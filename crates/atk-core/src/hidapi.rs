//! Runtime binding for the four `hid.dll` entry points this tool needs.
//!
//! Resolved once on first use. If the library or a symbol were missing the
//! device simply reports as unavailable - no panic, no silent zero reading.

use crate::win::*;
use std::ffi::c_void;
use std::sync::OnceLock;

type FnAttrs = unsafe extern "system" fn(HANDLE, *mut HIDD_ATTRIBUTES) -> BOOL;
type FnPreparsed = unsafe extern "system" fn(HANDLE, *mut *mut c_void) -> BOOL;
type FnFreePreparsed = unsafe extern "system" fn(*mut c_void) -> BOOL;
type FnCaps = unsafe extern "system" fn(*mut c_void, *mut HIDP_CAPS) -> i32;

pub struct HidApi {
    pub get_attributes: FnAttrs,
    pub get_preparsed: FnPreparsed,
    pub free_preparsed: FnFreePreparsed,
    pub get_caps: FnCaps,
}

static API: OnceLock<Option<HidApi>> = OnceLock::new();

pub fn get() -> Option<&'static HidApi> {
    API.get_or_init(|| unsafe { load() }).as_ref()
}

unsafe fn load() -> Option<HidApi> {
    let name = wide("hid.dll");
    let lib = LoadLibraryW(name.as_ptr());
    if lib.is_null() || lib == invalid_handle() {
        return None;
    }
    Some(HidApi {
        get_attributes: sym::<FnAttrs>(lib, b"HidD_GetAttributes\0")?,
        get_preparsed: sym::<FnPreparsed>(lib, b"HidD_GetPreparsedData\0")?,
        free_preparsed: sym::<FnFreePreparsed>(lib, b"HidD_FreePreparsedData\0")?,
        get_caps: sym::<FnCaps>(lib, b"HidP_GetCaps\0")?,
    })
}

unsafe fn sym<T: Copy>(lib: HANDLE, name: &[u8]) -> Option<T> {
    let p = GetProcAddress(lib, name.as_ptr());
    if p.is_null() {
        return None;
    }
    Some(std::mem::transmute_copy(&p))
}
