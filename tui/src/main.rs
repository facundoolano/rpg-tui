use std::{io, thread, time::Duration};
use tui::{
    backend::CrosstermBackend,
    widgets::{Widget, Block, Borders},
    layout::{Layout, Constraint, Direction, Rect, Alignment},
    Terminal
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

// for now working with a fixed map size and assuming that the view size
// is the same. later those can be separated and scrolling can be introduced
// to handle bigger maps and smaller terminal sizes.
const MAP_WIDTH: u16 = 80;
const MAP_HEIGHT: u16 = 20;

fn main() -> Result<(), io::Error> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.draw(|f| {
        let term_size = f.size();
        let left_padding = (term_size.width - MAP_WIDTH) / 2;
        let top_padding = (term_size.height - MAP_HEIGHT) / 2;
        let size = Rect::new(left_padding, top_padding, MAP_WIDTH, MAP_HEIGHT);
        let block = Block::default()
            .title("rpg-tui")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL);
        f.render_widget(block, size);
    })?;

    thread::sleep(Duration::from_millis(5000));

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
