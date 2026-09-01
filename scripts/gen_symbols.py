#!/usr/bin/env python3
"""Regenerate src/gui/symbols.rs from the 1-bit indexed .xcf files in
assets/pixel/symbols/ (currently just down-arrow.xcf, the "this creates a
stack" hover cue drawn on top of a piece — see docs/GUI.md).

Rasterized the same way as the icons/base plaque (see pixel_raster.py) into
a [u32; SYMBOL_H] bitmask, SYMBOL_W bits wide.

Invoked by `make gui` whenever a .xcf here is newer than src/gui/symbols.rs
— see the Makefile. Do not edit src/gui/symbols.rs by hand, it is
overwritten here.
"""
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from pixel_raster import format_rows, rasterize

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
SYMBOL_DIR = REPO_ROOT / "assets" / "pixel" / "symbols"
OUT_FILE = REPO_ROOT / "src" / "gui" / "symbols.rs"
SYMBOL_W = 23
SYMBOL_H = 16


def variant_name(stem: str) -> str:
    return "".join(part.capitalize() for part in stem.replace("-", "_").split("_"))


def main() -> None:
    xcf_files = sorted(SYMBOL_DIR.glob("*.xcf"))
    if not xcf_files:
        raise SystemExit(f"no .xcf files found in {SYMBOL_DIR}")

    arms = []
    variants = []
    for xcf_path in xcf_files:
        variant = variant_name(xcf_path.stem)
        variants.append(variant)
        rows = rasterize(xcf_path, SYMBOL_W, SYMBOL_H)
        arms.append(
            f"        Symbol::{variant} => [\n{format_rows(rows, SYMBOL_W, indent=12)}\n        ],"
        )

    lines = [
        f"// Auto-generated {SYMBOL_W}x{SYMBOL_H} 1-bit UI symbol masks.",
        "// Regenerate with `scripts/gen_symbols.py` (run automatically by `make gui`)",
        "// after editing the .xcf sources in assets/pixel/symbols/. Do not edit by hand.",
        f"pub const SYMBOL_W: usize = {SYMBOL_W};",
        "",
        f"pub fn symbol_bits(name: Symbol) -> [u32; {SYMBOL_H}] {{",
        "    match name {",
        *arms,
        "    }",
        "}",
        "",
        "#[derive(Clone, Copy, Debug, PartialEq, Eq)]",
        "pub enum Symbol {",
        *(f"    {variant}," for variant in variants),
        "}",
        "",
    ]

    OUT_FILE.write_text("\n".join(lines))
    print(f"wrote {OUT_FILE} ({len(variants)} symbols: {', '.join(variants)})")


if __name__ == "__main__":
    main()
