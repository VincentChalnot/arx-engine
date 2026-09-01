#!/usr/bin/env python3
"""Regenerate src/gui/icons.rs from the 25x25 1-bit indexed .xcf files in
assets/pixel/icons/.

Each .xcf is rasterized by headless GIMP (see pixel_raster.py) into a 25x25
grid of on/off pixels, then packed into the [u32; 25] bitmask format
`icon_bits` expects. 25 is odd so there is a true center column/row, which
lets icons like the soldier or ballista come to a symmetric point instead
of a flat two-pixel-wide top (a 24x24 grid has no center pixel).

Invoked by `make gui` whenever an .xcf is newer than src/gui/icons.rs — see
the Makefile. Do not edit src/gui/icons.rs by hand, it is overwritten here.
"""
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from pixel_raster import format_rows, rasterize

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
ICON_DIR = REPO_ROOT / "assets" / "pixel" / "icons"
OUT_FILE = REPO_ROOT / "src" / "gui" / "icons.rs"
ICON_SIZE = 25


def variant_name(stem: str) -> str:
    return "".join(part.capitalize() for part in stem.split("_"))


def main() -> None:
    xcf_files = sorted(ICON_DIR.glob("*.xcf"))
    if not xcf_files:
        raise SystemExit(f"no .xcf files found in {ICON_DIR}")

    arms = []
    variants = []
    for xcf_path in xcf_files:
        variant = variant_name(xcf_path.stem)
        variants.append(variant)
        rows = rasterize(xcf_path, ICON_SIZE, ICON_SIZE)
        arms.append(
            f"        PieceIcon::{variant} => [\n{format_rows(rows, ICON_SIZE, indent=12)}\n        ],"
        )

    lines = [
        f"// Auto-generated {ICON_SIZE}x{ICON_SIZE} 1-bit piece icon masks.",
        "// Regenerate with `scripts/gen_icons.py` (run automatically by `make gui`)",
        "// after editing the .xcf sources in assets/pixel/icons/. Do not edit by hand.",
        f"pub const ICON_N: usize = {ICON_SIZE};",
        "",
        f"pub fn icon_bits(name: PieceIcon) -> [u32; {ICON_SIZE}] {{",
        "    match name {",
        *arms,
        "    }",
        "}",
        "",
        "#[derive(Clone, Copy, Debug, PartialEq, Eq)]",
        "pub enum PieceIcon {",
        *(f"    {variant}," for variant in variants),
        "}",
        "",
    ]

    OUT_FILE.write_text("\n".join(lines))
    print(f"wrote {OUT_FILE} ({len(variants)} icons: {', '.join(variants)})")


if __name__ == "__main__":
    main()
