"""Read a .lnk back through Shell COM and print its properties.

Byte-searching the file is unreliable: the strings sit at arbitrary offsets and
a global UTF-16 decode misses every odd-aligned one. Loading through
IPersistFile is the only trustworthy check.
"""

import ctypes
import sys
import uuid
from ctypes import wintypes as wt


class GUID(ctypes.Structure):
    _fields_ = [
        ("Data1", wt.DWORD), ("Data2", wt.WORD),
        ("Data3", wt.WORD), ("Data4", ctypes.c_ubyte * 8),
    ]


def guid(s):
    g = GUID()
    ctypes.memmove(ctypes.byref(g), uuid.UUID(s).bytes_le, 16)
    return g


ole32 = ctypes.WinDLL("ole32")
ole32.CoInitializeEx.argtypes = [ctypes.c_void_p, wt.DWORD]
ole32.CoCreateInstance.argtypes = [
    ctypes.POINTER(GUID), ctypes.c_void_p, wt.DWORD,
    ctypes.POINTER(GUID), ctypes.POINTER(ctypes.c_void_p),
]
ole32.CoCreateInstance.restype = ctypes.c_long
ole32.CoInitializeEx(None, 2)

CLSID = guid("{00021401-0000-0000-C000-000000000046}")
IID_LINK = guid("{000214F9-0000-0000-C000-000000000046}")
IID_PF = guid("{0000010B-0000-0000-C000-000000000046}")

HR = ctypes.c_long
Get2 = ctypes.WINFUNCTYPE(HR, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_int)
Get3 = ctypes.WINFUNCTYPE(HR, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_int, ctypes.POINTER(ctypes.c_int))
QI = ctypes.WINFUNCTYPE(HR, ctypes.c_void_p, ctypes.POINTER(GUID), ctypes.POINTER(ctypes.c_void_p))
LoadFn = ctypes.WINFUNCTYPE(HR, ctypes.c_void_p, ctypes.c_wchar_p, ctypes.c_int)


def slot(obj, idx, proto):
    vt = ctypes.cast(obj, ctypes.POINTER(ctypes.POINTER(ctypes.c_void_p)))[0]
    return proto(vt[idx])


def inspect(path):
    obj = ctypes.c_void_p()
    hr = ole32.CoCreateInstance(
        ctypes.byref(CLSID), None, 1, ctypes.byref(IID_LINK), ctypes.byref(obj)
    )
    if hr < 0:
        return f"CoCreateInstance 0x{hr & 0xffffffff:08X}"
    pf = ctypes.c_void_p()
    slot(obj, 0, QI)(obj, ctypes.byref(IID_PF), ctypes.byref(pf))
    hr = slot(pf, 5, LoadFn)(pf, path, 0)
    if hr < 0:
        return f"Load 0x{hr & 0xffffffff:08X}"

    out = {}
    b = ctypes.create_unicode_buffer(512)
    slot(obj, 3, ctypes.WINFUNCTYPE(HR, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_int, ctypes.c_void_p, wt.DWORD))(
        obj, ctypes.cast(b, ctypes.c_void_p), 512, None, 0
    )
    out["target"] = b.value

    b = ctypes.create_unicode_buffer(512)
    slot(obj, 10, Get2)(obj, ctypes.cast(b, ctypes.c_void_p), 512)
    out["args"] = b.value

    b = ctypes.create_unicode_buffer(512)
    slot(obj, 6, Get2)(obj, ctypes.cast(b, ctypes.c_void_p), 512)
    out["desc"] = b.value

    b = ctypes.create_unicode_buffer(512)
    idx = ctypes.c_int(-1)
    slot(obj, 16, Get3)(obj, ctypes.cast(b, ctypes.c_void_p), 512, ctypes.byref(idx))
    out["icon"] = f"{b.value},{idx.value}" if b.value else ""

    return "\n".join(f"  {k:7}= {v!r}" for k, v in out.items())


if __name__ == "__main__":
    for p in sys.argv[1:]:
        print(p)
        print(inspect(p))
