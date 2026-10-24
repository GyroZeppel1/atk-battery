#!/usr/bin/env python3
"""Read-only HID probe for ATK / VXE 2.4G mice.

Enumerates every HID collection whose vendor id looks like ATK-family hardware,
then tries the two known battery transactions against each vendor collection:

  * Compx / Nordic 52840 : 17-byte report, id 0x08, sub-command 0x04
  * Nordic 54L15         : 64-byte report, id 0x08, command 0x72, sub 0x07

Nothing is written to the device except those two read-only queries. No driver
is installed, no vendor software is launched, no data leaves the machine.

Usage:  python probe_hid.py [--no-transact]
"""

import ctypes
import sys
import uuid
from ctypes import wintypes as wt

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

hid = ctypes.WinDLL("hid.dll")
setupapi = ctypes.WinDLL("setupapi.dll")
kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)

# ---------------------------------------------------------------- constants
GUID_HID = uuid.UUID("{4D1E55B2-F16F-11CF-88CB-001111000030}")
DIGCF_PRESENT = 0x02
DIGCF_DEVICEINTERFACE = 0x10
FILE_SHARE_READ = 0x01
FILE_SHARE_WRITE = 0x02
OPEN_EXISTING = 3
GENERIC_READ = 0x80000000
GENERIC_WRITE = 0x40000000
FILE_FLAG_OVERLAPPED = 0x40000000
ERROR_IO_PENDING = 997
WAIT_OBJECT_0 = 0
WAIT_TIMEOUT = 258
INVALID_HANDLE = ctypes.c_void_p(-1).value

ATK_VIDS = {0x373B, 0x3554, 0x260D, 0x33E4}

# ---------------------------------------------------------------- structures


class GUID(ctypes.Structure):
    _fields_ = [
        ("Data1", wt.DWORD),
        ("Data2", wt.WORD),
        ("Data3", wt.WORD),
        ("Data4", ctypes.c_ubyte * 8),
    ]


class SP_DEVICE_INTERFACE_DATA(ctypes.Structure):
    _fields_ = [
        ("cbSize", wt.DWORD),
        ("InterfaceClassGuid", GUID),
        ("Flags", wt.DWORD),
        ("Reserved", ctypes.c_void_p),
    ]


class SP_DEVICE_INTERFACE_DETAIL_DATA_W(ctypes.Structure):
    _fields_ = [("cbSize", wt.DWORD), ("DevicePath", ctypes.c_wchar * 1)]


class HIDD_ATTRIBUTES(ctypes.Structure):
    _fields_ = [
        ("Size", wt.ULONG),
        ("VendorID", wt.USHORT),
        ("ProductID", wt.USHORT),
        ("VersionNumber", wt.USHORT),
    ]


class HIDP_CAPS(ctypes.Structure):
    _fields_ = [
        ("Usage", wt.USHORT),
        ("UsagePage", wt.USHORT),
        ("InputReportByteLength", wt.USHORT),
        ("OutputReportByteLength", wt.USHORT),
        ("FeatureReportByteLength", wt.USHORT),
        ("Reserved", wt.USHORT * 17),
        ("NumberLinkCollectionNodes", wt.USHORT),
        ("NumberInputButtonCaps", wt.USHORT),
        ("NumberInputValueCaps", wt.USHORT),
        ("NumberInputDataIndices", wt.USHORT),
        ("NumberOutputButtonCaps", wt.USHORT),
        ("NumberOutputValueCaps", wt.USHORT),
        ("NumberOutputDataIndices", wt.USHORT),
        ("NumberFeatureButtonCaps", wt.USHORT),
        ("NumberFeatureValueCaps", wt.USHORT),
        ("NumberFeatureDataIndices", wt.USHORT),
    ]


class OVERLAPPED(ctypes.Structure):
    _fields_ = [
        ("Internal", ctypes.c_void_p),
        ("InternalHigh", ctypes.c_void_p),
        ("Offset", wt.DWORD),
        ("OffsetHigh", wt.DWORD),
        ("hEvent", wt.HANDLE),
    ]


