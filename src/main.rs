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

    /// Import all CSV files from a folder (auto-names from filename)
    #[arg(short = 'f', long)]
    import_folder: Option<PathBuf>,

    /// Name for imported deck
    #[arg(long, default_value = "Imported Deck")]
    import_name: String,

    /// Export all decks to a backup file
    #[arg(short = 'x', long)]
    export_backup: Option<PathBuf>,

    /// Import decks from a backup file
    #[arg(short = 'b', long)]
    import_backup: Option<PathBuf>,

    /// Import from Anki export (.apkg or tab-separated .txt)
    #[arg(short = 'a', long)]
    import_anki: Option<PathBuf>,

    /// Name for Anki text import (ignored for .apkg files)
    #[arg(long)]
    import_anki_name: Option<String>,

    /// Export all decks to Anki .apkg format (preserves scheduling)
    #[arg(short = 'A', long)]
    export_anki: Option<PathBuf>,
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

    // Handle single file import
    if let Some(csv_path) = args.import {
        // Check if deck with this name already exists
        if storage.deck_name_exists(&args.import_name) {
            println!("Skipped: deck '{}' already exists", args.import_name);
            return Ok(());
        }
        let deck = storage.import_csv(&csv_path, &args.import_name)?;
        storage.save_deck(&deck)?;
        println!(
            "Imported {} cards into '{}'",
            deck.cards.len(),
            deck.name
        );
        return Ok(());
    }

    // Handle folder import
    if let Some(folder_path) = args.import_folder {
        let (imported, skipped) = storage.import_folder(&folder_path)?;
        if imported.is_empty() && skipped.is_empty() {
            println!("No CSV files found in {:?}", folder_path);
        } else {
            if !imported.is_empty() {
                println!("Imported {} decks:", imported.len());
                for (name, count) in &imported {
                    println!("  {} ({} cards)", name, count);
                }
            }
            if !skipped.is_empty() {
                println!("Skipped {} decks (already exist):", skipped.len());
                for name in &skipped {
                    println!("  {}", name);
                }
            }
        }
        return Ok(());
    }

    // Handle backup export
    if let Some(backup_path) = args.export_backup {
        let count = storage.export_backup(&backup_path)?;
        println!("Exported {} decks to {}", count, backup_path.display());
        return Ok(());
    }

    // Handle Anki export
    if let Some(anki_path) = args.export_anki {
        let card_count = storage.export_apkg(&anki_path, None)?;
        println!(
            "Exported {} cards to {} (Anki format)",
            card_count,
            anki_path.display()
        );
        return Ok(());
    }

    // Handle backup import
    if let Some(backup_path) = args.import_backup {
        let (imported, skipped) = storage.import_backup(&backup_path)?;
        if skipped > 0 {
            println!("Imported {} decks ({} skipped - already exist)", imported, skipped);
        } else {
            println!("Imported {} decks", imported);
        }
        return Ok(());
    }

    // Handle Anki import
    if let Some(anki_path) = args.import_anki {
        let decks = storage.import_anki(&anki_path, args.import_anki_name.as_deref())?;
        let mut total_cards = 0;
        let mut saved_count = 0;
        let mut skipped_names = Vec::new();

        for deck in decks {
            if storage.deck_name_exists(&deck.name) {
                skipped_names.push(deck.name);
            } else {
                total_cards += deck.cards.len();
                storage.save_deck(&deck)?;
                println!("  {} ({} cards)", deck.name, deck.cards.len());
                saved_count += 1;
            }
        }

        if saved_count > 0 {
            println!("Imported {} deck(s) with {} total cards", saved_count, total_cards);
        }
        if !skipped_names.is_empty() {
            println!("Skipped {} deck(s) (already exist):", skipped_names.len());
            for name in skipped_names {
                println!("  {}", name);
            }
        }
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
