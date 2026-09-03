#!/usr/bin/env python3
"""Regenerate src/gui/splash.rs from the 1-bit indexed .xcf splash art in
assets/pixel/ (logo.xcf, title.xcf) — the crest and wordmark shown on the
splash screen and main menu (see `draw_splash`/`draw_menu` in
src/gui/main.rs). Replaces the placeholder bitmap-font "KERES" text that
stood in for this art.

Rasterized the same way as the icons/base/symbols (see pixel_raster.py),
then packed into `[[u32; ceil(W / 32)]; H]` — one row spans several u32
words because this art is wider than the 32px a single-word row holds (see
`pack_row`).

Invoked by `make gui` whenever a .xcf here is newer than src/gui/splash.rs
— see the Makefile. Do not edit src/gui/splash.rs by hand, it is
overwritten here.
"""
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from pixel_raster import rasterize

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
PIXEL_DIR = REPO_ROOT / "assets" / "pixel"
OUT_FILE = REPO_ROOT / "src" / "gui" / "splash.rs"

LOGO_W, LOGO_H = 46, 37
TITLE_W, TITLE_H = 200, 42


def pack_row(row: int, width: int) -> list[int]:
    """Split one `width`-bit row mask into ceil(width / 32) u32 words,
    most-significant (leftmost) word first. Each word is left-aligned — bit
    31 is that word's own first column — so a final short word pads on its
    low end and `Canvas::draw_wide_bitmap` can decode every word the same
    way. Splash art is far wider than the 32px a single-u32 row holds (the
    wordmark is 200px), which is why it can't use `pixel_raster.format_rows`
    like the icons/base/symbols do.
    """
    words = []
    remaining = width
    for _ in range((width + 31) // 32):
        chunk_bits = min(32, remaining)
        shift = remaining - chunk_bits
        chunk = (row >> shift) & ((1 << chunk_bits) - 1)
        words.append(chunk << (32 - chunk_bits))
        remaining -= chunk_bits
    return words


# rustfmt keeps an array literal on one line only while its contents fit
# `array_width` (60 by default — 60% of `max_width`), and breaks it onto its
# own indented line past that. Matching the rule here keeps the generated
# file `cargo fmt --check`-clean, the same reason `pixel_raster.format_rows`
# takes an `indent`. The 2-word crest rows stay inline; the 7-word wordmark
# rows wrap (and still fit `max_width` once wrapped, so rustfmt stops there).
ARRAY_WIDTH = 60


def format_wide(rows: list[int], width: int) -> str:
    lines = []
    for row in rows:
        words = ", ".join(f"0x{w:08X}" for w in pack_row(row, width))
        if len(words) <= ARRAY_WIDTH:
            lines.append(f"    [{words}],")
        else:
            lines.append(f"    [\n        {words},\n    ],")
    return "\n".join(lines)


def main() -> None:
    logo_path = PIXEL_DIR / "logo.xcf"
    title_path = PIXEL_DIR / "title.xcf"
    for p in (logo_path, title_path):
        if not p.exists():
            raise SystemExit(f"missing {p}")

    logo_rows = rasterize(logo_path, LOGO_W, LOGO_H)
    title_rows = rasterize(title_path, TITLE_W, TITLE_H)

    logo_words = (LOGO_W + 31) // 32
    title_words = (TITLE_W + 31) // 32

    lines = [
        "// Auto-generated splash-screen crest and wordmark 1-bit bitmasks.",
        "// Regenerate with `scripts/gen_splash.py` (run automatically by `make gui`)",
        "// after editing assets/pixel/logo.xcf / title.xcf. Do not edit by hand.",
        "// One row = ceil(width / 32) u32 words, leftmost word first; see",
        "// `Canvas::draw_wide_bitmap`.",
        f"pub const LOGO_W: usize = {LOGO_W};",
        f"pub const LOGO_H: usize = {LOGO_H};",
        f"pub const TITLE_W: usize = {TITLE_W};",
        f"pub const TITLE_H: usize = {TITLE_H};",
        "",
        f"pub const LOGO: [[u32; {logo_words}]; {LOGO_H}] = [",
        format_wide(logo_rows, LOGO_W),
        "];",
        "",
        f"pub const TITLE: [[u32; {title_words}]; {TITLE_H}] = [",
        format_wide(title_rows, TITLE_W),
        "];",
        "",
    ]

    OUT_FILE.write_text("\n".join(lines))
    print(f"wrote {OUT_FILE}")


if __name__ == "__main__":
    main()