# ---------------------------------------------------------------- prototypes
setupapi.SetupDiGetClassDevsW.restype = ctypes.c_void_p
setupapi.SetupDiGetClassDevsW.argtypes = [
    ctypes.POINTER(GUID), wt.LPCWSTR, wt.HANDLE, wt.DWORD,
]
setupapi.SetupDiEnumDeviceInterfaces.restype = wt.BOOL
setupapi.SetupDiEnumDeviceInterfaces.argtypes = [
    ctypes.c_void_p, ctypes.c_void_p, ctypes.POINTER(GUID), wt.DWORD,
    ctypes.POINTER(SP_DEVICE_INTERFACE_DATA),
]
setupapi.SetupDiGetDeviceInterfaceDetailW.restype = wt.BOOL
setupapi.SetupDiGetDeviceInterfaceDetailW.argtypes = [
    ctypes.c_void_p, ctypes.POINTER(SP_DEVICE_INTERFACE_DATA), ctypes.c_void_p,
    wt.DWORD, ctypes.POINTER(wt.DWORD), ctypes.c_void_p,
]
setupapi.SetupDiDestroyDeviceInfoList.restype = wt.BOOL
setupapi.SetupDiDestroyDeviceInfoList.argtypes = [ctypes.c_void_p]

hid.HidD_GetPreparsedData.restype = wt.BOOL
hid.HidD_GetPreparsedData.argtypes = [wt.HANDLE, ctypes.POINTER(ctypes.c_void_p)]
hid.HidD_FreePreparsedData.restype = wt.BOOL
hid.HidD_FreePreparsedData.argtypes = [ctypes.c_void_p]
hid.HidP_GetCaps.restype = ctypes.c_long
hid.HidP_GetCaps.argtypes = [ctypes.c_void_p, ctypes.POINTER(HIDP_CAPS)]
hid.HidD_GetAttributes.restype = wt.BOOL
hid.HidD_GetAttributes.argtypes = [wt.HANDLE, ctypes.POINTER(HIDD_ATTRIBUTES)]

kernel32.CreateFileW.restype = wt.HANDLE
kernel32.CreateFileW.argtypes = [
    wt.LPCWSTR, wt.DWORD, wt.DWORD, ctypes.c_void_p, wt.DWORD, wt.DWORD, wt.HANDLE,
]
kernel32.CloseHandle.restype = wt.BOOL
kernel32.CloseHandle.argtypes = [wt.HANDLE]
kernel32.CreateEventW.restype = wt.HANDLE
kernel32.CreateEventW.argtypes = [ctypes.c_void_p, wt.BOOL, wt.BOOL, wt.LPCWSTR]
kernel32.ResetEvent.restype = wt.BOOL
kernel32.ResetEvent.argtypes = [wt.HANDLE]
kernel32.WaitForSingleObject.restype = wt.DWORD
kernel32.WaitForSingleObject.argtypes = [wt.HANDLE, wt.DWORD]
kernel32.WriteFile.restype = wt.BOOL
kernel32.WriteFile.argtypes = [
    wt.HANDLE, ctypes.c_char_p, wt.DWORD, ctypes.POINTER(wt.DWORD),
    ctypes.POINTER(OVERLAPPED),
]
kernel32.ReadFile.restype = wt.BOOL
kernel32.ReadFile.argtypes = [
    wt.HANDLE, ctypes.c_char_p, wt.DWORD, ctypes.POINTER(wt.DWORD),
    ctypes.POINTER(OVERLAPPED),
]
kernel32.GetOverlappedResult.restype = wt.BOOL
kernel32.GetOverlappedResult.argtypes = [
    wt.HANDLE, ctypes.POINTER(OVERLAPPED), ctypes.POINTER(wt.DWORD), wt.BOOL,
]
kernel32.CancelIoEx.restype = wt.BOOL
kernel32.CancelIoEx.argtypes = [wt.HANDLE, ctypes.POINTER(OVERLAPPED)]


def guid_struct() -> GUID:
    g = GUID()
    ctypes.memmove(ctypes.byref(g), GUID_HID.bytes_le, 16)
    return g


def hexs(data: bytes) -> str:
    return " ".join(f"{b:02X}" for b in data)


