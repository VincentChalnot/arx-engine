"""Shared headless-GIMP rasterizer used by scripts/gen_icons.py and
scripts/gen_base.py to turn a 1-bit indexed .xcf into row bitmasks.

A pixel is "on" when its red channel is below 128 (the two expected colors
in these indexed sources are (0,0,0) and (255,255,255)). Each returned row
is a `width`-bit integer, bit `width - 1` = leftmost column, bit 0 =
rightmost — ready to pack into a Rust `[u32; height]` array.
"""
import pathlib
import subprocess
import sys

GIMP_APP_ID = "org.gimp.GIMP"

SCHEME_TEMPLATE = r"""
(let* ((image (car (gimp-file-load RUN-NONINTERACTIVE "{path}" "{name}")))
       (layer (vector-ref (car (gimp-image-get-layers image)) 0)))
  (define (row-value y)
    (let loop ((x 0) (acc 0))
      (if (= x {width})
          acc
          (loop (+ x 1)
                (if (< (car (car (gimp-drawable-get-pixel layer x y))) 128)
                    (+ acc (expt 2 (- {maxbit} x)))
                    acc)))))
  (let loop ((y 0))
    (if (< y {height})
        (begin
          (display (row-value y))
          (display " ")
          (loop (+ y 1)))))
  (newline))
"""


def rasterize(xcf_path: pathlib.Path, width: int, height: int) -> list[int]:
    """Rasterize one .xcf into `height` row bitmasks, `width` bits each."""
    script = SCHEME_TEMPLATE.format(
        path=str(xcf_path), name=xcf_path.name, width=width, height=height, maxbit=width - 1
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
        if len(parts) == height and all(p.isdigit() for p in parts):
            return [int(p) for p in parts]
    sys.stderr.write(result.stdout)
    sys.stderr.write(result.stderr)
    raise RuntimeError(f"failed to extract pixel rows from {xcf_path}")


def format_rows(rows: list[int], width: int, indent: int = 8) -> str:
    """`indent` is the leading-space count rustfmt expects for the array
    literal at its call site — 4 for a top-level `const X: [u32; N] = [...]`
    (see gen_base.py), 12 for one nested in a two-level-deep match arm
    (see gen_icons.py/gen_symbols.py). Pass the wrong depth and the file
    still compiles, but `cargo fmt --check` will flag it.
    """
    hex_digits = (width + 3) // 4
    hex_values = [f"0x{v:0{hex_digits}X}" for v in rows]
    lines = []
    prefix = " " * indent
    for i in range(0, len(hex_values), 8):
        chunk = hex_values[i : i + 8]
        lines.append(prefix + ", ".join(chunk) + ",")
    return "\n".join(lines)
