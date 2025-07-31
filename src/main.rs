// src/main.rs

mod app;
mod git;
pub mod types;
mod ui;

use crate::app::{App, AppMode};
use crate::ui::draw;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::{io, time::Duration};
use tui::{Terminal, backend::CrosstermBackend};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()?;
    let res = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error running app: {:?}", err);
    }

    Ok(())
}

async fn run_app<B: tui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| draw(f, app))?;

        // Use tokio::select! to handle both terminal events and async messages
        tokio::select! {
            // Handle terminal input
            result = tokio::task::spawn_blocking(event::read) => {
                if let Ok(Ok(Event::Key(key))) = result {
                    if key.kind == KeyEventKind::Press {
                        app.handle_key_event(key);
                    }
                }
            }
            // Handle async push feedback
            Some(msg) = app.push_feedback_receiver.recv() => {
                app.mode = AppMode::Pushing(msg);
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