# ---------------------------------------------------------------- enumeration
def enumerate_hid():
    """Yield (path, vid, pid, caps, version) for every HID interface present."""
    guid = guid_struct()
    hdev = setupapi.SetupDiGetClassDevsW(
        ctypes.byref(guid), None, None, DIGCF_PRESENT | DIGCF_DEVICEINTERFACE
    )
    if not hdev or hdev == INVALID_HANDLE:
        raise OSError("SetupDiGetClassDevsW failed")

    did = SP_DEVICE_INTERFACE_DATA()
    did.cbSize = ctypes.sizeof(did)
    out = []
    idx = 0
    while True:
        if not setupapi.SetupDiEnumDeviceInterfaces(
            hdev, None, ctypes.byref(guid), idx, ctypes.byref(did)
        ):
            break
        idx += 1
        need = wt.DWORD(0)
        setupapi.SetupDiGetDeviceInterfaceDetailW(
            hdev, ctypes.byref(did), None, 0, ctypes.byref(need), None
        )
        if need.value == 0:
            continue
        buf = ctypes.create_string_buffer(need.value)
        # cbSize must be the *whole struct* size (8 on x64 / 6 on x86) and the
        # API requires it, but WCHAR only needs 2-byte alignment so the real
        # DevicePath always starts at offset 4. Setting cbSize to the path
        # offset yields ERROR_INVALID_NAME for every device.
        struct_size = 8 if ctypes.sizeof(ctypes.c_void_p) == 8 else 6
        path_off = 4
        ctypes.cast(buf, ctypes.POINTER(wt.DWORD))[0] = struct_size
        if not setupapi.SetupDiGetDeviceInterfaceDetailW(
            hdev, ctypes.byref(did), buf, need.value, ctypes.byref(need), None
        ):
            continue
        path = buf.raw[path_off:].decode("utf-16-le", "ignore").split("\x00", 1)[0]

        h = kernel32.CreateFileW(
            path, 0, FILE_SHARE_READ | FILE_SHARE_WRITE, None, OPEN_EXISTING, 0, None
        )
        if not h or h == INVALID_HANDLE:
            continue
        try:
            attrs = HIDD_ATTRIBUTES()
            attrs.Size = ctypes.sizeof(attrs)
            if not hid.HidD_GetAttributes(h, ctypes.byref(attrs)):
                continue
            pp = ctypes.c_void_p()
            caps = HIDP_CAPS()
            if hid.HidD_GetPreparsedData(h, ctypes.byref(pp)):
                hid.HidP_GetCaps(pp, ctypes.byref(caps))
                hid.HidD_FreePreparsedData(pp)
            else:
                continue
            out.append((path, attrs.VendorID, attrs.ProductID, caps, attrs.VersionNumber))
        finally:
            kernel32.CloseHandle(h)
    setupapi.SetupDiDestroyDeviceInfoList(hdev)
    return out


# ---------------------------------------------------------------- transport
class Endpoint:
    """Overlapped-only HID endpoint: no blocking ReadFile on a shared handle."""

    def __init__(self, path):
        self.handle = kernel32.CreateFileW(
            path,
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED,
            None,
        )
        ok = self.handle not in (None, 0, INVALID_HANDLE)
        self.event = kernel32.CreateEventW(None, True, False, None) if ok else None
        self.closed = not ok

    def write(self, data: bytes, timeout_ms=250) -> bool:
        ov = OVERLAPPED()
        ov.hEvent = self.event
        kernel32.ResetEvent(self.event)
        written = wt.DWORD(0)
        kernel32.SetLastError(0)
        ok = kernel32.WriteFile(
            self.handle, data, len(data), ctypes.byref(written), ctypes.byref(ov)
        )
        if not ok:
            if ctypes.get_last_error() != ERROR_IO_PENDING:
                return False
            if kernel32.WaitForSingleObject(self.event, timeout_ms) != WAIT_OBJECT_0:
                kernel32.CancelIoEx(self.handle, ctypes.byref(ov))
                kernel32.WaitForSingleObject(self.event, 50)
                return False
        got = wt.DWORD(0)
        return bool(
            kernel32.GetOverlappedResult(
                self.handle, ctypes.byref(ov), ctypes.byref(got), False
            )
        )

    def read(self, size: int, timeout_ms=250):
        ov = OVERLAPPED()
        ov.hEvent = self.event
        kernel32.ResetEvent(self.event)
        buf = ctypes.create_string_buffer(size)
        got = wt.DWORD(0)
        kernel32.SetLastError(0)
        ok = kernel32.ReadFile(
            self.handle, buf, size, ctypes.byref(got), ctypes.byref(ov)
        )
        if not ok:
            if ctypes.get_last_error() != ERROR_IO_PENDING:
                return None
            if kernel32.WaitForSingleObject(self.event, timeout_ms) != WAIT_OBJECT_0:
                kernel32.CancelIoEx(self.handle, ctypes.byref(ov))
                kernel32.WaitForSingleObject(self.event, 50)
                return None
        if not kernel32.GetOverlappedResult(
            self.handle, ctypes.byref(ov), ctypes.byref(got), False
        ):
            return None
        return buf.raw[: got.value]

    def drain(self, size: int, rounds=4):
        for _ in range(rounds):
            if self.read(size, 15) is None:
                break

    def close(self):
        if not self.closed:
            kernel32.CloseHandle(self.handle)
            if self.event:
                kernel32.CloseHandle(self.event)
            self.closed = True


