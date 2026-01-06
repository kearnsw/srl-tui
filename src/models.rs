//! Data models for flashcards and decks.

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Rating for how well you remembered a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewRating {
    Again = 0, // Complete blackout
    Hard = 1,  // Serious difficulty
    Good = 2,  // Some hesitation
    Easy = 3,  // Perfect recall
}

impl ReviewRating {
    pub fn from_key(c: char) -> Option<Self> {
        match c {
            '1' => Some(Self::Again),
            '2' => Some(Self::Hard),
            '3' => Some(Self::Good),
            '4' => Some(Self::Easy),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Again => "Again",
            Self::Hard => "Hard",
            Self::Good => "Good",
            Self::Easy => "Easy",
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            Self::Again => Color::Red,
            Self::Hard => Color::Yellow,
            Self::Good => Color::Blue,
            Self::Easy => Color::Green,
        }
    }

    pub fn color_for_theme(&self, theme: &crate::ui::theme::Theme) -> ratatui::style::Color {
        match self {
            Self::Again => theme.colors.rating_again,
            Self::Hard => theme.colors.rating_hard,
            Self::Good => theme.colors.rating_good,
            Self::Easy => theme.colors.rating_easy,
        }
    }
}

/// A single flashcard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub front: String,
    pub back: String,

    // SM-2 fields
    pub ease_factor: f64,
    pub interval: u32,
    pub repetitions: u32,

    // Tracking
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_date: Option<DateTime<Local>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_reviewed: Option<DateTime<Local>>,
    pub total_reviews: u32,
    pub lapses: u32,

    // Metadata
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub notes: String,
    pub created_at: DateTime<Local>,
}

impl Card {
    pub fn new(front: String, back: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string()[..8].to_string(),
            front,
            back,
            ease_factor: 2.5,
            interval: 0,
            repetitions: 0,
            due_date: None,
            last_reviewed: None,
            total_reviews: 0,
            lapses: 0,
            tags: Vec::new(),
            notes: String::new(),
            created_at: Local::now(),
        }
    }

    pub fn is_new(&self) -> bool {
        self.repetitions == 0
    }

    pub fn is_due(&self) -> bool {
        match self.due_date {
            None => true,
            Some(due) => Local::now() >= due,
        }
    }
}

/// Statistics for a deck.
#[derive(Debug, Default)]
pub struct DeckStats {
    pub total_cards: usize,
    pub new_cards: usize,
    pub due_cards: usize,
    pub learning_cards: usize,
    pub mature_cards: usize,
}

/// A collection of flashcards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub cards: Vec<Card>,
    pub created_at: DateTime<Local>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_studied: Option<DateTime<Local>>,
}

impl Deck {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string()[..8].to_string(),
            name,
            description: String::new(),
            cards: Vec::new(),
            created_at: Local::now(),
            last_studied: None,
        }
    }

    pub fn add_card(&mut self, front: String, back: String) -> &Card {
        let card = Card::new(front, back);
        self.cards.push(card);
        self.cards.last().unwrap()
    }

    pub fn get_due_cards(&self) -> Vec<&Card> {
        self.cards.iter().filter(|c| c.is_due()).collect()
    }

    pub fn get_new_cards(&self) -> Vec<&Card> {
        self.cards.iter().filter(|c| c.is_new()).collect()
    }

    pub fn get_stats(&self) -> DeckStats {
        let mut stats = DeckStats {
            total_cards: self.cards.len(),
            ..Default::default()
        };

        for card in &self.cards {
            if card.is_new() {
                stats.new_cards += 1;
            } else if card.is_due() {
                stats.due_cards += 1;
            }

            if card.interval < 21 && !card.is_new() {
                stats.learning_cards += 1;
            } else if card.interval >= 21 {
                stats.mature_cards += 1;
            }
        }

        stats
    }
}
