//! Software rasterizer: framebuffer primitives, bitmap font blitting and
//! piece-sprite composition (background/border/icon/text layers).

use crate::base;
use crate::font;
use crate::icons::{self, PieceIcon};
use keres_engine::{Color, Piece, PieceType};

/// Board square width/height. Rectangular (not square) on purpose — it
/// matches the aspect ratio of the hand-drawn base plaque (see `draw_piece`).
pub const TILE_W: i32 = 67;
pub const TILE_H: i32 = 56;
pub const BOARD_PX_W: i32 = TILE_W * 9;
pub const BOARD_PX_H: i32 = TILE_H * 9;
pub const SIDEBAR_W: i32 = 300;

/// Left/bottom rank-and-file coordinate margin. Collapses to 0 when
/// coordinates are hidden, so the board fills that space instead of leaving
/// it as empty background — see `logical_w`/`logical_h`.
pub fn gutter(show_coords: bool) -> i32 {
    if show_coords {
        26
    } else {
        0
    }
}

/// Margin above the board; collapses together with `gutter` when
/// coordinates are hidden.
pub fn topbar(show_coords: bool) -> i32 {
    if show_coords {
        20
    } else {
        0
    }
}

pub fn board_w(show_coords: bool) -> i32 {
    gutter(show_coords) + BOARD_PX_W
}

/// The whole UI is drawn into a logical framebuffer this size, not the real
/// window buffer. The window itself can be resized freely; `blit_to_window`
/// nearest-neighbor scales this canvas into the real framebuffer so pixel
/// art never blurs or stretches. It shrinks when coordinates are hidden
/// (no gutter/topbar margin), which is what makes the board render bigger
/// on screen in that mode: the same physical window scales a smaller
/// canvas up further.
pub fn logical_w(show_coords: bool) -> i32 {
    board_w(show_coords) + SIDEBAR_W
}

pub fn logical_h(show_coords: bool) -> i32 {
    topbar(show_coords) + BOARD_PX_H + gutter(show_coords)
}

pub const COL_PAGE_BG: u32 = 0x14140f;
pub const COL_LIGHT_SQ: u32 = 0xf5f5dc;
pub const COL_DARK_SQ: u32 = 0xd2b48c;
pub const COL_COORD: u32 = 0x9c8a63;
pub const COL_STATUS: u32 = 0xe8dcc0;
pub const COL_SELECT: u32 = 0xe0913c;
pub const COL_SIDEBAR_BG: u32 = 0x1c1b16;
pub const COL_BTN_DISABLED: u32 = 0x5a5346;

/// The single button height used everywhere in the app — sidebar buttons,
/// menu buttons, dialog buttons, list-screen buttons — so every clickable
/// button reads as the same control regardless of which screen it's on.
pub const BTN_H: i32 = 28;
pub const BTN_BORDER: i32 = 2;

// Board tile-highlight palette, matched to the Web platform's SVG board
// (see keres-platform's SVGBoardView.ts / GameController.updateOverlays) so
// the native and Web clients read the same way. Each is a full-tile alpha
// wash rather than a dot/ring, again mirroring the Web renderer.
pub const COL_HL_SELECTED: u32 = 0x7fa0dd;
pub const COL_HL_SELECTED_A: f32 = 0.6;
pub const COL_HL_POTENTIAL: u32 = 0x55d157;
pub const COL_HL_POTENTIAL_A: f32 = 0.5;
/// Also used for the "this creates a stack" hover arrow (see
/// `App::hovered_stack_target`) — same gold as the hover-preview wash.
pub const COL_HL_HOVER: u32 = COL_GOLD;
pub const COL_HL_HOVER_A: f32 = 0.4;
pub const COL_HL_THREAT: u32 = 0xff4444;
pub const COL_HL_THREAT_A: f32 = 0.5;
/// Last-moved-piece highlight: the square it left and the square it landed
/// on (see `App::last_move`).
pub const COL_HL_LAST_MOVE: u32 = 0xe89038;
pub const COL_HL_LAST_MOVE_A: f32 = 0.45;

/// "Gold" accent from the palette, used for the hover-preview wash and (see
/// `draw_button`) for hovered buttons.
pub const COL_GOLD: u32 = 0xe1ca58;