# ---------------------------------------------------------------- queries
def try_compx52(ep: Endpoint, in_len: int):
    req = bytearray(17)
    req[0] = 0x08
    req[1] = 0x04
    req[16] = 0x49
    ep.drain(in_len)
    if not ep.write(bytes(req)):
        return None
    res = ep.read(in_len)
    return res


def try_nordic54(ep: Endpoint, in_len: int, link: int):
    req = bytearray(64)
    req[0] = 0x08
    req[1] = 0x7D
    req[2] = 0x72
    req[3] = 0x02
    req[5] = link
    req[6] = 0x07
    req[7] = 0x01
    ep.drain(in_len)
    if not ep.write(bytes(req)):
        return None
    return ep.read(in_len)


def score_compx52(res):
    if not res or len(res) < 8:
        return None
    if sum(res[:17]) & 0xFF != 0x55:
        return None
    pct = res[6]
    if pct > 100:
        return None
    # Voltage is big endian here (little endian in the 64-byte family).
    return pct, res[7], ((res[8] << 8) | res[9])


def score_nordic54(res):
    if not res or len(res) < 8:
        return None
    if res[1] != 0x72 or res[5] != 0x07:
        return None
    if len(res) > 2 and res[2] != 0x00:
        return None
    pct = res[7]
    if pct > 100:
        return None
    mv = (res[8] | (res[9] << 8)) if len(res) >= 10 else 0
    return pct, 0, mv


def main():
    transact = "--no-transact" not in sys.argv
    rows = [r for r in enumerate_hid() if r[1] in ATK_VIDS]
    if not rows:
        print("No ATK-family HID interface found (VID 373B/3554/260D/33E4).")
        return 1

    print(f"Found {len(rows)} HID collection(s) on ATK-family vendor ids.\n")
    print(f"{'vid:pid':>10}  {'ver':>6}  {'usage':>11}  {'in':>3} {'out':>3} {'feat':>4}  path")
    print("-" * 118)
    for path, vid, pid, caps, ver in rows:
        tail = path.split("&col")[-1].split("#")[0]
        print(
            f"{vid:04X}:{pid:04X}  {ver:>6X}  "
            f"{caps.UsagePage:04X}:{caps.Usage:04X}  "
            f"{caps.InputReportByteLength:>3} {caps.OutputReportByteLength:>3} "
            f"{caps.FeatureReportByteLength:>4}  ...&col{tail}"
        )

    if not transact:
        return 0

    vendor = [r for r in rows if r[3].UsagePage >= 0xFF00]
    print(f"\n{len(vendor)} vendor-defined collection(s) to probe.\n")

    solved = []
    for path, vid, pid, caps, ver in vendor:
        in_len = caps.InputReportByteLength
        out_len = caps.OutputReportByteLength
        label = f"{vid:04X}:{pid:04X} {caps.UsagePage:04X}:{caps.Usage:04X}"
        print(f"[{label}] in={in_len} out={out_len} feat={caps.FeatureReportByteLength}")

        ep = Endpoint(path)
        if ep.closed:
            print("   open failed (access denied / in use)\n")
            continue
        try:
            if out_len == 17 and in_len == 17:
                res = try_compx52(ep, in_len)
                print(f"   compx52 -> {hexs(res) if res else 'no reply'}")
                s = score_compx52(res)
                if s:
                    print(f"   *** MATCH compx52: {s[0]}%  flag={s[1]}  {s[2]} mV")
                    solved.append(("compx52", vid, pid, caps.UsagePage, s))
            elif out_len == 64:
                for link in (1, 0):
                    res = try_nordic54(ep, in_len, link)
                    print(f"   nordic54 link={link} -> {hexs(res[:16]) if res else 'no reply'}")
                    s = score_nordic54(res)
                    if s:
                        print(f"   *** MATCH nordic54 link={link}: {s[0]}%  {s[2]} mV")
                        solved.append((f"nordic54/link{link}", vid, pid, caps.UsagePage, s))
            else:
                print("   unhandled report geometry")
        finally:
            ep.close()
        print()

    print("=" * 60)
    if solved:
        for proto, vid, pid, up, (pct, flag, mv) in solved:
            print(f"RESULT {vid:04X}:{pid:04X} up={up:04X} proto={proto} battery={pct}%"
                  + (f" mv={mv}" if mv else ""))
    else:
        print("No protocol answered. Most likely the mouse is asleep -")
        print("move the mouse for a second and run again.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
