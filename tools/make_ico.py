"""Generate a 32x32 .ico for the shortcut, without any image library.

Draws a rounded green tile with a white battery glyph, then writes a
single-image ICO (32bpp BMP payload + AND mask).
"""

import struct
import sys

S = 32
GREEN = (29, 158, 117)  # #1D9E75, same accent as the tray icon
WHITE = (255, 255, 255)


def inside_round_rect(x, y, size=S - 1, r=8):
    if x < 0 or y < 0 or x > size or y > size:
        return False
    cx = min(max(x, r), size - r)
    cy = min(max(y, r), size - r)
    dx, dy = x - cx, y - cy
    return dx * dx + dy * dy <= r * r


def build_pixels():
    px = [[(0, 0, 0, 0)] * S for _ in range(S)]
    for y in range(S):
        for x in range(S):
            if inside_round_rect(x, y):
                px[y][x] = (*GREEN, 255)

    # battery body outline
    for y in range(10, 23):
        for x in range(7, 22):
            if x in (7, 21) or y in (10, 22):
                px[y][x] = (*WHITE, 255)
    # positive terminal
    for y in range(14, 19):
        for x in range(22, 25):
            px[y][x] = (*WHITE, 255)
    return px


def build_ico(pixels):
    rows = []
    for y in reversed(range(S)):  # ICO stores bottom-up
        row = bytearray()
        for x in range(S):
            r, g, b, a = pixels[y][x]
            row += bytes((b, g, r, a))
        rows.append(bytes(row))
    xor = b"".join(rows)
    and_mask = b"\x00" * (S * 4)  # 32 bits/row, 32 rows

    header = struct.pack(
        "<IiiHHIIiiII",
        40, S, S * 2, 1, 32, 0, len(xor), 0, 0, 0, 0,
    )
    payload = header + xor + and_mask

    icondir = struct.pack("<HHH", 0, 1, 1)
    entry = struct.pack(
        "<BBBBHHII", S, S, 0, 0, 1, 32, len(payload), 6 + 16
    )
    return icondir + entry + payload


if __name__ == "__main__":
    out = sys.argv[1] if len(sys.argv) > 1 else "atk-battery.ico"
    data = build_ico(build_pixels())
    with open(out, "wb") as f:
        f.write(data)
    print(f"{out} {len(data)} bytes")
