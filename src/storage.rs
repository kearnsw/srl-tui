//! Storage module for saving and loading flashcard decks.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::models::Deck;

/// Bundled deck: Development Workflow
const BUNDLED_DEV_WORKFLOW: &str = include_str!("../bundled_decks/development-workflow.json");

/// Handles deck persistence.
pub struct DeckStorage {
    decks_dir: PathBuf,
}

impl DeckStorage {
    pub fn new(decks_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&decks_dir)
            .with_context(|| format!("Failed to create decks directory: {:?}", decks_dir))?;

        let storage = Self { decks_dir };
        storage.install_bundled_decks();
        Ok(storage)
    }

    /// Install bundled decks if they don't already exist.
    fn install_bundled_decks(&self) {
        // Check if any decks exist - if so, user has already used the app
        if let Ok(entries) = fs::read_dir(&self.decks_dir) {
            if entries.filter_map(|e| e.ok()).any(|e| {
                e.path().extension().map_or(false, |ext| ext == "json")
            }) {
                return; // User already has decks, don't overwrite
            }
        }

        // Install bundled decks for first-time users
        if let Ok(mut deck) = serde_json::from_str::<Deck>(BUNDLED_DEV_WORKFLOW) {
            // Reset all cards to fresh state
            for card in &mut deck.cards {
                card.reset_progress();
            }
            let _ = self.save_deck(&deck);
        }
    }

    /// Get default storage location.
    pub fn default_path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("flashcards")
            .join("decks")
    }

    fn deck_path(&self, deck_id: &str) -> PathBuf {
        self.decks_dir.join(format!("{}.json", deck_id))
    }

    /// Save a deck to disk.
    pub fn save_deck(&self, deck: &Deck) -> Result<PathBuf> {
        let path = self.deck_path(&deck.id);
        let json = serde_json::to_string_pretty(deck)?;
        fs::write(&path, json)?;
        Ok(path)
    }

    /// Load a deck from disk.
    pub fn load_deck(&self, deck_id: &str) -> Result<Option<Deck>> {
        let path = self.deck_path(deck_id);
        if !path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&path)?;
        let deck: Deck = serde_json::from_str(&json)?;
        Ok(Some(deck))
    }

    /// Delete a deck file.
    pub fn delete_deck(&self, deck_id: &str) -> Result<bool> {
        let path = self.deck_path(deck_id);
        if path.exists() {
            fs::remove_file(&path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// List all available decks.
    pub fn list_decks(&self) -> Result<Vec<DeckInfo>> {
        let mut decks = Vec::new();

        for entry in fs::read_dir(&self.decks_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |e| e == "json") {
                if let Ok(json) = fs::read_to_string(&path) {
                    if let Ok(deck) = serde_json::from_str::<Deck>(&json) {
                        decks.push(DeckInfo {
                            id: deck.id,
                            name: deck.name,
                            card_count: deck.cards.len(),
                            description: deck.description,
                        });
                    }
                }
            }
        }

        decks.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(decks)
    }

    /// Import cards from a CSV file.
    pub fn import_csv(&self, csv_path: &Path, deck_name: &str) -> Result<Deck> {
        let mut deck = Deck::new(deck_name.to_string());
        let content = fs::read_to_string(csv_path)?;

        for (i, line) in content.lines().enumerate() {
            // Skip header
            if i == 0 && line.to_lowercase().contains("front") {
                continue;
            }

            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 2 {
                let front = parts[0].trim().to_string();
                let back = parts[1].trim().to_string();

                if !front.is_empty() && !back.is_empty() {
                    deck.add_card(front, back);
                }
            }
        }

        Ok(deck)
    }

    /// Import all CSV files from a folder.
    /// Names decks based on filename, converting snake_case/kebab-case to Title Case.
    /// Skips any deck whose name already exists.
    /// Returns (imported, skipped) tuple.
    pub fn import_folder(&self, folder_path: &Path) -> Result<(Vec<(String, usize)>, Vec<String>)> {
        let mut imported = Vec::new();
        let mut skipped = Vec::new();

        // Get existing deck names for duplicate check
        let existing_names: std::collections::HashSet<String> = self
            .list_decks()?
            .into_iter()
            .map(|d| d.name.to_lowercase())
            .collect();

        for entry in fs::read_dir(folder_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |e| e == "csv") {
                let deck_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(filename_to_title_case)
                    .unwrap_or_else(|| "Imported Deck".to_string());

                // Skip if deck with this name already exists
                if existing_names.contains(&deck_name.to_lowercase()) {
                    skipped.push(deck_name);
                    continue;
                }

                match self.import_csv(&path, &deck_name) {
                    Ok(deck) => {
                        let card_count = deck.cards.len();
                        if card_count > 0 {
                            self.save_deck(&deck)?;
                            imported.push((deck_name, card_count));
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to import {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok((imported, skipped))
    }

    /// Check if a deck with the given name already exists.
    pub fn deck_name_exists(&self, name: &str) -> bool {
        self.list_decks()
            .map(|decks| decks.iter().any(|d| d.name.to_lowercase() == name.to_lowercase()))
            .unwrap_or(false)
    }
}

/// Convert a filename (snake_case or kebab-case) to Title Case.
fn filename_to_title_case(name: &str) -> String {
    name.split(|c| c == '_' || c == '-')
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() + chars.as_str().to_lowercase().as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Summary info for a deck.
#[derive(Debug, Clone)]
pub struct DeckInfo {
    pub id: String,
    pub name: String,
    pub card_count: usize,
    pub description: String,
}

/// Backup format containing all decks.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Backup {
    pub version: u32,
    pub created_at: chrono::DateTime<chrono::Local>,
    pub decks: Vec<Deck>,
}

impl DeckStorage {
    /// Export all decks to a backup file.
    pub fn export_backup(&self, path: &Path) -> Result<usize> {
        let deck_infos = self.list_decks()?;
        let mut decks = Vec::new();

        for info in &deck_infos {
            if let Ok(Some(deck)) = self.load_deck(&info.id) {
                decks.push(deck);
            }
        }

        let backup = Backup {
            version: 1,
            created_at: chrono::Local::now(),
            decks,
        };

        let json = serde_json::to_string_pretty(&backup)?;
        fs::write(path, json)?;

        Ok(backup.decks.len())
    }

    /// Import decks from a backup file.
    /// Returns (imported_count, skipped_count).
    pub fn import_backup(&self, path: &Path) -> Result<(usize, usize)> {
        let json = fs::read_to_string(path)?;
        let backup: Backup = serde_json::from_str(&json)?;

        let existing_ids: std::collections::HashSet<String> = self
            .list_decks()?
            .into_iter()
            .map(|d| d.id)
            .collect();

        let mut imported = 0;
        let mut skipped = 0;

        for deck in backup.decks {
            if existing_ids.contains(&deck.id) {
                skipped += 1;
            } else {
                self.save_deck(&deck)?;
                imported += 1;
            }
        }

        Ok((imported, skipped))
    }

    /// Get default backup path.
    pub fn default_backup_path() -> PathBuf {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        dirs::document_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."))
            .join(format!("srl_backup_{}.json", timestamp))
    }
}
