use crate::{Color, Game, Piece, PieceType, Position, BOARD_DIMENSION};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color as RatatuiColor, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame, Terminal,
};
use std::io;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GameState {
    SelectingPiece,
    SelectingTarget { from: Position },
}

pub struct App {
    game: Game,
    cursor_position: Position,
    game_state: GameState,
    highlighted_moves: Vec<Position>,
}

impl App {
    pub fn new() -> Self {
        App {
            game: Game::new(),
            cursor_position: Position::new(0, 0),
            game_state: GameState::SelectingPiece,
            highlighted_moves: Vec::new(),
        }
    }

    pub fn from_game(game: Game) -> Self {
        App {
            game,
            cursor_position: Position::new(0, 0),
            game_state: GameState::SelectingPiece,
            highlighted_moves: Vec::new(),
        }
    }

    pub fn move_cursor(&mut self, dx: isize, dy: isize) {
        if let Some(new_pos) = self.cursor_position.get_new(dx, dy) {
            self.cursor_position = new_pos;
            self.update_highlights();
        }
    }

    pub fn handle_enter(&mut self) -> Result<(), String> {
        match self.game_state {
            GameState::SelectingPiece => {
                let moves = self.game.get_moves(&self.cursor_position);
                if !moves.is_empty() {
                    self.game_state = GameState::SelectingTarget {
                        from: self.cursor_position,
                    };
                    self.highlighted_moves = moves.iter().map(|m| m.to).collect();
                }
            }
            GameState::SelectingTarget { from } => {
                // Check if cursor is on a valid target position
                let moves = self.game.get_moves(&from);
                if let Some(potential_move) = moves
                    .iter()
                    .find(|m| m.to == self.cursor_position)
                {
                    // For now, always choose not to unstack unless forced
                    let unstack = potential_move.force_unstack;
                    let game_move = potential_move.to_move(unstack);
                    self.game.apply_move(game_move)?;
                    
                    self.game_state = GameState::SelectingPiece;
                    self.highlighted_moves.clear();
                } else {
                    // Invalid move, go back to selecting piece
                    self.game_state = GameState::SelectingPiece;
                    self.highlighted_moves.clear();
                }
            }
        }
        Ok(())
    }

    pub fn handle_escape(&mut self) {
        match self.game_state {
            GameState::SelectingPiece => {
                // Nothing to cancel
            }
            GameState::SelectingTarget { .. } => {
                self.game_state = GameState::SelectingPiece;
                self.highlighted_moves.clear();
            }
        }
    }

    fn update_highlights(&mut self) {
        match self.game_state {
            GameState::SelectingPiece => {
                let moves = self.game.get_moves(&self.cursor_position);
                self.highlighted_moves = moves.iter().map(|m| m.to).collect();
            }
            GameState::SelectingTarget { .. } => {
                // Keep existing highlights
            }
        }
    }

    fn get_piece_display(&self, piece: &Piece) -> String {
        let mut output = String::new();
        
        if let Some(ref top_piece) = piece.top {
            output.push_str(&self.piece_to_char(top_piece));
            output.push('+');
        }
        
        output.push_str(&self.piece_to_char(&piece.bottom));
        output
    }

    fn piece_to_char(&self, piece_type: &PieceType) -> String {
        match piece_type {
            PieceType::Soldier => "S".to_string(),
            PieceType::Jester => "J".to_string(),
            PieceType::Commander => "C".to_string(),
            PieceType::Paladin => "P".to_string(),
            PieceType::Guard => "G".to_string(),
            PieceType::Dragon => "D".to_string(),
            PieceType::Ballista => "B".to_string(),
            PieceType::King => "K".to_string(),
        }
    }
}

