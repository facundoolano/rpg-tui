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
    layout::{Alignment, Rect},
    text::Text,
    widgets::{Block, Borders, Paragraph, Wrap},
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
                KeyCode::Char('q') => break,
                KeyCode::Char('w') | KeyCode::Char('j') | KeyCode::Up => game.move_up(),
                KeyCode::Char('s') | KeyCode::Char('k') | KeyCode::Down => game.move_down(),
                KeyCode::Char('a') | KeyCode::Char('h') | KeyCode::Left => game.move_left(),
                KeyCode::Char('d') | KeyCode::Char('l') | KeyCode::Right => game.move_right(),
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
    let term_size = f.size();

    let view_width = Map::WIDTH + 2;
    let view_height = Map::HEIGHT + 2;

    if term_size.width < view_width || term_size.height < view_height {
        let message = Paragraph::new(Text::raw(
            "Terminal is too small, resize or press q to quit.",
        ))
        .wrap(Wrap { trim: false });
        f.render_widget(message, term_size);
        return;
    }

    let left_padding = (term_size.width - view_width) / 2;
    let top_padding = (term_size.height - view_height) / 2;
    let size = Rect::new(left_padding, top_padding, view_width, view_height);
    let block = Block::default()
        .title(format!("floor {}", game.floor))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);

    f.render_widget(block, size);

    for (pos, tile) in game.tiles() {
        let text = Text::raw(tile.to_string());
        f.render_widget(
            Paragraph::new(text),
            // need to +1 because map 0 shouldn't match view 0
            Rect::new(pos.x + left_padding + 1, pos.y + top_padding + 1, 1, 1),
        );
    }
}

struct Game {
    pub floor: usize,
    // this may eventually need to distinguish between tilemap and itemmap, maybe moving char position back to the map
    pub maps: Vec<Map>,
    pub character_position: Position,
}

impl Game {
    /// TODO
    fn new() -> Self {
        let first_map = Map::new(0);
        let character_position = first_map.random_position();
        Self {
            floor: 0,
            character_position,
            maps: vec![first_map],
        }
    }

    /// TODO
    pub fn tiles(&self) -> Vec<(Position, Tile)> {
        let mut tiles: Vec<_> = self.maps[self.floor].tiles.clone().into_iter().collect();
        tiles.push((self.character_position.clone(), Tile::Character));
        tiles
    }

    pub fn move_up(&mut self) {
        if self.character_position.y > 0 {
            self.character_position.y -= 1;
        }
        self.update_floor();
    }

    pub fn move_down(&mut self) {
        if self.character_position.y < self.maps[self.floor].height - 1 {
            self.character_position.y += 1;
        }
        self.update_floor();
    }

    pub fn move_left(&mut self) {
        if self.character_position.x > 0 {
            self.character_position.x -= 1;
        }
        self.update_floor();
    }

    pub fn move_right(&mut self) {
        if self.character_position.x < self.maps[self.floor].width - 1 {
            self.character_position.x += 1;
        }
        self.update_floor();
    }

    /// TODO
    fn update_floor(&mut self) {
        match self.maps[self.floor].tiles.get(&self.character_position) {
            Some(Tile::LadderUp) => {
                self.floor -= 1;

                // start at the ladder
                self.character_position = self.maps[self.floor]
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
                self.character_position = self.maps[self.floor]
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
    pub const WIDTH: u16 = 80;
    pub const HEIGHT: u16 = 20;

    // FIXME turn into default
    /// Create a map for the first floor, with randomly placed character and down ladder.
    pub fn new(floor: usize) -> Self {
        let mut map = Self {
            width: Self::WIDTH,
            height: Self::HEIGHT,
            tiles: HashMap::new(),
        };

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

    /// Return a random and unused position within the map to place a new tile.
    pub fn random_position(&self) -> Position {
        let mut rng = rand::thread_rng();

        loop {
            let pos = Position {
                x: rng.gen_range(0..self.width),
                y: rng.gen_range(0..self.height),
            };
            if !self.tiles.contains_key(&pos) {
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
}

impl Tile {
    // FIXME probably use some standard trait for this
    fn to_string(&self) -> &'static str {
        match self {
            Tile::Character => "@",
            Tile::LadderUp => "↑",
            Tile::LadderDown => "↓",
        }
    }
}

#[derive(Eq, Hash, PartialEq, Clone)]
struct Position {
    pub x: u16,
    pub y: u16,
}
