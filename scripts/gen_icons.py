#!/usr/bin/env python3
"""Regenerate src/gui/icons.rs from the 24x24 1-bit indexed .xcf files in
assets/pixel/icons/.

Each .xcf is rasterized by GIMP (Script-Fu batch mode) into a 24x24 grid of
on/off pixels, then packed into the [u32; 24] bitmask format `icon_bits`
expects (bit 23 = leftmost column, bit 0 = rightmost).

Invoked by `make gui` whenever an .xcf is newer than src/gui/icons.rs — see
the Makefile. Do not edit src/gui/icons.rs by hand, it is overwritten here.
"""
import pathlib
import subprocess
import sys

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
ICON_DIR = REPO_ROOT / "assets" / "pixel" / "icons"
OUT_FILE = REPO_ROOT / "src" / "gui" / "icons.rs"
ICON_SIZE = 24
GIMP_APP_ID = "org.gimp.GIMP"

# A dark pixel (indexed color resolved to RGB by GIMP's script-fu binding) is
# "on"; anything else ("off") is background. Threshold sits between the two
# expected colors (0,0,0) and (255,255,255) of a 1-bit indexed image.
SCHEME_TEMPLATE = r"""
(let* ((image (car (gimp-file-load RUN-NONINTERACTIVE "{path}" "{name}")))
       (layer (vector-ref (car (gimp-image-get-layers image)) 0)))
  (define (row-value y)
    (let loop ((x 0) (acc 0))
      (if (= x {size})
          acc
          (loop (+ x 1)
                (if (< (car (car (gimp-drawable-get-pixel layer x y))) 128)
                    (+ acc (expt 2 (- {maxbit} x)))
                    acc)))))
  (let loop ((y 0))
    (if (< y {size})
        (begin
          (display (row-value y))
          (display " ")
          (loop (+ y 1)))))
  (newline))
"""


def variant_name(stem: str) -> str:
    return "".join(part.capitalize() for part in stem.split("_"))


def extract_rows(xcf_path: pathlib.Path) -> list[int]:
    script = SCHEME_TEMPLATE.format(
        path=str(xcf_path), name=xcf_path.name, size=ICON_SIZE, maxbit=ICON_SIZE - 1
    )
    cmd = [
        "flatpak", "run", GIMP_APP_ID,
        "-i", "-d", "-f",
        "--batch-interpreter=plug-in-script-fu-eval",
        "-b", script,
        "-b", "(gimp-quit 0)",
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
    for line in result.stdout.splitlines():
        parts = line.split()
        if len(parts) == ICON_SIZE and all(p.isdigit() for p in parts):
            return [int(p) for p in parts]
    sys.stderr.write(result.stdout)
    sys.stderr.write(result.stderr)
    raise RuntimeError(f"failed to extract pixel rows from {xcf_path}")


def format_rows(rows: list[int]) -> str:
    hex_values = [f"0x{v:06X}" for v in rows]
    lines = []
    for i in range(0, len(hex_values), 8):
        chunk = hex_values[i : i + 8]
        lines.append("            " + ", ".join(chunk) + ",")
    return "\n".join(lines)


def main() -> None:
    xcf_files = sorted(ICON_DIR.glob("*.xcf"))
    if not xcf_files:
        raise SystemExit(f"no .xcf files found in {ICON_DIR}")

    arms = []
    variants = []
    for xcf_path in xcf_files:
        variant = variant_name(xcf_path.stem)
        variants.append(variant)
        rows = extract_rows(xcf_path)
        arms.append(f"        PieceIcon::{variant} => [\n{format_rows(rows)}\n        ],")

    lines = [
        "// Auto-generated 24x24 1-bit piece icon masks.",
        "// Regenerate with `scripts/gen_icons.py` (run automatically by `make gui`)",
        "// after editing the .xcf sources in assets/pixel/icons/. Do not edit by hand.",
        f"pub const ICON_N: usize = {ICON_SIZE};",
        "",
        "pub fn icon_bits(name: PieceIcon) -> [u32; 24] {",
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
