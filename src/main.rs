// FIXME qualify term imports
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::Rng;
use std::{collections::HashMap, io};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

fn main() -> Result<(), io::Error> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut game = Game::new();
    loop {
        terminal.draw(|f| ui(f, &game))?;

        // when q is pressed, finish the program
        if let Event::Key(key) = event::read()? {
            match key.code {
                // Quit game when pressing q
                KeyCode::Char('q') => break,

                // handle both arrows and vi keybindings for now
                KeyCode::Char('k') | KeyCode::Up => game.move_up(),
                KeyCode::Char('j') | KeyCode::Down => game.move_down(),
                KeyCode::Char('h') | KeyCode::Left => game.move_left(),
                KeyCode::Char('l') | KeyCode::Right => game.move_right(),
                _ => {}
            }
        }
    }

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui<B: Backend>(f: &mut Frame<B>, game: &Game) {
    // TODO make this function more readable
    let [panel, menu, map] = layout(f);

    let underlined = Style::default().add_modifier(Modifier::UNDERLINED);
    let separator = Span::raw(" | ");
    // TODO make generic
    let panel_titles = Spans::from(vec![
        Span::styled(" log", Style::default().fg(Color::White)),
        separator.clone(),
        Span::styled("s", underlined),
        Span::raw("tat"),
        separator.clone(),
        Span::styled("t", underlined),
        Span::raw("odo"),
        separator.clone(),
        Span::raw("h"),
        Span::styled("e", underlined),
        Span::raw("lp "),
    ]);
    let block = Block::default()
        .title(panel_titles)
        .borders(Borders::ALL)
        .title_alignment(Alignment::Center);
    f.render_widget(block, panel);

    let block = Block::default()
        .title(format!(
            " warrior[10][xx--]@{}.{}.{} ",
            game.floor, game.character_position.x, game.character_position.y
        ))
        .borders(Borders::ALL);

    let map_strings = map_as_text(map, game);
    f.render_widget(Paragraph::new(map_strings).block(block), map);

    let disabled = Style::default().fg(Color::DarkGray);
    f.render_widget(
        Paragraph::new(vec![Spans::from(vec![
            Span::styled("u", underlined),
            Span::raw("se "),
            Span::styled("b", disabled.add_modifier(Modifier::UNDERLINED)),
            Span::styled("uy ", disabled),
            Span::styled("c", disabled.add_modifier(Modifier::UNDERLINED)),
            Span::styled("lass ", disabled),
            Span::styled("q", underlined),
            Span::raw("uit "),
            Span::styled("r", underlined),
            Span::raw("eset"),
        ])])
        .alignment(Alignment::Center),
        menu,
    );
}

fn layout<B: Backend>(f: &mut Frame<B>) -> [Rect; 3] {
    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(3)].as_ref())
        .split(f.size());

    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(2)].as_ref())
        .split(horizontal_chunks[0]);

    [vertical_chunks[0], vertical_chunks[1], horizontal_chunks[1]]
}

fn map_as_text(area: Rect, game: &Game) -> Vec<Spans<'static>> {
    let char_pos = &game.character_position;

    // When a dimension (horizontal or vertical) fits entirely in the terminal view,
    // it will be drawn at a fixed position (the character will move in that direction but not the map).
    // When it doesn't fit, the character will be fixed at the center of the view for that dimension,
    // and the map will scroll when the character moves.
    let map_fits_width = game.map().width < area.width - 2;
    let map_fits_height = game.map().height < area.height - 2;

    // These offsets are used when converting terminal rect coordinates to the map coordinates.
    // When the dimension fits the view, it adds padding so the map is centered in the screen,
    // when when it doesn't, it moves the map to fix the character in the center of the screen.
    // Full-disclosure: I reasoned about both cases separately but found that the code was the same safe this offset
    let h_offset = if map_fits_width {
        game.map().width / 2
    } else {
        char_pos.x
    };
    let v_offset = if map_fits_height {
        game.map().height / 2
    } else {
        char_pos.y
    };

    // loop through all visible terminal positions
    let mut rows = Vec::new();
    for vy in 1..area.height - 1 {
        let mut row = String::new();

        for vx in 1..area.width - 1 {
            // convert the view coordinates to map coordinates
            let mx = (h_offset + vx - 1).checked_sub(area.width / 2);
            let my = (v_offset + vy - 1).checked_sub(area.height / 2);

            let tile = match (mx, my) {
                (Some(x), Some(y)) if (x, y) == (char_pos.x, char_pos.y) => Some(Tile::Character),
                (Some(x), Some(y)) => game.map().tile_at(&Position { x, y }),
                _ => None,
            };

            if let Some(tile) = tile {
                row.push_str(tile.to_string());
            } else {
                row.push(' ');
            }
        }

        rows.push(Spans::from(row));
    }

    rows
}

struct Game {
    pub floor: usize,
    // this may eventually need to distinguish between tilemap and itemmap, maybe moving char position back to the map
    maps: Vec<Map>,
    pub character_position: Position,
}

impl Game {
    /// TODO
    pub fn new() -> Self {
        let first_map = Map::new(0);
        let character_position = first_map.random_position();
        Self {
            floor: 0,
            character_position,
            maps: vec![first_map],
        }
    }

