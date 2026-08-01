//! Software rasterizer: framebuffer primitives, bitmap font blitting and
//! piece-sprite composition (background/border/icon/text layers).

use crate::font;
use crate::icons::{self, PieceIcon};
use keres_engine::{Color, Piece, PieceType};

pub const TILE: i32 = 72;
pub const GUTTER: i32 = 26;
pub const TOPBAR: i32 = 20;
pub const BOARD_PX: i32 = TILE * 9;
pub const SIDEBAR_W: i32 = 300;
pub const BOARD_W: i32 = GUTTER + BOARD_PX;
/// Fixed logical canvas the whole UI is drawn into. The window itself can
/// be resized freely; `blit_to_window` nearest-neighbor scales this canvas
/// into the real framebuffer so pixel art never blurs or stretches.
pub const LOGICAL_W: i32 = BOARD_W + SIDEBAR_W;
pub const LOGICAL_H: i32 = TOPBAR + BOARD_PX + GUTTER;

pub const COL_PAGE_BG: u32 = 0x14140f;
pub const COL_LIGHT_SQ: u32 = 0xefe3c8;
pub const COL_DARK_SQ: u32 = 0xc79a58;
pub const COL_COORD: u32 = 0x9c8a63;
pub const COL_STATUS: u32 = 0xe8dcc0;
pub const COL_SELECT: u32 = 0xe0913c;
pub const COL_MOVE_DOT: u32 = 0x4fae5a;
pub const COL_CAPTURE_RING: u32 = 0xcf4b4b;
pub const COL_THREAT: u32 = 0xff5a3c;
pub const COL_SIDEBAR_BG: u32 = 0x1c1b16;
pub const COL_BTN_DISABLED: u32 = 0x5a5346;

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

    /// Fill an axis-aligned ellipse. `half`: None = full, Some(true) = bottom
    /// half only (dy >= 0), Some(false) = top half only (dy <= 0).
    pub fn fill_ellipse(
        &mut self,
        cx: i32,
        cy: i32,
        rx: i32,
        ry: i32,
        half: Option<bool>,
        color: u32,
    ) {
        for y in (cy - ry).max(0)..=(cy + ry).min(self.h - 1) {
            let dy = y - cy;
            if let Some(bottom) = half {
                if bottom && dy < 0 {
                    continue;
                }
                if !bottom && dy > 0 {
                    continue;
                }
            }
            for x in (cx - rx).max(0)..=(cx + rx).min(self.w - 1) {
                let dx = x - cx;
                let v = (dx * dx) as f32 / (rx * rx) as f32 + (dy * dy) as f32 / (ry * ry) as f32;
                if v <= 1.0 {
                    self.put(x, y, color);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn stroke_ellipse(
        &mut self,
        cx: i32,
        cy: i32,
        rx: i32,
        ry: i32,
        thickness: i32,
        half: Option<bool>,
        color: u32,
    ) {
        let irx = (rx - thickness).max(0);
        let iry = (ry - thickness).max(0);
        for y in (cy - ry).max(0)..=(cy + ry).min(self.h - 1) {
            let dy = y - cy;
            if let Some(bottom) = half {
                if bottom && dy < 0 {
                    continue;
                }
                if !bottom && dy > 0 {
                    continue;
                }
            }
            for x in (cx - rx).max(0)..=(cx + rx).min(self.w - 1) {
                let dx = x - cx;
                let outer =
                    (dx * dx) as f32 / (rx * rx) as f32 + (dy * dy) as f32 / (ry * ry) as f32;
                let inner = (dx * dx) as f32 / (irx * irx).max(1) as f32
                    + (dy * dy) as f32 / (iry * iry).max(1) as f32;
                if outer <= 1.0 && inner > 1.0 {
                    self.put(x, y, color);
                }
            }
        }
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

    /// Draw a piece icon mask centered at (cx, cy) with integer `scale`.
    pub fn draw_icon(&mut self, cx: i32, cy: i32, icon: PieceIcon, scale: i32, color: u32) {
        let bits = icons::icon_bits(icon);
        let n = icons::ICON_N as i32;
        let ox = cx - (n * scale) / 2;
        let oy = cy - (n * scale) / 2;
        for (ry, row) in bits.iter().enumerate() {
            for cxi in 0..n {
                if (row >> (n - 1 - cxi)) & 1 == 1 {
                    let px = ox + cxi * scale;
                    let py = oy + (ry as i32) * scale;
                    self.fill_rect(px, py, px + scale, py + scale, color);
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

/// Draw a single piece token centered at (cx, cy) — the cylinder body
/// (background mask), its border (mask, color depends on piece color),
/// the icon mask and the shortened-notation text mask.
pub fn draw_piece(canvas: &mut Canvas, cx: i32, cy: i32, piece: &Piece) {
    let is_white = piece.color == Color::White;
    let bg = if is_white { COIN_WHITE } else { COIN_BLACK };
    let border = if is_white {
        BORDER_ON_WHITE
    } else {
        BORDER_ON_BLACK
    };
    let fg = if is_white { FG_ON_WHITE } else { FG_ON_BLACK };

    let rx = 32;
    let ry = 22;
    let lip = 9;
    let lip_ry = ry - 5;

    // Background mask, bottom lip (cylinder side), drawn first so the main
    // face overlaps its top half and only the bottom sliver peeks out.
    canvas.fill_ellipse(cx, cy + lip, rx, lip_ry, Some(true), bg);
    canvas.stroke_ellipse(cx, cy + lip, rx, lip_ry, 2, Some(true), border);

    // Background mask, main face.
    canvas.fill_ellipse(cx, cy, rx, ry, None, bg);
    canvas.stroke_ellipse(cx, cy, rx, ry, 2, None, border);

    // Icon mask: the top piece represents a stack visually.
    let icon_piece = piece.top.unwrap_or(piece.bottom);
    canvas.draw_icon(cx, cy - 3, icon_for(icon_piece), 2, fg);

    // Text mask: shortened notation, e.g. "K" or "B/S" when stacked.
    let mut label = String::new();
    if let Some(top) = piece.top {
        label.push(letter_for(top));
        label.push('/');
    }
    label.push(letter_for(piece.bottom));
    let tw = Canvas::text_width(&label, 1);
    let ty = cy + ry + lip - 9;
    canvas.fill_rect(
        cx - tw / 2 - 2,
        ty - 1,
        cx + tw / 2 + 2,
        ty + font::FONT_H as i32,
        bg,
    );
    canvas.stroke_rect(
        cx - tw / 2 - 2,
        ty - 1,
        cx + tw / 2 + 2,
        ty + font::FONT_H as i32,
        1,
        border,
    );
    canvas.draw_text_centered(cx, ty, &label, 1, fg);
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
pub fn blit_to_window(logical: &[u32], out: &mut [u32], out_w: i32, out_h: i32) {
    let scale = (out_w as f32 / LOGICAL_W as f32)
        .min(out_h as f32 / LOGICAL_H as f32)
        .max(0.05);
    let draw_w = (LOGICAL_W as f32 * scale) as i32;
    let draw_h = (LOGICAL_H as f32 * scale) as i32;
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
            let sx = sx.clamp(0, LOGICAL_W - 1);
            let sy = sy.clamp(0, LOGICAL_H - 1);
            out[idx] = logical[(sy * LOGICAL_W + sx) as usize];
        }
    }
}

/// Map a real window mouse position back into logical canvas coordinates,
/// inverse of `blit_to_window`. Returns None inside the letterbox margin.
pub fn window_to_logical(mx: i32, my: i32, out_w: i32, out_h: i32) -> Option<(i32, i32)> {
    let scale = (out_w as f32 / LOGICAL_W as f32)
        .min(out_h as f32 / LOGICAL_H as f32)
        .max(0.05);
    let draw_w = (LOGICAL_W as f32 * scale) as i32;
    let draw_h = (LOGICAL_H as f32 * scale) as i32;
    let off_x = (out_w - draw_w) / 2;
    let off_y = (out_h - draw_h) / 2;
    if mx < off_x || my < off_y || mx >= off_x + draw_w || my >= off_y + draw_h {
        return None;
    }
    let lx = (((mx - off_x) as f32) / scale) as i32;
    let ly = (((my - off_y) as f32) / scale) as i32;
    Some((lx.clamp(0, LOGICAL_W - 1), ly.clamp(0, LOGICAL_H - 1)))
}

/// Draw a labeled button outline; dims when `enabled` is false.
pub fn draw_button(c: &mut Canvas, x0: i32, y0: i32, x1: i32, y1: i32, label: &str, enabled: bool) {
    let color = if enabled {
        COL_STATUS
    } else {
        COL_BTN_DISABLED
    };
    c.stroke_rect(x0, y0, x1, y1, 2, color);
    c.draw_text_centered((x0 + x1) / 2, (y0 + y1) / 2 - 5, label, 1, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blit_preserves_aspect_and_letterboxes_wide_windows() {
        let logical = vec![0x00ffffffu32; (LOGICAL_W * LOGICAL_H) as usize];
        let out_w = LOGICAL_W * 2;
        let out_h = LOGICAL_H;
        let mut out = vec![0u32; (out_w * out_h) as usize];
        blit_to_window(&logical, &mut out, out_w, out_h);
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
        let out_w = LOGICAL_W * 3 / 2;
        let out_h = LOGICAL_H * 3 / 2;
        let (lx, ly) =
            window_to_logical(out_w / 2, out_h / 2, out_w, out_h).expect("center is drawn");
        assert!((0..LOGICAL_W).contains(&lx));
        assert!((0..LOGICAL_H).contains(&ly));

        // A point in the letterbox margin of a mismatched-aspect window maps to nothing.
        let wide_w = LOGICAL_W * 2;
        let wide_h = LOGICAL_H;
        assert!(window_to_logical(1, 1, wide_w, wide_h).is_none());
    }
}
