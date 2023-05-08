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

    let mut map = Map::first_floor();

    loop {
        terminal.draw(|f| ui(f, &map))?;

        // when q is pressed, finish the program
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('w') | KeyCode::Char('j') | KeyCode::Up => map.move_up(),
                KeyCode::Char('s') | KeyCode::Char('k') | KeyCode::Down => map.move_down(),
                KeyCode::Char('a') | KeyCode::Char('h') | KeyCode::Left => map.move_left(),
                KeyCode::Char('d') | KeyCode::Char('l') | KeyCode::Right => map.move_right(),
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

fn ui<B: Backend>(f: &mut Frame<B>, map: &Map) {
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
        .title(format!("floor {}", map.floor))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);

    f.render_widget(block, size);

    for (pos, tile) in map.tiles() {
        let text = Text::raw(tile.to_string());
        f.render_widget(
            Paragraph::new(text),
            // need to +1 because map 0 shouldn't match view 0
            Rect::new(pos.x + left_padding + 1, pos.y + top_padding + 1, 1, 1),
        );
    }
}

#[derive(Eq, Hash, PartialEq, Clone)]
struct Position {
    pub x: u16,
    pub y: u16,
}

struct Map {
    pub width: u16,
    pub height: u16,

    // for now working with a fixed map size and assuming that the view size
    // is the same. later those can be separated and scrolling can be introduced
    // to handle bigger maps and smaller terminal sizes.
    pub floor: u16,
    pub character_position: Position,
    tiles: HashMap<Position, Tile>,
}

impl Map {
    pub const WIDTH: u16 = 80;
    pub const HEIGHT: u16 = 20;

    // FIXME turn into default
    /// Create a map for the first floor, with randomly placed character and down ladder.
    pub fn first_floor() -> Self {
        let mut map = Self {
            width: Self::WIDTH,
            height: Self::HEIGHT,
            floor: 0,
            tiles: HashMap::new(),

            // placeholder initialization
            character_position: Position { x: 0, y: 0 },
        };

        map.tiles.insert(map.random_position(), Tile::LadderDown);
        map.character_position = map.random_position();
        map
    }

    /// Create a map for the floor below the given one, with randomly placed up and down ladders,
    /// and the character starting at the position of the up ladder.
    pub fn next_floor(&self) -> Self {
        let mut map = Self {
            width: self.width,
            height: self.height,
            floor: self.floor + 1,
            tiles: HashMap::new(),

            // placeholder initialization
            character_position: Position { x: 0, y: 0 },
        };

        let up_position = map.random_position();
        map.tiles.insert(up_position.clone(), Tile::LadderUp);
        map.character_position = up_position;
        map.tiles.insert(map.random_position(), Tile::LadderDown);
        map
    }

    /// Return a random and unused position within the map to place a new tile.
    fn random_position(&self) -> Position {
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

    /// TODO
    fn tiles(&self) -> Vec<(Position, Tile)> {
        let mut tiles: Vec<_> = self.tiles.clone().into_iter().collect();
        // FIXME this is weird, probably should be somewhere else
        tiles.push((self.character_position.clone(), Tile::Character));
        tiles
    }

    fn move_up(&mut self) {
        if self.character_position.y > 0 {
            self.character_position.y -= 1;
        }
    }

    fn move_down(&mut self) {
        if self.character_position.y < self.height - 1 {
            self.character_position.y += 1;
        }
    }

    fn move_left(&mut self) {
        if self.character_position.x > 0 {
            self.character_position.x -= 1;
        }
    }

    fn move_right(&mut self) {
        if self.character_position.x < self.width - 1 {
            self.character_position.x += 1;
        }
    }
}

#[derive(Clone)]
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
