# GUI

The `gui` binary (`src/gui/`) is a native desktop client for Keres built on
[minifb](https://crates.io/crates/minifb) — a bare window + framebuffer
crate with no widget toolkit. Everything on screen, from the board to the
menu text, is drawn pixel-by-pixel by a small software rasterizer. It is
optional: the `gui` Cargo feature pulls in minifb (and its X11 dependency)
so the `server`/`keres` binaries and the musl Docker build stay free of it.
See `Cargo.toml` (`[profile.gui]`, the `gui` feature) and the Makefile's
`##@ Pixel assets`/`gui` targets for the build side.

## Module map

| Module | Responsibility |
|---|---|
| `main.rs` | Window/event loop, layout (button/tile hit-testing), screen composition, the `KERES_SNAPSHOT` headless render path |
| `app.rs` | Game state machine (`Screen`, `Mode`, `App`): click handling, move selection/disambiguation, the background AI-search thread, undo |
| `render.rs` | The rasterizer: `Canvas` primitives (rects, glyphs, bitmaps), piece-sprite composition, logical→window scaling |
| `icons.rs` | **Generated** — piece icon bitmasks. Do not edit by hand |
| `base.rs` | **Generated** — piece base-plaque half bitmasks. Do not edit by hand |
| `symbols.rs` | **Generated** — UI symbol bitmasks (currently just the stacking-hover arrow). Do not edit by hand |
| `font.rs` | Generated 8×10 1-bit bitmap font |
| `save.rs` | Per-game autosave: every move is written to its own file in a dedicated folder (via `dirs_next::data_dir()`), browsable from the menu's LOAD GAME screen |

## Logical canvas and scaling

The whole UI is drawn into a logical framebuffer (`render::logical_w`/
`logical_h`), not the real window buffer. Every coordinate in
`main.rs`/`render.rs` is a logical pixel. Each frame, `render::blit_to_window`
nearest-neighbor scales that canvas into the actual (resizable) window
buffer, preserving aspect ratio with letterbox bars (`COL_PAGE_BG`) on the
other axis. This is why the pixel art stays crisp at any window size instead
of blurring: the source is always scaled by an integer-ish factor with no
interpolation. `window_to_logical` performs the inverse mapping for mouse
hit-testing.

Layout constants worth knowing (`render.rs`):

- `TILE_W = 67`, `TILE_H = 56` — one board square, in logical pixels.
  Deliberately rectangular, not square — it matches the aspect ratio of the
  hand-drawn base plaque (see below), so pieces render pixel-perfect.
- `gutter(show_coords)` — the rank/file coordinate margin, left and bottom
  of the board. Collapses to 0 when `show_coords` is false.
- `topbar(show_coords)` — margin above the board; also collapses to 0.
- `SIDEBAR_W` — the right-hand panel (status, buttons, move history).
- `board_w`, `logical_w`, `logical_h` are all derived from the above and
  take `show_coords`, since they're no longer fixed: hiding the rank/file
  coordinates shrinks the logical canvas instead of leaving that margin as
  empty background, so the board fills more of the window — and renders
  bigger on screen, since `blit_to_window` scales a smaller canvas up
  further within the same physical window. There is no separate "hi-DPI"
  path either way — resolution comes entirely from choosing `TILE_W`/
  `TILE_H` and letting `blit_to_window` upscale. `main()` opens the window at
  `logical_w(true)` x `logical_h(true)` (coordinates start shown), so the
  on-screen scale is always 1:1 at startup — no interpolation, no
  dithering — until the user resizes the window or hides the coordinates.
  Each frame recomputes the logical size from the live `show_coords` value
  and resizes the logical buffer if it changed.

`main.rs` turns a board `Position` into a screen column/row via
`screen_coord` (and the inverse `board_pos`/`tile_at`), the single place
that applies the board-flip transform for "switch sides".

## Piece rendering

Each piece is a hand-drawn "token": a rectangular base plaque (an oval
cylinder-coin design, drawn as pixel art rather than computed from ellipse
math) with the piece's icon stamped on its face and its one-letter notation
(`letter_for`) stamped on its front wall. Everything is blitted pixel-exact
— no anti-aliasing, no scaling within a tile — so it stays crisp at the
game's native 1:1 resolution.

### Icon sources

Icons live as GIMP `.xcf` files in `assets/pixel/icons/` (one 25×25 1-bit
indexed image per `PieceType`, drawn full-size — no anti-aliasing). 25 is
odd on purpose: it gives every icon a true center column and row, which is
what lets pieces like the soldier or ballista come to a symmetric point
instead of a flat two-pixel-wide top (a 24×24 grid has no center pixel).

`Canvas::draw_icon` blits an icon mask with its top-left corner at a given
point, at an integer `scale` (currently 1:1, i.e. one icon pixel = one
logical canvas pixel).

### Base plaque sources

The base plaque is drawn from three files in `assets/pixel/base/`, each a
29×46 1-bit indexed image holding *half* the plaque (the artwork is
symmetric, so only the left half is drawn):

- `base-white.xcf` / `base-black.xcf` — the plaque's fill/ink coloring.
- `base-mask.xcf` — which pixels belong to the piece at all. The plaque
  isn't a solid rectangle (rounded coin corners, a curved top/bottom edge),
  so without this every token would render as a plain rectangle; black
  pixels here mean "draw", white means "leave the tile's background
  untouched".

At render time `render::draw_base` stamps that half at the tile's left
margin, then stamps it again flipped horizontally one pixel further in —
the two halves' shared edge column is identical by construction, so the
seam is invisible (see `draw_base_half`'s `flip` parameter, which just
reads each mask's columns back-to-front).

The white and black color assets aren't just recolored copies of each
other — each is a plain on/off mask, but which mask value means "fill" vs.
"ink" is *inverted* between them:

- `base-white.xcf` is mostly background with a thin dark ink stroke traced
  on it (mask bit = ink, drawn in `BORDER_ON_WHITE`; everything else is the
  `COIN_WHITE` fill).
- `base-black.xcf` is mostly filled solid with a thin light stroke left
  unfilled (mask bit = fill, drawn in `COIN_BLACK`; the stroke is
  `BORDER_ON_BLACK`).

Both encode the *same* curve — one as an outline on a light background, the
other as a solid fill bounded by that outline — because that's the natural
way to hand-draw a light piece (ink the border, leave the rest blank) versus
a dark piece (fill it solid, leave a highlight line unfilled). `draw_base`
picks the right color asset and pair from `piece.color`, and always applies
`base-mask.xcf` on top to decide whether a pixel is drawn at all.

### Symbol sources

`assets/pixel/symbols/` holds standalone 1-bit UI glyphs that aren't piece
icons — currently just `down-arrow.xcf` (23×16), the "this creates a stack"
hover cue (see below). Rasterized by `scripts/gen_symbols.py` into
`src/gui/symbols.rs` (`symbols::symbol_bits`, `symbols::SYMBOL_W`), drawn
with `Canvas::draw_bitmap` — a width/bit-packing-only sibling of `draw_icon`
that doesn't assume a square glyph or support `rotate180`.

### Splash art sources

`assets/pixel/logo.xcf` (46×37 crest) and `assets/pixel/title.xcf` (200×42
wordmark) are the splash-screen and main-menu art, rasterized by
`scripts/gen_splash.py` into `src/gui/splash.rs`. `draw_wordmark` in
`main.rs` stacks the crest over the wordmark and bottom-aligns the pair,
so the splash screen and the menu can each drop it where the old
bitmap-font "KERES" text used to sit.

Both are too wide for the one-`u32`-per-row packing the icons/base/symbols
use, so a row here is `ceil(width / 32)` `u32` words, leftmost word first,
each word left-aligned on its own first column (bit 31) — a short final
word pads on its low end, which keeps the decode loop uniform. They are
drawn with `Canvas::draw_wide_bitmap` rather than `draw_bitmap`.

### Generating `icons.rs` / `base.rs` / `symbols.rs` / `splash.rs`

All four are rasterized by the same headless-GIMP pipeline
(`scripts/pixel_raster.py`, Script-Fu batch mode via
`flatpak run org.gimp.GIMP`): a pixel with red channel < 128 is "on", packed
into a `[u32; height]` bitmask (bit `width - 1` = leftmost column).
Script-Fu emits each row as a string of `'0'`/`'1'` rather than a packed
integer — its fixnum arithmetic silently overflows into useless floats once
a row needs more bits than a machine int holds, which the 200px-wide
wordmark hits — so the bit packing happens Python-side, where integers are
arbitrary precision. `scripts/gen_icons.py`, `scripts/gen_base.py`,
`scripts/gen_symbols.py` and `scripts/gen_splash.py` each call into it and
write their respective generated file.
`format_rows`'s `indent` parameter controls the leading whitespace on each
packed row so the output matches what `cargo fmt` expects at that call
site's nesting depth (4 for a top-level `const` array as in `base.rs`, 12
for one nested two levels deep in a match arm as in `icons.rs`/
`symbols.rs`) — get it wrong and the file still compiles, but `cargo fmt
--check` will flag it (`gen_splash.py` does its own formatting instead —
its rows are arrays, not single values). `make gui` (or `make
pixel-assets`) regenerates all of them automatically whenever a `.xcf` is
newer than its generated output — see the `##@ Pixel assets` section of the
`Makefile`. Never hand-edit `src/gui/icons.rs`, `src/gui/base.rs`,
`src/gui/symbols.rs` or `src/gui/splash.rs`.

`assets/pixel/tile.xcf` is an authoring aid (a blank canvas matching one
board tile) — not a build input.

### Token layout

All the following are tile-local pixel offsets (origin = the tile's
top-left corner), defined as constants at the top of the piece-rendering
section of `render.rs`. They come straight from the art and must be
re-derived together if the base artwork or `TILE_W`/`TILE_H` ever change:

- Base plaque: left half's top-left corner at `(BASE_LEFT_X, BASE_TOP_Y)` =
  `(5, 5)`; the mirrored right half starts one pixel before the left half's
  right edge, at `x = BASE_LEFT_X + BASE_W - 1`.
- Icon: top-left corner at `(ICON_X, ICON_Y)` = `(21, 9)`.
- Letter: centered in a `LETTER_BOX_W` x `LETTER_BOX_H` (13x12) box at
  `(LETTER_BOX_X, LETTER_BOX_Y)` = `(27, 37)`.

These numbers were chosen so the whole token — plaque, icon and letter —
sits centered on the tile with an even margin on every side, and so nothing
overflows its own square for a lone piece (see Stacking below for the one
deliberate exception).

`render::draw_token` composes one upright piece token (plaque + icon +
letter) at a given tile-local origin.

### Orientation: upside-down opponent icons

Physical two-player board games often print the far side's pieces upside
down. `keres` takes a lighter touch: only the piece's *icon* rotates 180
degrees for the opponent — the plaque and letter are drawn identically for
both sides. `draw_token` passes `upside_down` straight through to
`Canvas::draw_icon`, which samples its bitmask back-to-front in both axes
(about the icon's own center, so its bounding box doesn't move) instead of
resampling a rendered image — pixel-exact, no dithering.

`main.rs`'s `is_upside_down` decides the flag: not by whose turn it is, but
by which color is currently rendered at the *top* of the screen (i.e. the
"far" side) — which flips along with `app.flipped` ("switch sides"), not
with `color_to_move`. Without a flip, Black is far/upside-down and White is
near/upright; toggling "switch sides" swaps that, along with the board
positions themselves (`screen_coord`).

### Stacking

`draw_piece` draws the bottom piece's full token first, then the top
piece's token shifted up just far enough that it covers everything above
the bottom piece's letter box while leaving that letter box itself
visible — so a stack still shows both pieces' notation, with the top
piece's icon and plaque free to overflow into the square above it. Board
squares are all painted before any piece is drawn, and pieces are drawn in
**screen-row** order, top to bottom, so a lower row's overflowing piece
correctly paints over whatever the row above already drew. This matters
specifically because of the board flip ("switch sides"): `draw_board`
iterates screen rows/columns (`sy`/`sx` in `main.rs`) and maps each back to
a board `Position` via `board_pos`, rather than iterating board rows
directly — board row 0 is screen row 8 once flipped, so iterating board rows
in order would draw screen bottom-to-top and get the overlap backwards.
Iterating screen space keeps the paint order correct either way.

## Tile highlights and display options

Tile highlights (`draw_board` in `main.rs`) are full-tile alpha washes —
`Canvas::fill_rect_alpha` over the square, not a dot or ring — using a
palette matched to the Web platform's SVG board (`render::COL_HL_*`, see
keres-platform's `SVGBoardView.ts`/`board.css` and
`GameController.updateOverlays`) so the native and Web clients read the same
way. Layered weakest to strongest so a stronger highlight wins where they'd
overlap: enemy threat (`COL_HL_THREAT`, red) under hover preview
(`COL_HL_HOVER`, gold) under selection (`COL_HL_SELECTED`, blue) under the
selected piece's own target squares (`COL_HL_POTENTIAL`, green).

