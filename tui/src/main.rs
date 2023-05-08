use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Rect},
    widgets::{Block, Borders},
    Terminal,
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

    loop {
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