    pub fn map(&self) -> &Map {
        &self.maps[self.floor]
    }

    pub fn move_up(&mut self) {
        let dest_position = Position {
            x: self.character_position.x,
            y: self.character_position.y - 1,
        };
        self.move_to(dest_position);
    }

    pub fn move_down(&mut self) {
        let dest_position = Position {
            x: self.character_position.x,
            y: self.character_position.y + 1,
        };
        self.move_to(dest_position);
    }

    pub fn move_left(&mut self) {
        let dest_position = Position {
            x: self.character_position.x - 1,
            y: self.character_position.y,
        };
        self.move_to(dest_position);
    }

    pub fn move_right(&mut self) {
        let dest_position = Position {
            x: self.character_position.x + 1,
            y: self.character_position.y,
        };
        self.move_to(dest_position);
    }

    /// Update the character position to the given destination, when it's a valid movement
    /// (e.g. if there isn't a wall there). If the destination is an up or down ladder,
    /// move the character to the corresponding floor.
    fn move_to(&mut self, dest_position: Position) {
        let is_wall = self.map().tile_at(&dest_position) == Some(Tile::Wall);

        // assuming character is always inside a room, it can move as long as its not walking into a wall
        // this will change if there are other non walkable entities or tiles in the map
        if self.character_position.y > 0 && !is_wall {
            self.character_position = dest_position;
        }

        // when, after applying a movement, the character steps into a ladder
        // it needs to be moved over to the matching ladder on the next or previous floor.
        // New floors are added when going down to an unvisited floor
        match self.map().tile_at(&self.character_position) {
            Some(Tile::LadderUp) => {
                self.floor -= 1;

                // start at the ladder
                self.character_position = self
                    .map()
                    .find_tile(Tile::LadderDown)
                    .expect("all floors have a ladder down");
            }
            Some(Tile::LadderDown) => {
                self.floor += 1;

                if self.floor == self.maps.len() {
                    // haven't been to this floor before, need to create a new one
                    self.maps.push(Map::new(self.floor));
                }

                // start at the ladder
                self.character_position = self
                    .map()
                    .find_tile(Tile::LadderUp)
                    .expect("all non zero floors have a ladder up");
            }
            _ => {}
        }
    }
}

struct Map {
    pub width: u16,
    pub height: u16,
    tiles: HashMap<Position, Tile>,
}

impl Map {
    // for now working with a fixed map size and assuming that the view size
    // is the same. later those can be separated and scrolling can be introduced
    // to handle bigger maps and smaller terminal sizes.
    const MIN_WIDTH: u16 = 20;
    const MAX_WIDTH: u16 = 100;
    const MIN_HEIGHT: u16 = 10;
    const MAX_HEIGHT: u16 = 50;

    // FIXME turn into default
    /// Create a map for the first floor, with randomly placed character and down ladder.
    pub fn new(floor: usize) -> Self {
        let mut rng = rand::thread_rng();
        let width = rng.gen_range(Self::MIN_WIDTH..=Self::MAX_WIDTH);
        let height = rng.gen_range(Self::MIN_HEIGHT..=Self::MAX_HEIGHT);

        let mut map = Self {
            width,
            height,
            tiles: HashMap::new(),
        };

        // For now generate rectangular maps: a single room covering the whole map with walls
        // along the borders
        for x in 0..map.width {
            for y in 0..map.height {
                let tile = if x == 0 || x == map.width - 1 || y == 0 || y == map.height - 1 {
                    Tile::Wall
                } else {
                    Tile::Ground
                };
                map.tiles.insert(Position { x, y }, tile);
            }
        }

        map.tiles.insert(map.random_position(), Tile::LadderDown);
        if floor > 0 {
            map.tiles.insert(map.random_position(), Tile::LadderUp);
        }
        map
    }

    fn find_tile(&self, expected: Tile) -> Option<Position> {
        for (pos, current) in self.tiles.iter() {
            if *current == expected {
                return Some(pos.clone());
            }
        }
        None
    }

    pub fn tile_at(&self, position: &Position) -> Option<Tile> {
        self.tiles.get(position).cloned()
    }

    /// Return a random and unused position within the map to place a new tile.
    pub fn random_position(&self) -> Position {
        let mut rng = rand::thread_rng();

        loop {
            let pos = Position {
                x: rng.gen_range(0..self.width),
                y: rng.gen_range(0..self.height),
            };
            let tile = self.tiles.get(&pos);
            // FIXME floor is special case, may need something more generic for this
            if tile.is_none() || *tile.unwrap() == Tile::Ground {
                return pos;
            }
        }
    }
}

#[derive(Clone, PartialEq)]
enum Tile {
    Character,
    LadderUp,
    LadderDown,
    Wall,
    Ground,
}

impl Tile {
    // FIXME probably use some standard trait for this
    fn to_string(&self) -> &'static str {
        match self {
            Tile::Character => "@",
            Tile::LadderUp => "↑",
            Tile::LadderDown => "↓",
            Tile::Wall => "#",
            Tile::Ground => ".",
        }
    }
}

// TODO make this interoperable with tuples
#[derive(Eq, Hash, PartialEq, Clone)]
struct Position {
    pub x: u16,
    pub y: u16,
}
