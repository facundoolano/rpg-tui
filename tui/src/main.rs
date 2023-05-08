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

    let map = Map::first_floor();

    loop {
        terminal.draw(|mut f| ui(&mut f, &map))?;

        // when q is pressed, finish the program
        if let Event::Key(key) = event::read()? {
            if let KeyCode::Char('q') = key.code {
                break;
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

    if term_size.width < Map::WIDTH || term_size.height < Map::HEIGHT {
        let message = Paragraph::new(Text::raw(
            "Terminal is too small, resize or press q to quit.",
        ))
        .wrap(Wrap { trim: false });
        f.render_widget(message, term_size);
        return;
    }

    let left_padding = (term_size.width - Map::WIDTH) / 2;
    let top_padding = (term_size.height - Map::HEIGHT) / 2;
    let size = Rect::new(left_padding, top_padding, Map::WIDTH, Map::HEIGHT);
    let block = Block::default()
        .title(format!("floor {}", map.floor))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);

    f.render_widget(block, size);

    for ((x, y), tile) in map.tiles.iter() {
        let text = Text::raw(tile.to_string());
        f.render_widget(Paragraph::new(text), Rect::new(*x + left_padding, *y + top_padding, 1, 1));
    }
}

struct Map {
    pub width: u16,
    pub height: u16,

    // for now working with a fixed map size and assuming that the view size
    // is the same. later those can be separated and scrolling can be introduced
    // to handle bigger maps and smaller terminal sizes.
    pub floor: u16,
    pub tiles: HashMap<(u16, u16), Tile>,
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
        };

        map.tiles.insert(map.random_position(), Tile::LadderDown);
        map.tiles.insert(map.random_position(), Tile::Character);
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
        };

        let up_position = map.random_position();
        map.tiles.insert(up_position, Tile::LadderUp);
        map.tiles.insert(up_position, Tile::Character);
        map.tiles.insert(map.random_position(), Tile::LadderDown);
        map
    }

    /// Return a random and unused position within the map to place a new tile.
    fn random_position(&self) -> (u16, u16) {
        let mut rng = rand::thread_rng();

        loop {
            let pos = (rng.gen_range(0..self.width), rng.gen_range(0..self.height));
            if !self.tiles.contains_key(&pos) {
                return pos;
            }
        }
    }
}

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
