#!/usr/bin/env python3
"""Agent-One icon generator — Phase 7 Step 6 (PWA + Tauri skeletons).

Draws the DropAgentX app icon: Aurora dark gradient plate with the Din mark
"Đ" (U+0110) in the Aurora brand green. Deterministic — same inputs, same
bytes — so icons can be committed and regenerated identically.

Outputs (all paths relative to front/):
  apps/shell/public/icons/icon-{192,512}.png            # purpose "any"
  apps/shell/public/icons/icon-maskable-{192,512}.png   # purpose "maskable"
  apps/shell/public/icons/apple-touch-icon.png          # 180x180, opaque
  apps/shell/public/favicon.ico                         # 16/32/48
  apps/desktop/src-tauri/icons/{32x32,128x128,128x128@2x,icon}.png
  apps/mobile/splash/splash-{2732,2048,1284}.png        # Capacitor splash source

Requirements: Pillow >= 10 (pip install pillow). Fonts: any bold sans with
Latin Extended-A (U+0110 "Đ"); falls back through a small candidate list.

Run from front/:  python3 scripts/gen-icons.py
"""
from __future__ import annotations

import os
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

FRONT = Path(__file__).resolve().parent.parent

# Aurora token values (front/packages/ui/src/tokens.css) — one source of truth.
BG_TOP = (12, 22, 20)       # --aur-bg dark      #0c1614
BG_BOTTOM = (26, 47, 43)    # --aur-surface-2    #1a2f2b
BRAND = (15, 191, 146)      # --aur-brand-500    #0fbf92
BRAND_DEEP = (10, 125, 97)  # --aur-brand-700    #0a7d61
VIOLET = (124, 111, 255)    # --aur-violet-500   #7c6fff

FONT_CANDIDATES = [
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
]

OUT_SHELL_ICONS = FRONT / "apps/shell/public/icons"
OUT_FAVICON = FRONT / "apps/shell/public/favicon.ico"
OUT_FAVICON_SVG = FRONT / "apps/shell/public/favicon.svg"
OUT_TAURI_ICONS = FRONT / "apps/desktop/src-tauri/icons"
OUT_SPLASH = FRONT / "apps/mobile/splash"


def load_font(px: int) -> ImageFont.FreeTypeFont:
    for path in FONT_CANDIDATES:
        if os.path.exists(path):
            return ImageFont.truetype(path, px)
    raise SystemExit("no bold sans font with U+0110 found — install DejaVu Sans")


def gradient_plate(size: int, radius: int) -> Image.Image:
    """Aurora dark vertical gradient plate; alpha-rounded when radius > 0."""
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    grad = Image.new("RGBA", (size, size))
    top, bottom = BG_TOP + (255,), BG_BOTTOM + (255,)
    gd = ImageDraw.Draw(grad)
    for y in range(size):
        t = y / max(size - 1, 1)
        row = tuple(round(top[i] + (bottom[i] - top[i]) * t) for i in range(4))
        gd.line([(0, y), (size, y)], fill=row)
    if radius > 0:
        mask = Image.new("L", (size, size), 0)
        ImageDraw.Draw(mask).rounded_rectangle(
            [0, 0, size - 1, size - 1], radius=radius, fill=255
        )
        img.paste(grad, (0, 0), mask)
    else:
        img = grad
    return img


def draw_glyph(img: Image.Image, glyph_ratio: float, accent: bool) -> None:
    """Center the Din mark Đ at glyph_ratio of the plate side."""
    size = img.width
    font = load_font(round(size * glyph_ratio))
    d = ImageDraw.Draw(img)
    # bbox: (l, t, r, b) — DejaVu renders Đ with its full stroke inside bbox
    l, t, r, b = d.textbbox((0, 0), "\u0110", font=font)
    w, h = r - l, b - t
    x = (size - w) / 2 - l
    y = (size - h) / 2 - t
    if accent:  # soft violet under-glow for the "any" plate
        glow = Image.new("RGBA", img.size, (0, 0, 0, 0))
        ImageDraw.Draw(glow).text(
            (x, y + size * 0.012), "\u0110", font=font, fill=VIOLET + (110,)
        )
        img.alpha_composite(glow)
    d.text((x, y), "\u0110", font=font, fill=BRAND + (255,))


def make_icon(size: int, *, maskable: bool, opaque: bool) -> Image.Image:
    if maskable:  # full-bleed; glyph inside the ~80% safe zone (use 0.58)
        img = gradient_plate(size, radius=0)
        draw_glyph(img, glyph_ratio=0.58, accent=False)
        return img.convert("RGB") if opaque else img
    plate = gradient_plate(size, radius=round(size * 0.22))
    draw_glyph(plate, glyph_ratio=0.62, accent=True)
    if opaque:  # iOS flattens transparency: bake the dark bg behind corners
        flat = Image.new("RGB", (size, size), BG_TOP)
        flat.paste(plate, (0, 0), plate)
        return flat
    return plate


