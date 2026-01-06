//! Main application state and logic.

use std::time::Instant;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{block::BorderType, Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::theme::Theme;
use super::widgets::{CompletionScreen, FlashcardWidget, KeyHints, Logo, RatingButtons, StatsBar};
use crate::config::Config;
use crate::models::{Deck, ReviewRating};
use crate::sm2::Scheduler;
use crate::storage::{DeckInfo, DeckStorage};

// ══════════════════════════════════════════════════════════════════════════
// Application State
// ══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    DeckSelect,
    Study,
    AddCard,
    CardBrowser,
    Stats,
    Complete,
}

pub struct App {
    pub screen: Screen,
    pub running: bool,

    // Config and theme
    pub config: Config,
    pub theme: Theme,

    // Storage
    pub storage: DeckStorage,
    pub scheduler: Scheduler,

    // Deck selection
    pub deck_list: Vec<DeckInfo>,
    pub deck_list_state: ListState,

    // Current deck
    pub current_deck: Option<Deck>,

    // Study state
    pub study_queue: Vec<usize>,  // Indices into deck.cards
    pub current_card_idx: Option<usize>,
    pub showing_answer: bool,
    pub cards_studied: usize,
    pub session_start: Option<Instant>,
    pub interval_preview: [(ReviewRating, String); 4],

    // Add card state
    pub add_card_front: String,
    pub add_card_back: String,
    pub add_card_focus: usize,  // 0 = front, 1 = back

    // Card browser state
    pub card_list_state: ListState,
    pub card_edit_mode: bool,
    pub card_edit_front: String,
    pub card_edit_back: String,
    pub card_edit_focus: usize,  // 0 = front, 1 = back
    pub card_delete_pending: bool,

    // Status message (shown temporarily)
    pub status_message: Option<(String, Instant)>,
}

impl App {
    pub fn new(storage: DeckStorage, config: Config) -> Self {
        let deck_list = storage.list_decks().unwrap_or_default();
        let theme = Theme::from_name(&config.theme);

        Self {
            screen: Screen::DeckSelect,
            running: true,
            config,
            theme,
            storage,
            scheduler: Scheduler::new(),
            deck_list,
            deck_list_state: ListState::default().with_selected(Some(0)),
            current_deck: None,
            study_queue: Vec::new(),
            current_card_idx: None,
            showing_answer: false,
            cards_studied: 0,
            session_start: None,
            interval_preview: [
                (ReviewRating::Again, String::new()),
                (ReviewRating::Hard, String::new()),
                (ReviewRating::Good, String::new()),
                (ReviewRating::Easy, String::new()),
            ],
            add_card_front: String::new(),
            add_card_back: String::new(),
            add_card_focus: 0,
            // Card browser
            card_list_state: ListState::default(),
            card_edit_mode: false,
            card_edit_front: String::new(),
            card_edit_back: String::new(),
            card_edit_focus: 0,
            card_delete_pending: false,
            // Status
            status_message: None,
        }
    }

    pub fn delete_selected_deck(&mut self) {
        if let Some(i) = self.deck_list_state.selected() {
            if let Some(deck_info) = self.deck_list.get(i) {
                let deck_id = deck_info.id.clone();
                let _ = self.storage.delete_deck(&deck_id);
                self.refresh_deck_list();
                // Adjust selection if needed
                if i >= self.deck_list.len() && !self.deck_list.is_empty() {
                    self.deck_list_state.select(Some(self.deck_list.len() - 1));
                } else if self.deck_list.is_empty() {
                    self.deck_list_state.select(None);
                }
            }
        }
    }

    pub fn cycle_theme(&mut self) {
        let new_theme_name = self.theme.name.next();
        self.theme = Theme::new(new_theme_name);
        self.config.theme = new_theme_name.as_str().to_string();
        let _ = self.config.save();
    }

    pub fn refresh_deck_list(&mut self) {
        self.deck_list = self.storage.list_decks().unwrap_or_default();
    }

    pub fn select_deck(&mut self, deck_id: &str) {
        if let Ok(Some(deck)) = self.storage.load_deck(deck_id) {
            let stats = deck.get_stats();
            self.current_deck = Some(deck);

            if stats.total_cards == 0 || (stats.new_cards == 0 && stats.due_cards == 0) {
                self.screen = Screen::AddCard;
            } else {
                self.start_study();
            }
        }
    }

    pub fn start_study(&mut self) {
        if let Some(ref deck) = self.current_deck {
            // Build study queue
            self.study_queue.clear();

            // Add due cards first
            for (i, card) in deck.cards.iter().enumerate() {
                if card.is_due() && !card.is_new() {
                    self.study_queue.push(i);
                }
            }

            // Add new cards (limit to 20)
            let mut new_count = 0;
            for (i, card) in deck.cards.iter().enumerate() {
                if card.is_new() && new_count < 20 {
                    self.study_queue.push(i);
                    new_count += 1;
                }
            }

            self.cards_studied = 0;
            self.session_start = Some(Instant::now());
            self.screen = Screen::Study;

            self.next_card();
        }
    }