pub fn run_tui(game: Option<Game>) -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = if let Some(game) = game {
        App::from_game(game)
    } else {
        App::new()
    };

    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Esc => app.handle_escape(),
                    KeyCode::Enter => {
                        if let Err(_e) = app.handle_enter() {
                            // For now, just ignore move errors
                            // In a full implementation, you might want to show an error message
                        }
                    }
                    KeyCode::Up => app.move_cursor(0, -1),
                    KeyCode::Down => app.move_cursor(0, 1),
                    KeyCode::Left => app.move_cursor(-1, 0),
                    KeyCode::Right => app.move_cursor(1, 0),
                    _ => {}
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),     // Title
            Constraint::Min(20),       // Board
            Constraint::Length(5),     // Instructions
        ])
        .split(f.area());

    // Title
    let title = match app.game_state {
        GameState::SelectingPiece => {
            format!("{} to move - Select a piece", 
                if app.game.is_white_to_move() { "White" } else { "Black" })
        }
        GameState::SelectingTarget { .. } => {
            "Select target position".to_string()
        }
    };
    
    let title_paragraph = Paragraph::new(title)
        .block(Block::default().borders(Borders::ALL).title("Arx Game"))
        .alignment(Alignment::Center);
    f.render_widget(title_paragraph, chunks[0]);

    // Board
    render_board(f, app, chunks[1]);

    // Instructions
    let instructions = vec![
        Line::from(vec![
            Span::raw("Use "),
            Span::styled("Arrow Keys", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" to move cursor, "),
            Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" to select"),
        ]),
        Line::from(vec![
            Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" to cancel selection, "),
            Span::styled("Q", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" to quit"),
        ]),
    ];
    
    let instructions_paragraph = Paragraph::new(instructions)
        .block(Block::default().borders(Borders::ALL).title("Controls"))
        .alignment(Alignment::Center);
    f.render_widget(instructions_paragraph, chunks[2]);
}

fn render_board(f: &mut Frame, app: &App, area: Rect) {
    let board = app.game.board();
    
    // Create column headers (A-I)
    let mut header_cells = vec![Cell::from("")]; // Empty cell for row labels
    for col in 0..BOARD_DIMENSION {
        let letter = ((b'A' + col as u8) as char).to_string();
        header_cells.push(Cell::from(letter).style(Style::default().add_modifier(Modifier::BOLD)));
    }
    
    let mut rows = vec![Row::new(header_cells)];
    
    // Create board rows
    for y in 0..BOARD_DIMENSION {
        let row_label = (9 - y).to_string();
        let mut cells = vec![Cell::from(row_label).style(Style::default().add_modifier(Modifier::BOLD))];
        
        for x in 0..BOARD_DIMENSION {
            let position = Position::new(x, y);
            let mut cell_content = "   ".to_string();
            let mut cell_style = Style::default();
            
            // Check if this position has a piece
            if let Some(piece) = board.get_piece(&position) {
                cell_content = format!(" {} ", app.get_piece_display(piece));
                
                // Color the piece based on its color
                cell_style = match piece.color {
                    Color::White => Style::default().fg(RatatuiColor::White),
                    Color::Black => Style::default().fg(RatatuiColor::Red),
                };
            }
            
            // Highlight cursor position
            if position == app.cursor_position {
                cell_style = cell_style.bg(RatatuiColor::Blue);
            }
            // Highlight possible moves
            else if app.highlighted_moves.contains(&position) {
                cell_style = cell_style.bg(RatatuiColor::Green);
            }
            
            cells.push(Cell::from(cell_content).style(cell_style));
        }
        
        rows.push(Row::new(cells));
    }
    
    // Calculate column widths - make each board cell 4 characters wide
    let mut column_widths = vec![Constraint::Length(2)]; // Row label column
    for _ in 0..BOARD_DIMENSION {
        column_widths.push(Constraint::Length(4));
    }
    
    let table = Table::new(rows, column_widths)
        .block(Block::default().borders(Borders::ALL).title("Board"))
        .column_spacing(0);
    
    f.render_widget(table, area);
}
