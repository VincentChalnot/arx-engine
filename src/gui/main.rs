mod app;
mod base;
mod font;
mod icons;
mod render;
mod save;
mod symbols;

use app::{move_choice_label, App, Mode, Screen};
use keres_engine::{Move, Piece, Position};
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use render::{Canvas, BOARD_PX_H, BOARD_PX_W, TILE_H, TILE_W};

struct Rect {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

impl Rect {
    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x0 && x < self.x1 && y >= self.y0 && y < self.y1
    }
    fn cx(&self) -> i32 {
        (self.x0 + self.x1) / 2
    }
    fn cy(&self) -> i32 {
        (self.y0 + self.y1) / 2
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SidebarAction {
    MainMenu,
    SwitchSides,
    ToggleThreats,
    ToggleCoords,
    Undo,
    Resign,
}

fn menu_mode_buttons(show_coords: bool) -> [(Rect, &'static str, Mode); 3] {
    let bw = 460;
    let bh = 52;
    let cx = render::logical_w(show_coords) / 2;
    let gap = 16;
    let top = 260;
    [
        (
            Rect {
                x0: cx - bw / 2,
                y0: top,
                x1: cx + bw / 2,
                y1: top + bh,
            },
            "1: HOTSEAT (SAME KEYBOARD)",
            Mode::Hotseat,
        ),
        (
            Rect {
                x0: cx - bw / 2,
                y0: top + bh + gap,
                x1: cx + bw / 2,
                y1: top + 2 * bh + gap,
            },
            "2: VS AI - PLAY WHITE",
            Mode::VsAiWhite,
        ),
        (
            Rect {
                x0: cx - bw / 2,
                y0: top + 2 * (bh + gap),
                x1: cx + bw / 2,
                y1: top + 3 * bh + 2 * gap,
            },
            "3: VS AI - PLAY BLACK",
            Mode::VsAiBlack,
        ),
    ]
}

/// Ten clickable boxes acting as a discrete 1-10 "slider" for AI strength
/// (`App::level`), laid out between the subtitle text and the mode buttons.
fn menu_level_boxes(show_coords: bool) -> [Rect; 10] {
    let bw = 40;
    let bh = 32;
    let gap = 6;
    let cx = render::logical_w(show_coords) / 2;
    let total_w = 10 * bw + 9 * gap;
    let x0 = cx - total_w / 2;
    let top = 214;
    std::array::from_fn(|i| {
        let x = x0 + i as i32 * (bw + gap);
        Rect {
            x0: x,
            y0: top,
            x1: x + bw,
            y1: top + bh,
        }
    })
}

fn menu_resume_button(show_coords: bool) -> Rect {
    let bw = 460;
    let bh = 52;
    let cx = render::logical_w(show_coords) / 2;
    let top = 260 + 3 * (bh + 16) + 20;
    Rect {
        x0: cx - bw / 2,
        y0: top,
        x1: cx + bw / 2,
        y1: top + bh,
    }
}

fn choice_buttons(n: usize, show_coords: bool) -> Vec<Rect> {
    let bw = 340;
    let bh = 44;
    let gap = 10;
    let cx = render::board_w(show_coords) / 2;
    let total_h = n as i32 * bh + (n as i32 - 1) * gap;
    let top = (render::logical_h(show_coords) - total_h) / 2;
    (0..n)
        .map(|i| {
            let y0 = top + i as i32 * (bh + gap);
            Rect {
                x0: cx - bw / 2,
                y0,
                x1: cx + bw / 2,
                y1: y0 + bh,
            }
        })
        .collect()
}

const SIDEBAR_PAD: i32 = 14;
const BTN_H: i32 = 28;
const BTN_GAP: i32 = 6;
const SIDEBAR_BUTTONS_TOP: i32 = 78;

fn sidebar_button_rect(index: i32, show_coords: bool) -> Rect {
    let x0 = render::board_w(show_coords) + SIDEBAR_PAD;
    let x1 = render::logical_w(show_coords) - SIDEBAR_PAD;
    let y0 = SIDEBAR_BUTTONS_TOP + index * (BTN_H + BTN_GAP);
    Rect {
        x0,
        y0,
        x1,
        y1: y0 + BTN_H,
    }
}

fn sidebar_buttons(app: &App) -> [(Rect, String, SidebarAction, bool); 6] {
    [
        (
            sidebar_button_rect(0, app.show_coords),
            "MAIN MENU".to_string(),
            SidebarAction::MainMenu,
            true,
        ),
        (
            sidebar_button_rect(1, app.show_coords),
            "SWITCH SIDES".to_string(),
            SidebarAction::SwitchSides,
            true,
        ),
        (
            sidebar_button_rect(2, app.show_coords),
            if app.show_threats {
                "HIDE THREATS".to_string()
            } else {
                "SHOW THREATS".to_string()
            },
            SidebarAction::ToggleThreats,
            true,
        ),
        (
            sidebar_button_rect(3, app.show_coords),
            if app.show_coords {
                "HIDE COORDS".to_string()
            } else {
                "SHOW COORDS".to_string()
            },
            SidebarAction::ToggleCoords,
            true,
        ),
        (
            sidebar_button_rect(4, app.show_coords),
            "UNDO".to_string(),
            SidebarAction::Undo,
            app.can_undo(),
        ),
        (
            sidebar_button_rect(5, app.show_coords),
            "RESIGN".to_string(),
            SidebarAction::Resign,
            true,
        ),
    ]
}

const HISTORY_TOP: i32 = 300;
const HISTORY_ROW_H: i32 = 13;

fn history_visible_rows(show_coords: bool) -> i32 {
    (render::logical_h(show_coords) - HISTORY_TOP - SIDEBAR_PAD) / HISTORY_ROW_H
}

/// Screen-space (col, row) for a board position, honoring the board flip.
fn screen_coord(pos: Position, flipped: bool) -> (i32, i32) {
    if flipped {
        (8 - pos.x as i32, 8 - pos.y as i32)
    } else {
        (pos.x as i32, pos.y as i32)
    }
}

/// Inverse of `screen_coord`: board position from a screen column/row.
fn board_pos(col: i32, row: i32, flipped: bool) -> Position {
    if flipped {
        Position::new((8 - col) as usize, (8 - row) as usize)
    } else {
        Position::new(col as usize, row as usize)
    }
}

fn tile_at(lx: i32, ly: i32, flipped: bool, show_coords: bool) -> Option<Position> {
    let bx = lx - render::gutter(show_coords);
    let by = ly - render::topbar(show_coords);
    if bx < 0 || by < 0 || bx >= BOARD_PX_W || by >= BOARD_PX_H {
        return None;
    }
    let col = bx / TILE_W;
    let row = by / TILE_H;
    Some(board_pos(col, row, flipped))
}

/// The opponent's pieces are drawn upside down, like a physical two-player
/// board where each side's own tokens face them. "Opponent" here means
/// whichever color is rendered at the top of the screen — which flips with
/// `app.flipped`, not with whose turn it is.
fn is_upside_down(color: keres_engine::Color, flipped: bool) -> bool {
    let near_side = if flipped {
        keres_engine::Color::Black
    } else {
        keres_engine::Color::White
    };
    color != near_side
}

fn move_notation(mv: &Move, is_capture: bool) -> String {
    format!(
        "{}-{}{}{}",
        mv.from.to_string(),
        mv.to.to_string(),
        if mv.unstack { "-" } else { "" },
        if is_capture { "*" } else { "" },
    )
}

fn draw_menu(c: &mut Canvas, show_coords: bool, level: u8) {
    let lw = render::logical_w(show_coords);
    let lh = render::logical_h(show_coords);
    c.fill_rect(0, 0, lw, lh, render::COL_PAGE_BG);
    c.draw_text_centered(lw / 2, 60, "KERES", 6, render::COL_STATUS);
    c.draw_text_centered(lw / 2, 140, "9X9 STACKING CHESS", 2, render::COL_COORD);
    c.draw_text_centered(
        lw / 2,
        178,
        "CLICK A PIECE, THEN A HIGHLIGHTED TILE",
        1,
        render::COL_COORD,
    );
    c.draw_text_centered(lw / 2, 202, "AI DIFFICULTY", 1, render::COL_COORD);
    for (i, rect) in menu_level_boxes(show_coords).into_iter().enumerate() {
        let selected = level as usize == i + 1;
        let color = if selected {
            render::COL_SELECT
        } else {
            render::COL_STATUS
        };
        if selected {
            c.fill_rect_alpha(rect.x0, rect.y0, rect.x1, rect.y1, render::COL_SELECT, 0.35);
        }
        c.stroke_rect(rect.x0, rect.y0, rect.x1, rect.y1, 2, color);
        c.draw_text_centered(rect.cx(), rect.cy() - 5, &(i + 1).to_string(), 1, color);
    }
    for (rect, label, _) in menu_mode_buttons(show_coords) {
        c.stroke_rect(rect.x0, rect.y0, rect.x1, rect.y1, 2, render::COL_STATUS);
        c.draw_text_centered(rect.cx(), rect.cy() - 5, label, 1, render::COL_STATUS);
    }
    if save::exists() {
        let rect = menu_resume_button(show_coords);
        c.stroke_rect(rect.x0, rect.y0, rect.x1, rect.y1, 2, render::COL_SELECT);
        c.draw_text_centered(
            rect.cx(),
            rect.cy() - 5,
            "4: RESUME SAVED GAME",
            1,
            render::COL_SELECT,
        );
    }
    c.draw_text_centered(lw / 2, lh - 30, "ESC TO QUIT", 1, render::COL_COORD);
}

fn draw_sidebar(c: &mut Canvas, app: &App, history_scroll: i32) {
    let x0 = render::board_w(app.show_coords);
    let lw = render::logical_w(app.show_coords);
    let lh = render::logical_h(app.show_coords);
    c.fill_rect(x0, 0, lw, lh, render::COL_SIDEBAR_BG);
    c.draw_text(x0 + SIDEBAR_PAD, 14, "KERES", 2, render::COL_STATUS);

    let status = if app.ai_thinking {
        "AI IS THINKING...".to_string()
    } else if app.pending.is_some() {
        "CHOOSE A MOVE".to_string()
    } else {
        match app.game.color_to_move() {
            keres_engine::Color::White => "WHITE TO MOVE".to_string(),
            keres_engine::Color::Black => "BLACK TO MOVE".to_string(),
        }
    };
    c.draw_text(x0 + SIDEBAR_PAD, 44, &status, 1, render::COL_STATUS);

    for (rect, label, _, enabled) in sidebar_buttons(app) {
        render::draw_button(c, rect.x0, rect.y0, rect.x1, rect.y1, &label, enabled);
    }

    c.draw_text(
        x0 + SIDEBAR_PAD,
        HISTORY_TOP - 16,
        "MOVE HISTORY",
        1,
        render::COL_COORD,
    );
    c.stroke_rect(
        x0 + SIDEBAR_PAD,
        HISTORY_TOP - 4,
        lw - SIDEBAR_PAD,
        HISTORY_TOP - 3,
        1,
        render::COL_COORD,
    );

    let pairs: Vec<_> = app
        .history
        .chunks(2)
        .enumerate()
        .map(|(i, chunk)| (i + 1, chunk.first(), chunk.get(1)))
        .collect();
    let visible = history_visible_rows(app.show_coords);
    let max_scroll = (pairs.len() as i32 - visible).max(0);
    let scroll = history_scroll.clamp(0, max_scroll);
    for (row, (n, white, black)) in pairs
        .iter()
        .skip(scroll as usize)
        .take(visible as usize)
        .enumerate()
    {
        let y = HISTORY_TOP + row as i32 * HISTORY_ROW_H;
        let mut line = format!("{}.", n);
        if let Some((mv, cap)) = white {
            line.push(' ');
            line.push_str(&move_notation(mv, *cap));
        }
        if let Some((mv, cap)) = black {
            line.push(' ');
            line.push_str(&move_notation(mv, *cap));
        }
        c.draw_text(x0 + SIDEBAR_PAD, y, &line, 1, render::COL_COORD);
    }
}

fn draw_board(c: &mut Canvas, app: &App, history_scroll: i32) {
    let show_coords = app.show_coords;
    let gutter = render::gutter(show_coords);
    let topbar = render::topbar(show_coords);
    let lw = render::logical_w(show_coords);
    let lh = render::logical_h(show_coords);

    c.fill_rect(0, 0, lw, lh, render::COL_PAGE_BG);
    draw_sidebar(c, app, history_scroll);

    for y in 0..9i32 {
        for x in 0..9i32 {
            let light = (x + y) % 2 == 0;
            let color = if light {
                render::COL_LIGHT_SQ
            } else {
                render::COL_DARK_SQ
            };
            let px0 = gutter + x * TILE_W;
            let py0 = topbar + y * TILE_H;
            c.fill_rect(px0, py0, px0 + TILE_W, py0 + TILE_H, color);
        }
    }

    // Tile-highlight overlays, palette matched to the Web platform (see
    // render::COL_HL_*): a full-tile alpha wash rather than a dot/ring.
    // Layered weakest to strongest so a stronger highlight always wins where
    // they'd otherwise overlap.
    for pos in app.hover_threat_squares() {
        let (cx, cy) = screen_coord(pos, app.flipped);
        let px0 = gutter + cx * TILE_W;
        let py0 = topbar + cy * TILE_H;
        c.fill_rect_alpha(
            px0,
            py0,
            px0 + TILE_W,
            py0 + TILE_H,
            render::COL_HL_THREAT,
            render::COL_HL_THREAT_A,
        );
    }

    for pos in app.hover_preview_squares() {
        let (cx, cy) = screen_coord(pos, app.flipped);
        let px0 = gutter + cx * TILE_W;
        let py0 = topbar + cy * TILE_H;
        c.fill_rect_alpha(
            px0,
            py0,
            px0 + TILE_W,
            py0 + TILE_H,
            render::COL_HL_HOVER,
            render::COL_HL_HOVER_A,
        );
    }

    if let Some(sel) = app.selected {
        let (cx, cy) = screen_coord(sel, app.flipped);
        let px0 = gutter + cx * TILE_W;
        let py0 = topbar + cy * TILE_H;
        c.fill_rect_alpha(
            px0,
            py0,
            px0 + TILE_W,
            py0 + TILE_H,
            render::COL_HL_SELECTED,
            render::COL_HL_SELECTED_A,
        );
    }

    for (pos, _is_capture) in app.target_squares() {
        let (cx, cy) = screen_coord(pos, app.flipped);
        let px0 = gutter + cx * TILE_W;
        let py0 = topbar + cy * TILE_H;
        c.fill_rect_alpha(
            px0,
            py0,
            px0 + TILE_W,
            py0 + TILE_H,
            render::COL_HL_POTENTIAL,
            render::COL_HL_POTENTIAL_A,
        );
    }

    // Pieces are drawn in screen-row order (top to bottom), not board-row
    // order — the two differ once the board is flipped (see `board_pos`).
    // This is what a stack's overflow-above-the-tile art (see
    // `render::draw_piece`) relies on: a lower screen row is always drawn
    // after the row above it, so its stack correctly paints over that row's
    // content instead of being painted over by it.
    for sy in 0..9i32 {
        for sx in 0..9i32 {
            let board_p = board_pos(sx, sy, app.flipped);
            if let Some(piece) = app.game.board.get_piece(&board_p) {
                let tx = gutter + sx * TILE_W;
                let ty = topbar + sy * TILE_H;
                let upside_down = is_upside_down(piece.color, app.flipped);
                render::draw_piece(c, tx, ty, piece as &Piece, upside_down);
            }
        }
    }

    // Big "this creates a stack" arrow: when a piece is selected and the
    // hovered square is a legal stacking target (a friendly piece it could
    // land on), it's ambiguous whether clicking merges the stack or just
    // reselects — the arrow makes the stacking outcome unmistakable. Drawn
    // last so it sits on top of the square and piece underneath it.
    if app.hovered_stack_target() {
        if let Some(pos) = app.hovered {
            let (cx, cy) = screen_coord(pos, app.flipped);
            let tx = gutter + cx * TILE_W;
            let ty = topbar + cy * TILE_H;
            c.draw_bitmap(
                tx + 22,
                ty - 7,
                &symbols::symbol_bits(symbols::Symbol::DownArrow),
                symbols::SYMBOL_W as i32,
                render::COL_HL_THREAT,
            );
        }
    }

    if app.show_coords {
        for y in 0..9i32 {
            let (_, sy) = screen_coord(Position::new(0, y as usize), app.flipped);
            let rank = 9 - y;
            c.draw_text(
                6,
                topbar + sy * TILE_H + TILE_H / 2 - 5,
                &rank.to_string(),
                1,
                render::COL_COORD,
            );
        }
        for x in 0..9i32 {
            let (sx, _) = screen_coord(Position::new(x as usize, 0), app.flipped);
            let file = ((b'A' + x as u8) as char).to_string();
            c.draw_text_centered(
                gutter + sx * TILE_W + TILE_W / 2,
                topbar + BOARD_PX_H + 8,
                &file,
                1,
                render::COL_COORD,
            );
        }
    }

    if let Some(pending) = &app.pending {
        c.fill_rect_alpha(0, 0, render::board_w(show_coords), lh, 0x000000, 0.72);
        let rects = choice_buttons(pending.options.len(), show_coords);
        for (i, mv) in pending.options.iter().enumerate() {
            let rect = &rects[i];
            c.stroke_rect(rect.x0, rect.y0, rect.x1, rect.y1, 2, render::COL_STATUS);
            let label = move_choice_label(&app.game, mv);
            c.draw_text_centered(rect.cx(), rect.cy() - 5, &label, 1, render::COL_STATUS);
        }
    }
}

fn draw_game_over(c: &mut Canvas, app: &App, history_scroll: i32) {
    draw_board(c, app, history_scroll);
    let board_w = render::board_w(app.show_coords);
    let lh = render::logical_h(app.show_coords);
    c.fill_rect_alpha(0, 0, board_w, lh, 0x000000, 0.72);
    let msg = if app.game.is_draw() {
        "DRAW"
    } else if app.game.white_wins() {
        "WHITE WINS"
    } else {
        "BLACK WINS"
    };
    c.draw_text_centered(board_w / 2, lh / 2 - 40, msg, 4, render::COL_STATUS);
    c.draw_text_centered(
        board_w / 2,
        lh / 2 + 30,
        "CLICK TO RETURN TO MENU",
        1,
        render::COL_COORD,
    );
}

fn write_ppm(path: &str, buf: &[u32], w: i32, h: i32) {
    use std::io::Write;
    let mut out = std::fs::File::create(path).expect("create snapshot file");
    write!(out, "P6\n{} {}\n255\n", w, h).unwrap();
    let mut bytes = Vec::with_capacity((w * h * 3) as usize);
    for &px in buf {
        bytes.push(((px >> 16) & 0xff) as u8);
        bytes.push(((px >> 8) & 0xff) as u8);
        bytes.push((px & 0xff) as u8);
    }
    out.write_all(&bytes).unwrap();
}

/// Headless render-to-file path used only for local visual verification
/// (`KERES_SNAPSHOT=<path>.ppm KERES_SCREEN=menu|playing|selected|stacked|
/// gameover|flipped|threats|history|nocoords|hover_friendly|hover_threat|
/// hover_stack`). Never touches a window or the real display.
fn run_snapshot(path: &str) {
    use keres_engine::{Board, Color, PieceType};
    let mut app = App::new();
    let screen = std::env::var("KERES_SCREEN").unwrap_or_else(|_| "menu".to_string());
    match screen.as_str() {
        "menu" => {}
        "playing" => app.start_game(Mode::Hotseat),
        "selected" => {
            app.start_game(Mode::Hotseat);
            app.click_square(Position::new(3, 6));
        }
        "stacked" => {
            app.start_game(Mode::Hotseat);
            app.game.board.set_piece(
                &Position::new(3, 6),
                Some(Piece::new(
                    Color::White,
                    PieceType::Soldier,
                    Some(PieceType::Bishop),
                )),
            );
            app.click_square(Position::new(3, 6));
        }
        "gameover" => {
            app.start_game(Mode::Hotseat);
            app.screen = Screen::GameOver;
        }
        "flipped" => {
            app.start_game(Mode::Hotseat);
            app.toggle_flip();
        }
        "threats" => {
            app.start_game(Mode::Hotseat);
            let mut board = Board::empty();
            board.set_piece(
                &Position::new(4, 8),
                Some(Piece::new(Color::White, PieceType::King, None)),
            );
            board.set_piece(
                &Position::new(4, 0),
                Some(Piece::new(Color::Black, PieceType::King, None)),
            );
            board.set_piece(
                &Position::new(4, 4),
                Some(Piece::new(Color::White, PieceType::Soldier, None)),
            );
            board.set_piece(
                &Position::new(3, 3),
                Some(Piece::new(Color::Black, PieceType::Bishop, None)),
            );
            app.game.board = board;
        }
        "history" => {
            app.start_game(Mode::Hotseat);
            let moves = [
                Move {
                    from: Position::new(2, 6),
                    to: Position::new(1, 5),
                    unstack: false,
                },
                Move {
                    from: Position::new(6, 2),
                    to: Position::new(7, 3),
                    unstack: false,
                },
                Move {
                    from: Position::new(4, 6),
                    to: Position::new(3, 5),
                    unstack: false,
                },
                Move {
                    from: Position::new(4, 2),
                    to: Position::new(3, 3),
                    unstack: false,
                },
            ];
            for mv in moves {
                app.game.make(&mv);
                let cap = false;
                app.history.push((mv, cap));
            }
        }
        "nocoords" => {
            app.start_game(Mode::Hotseat);
            app.toggle_coords();
        }
        "hover_friendly" => {
            app.start_game(Mode::Hotseat);
            app.set_hovered(Some(Position::new(3, 6)));
        }
        "hover_threat" => {
            app.start_game(Mode::Hotseat);
            let mut board = Board::empty();
            board.set_piece(
                &Position::new(4, 8),
                Some(Piece::new(Color::White, PieceType::King, None)),
            );
            board.set_piece(
                &Position::new(4, 0),
                Some(Piece::new(Color::Black, PieceType::King, None)),
            );
            board.set_piece(
                &Position::new(4, 4),
                Some(Piece::new(Color::White, PieceType::Soldier, None)),
            );
            board.set_piece(
                &Position::new(3, 3),
                Some(Piece::new(Color::Black, PieceType::Bishop, None)),
            );
            app.game.board = board;
            app.set_hovered(Some(Position::new(3, 3)));
        }
        "hover_stack" => {
            app.start_game(Mode::Hotseat);
            let mut board = Board::empty();
            board.set_piece(
                &Position::new(4, 8),
                Some(Piece::new(Color::White, PieceType::King, None)),
            );
            board.set_piece(
                &Position::new(4, 0),
                Some(Piece::new(Color::Black, PieceType::King, None)),
            );
            board.set_piece(
                &Position::new(4, 4),
                Some(Piece::new(Color::White, PieceType::Rook, None)),
            );
            board.set_piece(
                &Position::new(4, 3),
                Some(Piece::new(Color::White, PieceType::Soldier, None)),
            );
            app.game.board = board;
            app.click_square(Position::new(4, 4));
            app.set_hovered(Some(Position::new(4, 3)));
        }
        other => panic!("unknown KERES_SCREEN {other}"),
    }
    let lw = render::logical_w(app.show_coords);
    let lh = render::logical_h(app.show_coords);
    let mut buffer = vec![0u32; (lw * lh) as usize];
    let mut canvas = Canvas {
        buf: &mut buffer,
        w: lw,
        h: lh,
    };
    match app.screen {
        Screen::Menu => draw_menu(&mut canvas, app.show_coords, app.level),
        Screen::Playing => draw_board(&mut canvas, &app, 0),
        Screen::GameOver => draw_game_over(&mut canvas, &app, 0),
    }
    write_ppm(path, &buffer, lw, lh);
}

fn main() {
    if let Ok(path) = std::env::var("KERES_SNAPSHOT") {
        run_snapshot(&path);
        return;
    }

    let mut app = App::new();

    let mut window = Window::new(
        "Keres",
        render::logical_w(app.show_coords) as usize,
        render::logical_h(app.show_coords) as usize,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )
    .expect("unable to open window");
    window.set_target_fps(60);

    let mut logical: Vec<u32> = Vec::new();
    let mut output: Vec<u32> = Vec::new();
    let mut prev_mouse_down = false;
    let mut history_scroll: i32 = 0;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        app.poll_ai();

        let show_coords = app.show_coords;
        let lw = render::logical_w(show_coords);
        let lh = render::logical_h(show_coords);
        if logical.len() != (lw * lh) as usize {
            logical.resize((lw * lh) as usize, 0);
        }

        let (win_w, win_h) = window.get_size();
        let (win_w, win_h) = (win_w as i32, win_h as i32);

        let mouse_down = window.get_mouse_down(MouseButton::Left);
        let clicked = mouse_down && !prev_mouse_down;
        prev_mouse_down = mouse_down;
        let mouse_pos = window.get_mouse_pos(MouseMode::Clamp);
        let logical_mouse = mouse_pos.and_then(|(mx, my)| {
            render::window_to_logical(mx as i32, my as i32, win_w, win_h, lw, lh)
        });

        // Live hover tracking drives the immediate move preview and the
        // "this creates a stack" arrow (see App::hover_preview_squares /
        // hovered_stack_target) — only meaningful while a move can actually
        // be made.
        if app.screen == Screen::Playing && !app.ai_thinking && app.pending.is_none() {
            let hovered =
                logical_mouse.and_then(|(lx, ly)| tile_at(lx, ly, app.flipped, show_coords));
            app.set_hovered(hovered);
        } else {
            app.set_hovered(None);
        }

        if let Some((_, dy)) = window.get_scroll_wheel() {
            if app.screen == Screen::Playing && dy.abs() > 0.01 {
                history_scroll -= dy.signum() as i32;
                history_scroll = history_scroll.max(0);
            }
        }

        if clicked {
            if let Some((lx, ly)) = logical_mouse {
                match app.screen {
                    Screen::Menu => {
                        let mut handled = false;
                        for (i, rect) in menu_level_boxes(show_coords).into_iter().enumerate() {
                            if rect.contains(lx, ly) {
                                app.set_level(i as u8 + 1);
                                handled = true;
                                break;
                            }
                        }
                        for (rect, _label, mode) in menu_mode_buttons(show_coords) {
                            if rect.contains(lx, ly) {
                                app.start_game(mode);
                                history_scroll = 0;
                                handled = true;
                                break;
                            }
                        }
                        if !handled
                            && save::exists()
                            && menu_resume_button(show_coords).contains(lx, ly)
                        {
                            if let Some((mode, moves)) = save::load() {
                                app.resume_game(mode, moves);
                                history_scroll = 0;
                            }
                        }
                    }
                    Screen::Playing => {
                        if let Some(pending) = &app.pending {
                            let rects = choice_buttons(pending.options.len(), show_coords);
                            for (i, rect) in rects.iter().enumerate() {
                                if rect.contains(lx, ly) {
                                    app.resolve_choice(i);
                                    break;
                                }
                            }
                        } else {
                            let mut handled = false;
                            for (rect, _label, action, enabled) in sidebar_buttons(&app) {
                                if enabled && rect.contains(lx, ly) {
                                    match action {
                                        SidebarAction::MainMenu => app.back_to_menu(),
                                        SidebarAction::SwitchSides => app.toggle_flip(),
                                        SidebarAction::ToggleThreats => app.toggle_threats(),
                                        SidebarAction::ToggleCoords => app.toggle_coords(),
                                        SidebarAction::Undo => app.undo(),
                                        SidebarAction::Resign => app.resign(),
                                    }
                                    handled = true;
                                    break;
                                }
                            }
                            if !handled {
                                if let Some(pos) = tile_at(lx, ly, app.flipped, show_coords) {
                                    app.click_square(pos);
                                }
                            }
                        }
                    }
                    Screen::GameOver => app.back_to_menu(),
                }
            }
        }

        if window.is_key_pressed(Key::Escape, KeyRepeat::No) && app.screen != Screen::Menu {
            if app.pending.is_some() {
                app.cancel_choice();
            } else {
                app.back_to_menu();
            }
        }
        if window.is_key_pressed(Key::U, KeyRepeat::No) && app.screen == Screen::Playing {
            app.undo();
        }
        if let Some(pending) = &app.pending {
            let n = pending.options.len();
            for (i, key) in [Key::Key1, Key::Key2, Key::Key3].iter().enumerate() {
                if i < n && window.is_key_pressed(*key, KeyRepeat::No) {
                    app.resolve_choice(i);
                    break;
                }
            }
        } else if app.screen == Screen::Menu {
            for (key, mode) in [
                (Key::Key1, Mode::Hotseat),
                (Key::Key2, Mode::VsAiWhite),
                (Key::Key3, Mode::VsAiBlack),
            ] {
                if window.is_key_pressed(key, KeyRepeat::No) {
                    app.start_game(mode);
                    history_scroll = 0;
                }
            }
            if window.is_key_pressed(Key::Key4, KeyRepeat::No) && save::exists() {
                if let Some((mode, moves)) = save::load() {
                    app.resume_game(mode, moves);
                    history_scroll = 0;
                }
            }
        }

        let mut canvas = Canvas {
            buf: &mut logical,
            w: lw,
            h: lh,
        };
        match app.screen {
            Screen::Menu => draw_menu(&mut canvas, show_coords, app.level),
            Screen::Playing => draw_board(&mut canvas, &app, history_scroll),
            Screen::GameOver => draw_game_over(&mut canvas, &app, history_scroll),
        }

        let out_len = (win_w.max(1) * win_h.max(1)) as usize;
        if output.len() != out_len {
            output.resize(out_len, 0);
        }
        render::blit_to_window(&logical, &mut output, win_w.max(1), win_h.max(1), lw, lh);

        window
            .update_with_buffer(&output, win_w.max(1) as usize, win_h.max(1) as usize)
            .expect("update_with_buffer failed");
    }
}
