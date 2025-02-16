use crossterm::{
    event::{self, Event, KeyCode},
    terminal,
};
use rand::Rng;
use std::{collections::HashMap, io};
use tui::{layout, style, text, widgets};

type TerminalBackend = tui::backend::CrosstermBackend<io::Stdout>;
type TerminalFrame<'a> = tui::Frame<'a, TerminalBackend>;

fn main() -> Result<(), io::Error> {
    // setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, terminal::EnterAlternateScreen)?;
    let backend = TerminalBackend::new(stdout);
    let mut terminal = tui::Terminal::new(backend)?;

    let mut game = Game::new();

    loop {
        terminal.draw(|frame| render(&game, frame))?;

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
    terminal::disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), terminal::LeaveAlternateScreen)?;

    Ok(())
}

fn render(game: &Game, frame: &mut TerminalFrame) {
    let [info_panel, command_menu, map_panel] = layout(frame.size());

    // show an info panel with available "views":
    // event logs, character status, quest todos and game help (eg. keybindings)
    // for now the panel is empty.

    // TODO add helpers for more readable styles
    // these could eventually be turned into custom tui-rs widgets
    let underlined = style::Style::default().add_modifier(style::Modifier::UNDERLINED);
    let separator = text::Span::raw(" | ");

    let panel_titles = text::Spans::from(vec![
        text::Span::styled(" log", style::Style::default().fg(style::Color::White)),
        separator.clone(),
        text::Span::styled("s", underlined),
        text::Span::raw("tat"),
        separator.clone(),
        text::Span::styled("t", underlined),
        text::Span::raw("odo"),
        separator.clone(),
        text::Span::raw("h"),
        text::Span::styled("e", underlined),
        text::Span::raw("lp "),
    ]);
    let block = widgets::Block::default()
        .title(panel_titles)
        .borders(widgets::Borders::ALL)
        .title_alignment(layout::Alignment::Center);
    frame.render_widget(block, info_panel);

    // show a menu of additional commands, not associated with a view
    // use an inventory item; buy items or change character class (only at floor zero);
    // quit or reset the game
    let disabled = style::Style::default().fg(style::Color::DarkGray);
    frame.render_widget(
        widgets::Paragraph::new(vec![text::Spans::from(vec![
            text::Span::styled("u", underlined),
            text::Span::raw("se "),
            text::Span::styled("b", disabled.add_modifier(style::Modifier::UNDERLINED)),
            text::Span::styled("uy ", disabled),
            text::Span::styled("c", disabled.add_modifier(style::Modifier::UNDERLINED)),
            text::Span::styled("lass ", disabled),
            text::Span::styled("q", underlined),
            text::Span::raw("uit "),
            text::Span::styled("r", underlined),
            text::Span::raw("eset"),
        ])])
        .alignment(layout::Alignment::Center),
        command_menu,
    );

    // Render the current map state as a string (a vec of tui-rs text Spans).
    // The title of the map panel shows basic stats, mostly hardcoded for now
    let map_block = widgets::Block::default()
        .title(format!(
            " warrior[10][xx--]@{}.{}.{} ",
            game.floor, game.character_position.x, game.character_position.y
        ))
        .borders(widgets::Borders::ALL);
    let map_container = map_block.inner(map_panel);
    let map_spans: Vec<_> = map_as_strings(game, map_container.width, map_container.height)
        .into_iter()
        .map(text::Spans::from)
        .collect();

    frame.render_widget(
        widgets::Paragraph::new(map_spans).block(map_block),
        map_panel,
    );
}

/// Split the available frame size in three blocks:
///   a panel to display information (e.g. battle logs)
///   a menu of available actions (e.g. quit or reset game)
///   a block to display the map
fn layout(frame_size: layout::Rect) -> [layout::Rect; 3] {
    // split into a fixed size column on the left, and the rest of the available screen for the map
    let horizontal_chunks = layout::Layout::default()
        .direction(layout::Direction::Horizontal)
        .constraints([layout::Constraint::Length(30), layout::Constraint::Min(3)].as_ref())
        .split(frame_size);

    // leave a line for commands at the bottom, and use the rest of the column for the display panel
    let vertical_chunks = layout::Layout::default()
        .direction(layout::Direction::Vertical)
        .constraints([layout::Constraint::Min(3), layout::Constraint::Length(2)].as_ref())
        .split(horizontal_chunks[0]);

    [vertical_chunks[0], vertical_chunks[1], horizontal_chunks[1]]
}

/// Return a vector of strings representing the current map according to the player position
/// and available terminal view size. When a dimension (horizontal or vertical) fits entirely in the view,
/// the map will be centered in the screen in that direction.
/// When it doesn't fit, the character will be fixed at the center of the view for that dimension,
/// and the map will scroll when the character moves.
fn map_as_strings(game: &Game, view_width: u16, view_height: u16) -> Vec<String> {
    let char_x = game.character_position.x;
    let char_y = game.character_position.y;

    // loop through all visible terminal positions, building a span of text for each row in the map
    let mut rows = Vec::new();
    for vy in 0..view_height {
        let mut row = String::new();

        for vx in 0..view_width {
            let mx = to_world_coords(vx, char_x, view_width, game.map().width);
            let my = to_world_coords(vy, char_y, view_height, game.map().height);

            // if there's a tile at this position, push its ascii representation to the text row
            // otherwise just add an empty space
            let tile = match (mx, my) {
                (Some(x), Some(y)) if (x, y) == (char_x, char_y) => Tile::Character,
                (Some(x), Some(y)) => game.map().tile_at(&Position { x, y }),
                _ => Tile::Empty,
            };

            row.push_str(&tile.to_string());
        }

        rows.push(row);
    }

    rows
}

