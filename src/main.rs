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
                KeyCode::Char('q') => break,
                KeyCode::Char('w') | KeyCode::Char('k') | KeyCode::Up => game.move_up(),
                KeyCode::Char('s') | KeyCode::Char('j') | KeyCode::Down => game.move_down(),
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

    // can rust-tui handle some of the padding and relative coords stuff?
    let h_padding = 5;
    let v_padding = 3;
    let view_width = term_size.width - h_padding * 2;
    let view_height = term_size.height - v_padding * 2;
    let size = Rect::new(h_padding, v_padding, view_width, view_height);

    let block = Block::default()
        .title(format!("floor {}", game.floor))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);

    f.render_widget(block, size);

    let char_pos = &game.character_position;

    // When a dimension (horizontal or vertical) fits entirely in the terminal view,
    // it will be drawn at a fixed position (the character will move in that direction but not the map).
    // When it doesn't fit, the character will be fixed at the center of the view for that dimension,
    // and the map will scroll when the character moves.
    let map_fits_width = game.map().width < view_width;
    let map_fits_height = game.map().height < view_height;

    let (start_vx, end_vx) = if map_fits_width {
        let start_vx = (view_width - game.map().width) / 2;
        (start_vx, start_vx + game.map().width)
    } else {
        (0, view_width - 2)
    };

    let (start_vy, end_vy) = if map_fits_height {
        let start_vy = (view_height - game.map().height) / 2;
        (start_vy, start_vy + game.map().height)
    } else {
        (0, view_height - 2)
    };

    // loop through all visible terminal positions
    for vx in start_vx..end_vx {
        for vy in start_vy..end_vy {
            // convert the view coordinates to map coordinates
            let mx = if map_fits_width {
                Some(vx - start_vx)
            } else {
                (char_pos.x + vx).checked_sub(view_width / 2)
            };

            let my = if map_fits_height {
                Some(vy - start_vy)
            } else {
                (char_pos.y + vy).checked_sub(view_height / 2)
            };

            // if the map position exists and has a tile in it, get it
            let tile = match (mx, my) {
                (Some(x), Some(y)) if (x, y) == (char_pos.x, char_pos.y) => Some(Tile::Character),
                (Some(x), Some(y)) => game.map().tile_at(&Position { x, y }),
                _ => None,
            };

            // put the tile ascii representation in the screen
            if let Some(tile) = tile {
                let text = Text::raw(tile.to_string());
                f.render_widget(
                    Paragraph::new(text),
                    Rect::new(vx + h_padding + 1, vy + v_padding + 1, 1, 1),
                );
            }
        }
    }
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
                if y == 0 || y == map.height - 1 || x == 0 || x == map.width - 1 {
                    map.tiles.insert(Position { x, y }, Tile::Wall);
                }
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
    Wall,
}

impl Tile {
    // FIXME probably use some standard trait for this
    fn to_string(&self) -> &'static str {
        match self {
            Tile::Character => "@",
            Tile::LadderUp => "↑",
            Tile::LadderDown => "↓",
            Tile::Wall => "#",
        }
    }
}

// TODO make this interoperable with tuples
#[derive(Eq, Hash, PartialEq, Clone)]
struct Position {
    pub x: u16,
    pub y: u16,
}
