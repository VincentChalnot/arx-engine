#!/usr/bin/env python3
"""Regenerate assets/generated/keres.ico and keres.icns — the Windows
executable icon (embedded as a PE resource by build.rs via winres) and the
macOS .app bundle icon (Contents/Resources/keres.icns, see
scripts/package_macos_app.sh) — from the crest in assets/pixel/logo.xcf.

Rasterized the same way as the window/taskbar icon (see
gen_window_icon.py), then block-scaled with nearest-neighbor pixel
replication (no interpolation, so the pixel art stays crisp) onto a padded
square canvas, matching the game's `render::COL_GOLD` crest on
`render::COL_PAGE_BG` — unlike the window/taskbar icon, an app icon needs
its own opaque backdrop rather than sitting transparent against the desktop.

Pillow writes both container formats directly from pre-sized frames (passed
through `append_images`, which both plugins prefer verbatim over their own
frame's fallback resize) — no `iconutil`/ImageMagick dependency, so this
runs the same on every platform.

Invoked by `make gui` whenever assets/pixel/logo.xcf is newer than the
generated files — see the Makefile.
"""
import pathlib
import sys

from PIL import Image

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from pixel_raster import rasterize

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC_XCF = REPO_ROOT / "assets" / "pixel" / "logo.xcf"
OUT_DIR = REPO_ROOT / "assets" / "generated"
LOGO_W, LOGO_H = 46, 37

GOLD = (0xE1, 0xCA, 0x58, 255)  # render::COL_GOLD
BG = (0x14, 0x14, 0x0F, 255)  # render::COL_PAGE_BG

# The master is rendered once at the largest size any format wants, with an
# integer per-pixel block scale (so every crest pixel becomes an
# identically-sized square — no interpolation blur), then nearest-neighbor
# resized down for every smaller size. 1024 keeps that resize an exact
# power-of-two ratio for every ICNS size and most ICO sizes.
MASTER = 1024
SCALE = int((MASTER * 0.75) // LOGO_W)

ICO_SIZES = [16, 24, 32, 48, 64, 128, 256]
ICNS_SIZES = [32, 64, 128, 256, 512, 1024]


def render_master(rows: list[int]) -> Image.Image:
    content_w, content_h = LOGO_W * SCALE, LOGO_H * SCALE
    x_off, y_off = (MASTER - content_w) // 2, (MASTER - content_h) // 2
    img = Image.new("RGBA", (MASTER, MASTER), BG)
    px = img.load()
    for y in range(LOGO_H):
        row = rows[y]
        for x in range(LOGO_W):
            if not (row >> (LOGO_W - 1 - x)) & 1:
                continue
            for by in range(SCALE):
                for bx in range(SCALE):
                    px[x_off + x * SCALE + bx, y_off + y * SCALE + by] = GOLD
    return img


def main() -> None:
    rows = rasterize(SRC_XCF, LOGO_W, LOGO_H)
    master = render_master(rows)
    OUT_DIR.mkdir(parents=True, exist_ok=True)

    # Pillow's ICO/ICNS writers only accept sizes up to (and, for ICO, cap
    # candidates at) the base image's own size, so the base frame passed to
    # `.save()` must be the largest one — the rest ride along as
    # `append_images` and get matched by exact size instead of resized.
    ico_frames = [master.resize((s, s), Image.NEAREST) for s in ICO_SIZES]
    ico_path = OUT_DIR / "keres.ico"
    ico_frames[-1].save(ico_path, sizes=[(s, s) for s in ICO_SIZES], append_images=ico_frames)
    print(f"wrote {ico_path} ({', '.join(str(s) for s in ICO_SIZES)}px)")

    icns_frames = [master.resize((s, s), Image.NEAREST) for s in ICNS_SIZES]
    icns_path = OUT_DIR / "keres.icns"
    icns_frames[-1].save(icns_path, append_images=icns_frames)
    print(f"wrote {icns_path} ({', '.join(str(s) for s in ICNS_SIZES)}px)")


if __name__ == "__main__":
    main()