def write_png(img: Image.Image, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    img.save(path, "PNG", optimize=True)
    print(f"  ✓ {path.relative_to(FRONT)} ({img.width}x{img.height})")


def make_splash(width: int, height: int) -> Image.Image:
    """Capacitor splash source: full-bleed Aurora gradient, Đ mark centered
    with the Latin wordmark below (@capacitor/assets scales/crops per device)."""
    img = Image.new("RGBA", (width, height))
    d = ImageDraw.Draw(img)
    top, bottom = BG_TOP + (255,), BG_BOTTOM + (255,)
    for y in range(height):
        t = y / max(height - 1, 1)
        d.line([(0, y), (width, y)], fill=tuple(round(top[i] + (bottom[i] - top[i]) * t) for i in range(4)))
    glyph = load_font(round(height * 0.22))
    l, t2, r, b = d.textbbox((0, 0), "\u0110", font=glyph)
    d.text(((width - (r - l)) / 2 - l, height * 0.42 - (b - t2) / 2 - t2), "\u0110", font=glyph, fill=BRAND + (255,))
    word = load_font(round(height * 0.036))
    l, t2, r, b = d.textbbox((0, 0), "DropAgentX", font=word)
    d.text(((width - (r - l)) / 2 - l, height * 0.56 - t2), "DropAgentX", font=word, fill=(230, 242, 238, 235))
    return img.convert("RGB")


def main() -> None:
    print("▸ generating Agent-One icons (Aurora dark plate + Đ brand mark)…")
    # PWA — purpose "any" (rounded plate, transparent corners)
    for s in (192, 512):
        write_png(make_icon(s, maskable=False, opaque=False), OUT_SHELL_ICONS / f"icon-{s}.png")
    # PWA — purpose "maskable" (full-bleed, safe-zone glyph)
    for s in (192, 512):
        write_png(
            make_icon(s, maskable=True, opaque=False), OUT_SHELL_ICONS / f"icon-maskable-{s}.png"
        )
    # apple-touch-icon — opaque 180x180 (iOS masks its own corners)
    write_png(
        make_icon(180, maskable=False, opaque=True), OUT_SHELL_ICONS / "apple-touch-icon.png"
    )
    # favicon.ico — 16/32/48 opaque (transparent corners are unreadable at 16px)
    frames = [make_icon(s, maskable=True, opaque=True).convert("RGBA") for s in (16, 32, 48)]
    OUT_FAVICON.parent.mkdir(parents=True, exist_ok=True)
    frames[-1].save(OUT_FAVICON, format="ICO", sizes=[(16, 16), (32, 32), (48, 48)])
    print(f"  ✓ {OUT_FAVICON.relative_to(FRONT)} (16/32/48)")
    # favicon.svg — vector twin of the plate (system bold sans renders Đ)
    svg = (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">'
        '<defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="1">'
        f'<stop offset="0" stop-color="rgb({BG_TOP[0]},{BG_TOP[1]},{BG_TOP[2]})"/>'
        f'<stop offset="1" stop-color="rgb({BG_BOTTOM[0]},{BG_BOTTOM[1]},{BG_BOTTOM[2]})"/>'
        "</linearGradient></defs>"
        '<rect width="64" height="64" rx="14" fill="url(#g)"/>'
        f'<text x="32" y="34" text-anchor="middle" dominant-baseline="central" '
        'font-family="system-ui, \'DejaVu Sans\', sans-serif" font-weight="700" font-size="38" '
        f'fill="rgb({BRAND[0]},{BRAND[1]},{BRAND[2]})">\u0110</text></svg>\n'
    )
    OUT_FAVICON_SVG.write_text(svg, encoding="utf-8")
    print(f"  ✓ {OUT_FAVICON_SVG.relative_to(FRONT)}")
    # Tauri v2 bundle icons (icon.png may be any square; ico/icns via `tauri icon`)
    for name, s in (("32x32.png", 32), ("128x128.png", 128), ("128x128@2x.png", 256), ("icon.png", 512)):
        write_png(make_icon(s, maskable=False, opaque=False), OUT_TAURI_ICONS / name)
    # Capacitor splash sources (full-bleed; @capacitor/assets derives per-device)
    OUT_SPLASH.mkdir(parents=True, exist_ok=True)
    for name, w, h in (("splash-2732.png", 2732, 2732), ("splash-2048.png", 2048, 2048), ("splash-1284.png", 1284, 2778)):
        img = make_splash(w, h)
        img.save(OUT_SPLASH / name, "PNG", optimize=True)
        print(f"  ✓ {img_path_rel(OUT_SPLASH / name)} ({w}x{h})")
    print("✓ icons done — deterministic (Aurora tokens + Đ), committed to the repo")


def img_path_rel(path: Path) -> str:
    return str(path.relative_to(FRONT))


if __name__ == "__main__":
    main()
