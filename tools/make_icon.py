#!/usr/bin/env python3
"""Draw the BW Darkroom app icon (1024 x 1024 PNG) with no dependencies.

A 21-step grey wedge, like a sensitometric step tablet, inside a circle on a dark
rounded square. Usage: python3 -I tools/make_icon.py packaging/macos/icon_1024.png
"""
import math
import struct
import sys
import zlib

N = 1024


def png(path, rows):
    raw = b"".join(b"\x00" + bytes(r) for r in rows)
    def chunk(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    data = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", N, N, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))
    open(path, "wb").write(data)


def coverage(d):
    """Antialiased edge: 1 inside (d < -0.5), 0 outside."""
    return max(0.0, min(1.0, 0.5 - d))


def main(path):
    margin, radius_corner = 100, 185
    cx = cy = N / 2
    r_outer, ring = 330, 14
    rows = []
    for y in range(N):
        row = []
        for x in range(N):
            # rounded-square background (macOS icon grid leaves a margin)
            qx = max(abs(x + 0.5 - cx) - (N / 2 - margin - radius_corner), 0)
            qy = max(abs(y + 0.5 - cy) - (N / 2 - margin - radius_corner), 0)
            bg_a = coverage(math.hypot(qx, qy) - radius_corner)
            r = math.hypot(x + 0.5 - cx, y + 0.5 - cy)
            # background: near-black with a faint vertical gradient
            v = 22 + 18 * (1 - y / N)
            # step wedge clipped to the circle, 21 steps left (white) to right (black)
            inside = coverage(r - (r_outer - ring))
            step = min(20, max(0, int((x - (cx - r_outer)) / (2 * r_outer) * 21)))
            wedge = 255 * (1 - step / 20) ** 2.2 ** 0.5
            v = v * (1 - inside) + wedge * inside
            # thin light ring around the wedge
            ring_a = coverage(abs(r - (r_outer - ring / 2)) - ring / 2)
            v = v * (1 - ring_a) + 235 * ring_a
            g = int(max(0, min(255, v)))
            row += [g, g, g, int(255 * bg_a)]
        rows.append(row)
    png(path, rows)


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "packaging/macos/icon_1024.png")
