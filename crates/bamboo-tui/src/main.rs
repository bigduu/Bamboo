use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;

mod app;
mod ui;
mod client;
mod components;

use app::{App, InputMode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new("http://localhost:8081").await?;
    
    // Check connection
    app.check_connection().await;
    
    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    let mut last_tick = tokio::time::Instant::now();
    let tick_rate = tokio::time::Duration::from_millis(100);

    loop {
        // Draw UI
        terminal.draw(|f| ui::draw(f, app))?;

        // Handle timeout for event polling
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| tokio::time::Duration::from_secs(0));

        // Handle events
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = crossterm::event::read()? {
                match handle_key_event(app, key).await {
                    Ok(should_quit) => {
                        if should_quit {
                            return Ok(());
                        }
                    }
                    Err(e) => {
                        app.add_system_message(format!("Error: {}", e));
                    }
                }
            }
        }

        // Process SSE events
        app.process_events().await;

        // Update on tick
        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = tokio::time::Instant::now();
        }
    }
}

async fn handle_key_event(app: &mut App, key: KeyEvent) -> anyhow::Result<bool> {
    match app.input_mode() {
        InputMode::Normal => match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(true); // Quit
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.stop_generation().await;
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.new_session().await?;
            }
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.clear_messages();
            }
            KeyCode::Enter => {
                app.send_message().await?;
            }
            KeyCode::Char(c) => {
                app.push_input(c);
            }
            KeyCode::Backspace => {
                app.pop_input();
            }
            KeyCode::Up => {
                app.scroll_up();
            }
            KeyCode::Down => {
                app.scroll_down();
            }
            KeyCode::PageUp => {
                app.scroll_page_up();
            }
            KeyCode::PageDown => {
                app.scroll_page_down();
            }
            _ => {}
        },
    }
    Ok(false)
}