The `App` methods driving this (`app.rs`) are hover-based, mirroring the Web
client's `GameController`:

- `App::hovered` tracks the board square under the mouse, updated live every
  frame in `main.rs` (not just on click).
- `hover_preview_squares()` — hovering a friendly piece (the side to move's
  own) immediately previews its moves, gold, whenever nothing is selected.
- `hover_threat_squares()` — hovering an enemy piece shows *that piece's*
  own possible moves, red, but only when "show threats" is enabled. That
  toggle's only purpose is gating this: it is not a general always-on
  threat display.
- Both return empty once something is selected — the selected piece's
  (stronger) `target_squares()` highlight takes over and the hover preview
  disappears, so the two never compete for attention.
- `hovered_stack_target()` — true when a piece is selected and the hovered
  square is a legal target that lands on a friendly, stackable piece.
  Landing there merges into a stack rather than just moving there, which is
  easy to mistake for "reselect this piece instead" — so `draw_board` stamps
  a big gold down-arrow (`symbols::Symbol::DownArrow`, see above) on top of
  the square and piece, at `(23, -7)` relative to the tile's top-left
  corner, whenever this is true. This arrow doesn't exist in the Web client
  yet.

## Verifying changes visually

There is no interactive test harness, but the binary supports a headless
snapshot mode used for exactly this kind of visual check:

