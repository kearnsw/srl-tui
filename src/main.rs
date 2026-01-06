//! Flashcards - Anki-style spaced repetition TUI
//!
//! A beautiful terminal-based flashcard application with SM-2 spaced repetition.

mod config;
mod models;
mod sm2;
mod storage;
mod ui;

use std::io;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use storage::DeckStorage;
use ui::App;

// ══════════════════════════════════════════════════════════════════════════
// CLI Arguments
// ══════════════════════════════════════════════════════════════════════════

#[derive(Parser, Debug)]
#[command(name = "flashcards")]
#[command(author, version, about = "Anki-style spaced repetition flashcard TUI", long_about = None)]
struct Args {
    /// Directory containing deck files
    #[arg(short, long)]
    decks_dir: Option<PathBuf>,

    /// Import cards from a CSV file
    #[arg(short, long)]
    import: Option<PathBuf>,

    /// Name for imported deck
    #[arg(long, default_value = "Imported Deck")]
    import_name: String,
}

// ══════════════════════════════════════════════════════════════════════════
// Main Entry Point
// ══════════════════════════════════════════════════════════════════════════

fn main() -> Result<()> {
    let args = Args::parse();

    // Determine decks directory
    let decks_dir = args.decks_dir.unwrap_or_else(DeckStorage::default_path);

    // Initialize storage
    let storage = DeckStorage::new(decks_dir)?;

    // Handle import if requested
    if let Some(csv_path) = args.import {
        let deck = storage.import_csv(&csv_path, &args.import_name)?;
        storage.save_deck(&deck)?;
        println!(
            "✓ Imported {} cards into '{}'",
            deck.cards.len(),
            deck.name
        );
        return Ok(());
    }

    // Run TUI
    run_tui(storage)
}

fn run_tui(storage: DeckStorage) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Load config
    let config = config::Config::load().unwrap_or_default();

    // Create app
    let mut app = App::new(storage, config);

    // Run main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Handle any errors
    if let Err(err) = result {
        eprintln!("Error: {}", err);
        return Err(err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    while app.running {
        terminal.draw(|frame| app.render(frame))?;
        app.handle_events()?;
    }
    Ok(())
}