/// Convert a view coordinate to a map coordinate, with different strategies depending on
/// view and map relative sizes as well as character position.
fn to_world_coords(view_x: u16, player_x: u16, view_width: u16, map_width: u16) -> Option<u16> {
    if map_width < view_width {
        // When the map fits in the view, padding is added so the map is centered in the screen.
        // This underflows for view points in the starting padding area
        (view_x + map_width / 2).checked_sub(view_width / 2)
    } else if player_x <= view_width / 2 {
        // if player is close to the starting edge of the map, fix the map at that edge and move the player
        Some(view_x)
    } else if player_x >= map_width - view_width / 2 {
        // if player is close to the final edge of the map, fix the map at that edge and move the player
        Some(view_x + map_width - view_width)
    } else {
        // if player is not close to the edges, fix the player at the center and scroll the map
        Some(view_x + player_x - view_width / 2)
    }
}

struct Game {
    pub floor: usize,
    // this may eventually need to distinguish between tilemap and itemmap, maybe moving char position back to the map
    maps: Vec<Map>,
    pub character_position: Position,
}

impl Game {
    /// Start a game with an initial map for the ground floor.
    /// Additional maps will be added as the player moves down.
    pub fn new() -> Self {
        let first_map = Map::new(0);
        let character_position = first_map.random_unocuppied_position();
        Self {
            floor: 0,
            character_position,
            maps: vec![first_map],
        }
    }

    /// Return the map the player is currently at.
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
        match self.map().tile_at(&dest_position) {
            // When stepping on a down ladder, move to the next floor at the position of the up ladder.
            // The map is created if the floor hasn't been visited before
            Tile::LadderDown => {
                self.floor += 1;

                if self.floor == self.maps.len() {
                    self.maps.push(Map::new(self.floor));
                }

                self.character_position = self
                    .map()
                    .find_tile(Tile::LadderUp)
                    .expect("all non zero floors have a ladder up");
            }

            // When stepping on a down ladder, move to the previous floor at the position of the down ladder.
            Tile::LadderUp => {
                self.floor -= 1;

                self.character_position = self
                    .map()
                    .find_tile(Tile::LadderDown)
                    .expect("all floors have a ladder down");
            }

            // Do nothing if attempting to move into a wall.
            Tile::Wall => {}

            // Otherwise update the current position
            _ => {
                self.character_position = dest_position;
            }
        }
    }
}

struct Map {
    pub width: u16,
    pub height: u16,
    tiles: HashMap<Position, Tile>,
}

impl Map {
    const MIN_WIDTH: u16 = 20;
    const MAX_WIDTH: u16 = 100;
    const MIN_HEIGHT: u16 = 10;
    const MAX_HEIGHT: u16 = 50;

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

        map.tiles
            .insert(map.random_unocuppied_position(), Tile::LadderDown);
        if floor > 0 {
            map.tiles
                .insert(map.random_unocuppied_position(), Tile::LadderUp);
        }
        map
    }

    /// Return the position of the first tile of the given type found in the map or None if not found.
    fn find_tile(&self, expected: Tile) -> Option<Position> {
        for (pos, current) in self.tiles.iter() {
            if *current == expected {
                return Some(pos.clone());
            }
        }
        None
    }

    pub fn tile_at(&self, position: &Position) -> Tile {
        self.tiles.get(position).cloned().unwrap_or(Tile::Empty)
    }

    /// Return a random position within the map that can be used to place an object.
    /// For now, this means that there's no tile or a ground type tile in it.
    pub fn random_unocuppied_position(&self) -> Position {
        let mut rng = rand::thread_rng();

        loop {
            let pos = Position {
                x: rng.gen_range(0..self.width),
                y: rng.gen_range(0..self.height),
            };
            let tile = self.tiles.get(&pos);

            // FIXME floor is special case, will need a more official way to tell if the position is unoccupied
            if tile.is_none() || *tile.unwrap() == Tile::Ground {
                return pos;
            }
        }
    }
}

#[derive(Clone, PartialEq)]
enum Tile {
    Wall,
    Ground,
    Character,
    LadderUp,
    LadderDown,
    Empty,
}

impl std::fmt::Display for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let char = match self {
            Tile::Wall => '#',
            Tile::Ground => '.',
            Tile::Character => '@',
            Tile::LadderUp => '↑',
            Tile::LadderDown => '↓',
            Tile::Empty => ' ',
        };
        write!(f, "{}", char)
    }
}

#[derive(Eq, Hash, PartialEq, Clone)]
struct Position {
    pub x: u16,
    pub y: u16,
}
