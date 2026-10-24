"""Grab the primary screen to a BMP file using only Win32 GDI.

No third-party dependency, so it works on a bare Python install. Used to
visually verify the tray panel while developing.
"""

import ctypes
import sys
from ctypes import wintypes as wt

user32 = ctypes.WinDLL("user32", use_last_error=True)
gdi32 = ctypes.WinDLL("gdi32", use_last_error=True)

SRCCOPY = 0x00CC0020
DIB_RGB_COLORS = 0


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


user32.GetDC.restype = ctypes.c_void_p
user32.GetDC.argtypes = [ctypes.c_void_p]
user32.ReleaseDC.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
user32.GetSystemMetrics.restype = ctypes.c_int
user32.GetSystemMetrics.argtypes = [ctypes.c_int]
gdi32.CreateCompatibleDC.restype = ctypes.c_void_p
gdi32.CreateCompatibleDC.argtypes = [ctypes.c_void_p]
gdi32.CreateCompatibleBitmap.restype = ctypes.c_void_p
gdi32.CreateCompatibleBitmap.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int]
gdi32.SelectObject.restype = ctypes.c_void_p
gdi32.SelectObject.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
gdi32.BitBlt.restype = wt.BOOL
gdi32.BitBlt.argtypes = [
    ctypes.c_void_p, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int,
    ctypes.c_void_p, ctypes.c_int, ctypes.c_int, wt.DWORD,
]
gdi32.GetDIBits.restype = ctypes.c_int
gdi32.GetDIBits.argtypes = [
    ctypes.c_void_p, ctypes.c_void_p, wt.UINT, wt.UINT, ctypes.c_void_p,
    ctypes.POINTER(BITMAPINFO), wt.UINT,
]
gdi32.DeleteObject.argtypes = [ctypes.c_void_p]
gdi32.DeleteDC.argtypes = [ctypes.c_void_p]

# region = (x, y, w, h) or None for the whole primary screen
def grab(path: str, region=None) -> str:
    user32.SetProcessDPIAware()
    if region is None:
        w = user32.GetSystemMetrics(0)
        h = user32.GetSystemMetrics(1)
        x = y = 0
    else:
        x, y, w, h = region

    screen = user32.GetDC(None)
    mem = gdi32.CreateCompatibleDC(screen)
    bmp = gdi32.CreateCompatibleBitmap(screen, w, h)
    old = gdi32.SelectObject(mem, bmp)
    gdi32.BitBlt(mem, 0, 0, w, h, screen, x, y, SRCCOPY)

    bmi = BITMAPINFO()
    bmi.bmiHeader.biSize = ctypes.sizeof(BITMAPINFOHEADER)
    bmi.bmiHeader.biWidth = w
    bmi.bmiHeader.biHeight = -h  # top-down
    bmi.bmiHeader.biPlanes = 1
    bmi.bmiHeader.biBitCount = 32
    bmi.bmiHeader.biCompression = 0

    stride = w * 4
    size = stride * h
    buf = ctypes.create_string_buffer(size)
    gdi32.GetDIBits(mem, bmp, 0, h, buf, ctypes.byref(bmi), DIB_RGB_COLORS)

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
        f.write(buf.raw)

    gdi32.SelectObject(mem, old)
    gdi32.DeleteObject(bmp)
    gdi32.DeleteDC(mem)
    user32.ReleaseDC(None, screen)
    return f"{path} {w}x{h}"


if __name__ == "__main__":
    out = sys.argv[1] if len(sys.argv) > 1 else "shot.bmp"
    print(grab(out))