    pub fn next_card(&mut self) {
        if self.study_queue.is_empty() {
            self.screen = Screen::Complete;
            return;
        }

        self.current_card_idx = Some(self.study_queue.remove(0));
        self.showing_answer = false;

        // Update interval preview
        if let (Some(deck), Some(idx)) = (&self.current_deck, self.current_card_idx) {
            self.interval_preview = self.scheduler.preview_intervals(&deck.cards[idx]);
        }
    }

    pub fn show_answer(&mut self) {
        self.showing_answer = true;
    }

    pub fn rate_card(&mut self, rating: ReviewRating) {
        if !self.showing_answer {
            return;
        }

        if let (Some(ref mut deck), Some(idx)) = (&mut self.current_deck, self.current_card_idx) {
            self.scheduler.review_card(&mut deck.cards[idx], rating);
            self.cards_studied += 1;

            // If failed, add back to queue
            if rating == ReviewRating::Again {
                self.study_queue.push(idx);
            }

            // Save deck
            let _ = self.storage.save_deck(deck);

            self.next_card();
        }
    }

    pub fn add_card(&mut self) {
        if self.add_card_front.is_empty() || self.add_card_back.is_empty() {
            return;
        }

        if let Some(ref mut deck) = self.current_deck {
            deck.add_card(self.add_card_front.clone(), self.add_card_back.clone());
            let _ = self.storage.save_deck(deck);

            self.add_card_front.clear();
            self.add_card_back.clear();
            self.add_card_focus = 0;
        }
    }

    pub fn create_new_deck(&mut self, name: &str) {
        let deck = Deck::new(name.to_string());
        let _ = self.storage.save_deck(&deck);
        self.refresh_deck_list();
    }

    pub fn set_status(&mut self, message: String) {
        self.status_message = Some((message, Instant::now()));
    }

    pub fn export_backup(&mut self) {
        let path = DeckStorage::default_backup_path();
        match self.storage.export_backup(&path) {
            Ok(count) => {
                self.set_status(format!("Exported {} decks to {}", count, path.display()));
            }
            Err(e) => {
                self.set_status(format!("Export failed: {}", e));
            }
        }
    }

    pub fn import_backup(&mut self, path: &std::path::Path) {
        match self.storage.import_backup(path) {
            Ok((imported, skipped)) => {
                self.refresh_deck_list();
                if skipped > 0 {
                    self.set_status(format!("Imported {} decks ({} skipped - already exist)", imported, skipped));
                } else {
                    self.set_status(format!("Imported {} decks", imported));
                }
            }
            Err(e) => {
                self.set_status(format!("Import failed: {}", e));
            }
        }
    }

    pub fn enter_card_browser(&mut self) {
        if let Some(ref deck) = self.current_deck {
            if !deck.cards.is_empty() {
                self.card_list_state = ListState::default().with_selected(Some(0));
            } else {
                self.card_list_state = ListState::default();
            }
            self.card_edit_mode = false;
            self.card_delete_pending = false;
            self.screen = Screen::CardBrowser;
        }
    }

    pub fn browse_selected_deck(&mut self) {
        if let Some(i) = self.deck_list_state.selected() {
            if let Some(deck_info) = self.deck_list.get(i) {
                if let Ok(Some(deck)) = self.storage.load_deck(&deck_info.id) {
                    self.current_deck = Some(deck);
                    self.enter_card_browser();
                }
            }
        }
    }

    pub fn start_edit_card(&mut self) {
        if let Some(i) = self.card_list_state.selected() {
            if let Some(ref deck) = self.current_deck {
                if let Some(card) = deck.cards.get(i) {
                    self.card_edit_front = card.front.clone();
                    self.card_edit_back = card.back.clone();
                    self.card_edit_focus = 0;
                    self.card_edit_mode = true;
                    self.card_delete_pending = false;
                }
            }
        }
    }

    pub fn save_card_edit(&mut self) {
        if let Some(i) = self.card_list_state.selected() {
            if let Some(ref mut deck) = self.current_deck {
                if let Some(card) = deck.cards.get(i) {
                    let card_id = card.id.clone();
                    deck.update_card(&card_id, self.card_edit_front.clone(), self.card_edit_back.clone());
                    let _ = self.storage.save_deck(deck);
                }
            }
        }
        self.card_edit_mode = false;
        self.card_edit_front.clear();
        self.card_edit_back.clear();
    }

    pub fn cancel_card_edit(&mut self) {
        self.card_edit_mode = false;
        self.card_edit_front.clear();
        self.card_edit_back.clear();
    }

