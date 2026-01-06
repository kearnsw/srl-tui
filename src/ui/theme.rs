//! Theme and styling for the TUI.

use ratatui::style::{Color, Modifier, Style};

/// Color palette for a theme.
#[derive(Debug, Clone)]
pub struct ThemeColors {
    // Brand Colors
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,

    // Semantic Colors
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // Background Colors
    pub bg_dark: Color,
    pub bg_card: Color,
    pub bg_elevated: Color,
    pub bg_highlight: Color,

    // Text Colors
    pub text: Color,
    pub text_muted: Color,
    pub text_dim: Color,

    // Rating Colors
    pub rating_again: Color,
    pub rating_hard: Color,
    pub rating_good: Color,
    pub rating_easy: Color,
}

/// Available theme names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeName {
    Default,
    KanagawaWave,
}

impl ThemeName {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeName::Default => "default",
            ThemeName::KanagawaWave => "kanagawa-wave",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ThemeName::Default => "Default",
            ThemeName::KanagawaWave => "Kanagawa Wave",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "kanagawa-wave" | "kanagawa_wave" | "kanagawa" => ThemeName::KanagawaWave,
            _ => ThemeName::Default,
        }
    }

    pub fn all() -> &'static [ThemeName] {
        &[ThemeName::Default, ThemeName::KanagawaWave]
    }

    pub fn next(&self) -> Self {
        match self {
            ThemeName::Default => ThemeName::KanagawaWave,
            ThemeName::KanagawaWave => ThemeName::Default,
        }
    }
}

/// Theme struct that holds colors and provides style methods.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: ThemeName,
    pub colors: ThemeColors,
}

impl Theme {
    pub fn new(name: ThemeName) -> Self {
        let colors = match name {
            ThemeName::Default => Self::default_colors(),
            ThemeName::KanagawaWave => Self::kanagawa_wave_colors(),
        };
        Self { name, colors }
    }

    pub fn from_name(name: &str) -> Self {
        Self::new(ThemeName::from_str(name))
    }

    fn default_colors() -> ThemeColors {
        ThemeColors {
            // Brand Colors
            primary: Color::Rgb(99, 102, 241),      // Indigo
            secondary: Color::Rgb(139, 92, 246),    // Violet
            accent: Color::Rgb(236, 72, 153),       // Pink

            // Semantic Colors
            success: Color::Rgb(34, 197, 94),       // Green
            warning: Color::Rgb(250, 204, 21),      // Yellow
            error: Color::Rgb(239, 68, 68),         // Red
            info: Color::Rgb(59, 130, 246),         // Blue

            // Background Colors
            bg_dark: Color::Rgb(15, 23, 42),        // Slate 900
            bg_card: Color::Rgb(30, 41, 59),        // Slate 800
            bg_elevated: Color::Rgb(51, 65, 85),    // Slate 700
            bg_highlight: Color::Rgb(71, 85, 105),  // Slate 600

            // Text Colors
            text: Color::Rgb(248, 250, 252),        // Slate 50
            text_muted: Color::Rgb(148, 163, 184),  // Slate 400
            text_dim: Color::Rgb(100, 116, 139),    // Slate 500

            // Rating Colors
            rating_again: Color::Rgb(239, 68, 68),  // Red
            rating_hard: Color::Rgb(251, 191, 36),  // Amber
            rating_good: Color::Rgb(59, 130, 246),  // Blue
            rating_easy: Color::Rgb(34, 197, 94),   // Green
        }
    }