pub const COIN_WHITE: u32 = 0xf3ead2;
pub const COIN_BLACK: u32 = 0x1c1a16;
pub const BORDER_ON_WHITE: u32 = 0x1c1a16;
pub const BORDER_ON_BLACK: u32 = 0xd8cba4;
pub const FG_ON_WHITE: u32 = 0x1c1a16;
pub const FG_ON_BLACK: u32 = 0xf3ead2;

pub struct Canvas<'a> {
    pub buf: &'a mut [u32],
    pub w: i32,
    pub h: i32,
}

impl<'a> Canvas<'a> {
    #[inline]
    pub fn put(&mut self, x: i32, y: i32, color: u32) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        self.buf[(y * self.w + x) as usize] = color;
    }

    pub fn fill_rect_alpha(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u32, alpha: f32) {
        for y in y0.max(0)..y1.min(self.h) {
            for x in x0.max(0)..x1.min(self.w) {
                let idx = (y * self.w + x) as usize;
                self.buf[idx] = blend(self.buf[idx], color, alpha);
            }
        }
    }

    pub fn fill_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
        for y in y0.max(0)..y1.min(self.h) {
            for x in x0.max(0)..x1.min(self.w) {
                self.put(x, y, color);
            }
        }
    }

    pub fn stroke_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, thickness: i32, color: u32) {
        self.fill_rect(x0, y0, x1, y0 + thickness, color);
        self.fill_rect(x0, y1 - thickness, x1, y1, color);
        self.fill_rect(x0, y0, x0 + thickness, y1, color);
        self.fill_rect(x1 - thickness, y0, x1, y1, color);
    }

    /// Draw one font glyph at integer `scale`, top-left corner at (x, y).
    pub fn draw_glyph(&mut self, x: i32, y: i32, ch: char, scale: i32, color: u32) {
        let rows = font::glyph(ch);
        for (ry, row) in rows.iter().enumerate() {
            for cx in 0..font::FONT_W {
                if (row >> (font::FONT_W - 1 - cx)) & 1 == 1 {
                    let px = x + (cx as i32) * scale;
                    let py = y + (ry as i32) * scale;
                    self.fill_rect(px, py, px + scale, py + scale, color);
                }
            }
        }
    }

    /// Draw left-aligned text; returns the total pixel width consumed.
    pub fn draw_text(&mut self, x: i32, y: i32, text: &str, scale: i32, color: u32) -> i32 {
        let advance = (font::FONT_W as i32 + 1) * scale;
        let mut cx = x;
        for ch in text.chars() {
            self.draw_glyph(cx, y, ch, scale, color);
            cx += advance;
        }
        cx - x - scale
    }

    pub fn text_width(text: &str, scale: i32) -> i32 {
        let advance = (font::FONT_W as i32 + 1) * scale;
        (text.chars().count() as i32) * advance - scale
    }

    /// Draw text horizontally centered on `cx`.
    pub fn draw_text_centered(&mut self, cx: i32, y: i32, text: &str, scale: i32, color: u32) {
        let w = Self::text_width(text, scale);
        self.draw_text(cx - w / 2, y, text, scale, color);
    }

    /// Draw a piece icon mask with its top-left corner at (x0, y0), at
    /// integer `scale`. `rotate180` samples the mask back-to-front in both
    /// axes (about its own center, so the bounding box at (x0, y0) is
    /// unchanged) — used to draw the opponent's icon upside down.
    pub fn draw_icon(
        &mut self,
        x0: i32,
        y0: i32,
        icon: PieceIcon,
        scale: i32,
        color: u32,
        rotate180: bool,
    ) {
        let bits = icons::icon_bits(icon);
        let n = icons::ICON_N as i32;
        for ry in 0..n {
            let row = if rotate180 {
                bits[(n - 1 - ry) as usize]
            } else {
                bits[ry as usize]
            };
            for cxi in 0..n {
                let src_col = if rotate180 { n - 1 - cxi } else { cxi };
                if (row >> (n - 1 - src_col)) & 1 == 1 {
                    let px = x0 + cxi * scale;
                    let py = y0 + ry * scale;
                    self.fill_rect(px, py, px + scale, py + scale, color);
                }
            }
        }
    }

    /// Draw an arbitrary 1-bit bitmap (see `symbols::symbol_bits`) with its
    /// top-left corner at `(x0, y0)`, one bitmap pixel = one canvas pixel.
    /// `width` bits per row, bit `width - 1` = leftmost column (same packing
    /// as `draw_icon`).
    pub fn draw_bitmap(&mut self, x0: i32, y0: i32, bits: &[u32], width: i32, color: u32) {
        for (ry, row) in bits.iter().enumerate() {
            for cx in 0..width {
                if (row >> (width - 1 - cx)) & 1 == 1 {
                    self.put(x0 + cx, y0 + ry as i32, color);
                }
            }
        }
    }
}