```
KERES_SNAPSHOT=/tmp/out.ppm KERES_SCREEN=<screen> cargo run --features gui --bin gui
```

`KERES_SCREEN` selects a canned scenario — `run_snapshot`'s `match` in
`main.rs` is the authoritative list, since this one drifts as screens are
added: `splash`, `menu`, `new_game`, `rules`, `rules_in_game`,
`quick_help`, `load_game`, `playing`, `selected`, `stacked`, `gameover`,
`flipped`, `threats`, `history`, `nocoords`, `hover_friendly`,
`hover_threat`, `hover_stack`, `last_move`, `confirm_menu`,
`stacked_close_hover`, `help_tab`, `help_tab_stack`, `help_tab_empty`,
`settings_tab`, `move_anim_start`, `move_anim_mid`. Add
`KERES_MOUSE=x,y` (logical-canvas pixel coordinates) to also check a dialog
button's hover state. It never opens a window or touches the display, so it
works over SSH/CI; save I/O is also redirected to a throwaway temp directory
(see `save::set_test_dir_override`), so taking a snapshot never writes into
the real save folder. The output is a raw PPM; convert it for viewing,
e.g. `magick out.ppm -filter point -resize 300% out.png` (nearest-neighbor,
matching how the real window scales the canvas, so what you see is what
the game actually renders).

## Tests

`make test` runs the full workspace suite including the `gui` feature.
GUI-specific tests live in `src/gui/app.rs` (state machine: undo, save/
resume, AI turn handling, move disambiguation) and `src/gui/render.rs`
(the logical/window scaling round-trip). There is no pixel-diffing test —
rendering correctness is checked visually via snapshots (above).
