#!/usr/bin/env python3
"""Regenerate src/gui/base.rs from the 29x46 1-bit indexed .xcf files in
assets/pixel/base/ (base-white.xcf, base-black.xcf, base-mask.xcf).

Each file is a *half* of a piece token's base plaque — see docs/GUI.md for
how render.rs mirrors it into the full base. base-white/base-black encode
fill-vs-ink color; base-mask marks which pixels are part of the piece at
all (black = piece, white = background left untouched — the plaque isn't a
solid rectangle). Rasterized the same way as the icons (see
pixel_raster.py) into a [u32; 46] bitmask (bit 28 = leftmost column of the
29-wide half, bit 0 = rightmost).

Invoked by `make gui` whenever a .xcf here is newer than src/gui/base.rs —
see the Makefile. Do not edit src/gui/base.rs by hand, it is overwritten
here.
"""
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from pixel_raster import format_rows, rasterize

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
BASE_DIR = REPO_ROOT / "assets" / "pixel" / "base"
OUT_FILE = REPO_ROOT / "src" / "gui" / "base.rs"
BASE_W = 29
BASE_H = 46


def main() -> None:
    paths = {
        "WHITE": BASE_DIR / "base-white.xcf",
        "BLACK": BASE_DIR / "base-black.xcf",
        "MASK": BASE_DIR / "base-mask.xcf",
    }
    for p in paths.values():
        if not p.exists():
            raise SystemExit(f"missing {p}")

    lines = [
        f"// Auto-generated {BASE_W}x{BASE_H} 1-bit piece base-plaque half masks.",
        "// Regenerate with `scripts/gen_base.py` (run automatically by `make gui`)",
        "// after editing the .xcf sources in assets/pixel/base/. Do not edit by hand.",
        f"pub const BASE_W: usize = {BASE_W};",
        f"pub const BASE_H: usize = {BASE_H};",
        "",
    ]
    for name, path in paths.items():
        rows = rasterize(path, BASE_W, BASE_H)
        lines += [
            f"pub const BASE_{name}: [u32; {BASE_H}] = [",
            format_rows(rows, BASE_W, indent=4),
            "];",
            "",
        ]

    OUT_FILE.write_text("\n".join(lines))
    print(f"wrote {OUT_FILE}")


if __name__ == "__main__":
    main()