fn icon_for(pt: PieceType) -> PieceIcon {
    match pt {
        PieceType::Soldier => PieceIcon::Soldier,
        PieceType::Bishop => PieceIcon::Bishop,
        PieceType::Rook => PieceIcon::Rook,
        PieceType::Paladin => PieceIcon::Paladin,
        PieceType::Guard => PieceIcon::Guard,
        PieceType::Knight => PieceIcon::Knight,
        PieceType::Ballista => PieceIcon::Ballista,
        PieceType::King => PieceIcon::King,
    }
}

pub fn letter_for(pt: PieceType) -> char {
    match pt {
        PieceType::Soldier => 'S',
        PieceType::Bishop => 'B',
        PieceType::Rook => 'R',
        PieceType::Paladin => 'P',
        PieceType::Guard => 'G',
        PieceType::Knight => 'N',
        PieceType::Ballista => 'L',
        PieceType::King => 'K',
    }
}

// Piece-token layout, all in tile-local pixels (origin = tile's top-left
// corner). The base plaque is drawn from two mirrored halves of the
// `base::BASE_W` x `base::BASE_H` artwork with a 1px seam at the tile's
// horizontal center; icon and letter are then stamped on top of it. These
// numbers come directly from the art (see docs/GUI.md) — if the base
// artwork or TILE_W/TILE_H ever change, re-derive them together.
const BASE_LEFT_X: i32 = 5;
const BASE_TOP_Y: i32 = 5;
const ICON_X: i32 = 21;
const ICON_Y: i32 = 9;
const LETTER_BOX_X: i32 = 27;
const LETTER_BOX_Y: i32 = 37;
const LETTER_BOX_W: i32 = 13;
const LETTER_BOX_H: i32 = 12;

/// Draw one mirrored-pair half of the base plaque's bitmask with its
/// top-left corner at `(x0, y0)`. `flip` reads the source columns
/// back-to-front, which is how the right half is produced from the same
/// artwork as the left half. `mask` marks which pixels belong to the piece
/// at all — the plaque isn't a solid rectangle, so a masked-off pixel is
/// left untouched (the square's own background shows through) rather than
/// painted `off_color`. `on_color`/`off_color` are which color each
/// `bits` mask bit paints where the mask allows drawing — see `draw_base`
/// for why that mapping is inverted between the white and black assets.
#[allow(clippy::too_many_arguments)]
fn draw_base_half(
    canvas: &mut Canvas,
    x0: i32,
    y0: i32,
    bits: &[u32; base::BASE_H],
    mask: &[u32; base::BASE_H],
    flip: bool,
    on_color: u32,
    off_color: u32,
) {
    let w = base::BASE_W as i32;
    for (row, (bitrow, maskrow)) in bits.iter().zip(mask.iter()).enumerate() {
        for col in 0..w {
            let src_col = if flip { w - 1 - col } else { col };
            let bit = w - 1 - src_col;
            if (maskrow >> bit) & 1 == 0 {
                continue;
            }
            let on = (bitrow >> bit) & 1 == 1;
            let color = if on { on_color } else { off_color };
            canvas.put(x0 + col, y0 + row as i32, color);
        }
    }
}

/// Draw the full base plaque (both mirrored halves) for one color at
/// tile-local origin. Both `base::BASE_WHITE` and `base::BASE_BLACK` are
/// plain on/off masks, but which mask value means "fill" vs. "border ink"
/// is flipped between them: the white asset is mostly background with a
/// thin dark ink stroke (mask bit = ink), while the black asset is mostly
/// filled with a thin light stroke (mask bit = fill). See docs/GUI.md.
fn draw_base(canvas: &mut Canvas, tx: i32, ty: i32, is_white: bool) {
    let (bits, on_color, off_color) = if is_white {
        (&base::BASE_WHITE, BORDER_ON_WHITE, COIN_WHITE)
    } else {
        (&base::BASE_BLACK, COIN_BLACK, BORDER_ON_BLACK)
    };
    let mask = &base::BASE_MASK;
    let w = base::BASE_W as i32;
    draw_base_half(
        canvas,
        tx + BASE_LEFT_X,
        ty + BASE_TOP_Y,
        bits,
        mask,
        false,
        on_color,
        off_color,
    );
    draw_base_half(
        canvas,
        tx + BASE_LEFT_X + w - 1,
        ty + BASE_TOP_Y,
        bits,
        mask,
        true,
        on_color,
        off_color,
    );
}

