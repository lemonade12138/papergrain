#!/usr/bin/env python3
"""Generate preview PNGs of PaperGrain's four procedural texture presets.

These are visual documentation of the runtime algorithms (ported from
src/noise.rs + src/textures.rs); the app itself generates textures at native
monitor resolution and never reads these files. They also double as
ready-to-use custom texture files.
"""
from PIL import Image
import struct

# ---- value noise / fBm (mirrors src/noise.rs) ----------------------------
def hash2(ix, iy, seed):
    M = 0xFFFFFFFF
    h = ((ix * 0x27d4eb2d) & M) ^ ((iy * 0x165667b1) & M) ^ ((seed * 0x9e3779b9) & M)
    h &= M
    h ^= h >> 15
    h = (h * 0x85ebca6b) & M
    h ^= h >> 13
    h = (h * 0xc2b2ae35) & M
    h ^= h >> 16
    return (h & 0x7FFFFF) / 8388607.0

def smooth(t):
    return t * t * (3.0 - 2.0 * t)

def vnoise(x, y, seed):
    xif, yif = int(x // 1), int(y // 1)
    fx, fy = x - xif, y - yif
    sx, sy = smooth(fx), smooth(fy)
    xi, yi = int(x), int(y)
    a = hash2(xi, yi, seed)
    b = hash2(xi + 1, yi, seed)
    c = hash2(xi, yi + 1, seed)
    d = hash2(xi + 1, yi + 1, seed)
    return a + (b - a) * sx + (c - a) * sy + (a - b - c + d) * sx * sy

def fbm(x, y, octaves, seed):
    total, amp, norm = 0.0, 0.5, 0.0
    fx = fy = 1.0
    for o in range(octaves):
        total += amp * vnoise(x * fx, y * fy, (seed + o * 1013) & 0xFFFFFFFF)
        norm += amp
        amp *= 0.5
        fx *= 2.02
        fy *= 2.02
    return total / norm if norm else 0.5

SIZE = 512
DPI = 96
S = 1.0
INTENSITY = 60 / 100.0

def clamp(v, lo, hi):
    return lo if v < lo else hi if v > hi else v

def put_px(px, x, y, b, g, r, a):
    a = clamp(a, 0.0, 255.0)
    b = clamp(b, 0.0, a)
    g = clamp(g, 0.0, a)
    r = clamp(r, 0.0, a)
    px[y * SIZE + x] = (int(b), int(g), int(r), int(a))

def fine_grain(px):
    inten = INTENSITY
    for y in range(SIZE):
        for x in range(SIZE):
            n = fbm(x * 0.85, y * 0.85, 3, 101)
            g = hash2(x, y, 77)
            d = (n - 0.5) * inten
            if d > 0:
                put_px(px, x, y, 255, 255, 255, d * 2.0 * 140.0)
            else:
                put_px(px, x, y, 38, 36, 34, -d * 2.0 * 120.0)
            if g > 0.9965:
                put_px(px, x, y, 255, 255, 255, inten * 150.0)

def craft_paper(px):
    inten = INTENSITY
    for y in range(SIZE):
        for x in range(SIZE):
            blotch = fbm(x * 0.0045, y * 0.0045, 4, 201) - 0.5
            fh = fbm(x * 0.045, y * 0.011, 2, 202)
            fv = fbm(x * 0.010, y * 0.050, 2, 203)
            fleck = hash2(x, y, 204)
            br = 194.0 + blotch * 60.0
            bg = 164.0 + blotch * 48.0
            bb = 122.0 + blotch * 30.0
            a = inten * 108.0
            if fh > 0.60:
                k = (fh - 0.60) * inten
                br += k * 60.0; bg += k * 70.0; bb += k * 60.0; a += k * 130.0
            elif fh < 0.36:
                k = (0.36 - fh) * inten
                br -= k * 60.0; bg -= k * 55.0; bb -= k * 40.0; a += k * 110.0
            if fv > 0.63:
                k = (fv - 0.63) * inten
                br += k * 50.0; bg += k * 55.0; bb += k * 45.0; a += k * 90.0
            if fleck > 0.9955:
                br, bg, bb, a = 58.0, 44.0, 26.0, inten * 190.0
            put_px(px, x, y, bb, bg, br, a)

def notebook(px):
    inten = INTENSITY
    spacing, line_th = 32.0, 1.4
    margin_x, margin_th = 96.0, 1.1
    for y in range(SIZE):
        row = (y % spacing) if spacing else 0
        for x in range(SIZE):
            if row < line_th:
                put_px(px, x, y, 235, 160, 120, inten * 190.0)
            elif abs(x - margin_x) < margin_th:
                put_px(px, x, y, 130, 120, 235, inten * 205.0)
            else:
                g = hash2(x, y, 303)
                a = inten * 26.0 + (g - 0.5) * inten * 22.0
                put_px(px, x, y, 244, 246, 250, a)

def parchment(px):
    inten = INTENSITY
    edge_band = 90.0
    for y in range(SIZE):
        dy = min(y, SIZE - 1 - y)
        for x in range(SIZE):
            dx = min(x, SIZE - 1 - x)
            b1 = fbm(x * 0.0035, y * 0.0035, 4, 401)
            b2 = fbm(x * 0.013, y * 0.013, 3, 402)
            fi = fbm(x * 0.09, y * 0.022, 2, 403)
            edge = clamp(1.0 - min(dx, dy) / edge_band, 0.0, 1.0)
            warm = clamp(0.5 - b1, 0.0, 1.0)
            br = 233.0 - warm * 63.0
            bg = 216.0 - warm * 86.0
            bb = 180.0 - warm * 100.0
            a = inten * (76.0 + warm * 64.0)
            if b2 < 0.42:
                k = (0.42 - b2) * inten
                br -= k * 113.0; bg -= k * 128.0; bb -= k * 132.0; a += k * 190.0
            if edge > 0.0:
                k = edge * inten
                br -= k * 137.0; bg -= k * 142.0; bb -= k * 136.0; a += k * 97.0
            if fi > 0.63:
                k = (fi - 0.63) * inten
                br += k * 22.0; bg += k * 38.0; bb += k * 55.0; a += k * 90.0
            elif fi < 0.34:
                k = (0.34 - fi) * inten
                br -= k * 50.0; bg -= k * 42.0; bb -= k * 30.0; a += k * 60.0
            put_px(px, x, y, bb, bg, br, a)

GEN = {
    "fine-grain": fine_grain,
    "craft-paper": craft_paper,
    "notebook": notebook,
    "parchment": parchment,
}

OUT = "/home/z/my-project/papergrain/textures"
import os
os.makedirs(OUT, exist_ok=True)

# render onto a paper-white backdrop for preview purposes
for name, fn in GEN.items():
    px = [(255, 255, 255, 0)] * (SIZE * SIZE)
    fn(px)
    img = Image.new("RGBA", (SIZE, SIZE))
    # composite the semi-transparent texture over white to preview the look
    flat = []
    for (b, g, r, a) in px:
        af = a / 255.0
        fb = int(b + (255 - b) * (1 - af))
        fg = int(g + (255 - g) * (1 - af))
        fr = int(r + (255 - r) * (1 - af))
        flat.append((fr, fg, fb))
    img = Image.new("RGB", (SIZE, SIZE))
    img.putdata(flat)
    path = f"{OUT}/{name}.png"
    img.save(path)
    print("wrote", path)