    pub fn delete_selected_card(&mut self) {
        if let Some(i) = self.card_list_state.selected() {
            if let Some(ref mut deck) = self.current_deck {
                if let Some(card) = deck.cards.get(i) {
                    let card_id = card.id.clone();
                    deck.delete_card(&card_id);
                    let _ = self.storage.save_deck(deck);

                    // Adjust selection
                    if deck.cards.is_empty() {
                        self.card_list_state.select(None);
                    } else if i >= deck.cards.len() {
                        self.card_list_state.select(Some(deck.cards.len() - 1));
                    }
                }
            }
        }
        self.card_delete_pending = false;
    }

    // ══════════════════════════════════════════════════════════════════════
    // Event Handling
    // ══════════════════════════════════════════════════════════════════════

    pub fn handle_events(&mut self) -> anyhow::Result<()> {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    return Ok(());
                }

                match self.screen {
                    Screen::DeckSelect => self.handle_deck_select_keys(key.code),
                    Screen::Study => self.handle_study_keys(key.code),
                    Screen::AddCard => self.handle_add_card_keys(key.code),
                    Screen::CardBrowser => self.handle_card_browser_keys(key.code),
                    Screen::Stats => self.handle_stats_keys(key.code),
                    Screen::Complete => self.handle_complete_keys(key.code),
                }
            }
        }
        Ok(())
    }

    fn handle_deck_select_keys(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.running = false,
            KeyCode::Char('t') => self.cycle_theme(),
            KeyCode::Char('d') | KeyCode::Char('D') => self.delete_selected_deck(),
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.deck_list_state.selected().unwrap_or(0);
                let new_i = if i == 0 {
                    self.deck_list.len().saturating_sub(1)
                } else {
                    i - 1
                };
                self.deck_list_state.select(Some(new_i));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let i = self.deck_list_state.selected().unwrap_or(0);
                let new_i = if i >= self.deck_list.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                };
                self.deck_list_state.select(Some(new_i));
            }
            KeyCode::Enter => {
                if let Some(i) = self.deck_list_state.selected() {
                    if let Some(deck_info) = self.deck_list.get(i) {
                        let deck_id = deck_info.id.clone();
                        self.select_deck(&deck_id);
                    }
                }
            }
            KeyCode::Char('n') => {
                // Quick create a new deck (for demo)
                self.create_new_deck("New Deck");
            }
            KeyCode::Char('b') => {
                self.browse_selected_deck();
            }
            KeyCode::Char('x') => {
                self.export_backup();
            }
            KeyCode::Char('s') => {
                self.screen = Screen::Stats;
            }
            _ => {}
        }
    }

    fn handle_study_keys(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.screen = Screen::DeckSelect;
                self.current_deck = None;
            }
            KeyCode::Char('t') => self.cycle_theme(),
            KeyCode::Char(' ') => {
                if !self.showing_answer {
                    self.show_answer();
                }
            }
            KeyCode::Char('1') => self.rate_card(ReviewRating::Again),
            KeyCode::Char('2') => self.rate_card(ReviewRating::Hard),
            KeyCode::Char('3') => self.rate_card(ReviewRating::Good),
            KeyCode::Char('4') => self.rate_card(ReviewRating::Easy),
            KeyCode::Char('a') => {
                self.screen = Screen::AddCard;
            }
            KeyCode::Char('b') => {
                self.enter_card_browser();
            }
            _ => {}
        }
    }

    fn handle_add_card_keys(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                if let Some(ref deck) = self.current_deck {
                    if deck.cards.is_empty() {
                        self.screen = Screen::DeckSelect;
                        self.current_deck = None;
                    } else {
                        self.start_study();
                    }
                } else {
                    self.screen = Screen::DeckSelect;
                }
            }
            KeyCode::Tab => {
                self.add_card_focus = (self.add_card_focus + 1) % 2;
            }
            KeyCode::Enter => {
                if self.add_card_focus == 0 {
                    self.add_card_focus = 1;
                } else {
                    self.add_card();
                }
            }
            KeyCode::Char(c) => {
                if self.add_card_focus == 0 {
                    self.add_card_front.push(c);
                } else {
                    self.add_card_back.push(c);
                }
            }
            KeyCode::Backspace => {
                if self.add_card_focus == 0 {
                    self.add_card_front.pop();
                } else {
                    self.add_card_back.pop();
                }
            }
            _ => {}
        }
    }

    fn handle_complete_keys(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                self.screen = Screen::DeckSelect;
                self.current_deck = None;
                self.refresh_deck_list();
            }
            _ => {}
        }
    }

    fn handle_stats_keys(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.screen = Screen::DeckSelect;
            }
            KeyCode::Char('t') => self.cycle_theme(),
            _ => {}
        }
    }

    fn handle_card_browser_keys(&mut self, key: KeyCode) {
        if self.card_edit_mode {
            // Edit mode
            match key {
                KeyCode::Esc => {
                    self.cancel_card_edit();
                }
                KeyCode::Tab => {
                    self.card_edit_focus = (self.card_edit_focus + 1) % 2;
                }
                KeyCode::Enter => {
                    self.save_card_edit();
                }
                KeyCode::Char(c) => {
                    if self.card_edit_focus == 0 {
                        self.card_edit_front.push(c);
                    } else {
                        self.card_edit_back.push(c);
                    }
                }
                KeyCode::Backspace => {
                    if self.card_edit_focus == 0 {
                        self.card_edit_front.pop();
                    } else {
                        self.card_edit_back.pop();
                    }
                }
                _ => {}
            }
        } else {
            // Browse mode
            match key {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.screen = Screen::DeckSelect;
                    self.current_deck = None;
                    self.refresh_deck_list();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.card_delete_pending = false;
                    if let Some(ref deck) = self.current_deck {
                        if !deck.cards.is_empty() {
                            let i = self.card_list_state.selected().unwrap_or(0);
                            let new_i = if i == 0 { deck.cards.len() - 1 } else { i - 1 };
                            self.card_list_state.select(Some(new_i));
                        }
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.card_delete_pending = false;
                    if let Some(ref deck) = self.current_deck {
                        if !deck.cards.is_empty() {
                            let i = self.card_list_state.selected().unwrap_or(0);
                            let new_i = if i >= deck.cards.len() - 1 { 0 } else { i + 1 };
                            self.card_list_state.select(Some(new_i));
                        }
                    }
                }
                KeyCode::Char('e') => {
                    self.card_delete_pending = false;
                    self.start_edit_card();
                }
                KeyCode::Char('d') => {
                    if self.card_delete_pending {
                        self.delete_selected_card();
                    } else {
                        self.card_delete_pending = true;
                    }
                }
                KeyCode::Char('a') => {
                    self.card_delete_pending = false;
                    self.screen = Screen::AddCard;
                }
                KeyCode::Char('t') => {
                    self.card_delete_pending = false;
                    self.cycle_theme();
                }
                _ => {
                    self.card_delete_pending = false;
                }
            }
        }
    }

    // ══════════════════════════════════════════════════════════════════════
    // Rendering
    // ══════════════════════════════════════════════════════════════════════

    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Clear with background
        frame.render_widget(Clear, area);
        frame.render_widget(
            Block::default().style(Style::default().bg(self.theme.colors.bg_dark)),
            area,
        );

        match self.screen {
            Screen::DeckSelect => self.render_deck_select(frame, area),
            Screen::Study => self.render_study(frame, area),
            Screen::AddCard => self.render_add_card(frame, area),
            Screen::CardBrowser => self.render_card_browser(frame, area),
            Screen::Stats => self.render_stats(frame, area),
            Screen::Complete => self.render_complete(frame, area),
        }
    }

    fn render_deck_select(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(2),   // Top padding
            Constraint::Length(7),   // Logo
            Constraint::Length(2),   // Spacing
            Constraint::Min(5),      // Deck list
            Constraint::Length(3),   // Help
        ])
        .split(area);

        // Logo
        Logo::render_to(&self.theme, chunks[1], frame.buffer_mut());

        // Deck list
        let list_area = centered_rect(60, 100, chunks[3]);

        let items: Vec<ListItem> = self
            .deck_list
            .iter()
            .map(|deck| {
                let content = Line::from(vec![
                    Span::styled(&deck.name, Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!(" ({} cards)", deck.card_count),
                        Style::default().fg(self.theme.colors.text_muted),
                    ),
                ]);
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(self.theme.colors.primary))
                    .title(" Decks ")
                    .title_style(self.theme.highlight()),
            )
            .highlight_style(self.theme.selected())
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, list_area, &mut self.deck_list_state);

        // Key hints with theme indicator
        let theme_hint = format!("[{}]", self.theme.name.display_name());
        let hints_data: [(&str, &str); 9] = [
            ("j/k", "nav"),
            ("Enter", "study"),
            ("b", "browse"),
            ("n", "new"),
            ("d", "del"),
            ("x", "export"),
            ("s", "stats"),
            ("t", &theme_hint),
            ("q", "quit"),
        ];
        let hints = KeyHints::new(&hints_data, &self.theme);
        frame.render_widget(hints, chunks[4]);

        // Show status message if recent (within 5 seconds)
        if let Some((ref msg, time)) = self.status_message {
            if time.elapsed().as_secs() < 5 {
                let status = Paragraph::new(msg.as_str())
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(self.theme.colors.success));
                // Render above the hints
                let status_area = Rect {
                    x: chunks[4].x,
                    y: chunks[4].y.saturating_sub(1),
                    width: chunks[4].width,
                    height: 1,
                };
                frame.render_widget(status, status_area);
            }
        }
    }

    fn render_study(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(3),   // Header
            Constraint::Length(1),   // Stats
            Constraint::Length(1),   // Separator
            Constraint::Min(10),     // Card
            Constraint::Length(1),   // Separator
            Constraint::Length(5),   // Buttons
            Constraint::Length(2),   // Hints
        ])
        .split(area);

        // Header with deck name
        if let Some(ref deck) = self.current_deck {
            let header = Paragraph::new(Line::from(vec![
                Span::styled(&deck.name, self.theme.title()),
            ]))
            .alignment(Alignment::Center);
            frame.render_widget(header, chunks[0]);

            // Stats bar
            let stats = deck.get_stats();
            frame.render_widget(StatsBar::new(stats, &self.theme), chunks[1]);
        }

        // Card display
        let card_area = centered_rect(80, 100, chunks[3]);

        if let (Some(ref deck), Some(idx)) = (&self.current_deck, self.current_card_idx) {
            let card = &deck.cards[idx];
            let (content, is_front) = if self.showing_answer {
                (&card.back, false)
            } else {
                (&card.front, true)
            };

            frame.render_widget(
                FlashcardWidget::new(content, is_front, &self.theme),
                card_area,
            );
        }

        // Rating buttons
        let buttons_area = centered_rect(90, 100, chunks[5]);
        frame.render_widget(
            RatingButtons::new(&self.interval_preview, self.showing_answer, &self.theme),
            buttons_area,
        );

        // Key hints
        let hints = if self.showing_answer {
            KeyHints::new(&[
                ("1", "Again"),
                ("2", "Hard"),
                ("3", "Good"),
                ("4", "Easy"),
                ("Esc", "quit"),
            ], &self.theme)
        } else {
            KeyHints::new(&[
                ("Space", "show answer"),
                ("a", "add"),
                ("b", "browse"),
                ("Esc", "quit"),
            ], &self.theme)
        };
        frame.render_widget(hints, chunks[6]);
    }

    fn render_add_card(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(3),   // Title
            Constraint::Length(1),   // Spacing
            Constraint::Length(3),   // Front label + input
            Constraint::Length(1),   // Spacing
            Constraint::Length(3),   // Back label + input
            Constraint::Length(2),   // Spacing
            Constraint::Length(3),   // Button
            Constraint::Min(1),      // Spacer
            Constraint::Length(2),   // Hints
        ])
        .split(centered_rect(60, 100, area));

        // Title
        let deck_name = self
            .current_deck
            .as_ref()
            .map(|d| d.name.as_str())
            .unwrap_or("Deck");
        let title = Paragraph::new(format!("Add Card to {}", deck_name))
            .alignment(Alignment::Center)
            .style(self.theme.title());
        frame.render_widget(title, chunks[0]);

        // Front input
        let front_style = if self.add_card_focus == 0 {
            Style::default().fg(self.theme.colors.accent)
        } else {
            Style::default().fg(self.theme.colors.text_muted)
        };
        let front = Paragraph::new(self.add_card_front.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(front_style)
                    .title(" Front (Question) ")
                    .title_style(front_style),
            );
        frame.render_widget(front, chunks[2]);

        // Back input
        let back_style = if self.add_card_focus == 1 {
            Style::default().fg(self.theme.colors.accent)
        } else {
            Style::default().fg(self.theme.colors.text_muted)
        };
        let back = Paragraph::new(self.add_card_back.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(back_style)
                    .title(" Back (Answer) ")
                    .title_style(back_style),
            );
        frame.render_widget(back, chunks[4]);

        // Card count
        let count = self
            .current_deck
            .as_ref()
            .map(|d| d.cards.len())
            .unwrap_or(0);
        let status = Paragraph::new(format!("Cards: {}", count))
            .alignment(Alignment::Center)
            .style(Style::default().fg(self.theme.colors.text_muted));
        frame.render_widget(status, chunks[6]);

        // Hints
        let hints = KeyHints::new(&[
            ("Tab", "switch field"),
            ("Enter", "add card"),
            ("Esc", "done"),
        ], &self.theme);
        frame.render_widget(hints, chunks[8]);
    }

    fn render_card_browser(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(3),   // Header
            Constraint::Length(1),   // Spacing
            Constraint::Min(10),     // Main content
            Constraint::Length(2),   // Hints
        ])
        .split(area);

        // Header with deck name
        let deck_name = self
            .current_deck
            .as_ref()
            .map(|d| d.name.as_str())
            .unwrap_or("Cards");
        let title = Paragraph::new(format!("{} - Card Browser", deck_name))
            .alignment(Alignment::Center)
            .style(self.theme.title());
        frame.render_widget(title, chunks[0]);

        // Main content: split into list and detail
        let main_chunks = Layout::horizontal([
            Constraint::Percentage(35),  // Card list
            Constraint::Percentage(65),  // Card details
        ])
        .split(chunks[2]);

        // Card list
        if let Some(ref deck) = self.current_deck {
            let items: Vec<ListItem> = deck
                .cards
                .iter()
                .map(|card| {
                    let front_preview: String = card.front.chars().take(25).collect();
                    let status = if card.is_new() {
                        "(new)".to_string()
                    } else if card.is_due() {
                        "(due)".to_string()
                    } else if card.interval == 1 {
                        "(1d)".to_string()
                    } else {
                        format!("({}d)", card.interval)
                    };
                    let content = Line::from(vec![
                        Span::styled(
                            front_preview,
                            Style::default().fg(self.theme.colors.text),
                        ),
                        Span::styled(
                            format!(" {}", status),
                            Style::default().fg(self.theme.colors.text_muted),
                        ),
                    ]);
                    ListItem::new(content)
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(self.theme.colors.primary))
                        .title(" Cards ")
                        .title_style(self.theme.highlight()),
                )
                .highlight_style(self.theme.selected())
                .highlight_symbol("> ");

            frame.render_stateful_widget(list, main_chunks[0], &mut self.card_list_state);

            // Card details panel
            if let Some(idx) = self.card_list_state.selected() {
                if let Some(card) = deck.cards.get(idx) {
                    self.render_card_details(frame, main_chunks[1], card);
                }
            }
        }

        // Key hints
        let hints = if self.card_edit_mode {
            KeyHints::new(&[
                ("Tab", "switch"),
                ("Enter", "save"),
                ("Esc", "cancel"),
            ], &self.theme)
        } else if self.card_delete_pending {
            KeyHints::new(&[
                ("d", "confirm delete"),
                ("any", "cancel"),
            ], &self.theme)
        } else {
            KeyHints::new(&[
                ("j/k", "nav"),
                ("e", "edit"),
                ("d", "delete"),
                ("a", "add"),
                ("Esc", "back"),
            ], &self.theme)
        };
        frame.render_widget(hints, chunks[3]);
    }

    fn render_card_details(&self, frame: &mut Frame, area: Rect, card: &crate::models::Card) {
        let chunks = Layout::vertical([
            Constraint::Length(5),   // Front
            Constraint::Length(1),   // Spacing
            Constraint::Length(5),   // Back
            Constraint::Length(1),   // Spacing
            Constraint::Min(5),      // Metadata
        ])
        .split(area);

        if self.card_edit_mode {
            // Edit mode - show editable fields with real blinking cursor
            let front_style = if self.card_edit_focus == 0 {
                Style::default().fg(self.theme.colors.accent)
            } else {
                Style::default().fg(self.theme.colors.text_muted)
            };
            let front = Paragraph::new(self.card_edit_front.as_str())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(front_style)
                        .title(" Front (editing) ")
                        .title_style(front_style),
                );
            frame.render_widget(front, chunks[0]);

            // Set cursor position for front field (accounting for wrap)
            if self.card_edit_focus == 0 {
                let inner_width = chunks[0].width.saturating_sub(2) as usize; // -2 for borders
                let text_len = self.card_edit_front.chars().count();
                let (cursor_x, cursor_y) = if inner_width > 0 {
                    let row = text_len / inner_width;
                    let col = text_len % inner_width;
                    (chunks[0].x + 1 + col as u16, chunks[0].y + 1 + row as u16)
                } else {
                    (chunks[0].x + 1, chunks[0].y + 1)
                };
                frame.set_cursor_position((cursor_x, cursor_y));
            }

            let back_style = if self.card_edit_focus == 1 {
                Style::default().fg(self.theme.colors.accent)
            } else {
                Style::default().fg(self.theme.colors.text_muted)
            };
            let back = Paragraph::new(self.card_edit_back.as_str())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(back_style)
                        .title(" Back (editing) ")
                        .title_style(back_style),
                );
            frame.render_widget(back, chunks[2]);

            // Set cursor position for back field (accounting for wrap)
            if self.card_edit_focus == 1 {
                let inner_width = chunks[2].width.saturating_sub(2) as usize; // -2 for borders
                let text_len = self.card_edit_back.chars().count();
                let (cursor_x, cursor_y) = if inner_width > 0 {
                    let row = text_len / inner_width;
                    let col = text_len % inner_width;
                    (chunks[2].x + 1 + col as u16, chunks[2].y + 1 + row as u16)
                } else {
                    (chunks[2].x + 1, chunks[2].y + 1)
                };
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        } else {
            // View mode
            let front = Paragraph::new(card.front.as_str())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(self.theme.colors.accent))
                        .title(" Front ")
                        .title_style(Style::default().fg(self.theme.colors.accent)),
                )
                .wrap(ratatui::widgets::Wrap { trim: true });
            frame.render_widget(front, chunks[0]);

            let back = Paragraph::new(card.back.as_str())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(self.theme.colors.success))
                        .title(" Back ")
                        .title_style(Style::default().fg(self.theme.colors.success)),
                )
                .wrap(ratatui::widgets::Wrap { trim: true });
            frame.render_widget(back, chunks[2]);
        }

        // Metadata
        let due_str = match card.due_date {
            None => "New card".to_string(),
            Some(due) => {
                let now = chrono::Local::now();
                let diff = due.signed_duration_since(now);
                let days = diff.num_days();
                if days < 0 {
                    format!("Overdue by {} days", -days)
                } else if days == 0 {
                    "Due today".to_string()
                } else if days == 1 {
                    "Due tomorrow".to_string()
                } else {
                    format!("Due in {} days", days)
                }
            }
        };

        let metadata = vec![
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(self.theme.colors.text_muted)),
                Span::styled(&due_str, Style::default().fg(self.theme.colors.primary)),
            ]),
            Line::from(vec![
                Span::styled("Interval: ", Style::default().fg(self.theme.colors.text_muted)),
                Span::styled(format!("{} days", card.interval), Style::default().fg(self.theme.colors.text)),
            ]),
            Line::from(vec![
                Span::styled("Ease: ", Style::default().fg(self.theme.colors.text_muted)),
                Span::styled(format!("{:.2}", card.ease_factor), Style::default().fg(self.theme.colors.text)),
            ]),
            Line::from(vec![
                Span::styled("Reviews: ", Style::default().fg(self.theme.colors.text_muted)),
                Span::styled(card.total_reviews.to_string(), Style::default().fg(self.theme.colors.text)),
            ]),
            Line::from(vec![
                Span::styled("Lapses: ", Style::default().fg(self.theme.colors.text_muted)),
                Span::styled(card.lapses.to_string(), Style::default().fg(self.theme.colors.rating_again)),
            ]),
        ];

        let metadata_block = Paragraph::new(metadata)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(self.theme.colors.text_dim))
                    .title(" Stats ")
                    .title_style(Style::default().fg(self.theme.colors.text_muted)),
            );
        frame.render_widget(metadata_block, chunks[4]);
    }

    fn render_stats(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(3),   // Title
            Constraint::Length(1),   // Spacing
            Constraint::Min(10),     // Stats content
            Constraint::Length(2),   // Hints
        ])
        .split(area);

        // Title
        let title = Paragraph::new("Stats")
            .alignment(Alignment::Center)
            .style(self.theme.title());
        frame.render_widget(title, chunks[0]);

        // Calculate aggregate stats from all decks
        let mut total_reviews: u32 = 0;
        let mut total_cards: usize = 0;
        let mut review_dates: Vec<chrono::NaiveDate> = Vec::new();
        let mut ease_counts = EaseLevelCounts::default();

        for deck_info in &self.deck_list {
            if let Ok(Some(deck)) = self.storage.load_deck(&deck_info.id) {
                for card in &deck.cards {
                    total_cards += 1;
                    total_reviews += card.total_reviews;

                    // Collect review dates for streak calculation
                    if let Some(reviewed) = card.last_reviewed {
                        review_dates.push(reviewed.date_naive());
                    }

                    // Categorize by ease factor
                    if card.is_new() {
                        ease_counts.new += 1;
                    } else if card.ease_factor >= 2.5 {
                        ease_counts.easy += 1;
                    } else if card.ease_factor >= 2.0 {
                        ease_counts.good += 1;
                    } else if card.ease_factor >= 1.5 {
                        ease_counts.hard += 1;
                    } else {
                        ease_counts.struggling += 1;
                    }
                }
            }
        }

        // Calculate streaks
        let (daily_streak, weekly_streak) = calculate_streaks(&review_dates);

        // Main content area
        let content_area = centered_rect(70, 100, chunks[2]);
        let stat_chunks = Layout::vertical([
            Constraint::Length(7),   // Overview stats
            Constraint::Length(1),   // Spacing
            Constraint::Min(8),      // Ease breakdown
        ])
        .split(content_area);

        // Overview stats
        let overview_lines = vec![
            Line::from(vec![
                Span::styled("Total Cards: ", Style::default().fg(self.theme.colors.text_muted)),
                Span::styled(total_cards.to_string(), Style::default().fg(self.theme.colors.primary).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Total Reviews: ", Style::default().fg(self.theme.colors.text_muted)),
                Span::styled(total_reviews.to_string(), Style::default().fg(self.theme.colors.primary).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Daily Streak: ", Style::default().fg(self.theme.colors.text_muted)),
                Span::styled(
                    format!("{} day{}", daily_streak, if daily_streak == 1 { "" } else { "s" }),
                    Style::default().fg(if daily_streak > 0 { self.theme.colors.success } else { self.theme.colors.text_dim }),
                ),
            ]),
            Line::from(vec![
                Span::styled("Weekly Streak: ", Style::default().fg(self.theme.colors.text_muted)),
                Span::styled(
                    format!("{} week{}", weekly_streak, if weekly_streak == 1 { "" } else { "s" }),
                    Style::default().fg(if weekly_streak > 0 { self.theme.colors.success } else { self.theme.colors.text_dim }),
                ),
            ]),
        ];

        let overview = Paragraph::new(overview_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(self.theme.colors.primary))
                    .title(" Overview ")
                    .title_style(self.theme.highlight()),
            );
        frame.render_widget(overview, stat_chunks[0]);

        // Ease level breakdown
        let ease_lines = vec![
            Line::from(vec![
                Span::styled("New: ", Style::default().fg(self.theme.colors.text_muted)),
                Span::styled(ease_counts.new.to_string(), Style::default().fg(self.theme.colors.accent)),
                Span::styled(" cards not yet studied", Style::default().fg(self.theme.colors.text_dim)),
            ]),
            Line::from(vec![
                Span::styled("Easy: ", Style::default().fg(self.theme.colors.text_muted)),
                Span::styled(ease_counts.easy.to_string(), Style::default().fg(self.theme.colors.rating_easy)),
                Span::styled(" cards (ease >= 2.5)", Style::default().fg(self.theme.colors.text_dim)),
            ]),
            Line::from(vec![
                Span::styled("Good: ", Style::default().fg(self.theme.colors.text_muted)),
                Span::styled(ease_counts.good.to_string(), Style::default().fg(self.theme.colors.rating_good)),
                Span::styled(" cards (ease 2.0-2.5)", Style::default().fg(self.theme.colors.text_dim)),
            ]),
            Line::from(vec![
                Span::styled("Hard: ", Style::default().fg(self.theme.colors.text_muted)),
                Span::styled(ease_counts.hard.to_string(), Style::default().fg(self.theme.colors.rating_hard)),
                Span::styled(" cards (ease 1.5-2.0)", Style::default().fg(self.theme.colors.text_dim)),
            ]),
            Line::from(vec![
                Span::styled("Struggling: ", Style::default().fg(self.theme.colors.text_muted)),
                Span::styled(ease_counts.struggling.to_string(), Style::default().fg(self.theme.colors.rating_again)),
                Span::styled(" cards (ease < 1.5)", Style::default().fg(self.theme.colors.text_dim)),
            ]),
        ];

        let ease_block = Paragraph::new(ease_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(self.theme.colors.accent))
                    .title(" Cards by Difficulty ")
                    .title_style(Style::default().fg(self.theme.colors.accent)),
            );
        frame.render_widget(ease_block, stat_chunks[2]);

        // Key hints
        let hints = KeyHints::new(&[
            ("t", "theme"),
            ("Esc", "back"),
        ], &self.theme);
        frame.render_widget(hints, chunks[3]);
    }

    fn render_complete(&mut self, frame: &mut Frame, area: Rect) {
        let card_area = centered_rect(50, 40, area);

        let duration_mins = self
            .session_start
            .map(|s| s.elapsed().as_secs() / 60)
            .unwrap_or(0);

        frame.render_widget(
            CompletionScreen::new(self.cards_studied, duration_mins, &self.theme),
            card_area,
        );
    }
}

