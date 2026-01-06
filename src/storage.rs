//! Storage module for saving and loading flashcard decks.

use anyhow::{Context, Result};
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use crate::models::{Card, Deck};

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

    /// Import cards from an Anki text export (tab-separated or semicolon-separated).
    /// Format: front<TAB>back or front;back, with optional tags column.
    pub fn import_anki_text(&self, path: &Path, deck_name: &str) -> Result<Deck> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read Anki text file: {:?}", path))?;

        let mut deck = Deck::new(deck_name.to_string());

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Detect delimiter: tab or semicolon
            let parts: Vec<&str> = if line.contains('\t') {
                line.split('\t').collect()
            } else {
                line.split(';').collect()
            };

            if parts.len() >= 2 {
                let front = parts[0].trim().to_string();
                let back = parts[1].trim().to_string();

                if !front.is_empty() && !back.is_empty() {
                    let mut card = Card::new(front, back);

                    // If there's a third column, treat it as tags
                    if parts.len() >= 3 {
                        let tags: Vec<String> = parts[2]
                            .split_whitespace()
                            .map(|t| t.to_string())
                            .collect();
                        card.tags = tags;
                    }

                    deck.cards.push(card);
                }
            }
        }

        Ok(deck)
    }

    /// Import a deck from an Anki .apkg package file.
    /// APKG files are ZIP archives containing a SQLite database.
    pub fn import_apkg(&self, path: &Path) -> Result<Vec<Deck>> {
        use rusqlite::Connection;
        use zip::ZipArchive;

        let file = File::open(path)
            .with_context(|| format!("Failed to open APKG file: {:?}", path))?;

        let mut archive = ZipArchive::new(file)
            .with_context(|| "Failed to read APKG as ZIP archive")?;

        // Find and extract the SQLite database
        // Anki 2.1+ uses collection.anki21, older versions use collection.anki2
        let db_name = if archive.file_names().any(|n| n == "collection.anki21") {
            "collection.anki21"
        } else if archive.file_names().any(|n| n == "collection.anki2") {
            "collection.anki2"
        } else {
            anyhow::bail!("No Anki database found in APKG file (expected collection.anki21 or collection.anki2)");
        };

        // Extract database to a temporary file
        let mut db_file = archive.by_name(db_name)
            .with_context(|| format!("Failed to extract {} from APKG", db_name))?;

        let temp_dir = std::env::temp_dir();
        let temp_db_path = temp_dir.join(format!("anki_import_{}.db", uuid::Uuid::new_v4()));

        let mut temp_file = File::create(&temp_db_path)
            .with_context(|| "Failed to create temporary database file")?;
        std::io::copy(&mut db_file, &mut temp_file)
            .with_context(|| "Failed to extract database")?;
        drop(temp_file);

        // Open the SQLite database
        let conn = Connection::open(&temp_db_path)
            .with_context(|| "Failed to open Anki database")?;

        // Get deck names from the col table
        let deck_names: std::collections::HashMap<i64, String> = {
            let mut stmt = conn.prepare("SELECT decks FROM col")?;
            let decks_json: String = stmt.query_row([], |row| row.get(0))?;
            let decks: serde_json::Value = serde_json::from_str(&decks_json)?;

            decks
                .as_object()
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(id, info)| {
                            let deck_id: i64 = id.parse().ok()?;
                            let name = info.get("name")?.as_str()?.to_string();
                            Some((deck_id, name))
                        })
                        .collect()
                })
                .unwrap_or_default()
        };

        // Query notes and cards with scheduling info
        // Join notes (for content) with cards (for scheduling and deck assignment)
        let mut stmt = conn.prepare(
            "SELECT n.flds, c.did, c.ivl, c.factor, c.reps, c.lapses
             FROM notes n
             JOIN cards c ON c.nid = n.id"
        )?;

        // Group cards by deck
        let mut decks_map: std::collections::HashMap<i64, Vec<Card>> = std::collections::HashMap::new();

        let rows = stmt.query_map([], |row| {
            let flds: String = row.get(0)?;
            let did: i64 = row.get(1)?;
            let ivl: i32 = row.get(2)?;
            let factor: i32 = row.get(3)?;
            let reps: i32 = row.get(4)?;
            let lapses: i32 = row.get(5)?;
            Ok((flds, did, ivl, factor, reps, lapses))
        })?;

        for row in rows {
            let (flds, did, ivl, factor, reps, lapses) = row?;

            // Split fields by Anki's field separator (0x1f)
            let fields: Vec<&str> = flds.split('\x1f').collect();
            if fields.len() < 2 {
                continue;
            }

            let front = strip_html(fields[0]);
            let back = strip_html(fields[1]);

            if front.is_empty() || back.is_empty() {
                continue;
            }

            // Create card with imported scheduling data
            let mut card = Card::new(front, back);
            card.interval = ivl.max(0) as u32;
            card.ease_factor = (factor as f64) / 1000.0;
            card.repetitions = reps.max(0) as u32;
            card.lapses = lapses.max(0) as u32;

            // Set due date if card has been reviewed
            if card.interval > 0 {
                card.due_date = Some(chrono::Local::now() + chrono::Duration::days(card.interval as i64));
            }

            decks_map.entry(did).or_default().push(card);
        }

        // Clean up temp file
        let _ = fs::remove_file(&temp_db_path);

        // Create Deck objects
        let mut result = Vec::new();
        for (did, cards) in decks_map {
            let name = deck_names
                .get(&did)
                .cloned()
                .unwrap_or_else(|| format!("Imported Deck {}", did));

            let mut deck = Deck::new(name);
            deck.cards = cards;
            result.push(deck);
        }

        if result.is_empty() {
            anyhow::bail!("No cards found in APKG file");
        }

        Ok(result)
    }

    /// Export decks to an Anki .apkg package file.
    /// Preserves scheduling data (interval, ease factor, repetitions, lapses).
    pub fn export_apkg(&self, path: &Path, deck_ids: Option<&[String]>) -> Result<usize> {
        use rusqlite::Connection;
        use std::io::Write;
        use zip::write::SimpleFileOptions;
        use zip::ZipWriter;

        // Load decks to export
        let deck_infos = self.list_decks()?;
        let decks_to_export: Vec<Deck> = if let Some(ids) = deck_ids {
            ids.iter()
                .filter_map(|id| self.load_deck(id).ok().flatten())
                .collect()
        } else {
            deck_infos
                .iter()
                .filter_map(|info| self.load_deck(&info.id).ok().flatten())
                .collect()
        };

        if decks_to_export.is_empty() {
            anyhow::bail!("No decks to export");
        }

        // Create temporary SQLite database
        let temp_dir = std::env::temp_dir();
        let temp_db_path = temp_dir.join(format!("anki_export_{}.db", uuid::Uuid::new_v4()));
        let conn = Connection::open(&temp_db_path)
            .with_context(|| "Failed to create temporary database")?;

        // Create Anki schema
        conn.execute_batch(
            r#"
            CREATE TABLE col (
                id INTEGER PRIMARY KEY,
                crt INTEGER NOT NULL,
                mod INTEGER NOT NULL,
                scm INTEGER NOT NULL,
                ver INTEGER NOT NULL,
                dty INTEGER NOT NULL,
                usn INTEGER NOT NULL,
                ls INTEGER NOT NULL,
                conf TEXT NOT NULL,
                models TEXT NOT NULL,
                decks TEXT NOT NULL,
                dconf TEXT NOT NULL,
                tags TEXT NOT NULL
            );
            CREATE TABLE notes (
                id INTEGER PRIMARY KEY,
                guid TEXT NOT NULL,
                mid INTEGER NOT NULL,
                mod INTEGER NOT NULL,
                usn INTEGER NOT NULL,
                tags TEXT NOT NULL,
                flds TEXT NOT NULL,
                sfld TEXT NOT NULL,
                csum INTEGER NOT NULL,
                flags INTEGER NOT NULL,
                data TEXT NOT NULL
            );
            CREATE TABLE cards (
                id INTEGER PRIMARY KEY,
                nid INTEGER NOT NULL,
                did INTEGER NOT NULL,
                ord INTEGER NOT NULL,
                mod INTEGER NOT NULL,
                usn INTEGER NOT NULL,
                type INTEGER NOT NULL,
                queue INTEGER NOT NULL,
                due INTEGER NOT NULL,
                ivl INTEGER NOT NULL,
                factor INTEGER NOT NULL,
                reps INTEGER NOT NULL,
                lapses INTEGER NOT NULL,
                left INTEGER NOT NULL,
                odue INTEGER NOT NULL,
                odid INTEGER NOT NULL,
                flags INTEGER NOT NULL,
                data TEXT NOT NULL
            );
            CREATE TABLE revlog (
                id INTEGER PRIMARY KEY,
                cid INTEGER NOT NULL,
                usn INTEGER NOT NULL,
                ease INTEGER NOT NULL,
                ivl INTEGER NOT NULL,
                lastIvl INTEGER NOT NULL,
                factor INTEGER NOT NULL,
                time INTEGER NOT NULL,
                type INTEGER NOT NULL
            );
            CREATE TABLE graves (
                usn INTEGER NOT NULL,
                oid INTEGER NOT NULL,
                type INTEGER NOT NULL
            );
            "#,
        )?;

        let now = chrono::Utc::now().timestamp();
        let now_millis = now * 1000;

        // Build deck JSON for col table
        let mut decks_json = serde_json::Map::new();
        // Default deck (id=1)
        decks_json.insert(
            "1".to_string(),
            serde_json::json!({
                "id": 1,
                "name": "Default",
                "mod": now,
                "usn": -1,
                "lrnToday": [0, 0],
                "revToday": [0, 0],
                "newToday": [0, 0],
                "timeToday": [0, 0],
                "collapsed": false,
                "desc": "",
                "dyn": 0,
                "conf": 1,
                "extendNew": 10,
                "extendRev": 50
            }),
        );

        // Add our decks
        for (i, deck) in decks_to_export.iter().enumerate() {
            let deck_id = (i as i64 + 2) * 1000000000000i64 + 1;
            decks_json.insert(
                deck_id.to_string(),
                serde_json::json!({
                    "id": deck_id,
                    "name": deck.name,
                    "mod": now,
                    "usn": -1,
                    "lrnToday": [0, 0],
                    "revToday": [0, 0],
                    "newToday": [0, 0],
                    "timeToday": [0, 0],
                    "collapsed": false,
                    "desc": deck.description,
                    "dyn": 0,
                    "conf": 1,
                    "extendNew": 10,
                    "extendRev": 50
                }),
            );
        }

        // Basic model (note type) for simple front/back cards
        let model_id: i64 = 1000000000001;
        let models_json = serde_json::json!({
            model_id.to_string(): {
                "id": model_id,
                "name": "Basic",
                "type": 0,
                "mod": now,
                "usn": -1,
                "sortf": 0,
                "did": 1,
                "tmpls": [{
                    "name": "Card 1",
                    "ord": 0,
                    "qfmt": "{{Front}}",
                    "afmt": "{{FrontSide}}<hr id=answer>{{Back}}",
                    "did": null,
                    "bqfmt": "",
                    "bafmt": ""
                }],
                "flds": [
                    {"name": "Front", "ord": 0, "sticky": false, "rtl": false, "font": "Arial", "size": 20, "media": []},
                    {"name": "Back", "ord": 1, "sticky": false, "rtl": false, "font": "Arial", "size": 20, "media": []}
                ],
                "css": ".card { font-family: arial; font-size: 20px; text-align: center; color: black; background-color: white; }",
                "latexPre": "",
                "latexPost": "",
                "latexsvg": false,
                "req": [[0, "all", [0]]]
            }
        });

        // Default deck config
        let dconf_json = serde_json::json!({
            "1": {
                "id": 1,
                "name": "Default",
                "replayq": true,
                "lapse": {"leechFails": 8, "minInt": 1, "delays": [10], "leechAction": 0, "mult": 0},
                "rev": {"perDay": 200, "fuzz": 0.05, "ivlFct": 1, "maxIvl": 36500, "ease4": 1.3, "bury": false, "hardFactor": 1.2},
                "new": {"perDay": 20, "delays": [1, 10], "separate": true, "ints": [1, 4, 7], "initialFactor": 2500, "bury": false, "order": 1},
                "maxTaken": 60,
                "timer": 0,
                "autoplay": true,
                "mod": 0,
                "usn": 0
            }
        });

        // Insert collection metadata
        conn.execute(
            "INSERT INTO col VALUES (1, ?, ?, ?, 11, 0, -1, 0, '{}', ?, ?, ?, '{}')",
            rusqlite::params![
                now,
                now,
                now_millis,
                models_json.to_string(),
                serde_json::Value::Object(decks_json).to_string(),
                dconf_json.to_string(),
            ],
        )?;

        // Insert notes and cards
        let mut note_id: i64 = now_millis;
        let mut card_id: i64 = now_millis;
        let mut total_cards = 0;

        for (deck_idx, deck) in decks_to_export.iter().enumerate() {
            let deck_id = (deck_idx as i64 + 2) * 1000000000000i64 + 1;

            for card in &deck.cards {
                note_id += 1;
                card_id += 1;

                // Fields separated by 0x1f
                let flds = format!("{}\x1f{}", card.front, card.back);
                let tags = card.tags.join(" ");

                // Simple checksum of front field
                let csum: i64 = card.front.bytes().map(|b| b as i64).sum::<i64>() % 2147483647;

                // Insert note
                conn.execute(
                    "INSERT INTO notes VALUES (?, ?, ?, ?, -1, ?, ?, ?, ?, 0, '')",
                    rusqlite::params![
                        note_id,
                        &card.id,  // guid
                        model_id,
                        now,
                        tags,
                        flds,
                        &card.front,  // sfld (sort field)
                        csum,
                    ],
                )?;

                // Determine card type and queue
                let (card_type, queue, due) = if card.repetitions == 0 {
                    (0, 0, note_id)  // New card
                } else if card.interval == 0 {
                    (1, 1, now)  // Learning
                } else {
                    // Review card - due is days since collection creation
                    let due_days = card.interval as i64;
                    (2, 2, due_days)
                };

                // Insert card with scheduling data
                conn.execute(
                    "INSERT INTO cards VALUES (?, ?, ?, 0, ?, -1, ?, ?, ?, ?, ?, ?, ?, 0, 0, 0, 0, '')",
                    rusqlite::params![
                        card_id,
                        note_id,
                        deck_id,
                        now,
                        card_type,
                        queue,
                        due,
                        card.interval as i64,
                        (card.ease_factor * 1000.0) as i64,
                        card.repetitions as i64,
                        card.lapses as i64,
                    ],
                )?;

                total_cards += 1;
            }
        }

        conn.close().map_err(|(_, e)| e)?;

        // Create the APKG (ZIP) file
        let apkg_file = File::create(path)
            .with_context(|| format!("Failed to create APKG file: {:?}", path))?;
        let mut zip = ZipWriter::new(apkg_file);

        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // Add the database
        zip.start_file("collection.anki2", options)?;
        let db_bytes = fs::read(&temp_db_path)?;
        zip.write_all(&db_bytes)?;

        // Add empty media file
        zip.start_file("media", options)?;
        zip.write_all(b"{}")?;

        zip.finish()?;

        // Clean up temp file
        let _ = fs::remove_file(&temp_db_path);

        Ok(total_cards)
    }

    /// Auto-detect Anki format and import.
    /// Returns the imported decks.
    pub fn import_anki(&self, path: &Path, deck_name: Option<&str>) -> Result<Vec<Deck>> {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        match extension.as_deref() {
            Some("apkg") => self.import_apkg(path),
            Some("txt") | Some("tsv") => {
                let name = deck_name.unwrap_or_else(|| {
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Imported Deck")
                });
                let deck = self.import_anki_text(path, name)?;
                Ok(vec![deck])
            }
            _ => {
                // Try to detect format from content
                let content = fs::read_to_string(path)?;
                if content.contains('\t') || content.contains(';') {
                    let name = deck_name.unwrap_or("Imported Deck");
                    let deck = self.import_anki_text(path, name)?;
                    Ok(vec![deck])
                } else {
                    anyhow::bail!(
                        "Unknown file format. Expected .apkg, .txt, or .tsv file."
                    )
                }
            }
        }
    }
}

/// Strip HTML tags from a string (basic implementation).
fn strip_html(s: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }

    // Also decode common HTML entities
    result
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .trim()
        .to_string()
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
