"""Render the ring gauge at several levels so the design can be judged
without waiting for the mouse to wake up. Mirrors the algorithm in
crates/atk-tray/src/icon.rs."""

import math
import sys

S = 32
MID, HALF, FEATHER = 11.5, 2.1, 0.9
TRACK = (214, 214, 214)
GREEN, AMBER, RED, BLUE, GREY = (29, 158, 117), (186, 117, 23), (216, 90, 48), (55, 138, 221), (136, 135, 128)


def color_for(pct, charging=False):
    if charging:
        return BLUE
    if pct >= 60:
        return GREEN
    if pct >= 30:
        return AMBER
    return RED


def render(pct, charging=False, idle=False):
    col = GREY if idle else color_for(pct, charging)
    px = [[(0, 0, 0, 0)] * S for _ in range(S)]
    for y in range(S):
        for x in range(S):
            dx, dy = x + 0.5 - 16.0, y + 0.5 - 16.0
            d = math.hypot(dx, dy)
            band = abs(d - MID)
            if band <= HALF + FEATHER:
                ang = math.degrees(math.atan2(dy, dx)) + 90.0
                if ang < 0:
                    ang += 360.0
                on = idle or ang <= pct * 3.6
                r, g, b = col if on else TRACK
                a = 255 if band <= HALF else max(0, min(255, int((HALF + FEATHER - band) / FEATHER * 255)))
                px[y][x] = (r, g, b, a)
            elif idle and d < 3.0:
                a = 255 if d < 2.2 else max(0, min(255, int((3.0 - d) / 0.8 * 255)))
                px[y][x] = (*GREY, a)
    return px


def strip(items, scale, out):
    """items: list of (label, pixels) laid out horizontally."""
    n = len(items)
    w, h = S * n, S
    canvas = [[(255, 255, 255, 255)] * w for _ in range(h)]
    for i, (_, pix) in enumerate(items):
        for y in range(S):
            for x in range(S):
                r, g, b, a = pix[y][x]
                if a > 0:
                    # composite over white for preview purposes
                    al = a / 255.0
                    canvas[y][i * S + x] = (
                        int(r * al + 255 * (1 - al)),
                        int(g * al + 255 * (1 - al)),
                        int(b * al + 255 * (1 - al)),
                        255,
                    )
    W, H = w * scale, h * scale
    data = bytearray()
    for y in reversed(range(h)):
        row = bytearray()
        for x in range(w):
            r, g, b, _ = canvas[y][x]
            row += bytes((b, g, r, 255)) * scale
        data += row * scale
    size = len(data)
    with open(out, "wb") as f:
        f.write(b"BM")
        f.write((14 + 40 + size).to_bytes(4, "little"))
        f.write(b"\x00" * 4)
        f.write((14 + 40).to_bytes(4, "little"))
        f.write((40).to_bytes(4, "little"))
        f.write(W.to_bytes(4, "little", signed=True))
        f.write((-H).to_bytes(4, "little", signed=True))
        f.write((1).to_bytes(2, "little"))
        f.write((32).to_bytes(2, "little"))
        f.write((0).to_bytes(4, "little"))
        f.write(size.to_bytes(4, "little"))
        f.write((2835).to_bytes(4, "little"))
        f.write((2835).to_bytes(4, "little"))
        f.write((0).to_bytes(4, "little"))
        f.write((0).to_bytes(4, "little"))
        f.write(data)
    return W, H


if __name__ == "__main__":
    out = sys.argv[1] if len(sys.argv) > 1 else "rings.bmp"
    items = [
        ("100", render(100)),
        ("75", render(75)),
        ("60", render(60)),
        ("45", render(45)),
        ("25", render(25)),
        ("8", render(8)),
        ("chg", render(60, charging=True)),
        ("idle", render(0, idle=True)),
    ]
    print(strip(items, 6, out))
