#!/usr/bin/env python3
"""Regenerate src/gui/window_icon.rs — the taskbar/window-manager icon,
installed by `src/gui/platform_icon.rs`, from the splash crest
(assets/pixel/logo.xcf).

Encodes the freedesktop `_NET_WM_ICON` buffer format directly: a flat
`[width, height, argb pixel...]` u64 array (X11 packs CARDINAL as 64-bit
slots on 64-bit systems even though the property format is nominally 32-bit
— see minifb's `os/posix/x11.rs::set_icon`). Windows gets no icon API it
could take a buffer through at all, so it reuses this one: the same
`0xAARRGGBB` pixels, read out of the low half of each slot, become the
`HICON` (see `platform_icon.rs`). macOS has no runtime icon API in minifb.

The crest is 46x37, so rather than scale it — nearest-neighbor down to the
16px the spec's smaller sizes want would mangle art drawn pixel by pixel —
it is centered unscaled on one transparent ICON_SIZE square and emitted as
a single image. The spec allows several sizes concatenated but does not
require them, and minifb's `set_icon` forwards the buffer verbatim without
interpreting it; window managers scale a single supplied image themselves
for whichever slot they're filling.

An "on" mask pixel is painted opaque gold (matches `render::COL_GOLD`);
"off" — including the whole margin around the centered crest — is fully
transparent, so the icon reads correctly against any desktop
theme/wallpaper.

Invoked by `make gui` whenever the source is newer than the generated file.
"""
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from pixel_raster import rasterize

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC_XCF = REPO_ROOT / "assets" / "pixel" / "logo.xcf"
OUT_FILE = REPO_ROOT / "src" / "gui" / "window_icon.rs"
LOGO_W, LOGO_H = 46, 37
ICON_SIZE = 64

# 0xAARRGGBB, opaque gold — matches render::COL_GOLD (0xe1ca58).
ON_ARGB = 0xFFE1CA58
OFF_ARGB = 0x00000000


def centered_pixels(rows: list[int]) -> list[int]:
    """Center the LOGO_W x LOGO_H mask on a transparent ICON_SIZE square,
    returning ICON_SIZE**2 ARGB values in row-major order."""
    x_off = (ICON_SIZE - LOGO_W) // 2
    y_off = (ICON_SIZE - LOGO_H) // 2
    pixels = []
    for y in range(ICON_SIZE):
        sy = y - y_off
        for x in range(ICON_SIZE):
            sx = x - x_off
            on = (
                0 <= sy < LOGO_H
                and 0 <= sx < LOGO_W
                and (rows[sy] >> (LOGO_W - 1 - sx)) & 1
            )
            pixels.append(ON_ARGB if on else OFF_ARGB)
    return pixels


def main() -> None:
    rows = rasterize(SRC_XCF, LOGO_W, LOGO_H)
    values = [ICON_SIZE, ICON_SIZE] + centered_pixels(rows)

    # One value per line: a u64 literal is wide enough that rustfmt breaks
    # this array anyway, and once broken it puts each element on its own
    # line — so packing several per line here would only get reformatted by
    # the next `cargo fmt`.
    body = "\n".join(f"    0x{v:016X}," for v in values)

    OUT_FILE.write_text(
        "// Auto-generated window icon (X11 _NET_WM_ICON ARGB buffer).\n"
        "// Regenerate with `scripts/gen_window_icon.py` (run automatically by\n"
        "// `make gui`) after editing the source .xcf. Do not edit by hand.\n"
        f"pub static WINDOW_ICON: [u64; {len(values)}] = [\n{body}\n];\n"
    )
    print(
        f"wrote {OUT_FILE} ({ICON_SIZE}x{ICON_SIZE}, "
        f"{LOGO_W}x{LOGO_H} {SRC_XCF.name} centered)"
    )


if __name__ == "__main__":
    main()