    /// Kanagawa Wave theme - inspired by the famous painting and kanagawa.nvim
    fn kanagawa_wave_colors() -> ThemeColors {
        ThemeColors {
            // Brand Colors - using Kanagawa palette
            primary: Color::Rgb(0x7E, 0x9C, 0xD8),      // crystalBlue - Functions/Titles
            secondary: Color::Rgb(0x95, 0x7F, 0xB8),    // oniViolet - Keywords
            accent: Color::Rgb(0xD2, 0x7E, 0x99),       // sakuraPink - Numbers

            // Semantic Colors
            success: Color::Rgb(0x98, 0xBB, 0x6C),      // springGreen - Strings
            warning: Color::Rgb(0xFF, 0x9E, 0x3B),      // roninYellow - Warning
            error: Color::Rgb(0xE8, 0x24, 0x24),        // samuraiRed - Error
            info: Color::Rgb(0x7F, 0xB4, 0xCA),         // springBlue - Specials

            // Background Colors
            bg_dark: Color::Rgb(0x16, 0x16, 0x1D),      // sumiInk0 - Dark bg
            bg_card: Color::Rgb(0x1F, 0x1F, 0x28),      // sumiInk1 - Default bg
            bg_elevated: Color::Rgb(0x2A, 0x2A, 0x37),  // sumiInk2 - Lighter bg
            bg_highlight: Color::Rgb(0x36, 0x36, 0x46), // sumiInk3 - Cursorline

            // Text Colors
            text: Color::Rgb(0xDC, 0xD7, 0xBA),         // fujiWhite - Default fg
            text_muted: Color::Rgb(0xC8, 0xC0, 0x93),   // oldWhite - Dark fg
            text_dim: Color::Rgb(0x54, 0x54, 0x6D),     // sumiInk4 - Darker fg

            // Rating Colors
            rating_again: Color::Rgb(0xE8, 0x24, 0x24), // samuraiRed
            rating_hard: Color::Rgb(0xFF, 0x9E, 0x3B),  // roninYellow
            rating_good: Color::Rgb(0x7E, 0x9C, 0xD8),  // crystalBlue
            rating_easy: Color::Rgb(0x98, 0xBB, 0x6C),  // springGreen
        }
    }

    // ══════════════════════════════════════════════════════════════════════
    // Styles
    // ══════════════════════════════════════════════════════════════════════

    pub fn title(&self) -> Style {
        Style::default()
            .fg(self.colors.text)
            .add_modifier(Modifier::BOLD)
    }

    pub fn subtitle(&self) -> Style {
        Style::default()
            .fg(self.colors.text_muted)
    }

    pub fn highlight(&self) -> Style {
        Style::default()
            .fg(self.colors.primary)
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected(&self) -> Style {
        Style::default()
            .bg(self.colors.bg_highlight)
            .fg(self.colors.text)
    }

    pub fn card_border(&self) -> Style {
        Style::default()
            .fg(self.colors.primary)
    }

    pub fn card_front(&self) -> Style {
        Style::default()
            .fg(self.colors.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn card_back(&self) -> Style {
        Style::default()
            .fg(self.colors.success)
            .add_modifier(Modifier::BOLD)
    }

    pub fn stats_new(&self) -> Style {
        Style::default()
            .fg(self.colors.info)
            .add_modifier(Modifier::BOLD)
    }

    pub fn stats_learning(&self) -> Style {
        Style::default()
            .fg(self.colors.warning)
            .add_modifier(Modifier::BOLD)
    }

    pub fn stats_due(&self) -> Style {
        Style::default()
            .fg(self.colors.success)
            .add_modifier(Modifier::BOLD)
    }

    pub fn key_hint(&self) -> Style {
        Style::default()
            .fg(self.colors.text_dim)
    }

    pub fn key_highlight(&self) -> Style {
        Style::default()
            .fg(self.colors.accent)
            .add_modifier(Modifier::BOLD)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new(ThemeName::Default)
    }
}

// ══════════════════════════════════════════════════════════════════════════
// Box Drawing Characters
// ══════════════════════════════════════════════════════════════════════════

pub mod borders {
    pub const DOUBLE_TOP_LEFT: &str = "╔";
    pub const DOUBLE_TOP_RIGHT: &str = "╗";
    pub const DOUBLE_BOTTOM_LEFT: &str = "╚";
    pub const DOUBLE_BOTTOM_RIGHT: &str = "╝";
    pub const DOUBLE_HORIZONTAL: &str = "═";
    pub const DOUBLE_VERTICAL: &str = "║";

    pub const ROUNDED_TOP_LEFT: &str = "╭";
    pub const ROUNDED_TOP_RIGHT: &str = "╮";
    pub const ROUNDED_BOTTOM_LEFT: &str = "╰";
    pub const ROUNDED_BOTTOM_RIGHT: &str = "╯";
}

// ══════════════════════════════════════════════════════════════════════════
// Icons
// ══════════════════════════════════════════════════════════════════════════

pub mod icons {
    pub const CARD: &str = "🃏";
    pub const CHECK: &str = "✓";
    pub const CROSS: &str = "✗";
    pub const STAR: &str = "★";
    pub const ARROW_RIGHT: &str = "→";
    pub const ARROW_LEFT: &str = "←";
    pub const BRAIN: &str = "🧠";
    pub const FIRE: &str = "🔥";
    pub const SPARKLE: &str = "✨";
    pub const CLOCK: &str = "⏱";
    pub const BOOK: &str = "📚";
}
