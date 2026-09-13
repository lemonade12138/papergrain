#!/usr/bin/env python3
"""Generate the PaperGrain application icon (paper sheet + grain + notebook
lines motif) as a multi-resolution .ico plus a preview PNG."""
from PIL import Image, ImageDraw
import random

def rounded_rect_path(d, box, radius):
    x0, y0, x1, y1 = box
    d.rounded_rectangle(box, radius=radius, fill=None, outline=None)

def draw_sheet(size):
    """Draw the master icon art at given square size, RGBA."""
    s = size
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    m = s // 16  # outer margin
    r = max(2, s // 9)  # corner radius

    # paper sheet with folded corner
    fold = max(3, s // 7)
    x0, y0, x1, y1 = m, m, s - m, s - m
    # shadow hint
    d.rounded_rectangle((x0 + s//48, y0 + s//48, x1 + s//48, y1 + s//48),
                        radius=r, fill=(60, 45, 20, 40))
    # sheet
    d.rounded_rectangle((x0, y0, x1, y1), radius=r, fill=(242, 232, 213, 255))
    # dog-ear (top-right)
    fx1 = x1
    fy0 = y0
    d.polygon([(fx1 - fold, fy0), (fx1, fy0 + fold), (fx1 - fold, fy0 + fold)],
              fill=(214, 199, 170, 255))
    d.polygon([(fx1 - fold, fy0), (fx1, fy0 + fold), (fx1 - fold, fy0 + fold)],
              fill=(198, 180, 148, 255))

    # notebook lines (soft blue) + red margin
    line_w = max(1, s // 64)
    line_c = (120, 160, 235, 200)
    margin_c = (235, 120, 130, 210)
    mx = x0 + (x1 - x0) // 4
    if s >= 32:
        d.line([(mx, y0 + fold // 2), (mx, y1 - fold // 2)], fill=margin_c,
               width=line_w)
    n_lines = 3 if s < 64 else 4
    gap = (y1 - y0 - fold) // (n_lines + 1)
    for i in range(1, n_lines + 1):
        yy = y0 + fold + gap * i
        d.line([(x0 + fold // 3, yy), (x1 - fold // 3, yy)], fill=line_c,
               width=line_w)

    # grain specks
    if s >= 48:
        rnd = random.Random(42)
        n = (s * s) // 220
        for _ in range(n):
            gx = rnd.randint(x0 + 2, x1 - 3)
            gy = rnd.randint(y0 + 2, y1 - 3)
            tone = rnd.random()
            if tone > 0.78:
                col = (255, 255, 255, rnd.randint(60, 110))
            else:
                col = (120, 100, 70, rnd.randint(30, 80))
            d.point((gx, gy), fill=col)
    return img

def main():
    out_ico = "/home/z/my-project/papergrain/assets/icon.ico"
    out_png = "/home/z/my-project/papergrain/assets/icon-preview.png"
    master = draw_sheet(256)
    master.save(out_png)
    sizes = [16, 24, 32, 48, 64, 128, 256]
    imgs = []
    for sz in sizes:
        if sz == 256:
            imgs.append(master)
        else:
            imgs.append(master.resize((sz, sz), Image.LANCZOS))
    imgs[0].save(out_ico, format="ICO",
                 append_images=imgs[1:])
    print("wrote", out_ico, "and", out_png)

if __name__ == "__main__":
    main()
