mod app;
mod font;
mod icons;
mod render;
mod save;

use app::{move_choice_label, App, Mode, Screen};
use keres_engine::{Move, Piece, Position};
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use render::{Canvas, BOARD_PX, BOARD_W, GUTTER, LOGICAL_H, LOGICAL_W, TILE, TOPBAR};

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

fn menu_mode_buttons() -> [(Rect, &'static str, Mode); 3] {
    let bw = 460;
    let bh = 52;
    let cx = LOGICAL_W / 2;
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

fn menu_resume_button() -> Rect {
    let bw = 460;
    let bh = 52;
    let cx = LOGICAL_W / 2;
    let top = 260 + 3 * (bh + 16) + 20;
    Rect {
        x0: cx - bw / 2,
        y0: top,
        x1: cx + bw / 2,
        y1: top + bh,
    }
}

fn choice_buttons(n: usize) -> Vec<Rect> {
    let bw = 340;
    let bh = 44;
    let gap = 10;
    let cx = BOARD_W / 2;
    let total_h = n as i32 * bh + (n as i32 - 1) * gap;
    let top = (LOGICAL_H - total_h) / 2;
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

fn sidebar_button_rect(index: i32) -> Rect {
    let x0 = BOARD_W + SIDEBAR_PAD;
    let x1 = LOGICAL_W - SIDEBAR_PAD;
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
            sidebar_button_rect(0),
            "MAIN MENU".to_string(),
            SidebarAction::MainMenu,
            true,
        ),
        (
            sidebar_button_rect(1),
            "SWITCH SIDES".to_string(),
            SidebarAction::SwitchSides,
            true,
        ),
        (
            sidebar_button_rect(2),
            if app.show_threats {
                "HIDE THREATS".to_string()
            } else {
                "SHOW THREATS".to_string()
            },
            SidebarAction::ToggleThreats,
            true,
        ),
        (
            sidebar_button_rect(3),
            if app.show_coords {
                "HIDE COORDS".to_string()
            } else {
                "SHOW COORDS".to_string()
            },
            SidebarAction::ToggleCoords,
            true,
        ),
        (
            sidebar_button_rect(4),
            "UNDO".to_string(),
            SidebarAction::Undo,
            app.can_undo(),
        ),
        (
            sidebar_button_rect(5),
            "RESIGN".to_string(),
            SidebarAction::Resign,
            true,
        ),
    ]
}

const HISTORY_TOP: i32 = 300;
const HISTORY_ROW_H: i32 = 13;

fn history_visible_rows() -> i32 {
    (LOGICAL_H - HISTORY_TOP - SIDEBAR_PAD) / HISTORY_ROW_H
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

fn tile_at(lx: i32, ly: i32, flipped: bool) -> Option<Position> {
    let bx = lx - GUTTER;
    let by = ly - TOPBAR;
    if bx < 0 || by < 0 || bx >= BOARD_PX || by >= BOARD_PX {
        return None;
    }
    let col = bx / TILE;
    let row = by / TILE;
    Some(board_pos(col, row, flipped))
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

fn draw_menu(c: &mut Canvas) {
    c.fill_rect(0, 0, LOGICAL_W, LOGICAL_H, render::COL_PAGE_BG);
    c.draw_text_centered(LOGICAL_W / 2, 60, "KERES", 6, render::COL_STATUS);
    c.draw_text_centered(
        LOGICAL_W / 2,
        140,
        "9X9 STACKING CHESS",
        2,
        render::COL_COORD,
    );
    c.draw_text_centered(
        LOGICAL_W / 2,
        178,
        "CLICK A PIECE, THEN A HIGHLIGHTED TILE",
        1,
        render::COL_COORD,
    );
    for (rect, label, _) in menu_mode_buttons() {
        c.stroke_rect(rect.x0, rect.y0, rect.x1, rect.y1, 2, render::COL_STATUS);
        c.draw_text_centered(rect.cx(), rect.cy() - 5, label, 1, render::COL_STATUS);
    }
    if save::exists() {
        let rect = menu_resume_button();
        c.stroke_rect(rect.x0, rect.y0, rect.x1, rect.y1, 2, render::COL_SELECT);
        c.draw_text_centered(
            rect.cx(),
            rect.cy() - 5,
            "4: RESUME SAVED GAME",
            1,
            render::COL_SELECT,
        );
    }
    c.draw_text_centered(
        LOGICAL_W / 2,
        LOGICAL_H - 30,
        "ESC TO QUIT",
        1,
        render::COL_COORD,
    );
}

fn draw_sidebar(c: &mut Canvas, app: &App, history_scroll: i32) {
    let x0 = BOARD_W;
    c.fill_rect(x0, 0, LOGICAL_W, LOGICAL_H, render::COL_SIDEBAR_BG);
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
        LOGICAL_W - SIDEBAR_PAD,
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
    let visible = history_visible_rows();
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
    c.fill_rect(0, 0, LOGICAL_W, LOGICAL_H, render::COL_PAGE_BG);
    draw_sidebar(c, app, history_scroll);

    for y in 0..9i32 {
        for x in 0..9i32 {
            let light = (x + y) % 2 == 0;
            let color = if light {
                render::COL_LIGHT_SQ
            } else {
                render::COL_DARK_SQ
            };
            let px0 = GUTTER + x * TILE;
            let py0 = TOPBAR + y * TILE;
            c.fill_rect(px0, py0, px0 + TILE, py0 + TILE, color);
        }
    }

    if let Some(sel) = app.selected {
        let (cx, cy) = screen_coord(sel, app.flipped);
        let px0 = GUTTER + cx * TILE;
        let py0 = TOPBAR + cy * TILE;
        c.stroke_rect(px0, py0, px0 + TILE, py0 + TILE, 4, render::COL_SELECT);
    }

    for (pos, is_capture) in app.target_squares() {
        let (cx, cy) = screen_coord(pos, app.flipped);
        let px0 = GUTTER + cx * TILE;
        let py0 = TOPBAR + cy * TILE;
        if is_capture {
            c.stroke_rect(
                px0 + 2,
                py0 + 2,
                px0 + TILE - 2,
                py0 + TILE - 2,
                3,
                render::COL_CAPTURE_RING,
            );
        } else {
            c.fill_ellipse(
                px0 + TILE / 2,
                py0 + TILE / 2,
                8,
                8,
                None,
                render::COL_MOVE_DOT,
            );
        }
    }

    if app.show_threats {
        for pos in app.threatened_squares() {
            let (cx, cy) = screen_coord(pos, app.flipped);
            let px0 = GUTTER + cx * TILE;
            let py0 = TOPBAR + cy * TILE;
            c.fill_ellipse(px0 + TILE - 10, py0 + 10, 6, 6, None, render::COL_THREAT);
        }
    }

    for y in 0..9usize {
        for x in 0..9usize {
            if let Some(piece) = app.game.board.get_piece(&Position::new(x, y)) {
                let (cx, cy) = screen_coord(Position::new(x, y), app.flipped);
                let px = GUTTER + cx * TILE + TILE / 2;
                let py = TOPBAR + cy * TILE + TILE / 2;
                render::draw_piece(c, px, py, piece as &Piece);
            }
        }
    }

    if app.show_coords {
        for y in 0..9i32 {
            let (_, sy) = screen_coord(Position::new(0, y as usize), app.flipped);
            let rank = 9 - y;
            c.draw_text(
                6,
                TOPBAR + sy * TILE + TILE / 2 - 5,
                &rank.to_string(),
                1,
                render::COL_COORD,
            );
        }
        for x in 0..9i32 {
            let (sx, _) = screen_coord(Position::new(x as usize, 0), app.flipped);
            let file = ((b'A' + x as u8) as char).to_string();
            c.draw_text_centered(
                GUTTER + sx * TILE + TILE / 2,
                TOPBAR + BOARD_PX + 8,
                &file,
                1,
                render::COL_COORD,
            );
        }
    }

    if let Some(pending) = &app.pending {
        c.fill_rect_alpha(0, 0, BOARD_W, LOGICAL_H, 0x000000, 0.72);
        let rects = choice_buttons(pending.options.len());
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
    c.fill_rect_alpha(0, 0, BOARD_W, LOGICAL_H, 0x000000, 0.72);
    let msg = if app.game.is_draw() {
        "DRAW"
    } else if app.game.white_wins() {
        "WHITE WINS"
    } else {
        "BLACK WINS"
    };
    c.draw_text_centered(BOARD_W / 2, LOGICAL_H / 2 - 40, msg, 4, render::COL_STATUS);
    c.draw_text_centered(
        BOARD_W / 2,
        LOGICAL_H / 2 + 30,
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
/// gameover|flipped|threats|sidebar_full`). Never touches a window or the
/// real display.
fn run_snapshot(path: &str) {
    use keres_engine::{Board, Color, PieceType};
    let mut buffer = vec![0u32; (LOGICAL_W * LOGICAL_H) as usize];
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
        other => panic!("unknown KERES_SCREEN {other}"),
    }
    let mut canvas = Canvas {
        buf: &mut buffer,
        w: LOGICAL_W,
        h: LOGICAL_H,
    };
    match app.screen {
        Screen::Menu => draw_menu(&mut canvas),
        Screen::Playing => draw_board(&mut canvas, &app, 0),
        Screen::GameOver => draw_game_over(&mut canvas, &app, 0),
    }
    write_ppm(path, &buffer, LOGICAL_W, LOGICAL_H);
}

fn main() {
    if let Ok(path) = std::env::var("KERES_SNAPSHOT") {
        run_snapshot(&path);
        return;
    }

    let mut window = Window::new(
        "Keres",
        LOGICAL_W as usize,
        LOGICAL_H as usize,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )
    .expect("unable to open window");
    window.set_target_fps(60);

    let mut logical = vec![0u32; (LOGICAL_W * LOGICAL_H) as usize];
    let mut output: Vec<u32> = Vec::new();
    let mut app = App::new();
    let mut prev_mouse_down = false;
    let mut history_scroll: i32 = 0;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        app.poll_ai();

        let (win_w, win_h) = window.get_size();
        let (win_w, win_h) = (win_w as i32, win_h as i32);

        let mouse_down = window.get_mouse_down(MouseButton::Left);
        let clicked = mouse_down && !prev_mouse_down;
        prev_mouse_down = mouse_down;
        let mouse_pos = window.get_mouse_pos(MouseMode::Clamp);
        let logical_mouse = mouse_pos
            .and_then(|(mx, my)| render::window_to_logical(mx as i32, my as i32, win_w, win_h));

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
                        for (rect, _label, mode) in menu_mode_buttons() {
                            if rect.contains(lx, ly) {
                                app.start_game(mode);
                                history_scroll = 0;
                                handled = true;
                                break;
                            }
                        }
                        if !handled && save::exists() && menu_resume_button().contains(lx, ly) {
                            if let Some((mode, moves)) = save::load() {
                                app.resume_game(mode, moves);
                                history_scroll = 0;
                            }
                        }
                    }
                    Screen::Playing => {
                        if let Some(pending) = &app.pending {
                            let rects = choice_buttons(pending.options.len());
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
                                if let Some(pos) = tile_at(lx, ly, app.flipped) {
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
            w: LOGICAL_W,
            h: LOGICAL_H,
        };
        match app.screen {
            Screen::Menu => draw_menu(&mut canvas),
            Screen::Playing => draw_board(&mut canvas, &app, history_scroll),
            Screen::GameOver => draw_game_over(&mut canvas, &app, history_scroll),
        }

        let out_len = (win_w.max(1) * win_h.max(1)) as usize;
        if output.len() != out_len {
            output.resize(out_len, 0);
        }
        render::blit_to_window(&logical, &mut output, win_w.max(1), win_h.max(1));

        window
            .update_with_buffer(&output, win_w.max(1) as usize, win_h.max(1) as usize)
            .expect("update_with_buffer failed");
    }
}
