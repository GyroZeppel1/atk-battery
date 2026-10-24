//! Minimal hand-written bindings for the Win32 HID stack.
//!
//! Deliberately dependency-free: no `windows-sys`, no `hidapi`, no C shim.
//! Only the 18 entry points this tool actually calls are declared, so the
//! resulting binary links against nothing but `kernel32` / `setupapi` / `hid`.

#![allow(non_snake_case, non_camel_case_types, clippy::upper_case_acronyms)]

use std::ffi::c_void;

pub type HANDLE = *mut c_void;
pub type BOOL = i32;
pub type DWORD = u32;
pub type ULONG = u32;
pub type WCHAR = u16;

pub const DIGCF_PRESENT: DWORD = 0x0000_0002;
pub const DIGCF_DEVICEINTERFACE: DWORD = 0x0000_0010;

pub const GENERIC_READ: DWORD = 0x8000_0000;
pub const GENERIC_WRITE: DWORD = 0x4000_0000;
pub const FILE_SHARE_READ: DWORD = 0x0000_0001;
pub const FILE_SHARE_WRITE: DWORD = 0x0000_0002;
pub const OPEN_EXISTING: DWORD = 3;
pub const FILE_FLAG_OVERLAPPED: DWORD = 0x4000_0000;

pub const ERROR_IO_PENDING: DWORD = 997;
pub const WAIT_OBJECT_0: DWORD = 0;

#[inline]
pub fn invalid_handle() -> HANDLE {
    -1isize as HANDLE
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GUID {
    pub Data1: u32,
    pub Data2: u16,
    pub Data3: u16,
    pub Data4: [u8; 8],
}

/// {4D1E55B2-F16F-11CF-88CB-001111000030} - the HID device interface class.
pub const GUID_DEVINTERFACE_HID: GUID = GUID {
    Data1: 0x4D1E_55B2,
    Data2: 0xF16F,
    Data3: 0x11CF,
    Data4: [0x88, 0xCB, 0x00, 0x11, 0x11, 0x00, 0x00, 0x30],
};

#[repr(C)]
pub struct SP_DEVICE_INTERFACE_DATA {
    pub cbSize: DWORD,
    pub InterfaceClassGuid: GUID,
    pub Flags: DWORD,
    pub Reserved: usize,
}

/// The API requires `cbSize` to be the whole struct (8 on x64, 6 on x86),
/// while the path string actually begins at offset 4 because a single WCHAR
/// only needs two-byte alignment. Do not let a binding generator hide this.
#[repr(C)]
pub struct SP_DEVICE_INTERFACE_DETAIL_DATA_W {
    pub cbSize: DWORD,
    pub DevicePath: [WCHAR; 1],
}

#[repr(C)]
pub struct HIDD_ATTRIBUTES {
    pub Size: ULONG,
    pub VendorID: u16,
    pub ProductID: u16,
    pub VersionNumber: u16,
}

#[repr(C)]
pub struct HIDP_CAPS {
    pub Usage: u16,
    pub UsagePage: u16,
    pub InputReportByteLength: u16,
    pub OutputReportByteLength: u16,
    pub FeatureReportByteLength: u16,
    pub Reserved: [u16; 17],
    pub NumberLinkCollectionNodes: u16,
    pub NumberInputButtonCaps: u16,
    pub NumberInputValueCaps: u16,
    pub NumberInputDataIndices: u16,
    pub NumberOutputButtonCaps: u16,
    pub NumberOutputValueCaps: u16,
    pub NumberOutputDataIndices: u16,
    pub NumberFeatureButtonCaps: u16,
    pub NumberFeatureValueCaps: u16,
    pub NumberFeatureDataIndices: u16,
}

#[repr(C)]
pub struct OVERLAPPED {
    pub Internal: usize,
    pub InternalHigh: usize,
    pub Offset: DWORD,
    pub OffsetHigh: DWORD,
    pub hEvent: HANDLE,
}

#[link(name = "setupapi")]
extern "system" {
    pub fn SetupDiGetClassDevsW(
        ClassGuid: *const GUID,
        Enumerator: *const WCHAR,
        hwndParent: HANDLE,
        Flags: DWORD,
    ) -> HANDLE;

    pub fn SetupDiEnumDeviceInterfaces(
        DeviceInfoSet: HANDLE,
        DeviceInfoData: *const c_void,
        InterfaceClassGuid: *const GUID,
        MemberIndex: DWORD,
        DeviceInterfaceData: *mut SP_DEVICE_INTERFACE_DATA,
    ) -> BOOL;

    pub fn SetupDiGetDeviceInterfaceDetailW(
        DeviceInfoSet: HANDLE,
        DeviceInterfaceData: *mut SP_DEVICE_INTERFACE_DATA,
        DeviceInterfaceDetailData: *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W,
        DeviceInterfaceDetailDataSize: DWORD,
        RequiredSize: *mut DWORD,
        DeviceInfoData: *mut c_void,
    ) -> BOOL;

    pub fn SetupDiDestroyDeviceInfoList(DeviceInfoSet: HANDLE) -> BOOL;
}

// `hid.dll` is resolved at runtime instead of being linked: the MinGW
// self-contained sysroot ships no libhid.a, and generating one would require
// dlltool. Only four entry points are needed, and dynamic lookup keeps the
// link line to kernel32 + setupapi.
#[link(name = "kernel32")]
extern "system" {
    pub fn LoadLibraryW(lpLibFileName: *const WCHAR) -> HANDLE;
    pub fn GetProcAddress(hModule: HANDLE, lpProcName: *const u8) -> *const c_void;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn CreateFileW(
        lpFileName: *const WCHAR,
        dwDesiredAccess: DWORD,
        dwShareMode: DWORD,
        lpSecurityAttributes: *const c_void,
        dwCreationDisposition: DWORD,
        dwFlagsAndAttributes: DWORD,
        hTemplateFile: HANDLE,
    ) -> HANDLE;

    pub fn CloseHandle(hObject: HANDLE) -> BOOL;

    pub fn CreateEventW(
        lpEventAttributes: *const c_void,
        bManualReset: BOOL,
        bInitialState: BOOL,
        lpName: *const WCHAR,
    ) -> HANDLE;

    pub fn ResetEvent(hEvent: HANDLE) -> BOOL;
    pub fn WaitForSingleObject(hHandle: HANDLE, dwMilliseconds: DWORD) -> DWORD;

    pub fn ReadFile(
        hFile: HANDLE,
        lpBuffer: *mut c_void,
        nNumberOfBytesToRead: DWORD,
        lpNumberOfBytesRead: *mut DWORD,
        lpOverlapped: *mut OVERLAPPED,
    ) -> BOOL;

    pub fn WriteFile(
        hFile: HANDLE,
        lpBuffer: *const c_void,
        nNumberOfBytesToWrite: DWORD,
        lpNumberOfBytesWritten: *mut DWORD,
        lpOverlapped: *mut OVERLAPPED,
    ) -> BOOL;

    pub fn GetOverlappedResult(
        hFile: HANDLE,
        lpOverlapped: *mut OVERLAPPED,
        lpNumberOfBytesTransferred: *mut DWORD,
        bWait: BOOL,
    ) -> BOOL;

    pub fn CancelIoEx(hFile: HANDLE, lpOverlapped: *mut OVERLAPPED) -> BOOL;
    pub fn GetLastError() -> DWORD;
}

/// UTF-16, NUL-terminated, as every `...W` entry point expects.
pub fn wide(s: &str) -> Vec<WCHAR> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