// ══════════════════════════════════════════════════════════════════════════
// Helper Functions
// ══════════════════════════════════════════════════════════════════════════

/// Create a centered rectangle.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}

/// Counts of cards at each ease level.
#[derive(Default)]
struct EaseLevelCounts {
    new: usize,
    easy: usize,
    good: usize,
    hard: usize,
    struggling: usize,
}

/// Calculate daily and weekly streaks from review dates.
fn calculate_streaks(review_dates: &[chrono::NaiveDate]) -> (u32, u32) {
    use chrono::Datelike;
    use std::collections::HashSet;

    if review_dates.is_empty() {
        return (0, 0);
    }

    let today = chrono::Local::now().date_naive();
    let unique_dates: HashSet<_> = review_dates.iter().cloned().collect();

    // Daily streak: consecutive days ending today or yesterday
    let mut daily_streak = 0u32;
    let mut check_date = today;

    // Allow starting from yesterday if no reviews today
    if !unique_dates.contains(&today) {
        check_date = today - chrono::Duration::days(1);
        if !unique_dates.contains(&check_date) {
            // No reviews today or yesterday, streak is 0
            check_date = today; // Reset so the loop doesn't count anything
        }
    }

    while unique_dates.contains(&check_date) {
        daily_streak += 1;
        check_date -= chrono::Duration::days(1);
    }

    // Weekly streak: consecutive weeks with at least one review
    // A week is Mon-Sun, count weeks ending with current or previous week
    let mut weekly_streak = 0u32;

    // Get the Monday of current week
    let days_since_monday = today.weekday().num_days_from_monday();
    let mut week_start = today - chrono::Duration::days(days_since_monday as i64);

    // Check if current week has reviews
    let current_week_has_reviews = (0..7).any(|d| {
        let day = week_start + chrono::Duration::days(d);
        unique_dates.contains(&day)
    });

    if !current_week_has_reviews {
        // Check previous week
        week_start -= chrono::Duration::days(7);
    }

    // Count consecutive weeks
    loop {
        let week_has_reviews = (0..7).any(|d| {
            let day = week_start + chrono::Duration::days(d);
            unique_dates.contains(&day)
        });

        if week_has_reviews {
            weekly_streak += 1;
            week_start -= chrono::Duration::days(7);
        } else {
            break;
        }
    }

    (daily_streak, weekly_streak)
}
