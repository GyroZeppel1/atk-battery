"""Capture a specific window by class name, even when it is occluded.

PrintWindow renders the window through WM_PRINT, so the result does not depend
on z-order or focus. Used to verify the panel rendering during development.
"""

import ctypes
import sys
from ctypes import wintypes as wt

user32 = ctypes.WinDLL("user32", use_last_error=True)
gdi32 = ctypes.WinDLL("gdi32", use_last_error=True)

user32.FindWindowW.restype = ctypes.c_void_p
user32.FindWindowW.argtypes = [wt.LPCWSTR, wt.LPCWSTR]
user32.GetWindowRect.restype = wt.BOOL
user32.GetWindowRect.argtypes = [ctypes.c_void_p, ctypes.POINTER(wt.RECT)]
user32.PrintWindow.restype = wt.BOOL
user32.PrintWindow.argtypes = [ctypes.c_void_p, ctypes.c_void_p, wt.UINT]
user32.GetDC.restype = ctypes.c_void_p
user32.GetDC.argtypes = [ctypes.c_void_p]
user32.ReleaseDC.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
gdi32.CreateCompatibleDC.restype = ctypes.c_void_p
gdi32.CreateCompatibleDC.argtypes = [ctypes.c_void_p]
gdi32.CreateCompatibleBitmap.restype = ctypes.c_void_p
gdi32.CreateCompatibleBitmap.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int]
gdi32.SelectObject.restype = ctypes.c_void_p
gdi32.SelectObject.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
gdi32.DeleteDC.argtypes = [ctypes.c_void_p]
gdi32.DeleteObject.argtypes = [ctypes.c_void_p]
gdi32.GetDIBits.restype = ctypes.c_int
gdi32.GetDIBits.argtypes = [
    ctypes.c_void_p, ctypes.c_void_p, wt.UINT, wt.UINT, ctypes.c_void_p,
    ctypes.c_void_p, wt.UINT,
]


class BITMAPINFOHEADER(ctypes.Structure):
    _fields_ = [
        ("biSize", wt.DWORD), ("biWidth", wt.LONG), ("biHeight", wt.LONG),
        ("biPlanes", wt.WORD), ("biBitCount", wt.WORD), ("biCompression", wt.DWORD),
        ("biSizeImage", wt.DWORD), ("biXPelsPerMeter", wt.LONG),
        ("biYPelsPerMeter", wt.LONG), ("biClrUsed", wt.DWORD),
        ("biClrImportant", wt.DWORD),
    ]


class BITMAPINFO(ctypes.Structure):
    _fields_ = [("bmiHeader", BITMAPINFOHEADER), ("bmiColors", wt.DWORD * 3)]


def save_bmp(path, w, h, data):
    stride = w * 4
    size = stride * h
    with open(path, "wb") as f:
        f.write(b"BM")
        f.write((14 + 40 + size).to_bytes(4, "little"))
        f.write(b"\x00" * 4)
        f.write((14 + 40).to_bytes(4, "little"))
        f.write((40).to_bytes(4, "little"))
        f.write(w.to_bytes(4, "little", signed=True))
        f.write((-h).to_bytes(4, "little", signed=True))
        f.write((1).to_bytes(2, "little"))
        f.write((32).to_bytes(2, "little"))
        f.write((0).to_bytes(4, "little"))
        f.write(size.to_bytes(4, "little"))
        f.write((2835).to_bytes(4, "little"))
        f.write((2835).to_bytes(4, "little"))
        f.write((0).to_bytes(4, "little"))
        f.write((0).to_bytes(4, "little"))
        f.write(data)


def capture(class_name: str, out: str) -> str:
    user32.SetProcessDPIAware()
    hwnd = user32.FindWindowW(class_name, None)
    if not hwnd:
        return f"WINDOW_NOT_FOUND class={class_name}"

    rc = wt.RECT()
    user32.GetWindowRect(hwnd, ctypes.byref(rc))
    w, h = rc.right - rc.left, rc.bottom - rc.top
    if w <= 0 or h <= 0:
        return f"BAD_RECT {w}x{h}"

    screen = user32.GetDC(None)
    mem = gdi32.CreateCompatibleDC(screen)
    bmp = gdi32.CreateCompatibleBitmap(screen, w, h)
    old = gdi32.SelectObject(mem, bmp)
    ok = user32.PrintWindow(hwnd, mem, 2)  # PW_RENDERFULLCONTENT

    bmi = BITMAPINFO()
    bmi.bmiHeader.biSize = ctypes.sizeof(BITMAPINFOHEADER)
    bmi.bmiHeader.biWidth = w
    bmi.bmiHeader.biHeight = -h
    bmi.bmiHeader.biPlanes = 1
    bmi.bmiHeader.biBitCount = 32
    bmi.bmiHeader.biCompression = 0

    buf = ctypes.create_string_buffer(w * 4 * h)
    gdi32.GetDIBits(mem, bmp, 0, h, buf, ctypes.byref(bmi), 0)
    save_bmp(out, w, h, buf.raw)

    gdi32.SelectObject(mem, old)
    gdi32.DeleteObject(bmp)
    gdi32.DeleteDC(mem)
    user32.ReleaseDC(None, screen)
    return f"{out} {w}x{h} printwindow={ok}"


if __name__ == "__main__":
    cls = sys.argv[1] if len(sys.argv) > 1 else "AtkBatteryPanel"
    out = sys.argv[2] if len(sys.argv) > 2 else "panel.bmp"
    print(capture(cls, out))
