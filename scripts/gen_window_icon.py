#!/usr/bin/env python3
"""Regenerate src/gui/window_icon.rs — the taskbar/window-manager icon set
via minifb's X11 `set_icon` (see `main.rs`), from a 1-bit indexed .xcf.

Encodes the freedesktop `_NET_WM_ICON` buffer format directly: a flat
`[width, height, argb pixel...]` u64 array (X11 packs CARDINAL as 64-bit
slots on 64-bit systems even though the property format is nominally 32-bit
— see minifb's `os/posix/x11.rs::set_icon`).

The spec allows *multiple* icon images concatenated back-to-back in the same
property ("possibly multiple icons of different sizes... different desktop
environments and window managers can be expected to use different sizes"),
and minifb's `set_icon` just forwards the whole buffer verbatim — it doesn't
interpret it — so this generator rasterizes the source once at NATIVE_SIZE
and nearest-neighbor scales that bit grid into every size in ICON_SIZES,
emitting one concatenated buffer. The window manager then picks whichever
embedded size fits best (its taskbar entry, alt-tab switcher, and titlebar
icon may each want a different one). 16/32/48/64 covers the common cases:
16 for small lists, 32 for most taskbars, 48-64 for high-DPI/app switchers.
Nearest-neighbor keeps the scaled-up sizes crisp (matches the rest of the
app's pixel art); the one scaled-down size (16) necessarily loses some
detail, which is expected for a small icon.

An "on" mask pixel is painted opaque gold (matches `render::COL_GOLD`);
"off" is fully transparent, so the icon reads correctly against any desktop
theme/wallpaper.

Placeholder source: reuses assets/pixel/icons/king.xcf (25x25) — the king
being the whole point of the game — until a dedicated logo icon replaces it
here once the splash art lands (see the Makefile / docs/GUI.md).

Invoked by `make gui` whenever the source is newer than the generated file.
"""
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from pixel_raster import rasterize

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC_XCF = REPO_ROOT / "assets" / "pixel" / "icons" / "king.xcf"
OUT_FILE = REPO_ROOT / "src" / "gui" / "window_icon.rs"
NATIVE_SIZE = 25
ICON_SIZES = [16, 32, 48, 64]

# 0xAARRGGBB, opaque gold — matches render::COL_GOLD (0xe1ca58).
ON_ARGB = 0xFFE1CA58
OFF_ARGB = 0x00000000


def scale_rows(rows: list[int], src_size: int, dst_size: int) -> list[int]:
    """Nearest-neighbor scale a `src_size`x`src_size` row-bitmask grid
    (see pixel_raster.rasterize) to `dst_size`x`dst_size`."""
    out = []
    for dy in range(dst_size):
        sy = min(src_size - 1, dy * src_size // dst_size)
        src_row = rows[sy]
        row_bits = 0
        for dx in range(dst_size):
            sx = min(src_size - 1, dx * src_size // dst_size)
            bit = (src_row >> (src_size - 1 - sx)) & 1
            row_bits |= bit << (dst_size - 1 - dx)
        out.append(row_bits)
    return out


def icon_values(rows: list[int], size: int) -> list[int]:
    pixels = []
    for row in rows:
        for bit in range(size - 1, -1, -1):
            pixels.append(ON_ARGB if (row >> bit) & 1 else OFF_ARGB)
    return [size, size] + pixels


def main() -> None:
    native_rows = rasterize(SRC_XCF, NATIVE_SIZE, NATIVE_SIZE)

    values: list[int] = []
    for size in ICON_SIZES:
        rows = scale_rows(native_rows, NATIVE_SIZE, size)
        values += icon_values(rows, size)

    lines = []
    for i in range(0, len(values), 6):
        chunk = values[i : i + 6]
        lines.append("    " + ", ".join(f"0x{v:016X}" for v in chunk) + ",")
    body = "\n".join(lines)

    OUT_FILE.write_text(
        "// Auto-generated window icon (X11 _NET_WM_ICON ARGB buffer).\n"
        "// Regenerate with `scripts/gen_window_icon.py` (run automatically by\n"
        "// `make gui`) after editing the source .xcf. Do not edit by hand.\n"
        f"pub static WINDOW_ICON: [u64; {len(values)}] = [\n{body}\n];\n"
    )
    print(
        f"wrote {OUT_FILE} ({len(ICON_SIZES)} sizes {ICON_SIZES} from {SRC_XCF.name})"
    )


if __name__ == "__main__":
    main()
