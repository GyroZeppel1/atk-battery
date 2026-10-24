"""Nearest-neighbour upscale for 32bpp BMP files (no PIL required)."""

import sys


def zoom(src: str, dst: str, factor: int) -> str:
    d = open(src, "rb").read()
    off = int.from_bytes(d[10:14], "little")
    w = int.from_bytes(d[18:22], "little", signed=True)
    h = int.from_bytes(d[22:26], "little", signed=True)
    bpp = int.from_bytes(d[28:30], "little")
    if bpp != 32:
        raise SystemExit(f"only 32bpp supported, got {bpp}")
    top_down = h < 0
    h = abs(h)
    stride = w * 4

    rows = []
    for y in range(h):
        row = d[off + y * stride : off + (y + 1) * stride]
        out = bytearray()
        for x in range(w):
            out += row[x * 4 : x * 4 + 4] * factor
        rows.append(bytes(out))

    data = b"".join(r for r in rows for _ in range(factor))
    nw, nh = w * factor, h * factor
    size = len(data)

    with open(dst, "wb") as f:
        f.write(b"BM")
        f.write((14 + 40 + size).to_bytes(4, "little"))
        f.write(b"\x00" * 4)
        f.write((14 + 40).to_bytes(4, "little"))
        f.write((40).to_bytes(4, "little"))
        f.write(nw.to_bytes(4, "little", signed=True))
        f.write((nh if not top_down else -nh).to_bytes(4, "little", signed=True))
        f.write((1).to_bytes(2, "little"))
        f.write((32).to_bytes(2, "little"))
        f.write((0).to_bytes(4, "little"))
        f.write(size.to_bytes(4, "little"))
        f.write((2835).to_bytes(4, "little"))
        f.write((2835).to_bytes(4, "little"))
        f.write((0).to_bytes(4, "little"))
        f.write((0).to_bytes(4, "little"))
        f.write(data)
    return f"{dst} {nw}x{nh}"


if __name__ == "__main__":
    print(zoom(sys.argv[1], sys.argv[2], int(sys.argv[3]) if len(sys.argv) > 3 else 4))