/// Draw one full piece token — base plaque, icon and one-letter notation —
/// at tile-local origin `(tx, ty)`. Only the icon is affected by
/// `upside_down`: the opponent's icon is rotated 180 degrees in place (see
/// `Canvas::draw_icon`) so it doesn't read as the near side's pictogram at
/// a glance, while the plaque and letter stay identical either way.
fn draw_token(
    canvas: &mut Canvas,
    tx: i32,
    ty: i32,
    pt: PieceType,
    is_white: bool,
    upside_down: bool,
) {
    let fg = if is_white { FG_ON_WHITE } else { FG_ON_BLACK };
    draw_base(canvas, tx, ty, is_white);
    canvas.draw_icon(tx + ICON_X, ty + ICON_Y, icon_for(pt), 1, fg, upside_down);
    let letter_cx = tx + LETTER_BOX_X + LETTER_BOX_W / 2;
    let letter_y = ty + LETTER_BOX_Y + (LETTER_BOX_H - font::FONT_H as i32) / 2;
    canvas.draw_text_centered(letter_cx, letter_y, &letter_for(pt).to_string(), 1, fg);
}

/// Draw a piece (lone or stacked) with its token(s) positioned at tile-local
/// origin `(tx, ty)` — i.e. the tile's top-left corner, not its center; the
/// base plaque's own margins center it within the tile. `upside_down` marks
/// the opponent's pieces (see `draw_token`).
///
/// A stack draws the bottom piece's full token first, then the top piece's
/// token shifted up just enough to cover everything above the bottom
/// piece's letter box while leaving that letter box itself visible — so
/// the bottom piece stays identifiable by its notation, peeking out from
/// under the top piece, which is free to overflow into the square above.
pub fn draw_piece(canvas: &mut Canvas, tx: i32, ty: i32, piece: &Piece, upside_down: bool) {
    let is_white = piece.color == Color::White;
    if let Some(top) = piece.top {
        draw_token(canvas, tx, ty, piece.bottom, is_white, upside_down);
        let stack_shift = BASE_TOP_Y + base::BASE_H as i32 - LETTER_BOX_Y;
        draw_token(canvas, tx, ty - stack_shift, top, is_white, upside_down);
    } else {
        draw_token(canvas, tx, ty, piece.bottom, is_white, upside_down);
    }
}

fn blend(base: u32, overlay: u32, alpha: f32) -> u32 {
    let br = (base >> 16) & 0xff;
    let bg = (base >> 8) & 0xff;
    let bb = base & 0xff;
    let or_ = (overlay >> 16) & 0xff;
    let og = (overlay >> 8) & 0xff;
    let ob = overlay & 0xff;
    let mix = |b: u32, o: u32| -> u32 { (b as f32 * (1.0 - alpha) + o as f32 * alpha) as u32 };
    (mix(br, or_) << 16) | (mix(bg, og) << 8) | mix(bb, ob)
}

/// Nearest-neighbor blit of the fixed logical framebuffer into an
/// arbitrarily sized output buffer, preserving aspect ratio via letterbox
/// bars so pixel art is scaled crisply instead of blurred or stretched.
pub fn blit_to_window(
    logical: &[u32],
    out: &mut [u32],
    out_w: i32,
    out_h: i32,
    logical_w: i32,
    logical_h: i32,
) {
    let scale = (out_w as f32 / logical_w as f32)
        .min(out_h as f32 / logical_h as f32)
        .max(0.05);
    let draw_w = (logical_w as f32 * scale) as i32;
    let draw_h = (logical_h as f32 * scale) as i32;
    let off_x = (out_w - draw_w) / 2;
    let off_y = (out_h - draw_h) / 2;
    for y in 0..out_h {
        for x in 0..out_w {
            let idx = (y * out_w + x) as usize;
            if x < off_x || y < off_y || x >= off_x + draw_w || y >= off_y + draw_h {
                out[idx] = COL_PAGE_BG;
                continue;
            }
            let sx = (((x - off_x) as f32) / scale) as i32;
            let sy = (((y - off_y) as f32) / scale) as i32;
            let sx = sx.clamp(0, logical_w - 1);
            let sy = sy.clamp(0, logical_h - 1);
            out[idx] = logical[(sy * logical_w + sx) as usize];
        }
    }
}

