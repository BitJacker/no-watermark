#!/usr/bin/env python3
"""Generate the no-watermark application icons.

The icon is drawn procedurally so that the repository carries no binary blob
that nobody can edit: change the numbers here, re-run the script, and every
size and format is regenerated.

    python packaging/make_icons.py

Outputs PNGs at several sizes plus a Windows .ico containing all of them.
Pass --check to verify the committed assets still match, without writing.
"""

from __future__ import annotations

import math
import struct
import sys
import zlib
from pathlib import Path

ASSETS = Path(__file__).resolve().parent.parent / "assets"

# Palette
BG_TOP = (18, 21, 30)
BG_BOTTOM = (30, 36, 52)
RING = (240, 244, 252)
SLASH = (91, 200, 235)
GHOST = (255, 255, 255)

SIZE = 1024  # supersampled canvas, downsampled to each target size


def lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def draw() -> list[list[tuple[int, int, int, int]]]:
    """Render the icon once at high resolution."""
    n = SIZE
    px = [[(0, 0, 0, 0)] * n for _ in range(n)]

    centre = n / 2.0
    corner = n * 0.20            # rounded-square corner radius
    ring_outer = n * 0.335
    ring_inner = n * 0.265
    slash_half = n * 0.045       # half thickness of the diagonal bar

    for y in range(n):
        for x in range(n):
            fx, fy = x + 0.5, y + 0.5

            # Rounded square mask.
            dx = max(abs(fx - centre) - (centre - corner), 0.0)
            dy = max(abs(fy - centre) - (centre - corner), 0.0)
            if math.hypot(dx, dy) > corner:
                continue

            t = y / (n - 1)
            r = int(lerp(BG_TOP[0], BG_BOTTOM[0], t))
            g = int(lerp(BG_TOP[1], BG_BOTTOM[1], t))
            b = int(lerp(BG_TOP[2], BG_BOTTOM[2], t))

            # Three faint marks: the invisible characters being removed.
            for i, gx in enumerate((-0.16, 0.0, 0.16)):
                gxp = centre + gx * n
                gyp = centre + 0.0
                if math.hypot(fx - gxp, fy - gyp) < n * 0.030:
                    a = 0.18 + 0.06 * i
                    r = int(lerp(r, GHOST[0], a))
                    g = int(lerp(g, GHOST[1], a))
                    b = int(lerp(b, GHOST[2], a))

            # Prohibition ring.
            d = math.hypot(fx - centre, fy - centre)
            if ring_inner <= d <= ring_outer:
                r, g, b = RING

            # Diagonal bar, drawn last so it sits on top of the ring.
            # Distance from the line y = x through the centre.
            line = abs((fx - centre) + (fy - centre)) / math.sqrt(2.0)
            along = abs((fx - centre) - (fy - centre)) / math.sqrt(2.0)
            if line <= slash_half and along <= ring_outer:
                r, g, b = SLASH

            px[y][x] = (r, g, b, 255)

    return px


def downsample(px, target: int):
    """Box filter from SIZE to target, giving cheap antialiasing."""
    step = SIZE // target
    out = []
    for ty in range(target):
        row = []
        for tx in range(target):
            r = g = b = a = 0
            for sy in range(ty * step, (ty + 1) * step):
                for sx in range(tx * step, (tx + 1) * step):
                    pr, pg, pb, pa = px[sy][sx]
                    r += pr * pa
                    g += pg * pa
                    b += pb * pa
                    a += pa
            count = step * step
            if a == 0:
                row.append((0, 0, 0, 0))
            else:
                row.append((r // a, g // a, b // a, a // count))
        out.append(row)
    return out


def png_bytes(rows) -> bytes:
    n = len(rows)
    raw = bytearray()
    for row in rows:
        raw.append(0)  # filter type: none
        for r, g, b, a in row:
            raw += bytes((r, g, b, a))

    def chunk(tag: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", n, n, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + chunk(b"IEND", b"")
    )


def ico_bytes(pngs: dict[int, bytes]) -> bytes:
    """Wrap PNGs in an .ico container (PNG-compressed entries, Vista+)."""
    sizes = sorted(pngs)
    header = struct.pack("<HHH", 0, 1, len(sizes))
    offset = len(header) + 16 * len(sizes)
    entries, blobs = b"", b""
    for s in sizes:
        data = pngs[s]
        entries += struct.pack(
            "<BBBBHHII", s if s < 256 else 0, s if s < 256 else 0, 0, 0, 1, 32,
            len(data), offset,
        )
        blobs += data
        offset += len(data)
    return header + entries + blobs


def png_pixels(data: bytes) -> bytes:
    """Extract the raw, uncompressed scanlines from one of our own PNGs.

    Comparison has to happen on pixels rather than on file bytes: zlib's
    output is not identical across versions, so a byte-for-byte check would
    fail on any machine whose zlib differs from the one that last ran this
    script.
    """
    pos, idat = 8, b""
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos : pos + 4])
        tag = data[pos + 4 : pos + 8]
        if tag == b"IDAT":
            idat += data[pos + 8 : pos + 8 + length]
        pos += 12 + length
    return zlib.decompress(idat)


SIZES = (16, 32, 48, 64, 128, 256, 512)
ICO_SIZES = (16, 32, 48, 64, 128, 256)


def main() -> None:
    check = "--check" in sys.argv
    ASSETS.mkdir(parents=True, exist_ok=True)
    master = draw()

    pngs = {size: png_bytes(downsample(master, size)) for size in SIZES}
    ico = ico_bytes({s: pngs[s] for s in ICO_SIZES})

    if check:
        for size, data in pngs.items():
            path = ASSETS / f"icon-{size}.png"
            if not path.exists():
                raise SystemExit(f"{path} is missing")
            if png_pixels(path.read_bytes()) != png_pixels(data):
                raise SystemExit(f"{path} does not match this script's output")
            print(f"assets/icon-{size}.png  ok")
        if not (ASSETS / "icon.ico").exists():
            raise SystemExit("assets/icon.ico is missing")
        print("assets/icon.ico          ok")
        return

    for size, data in pngs.items():
        (ASSETS / f"icon-{size}.png").write_bytes(data)
        print(f"assets/icon-{size}.png  {len(data):>7} bytes")
    (ASSETS / "icon.ico").write_bytes(ico)
    print(f"assets/icon.ico          {len(ico):>7} bytes")


if __name__ == "__main__":
    main()