/// Map a real window mouse position back into logical canvas coordinates,
/// inverse of `blit_to_window`. Returns None inside the letterbox margin.
pub fn window_to_logical(
    mx: i32,
    my: i32,
    out_w: i32,
    out_h: i32,
    logical_w: i32,
    logical_h: i32,
) -> Option<(i32, i32)> {
    let scale = (out_w as f32 / logical_w as f32)
        .min(out_h as f32 / logical_h as f32)
        .max(0.05);
    let draw_w = (logical_w as f32 * scale) as i32;
    let draw_h = (logical_h as f32 * scale) as i32;
    let off_x = (out_w - draw_w) / 2;
    let off_y = (out_h - draw_h) / 2;
    if mx < off_x || my < off_y || mx >= off_x + draw_w || my >= off_y + draw_h {
        return None;
    }
    let lx = (((mx - off_x) as f32) / scale) as i32;
    let ly = (((my - off_y) as f32) / scale) as i32;
    Some((lx.clamp(0, logical_w - 1), ly.clamp(0, logical_h - 1)))
}

/// Draw the one button style used across the whole app: outlined normally,
/// filled solid gold with inverted text when the mouse hovers it (matching
/// what used to be only the modal-dialog buttons' behavior), dimmed and
/// never hoverable when `enabled` is false.
#[allow(clippy::too_many_arguments)]
pub fn draw_button(
    c: &mut Canvas,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    label: &str,
    enabled: bool,
    hovered: bool,
) {
    let cx = (x0 + x1) / 2;
    let cy = (y0 + y1) / 2;
    if !enabled {
        c.stroke_rect(x0, y0, x1, y1, BTN_BORDER, COL_BTN_DISABLED);
        c.draw_text_centered(cx, cy - 5, label, 1, COL_BTN_DISABLED);
    } else if hovered {
        c.fill_rect(x0, y0, x1, y1, COL_GOLD);
        c.draw_text_centered(cx, cy - 5, label, 1, COL_PAGE_BG);
    } else {
        c.stroke_rect(x0, y0, x1, y1, BTN_BORDER, COL_STATUS);
        c.draw_text_centered(cx, cy - 5, label, 1, COL_STATUS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blit_preserves_aspect_and_letterboxes_wide_windows() {
        let lw = logical_w(true);
        let lh = logical_h(true);
        let logical = vec![0x00ffffffu32; (lw * lh) as usize];
        let out_w = lw * 2;
        let out_h = lh;
        let mut out = vec![0u32; (out_w * out_h) as usize];
        blit_to_window(&logical, &mut out, out_w, out_h, lw, lh);
        let cx = out_w / 2;
        let cy = out_h / 2;
        assert_eq!(
            out[(cy * out_w + cx) as usize],
            0x00ffffff,
            "center should show the logical canvas"
        );
        assert_eq!(
            out[(cy * out_w + 2) as usize],
            COL_PAGE_BG,
            "far-left column should be letterbox background"
        );
    }

    #[test]
    fn window_to_logical_round_trips_through_blit() {
        let lw = logical_w(true);
        let lh = logical_h(true);
        let out_w = lw * 3 / 2;
        let out_h = lh * 3 / 2;
        let (lx, ly) =
            window_to_logical(out_w / 2, out_h / 2, out_w, out_h, lw, lh).expect("center is drawn");
        assert!((0..lw).contains(&lx));
        assert!((0..lh).contains(&ly));

        // A point in the letterbox margin of a mismatched-aspect window maps to nothing.
        let wide_w = lw * 2;
        let wide_h = lh;
        assert!(window_to_logical(1, 1, wide_w, wide_h, lw, lh).is_none());
    }

    #[test]
    fn hiding_coords_shrinks_the_logical_canvas() {
        assert!(logical_w(false) < logical_w(true));
        assert!(logical_h(false) < logical_h(true));
        assert_eq!(gutter(false), 0);
        assert_eq!(topbar(false), 0);
    }
}
