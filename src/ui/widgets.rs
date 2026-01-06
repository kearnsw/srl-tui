//! Custom widgets for the flashcard TUI.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{block::BorderType, Block, Borders, Paragraph, Widget, Wrap},
};

use super::theme::Theme;
use crate::models::DeckStats;

// ══════════════════════════════════════════════════════════════════════════
// Logo Widget
// ══════════════════════════════════════════════════════════════════════════

pub struct Logo<'a> {
    theme: &'a Theme,
}

impl<'a> Logo<'a> {
    const ART: &'static str = r#"
    ╭─────────────────────────────────────────╮
    │  _____ _           _                    │
    │ |  ___| | __ _ ___| |__   ___ __ _ _ __ │
    │ | |_  | |/ _` / __| '_ \ / __/ _` | '__││
    │ |  _| | | (_| \__ \ | | | (_| (_| | |   │
    │ |_|   |_|\__,_|___/_| |_|\___\__,_|_|   │
    │                     ┌───────────────┐   │
    │      ╭────╮         │ Spaced        │   │
    │      │ 🧠 │         │ Repetition    │   │
    │      ╰────╯         │ Learning      │   │
    │                     └───────────────┘   │
    ╰─────────────────────────────────────────╯"#;

    pub fn new(theme: &'a Theme) -> Self {
        Self { theme }
    }

    pub fn render_to(theme: &Theme, area: Rect, buf: &mut Buffer) {
        let lines: Vec<Line> = Self::ART
            .lines()
            .skip(1)
            .map(|line| {
                Line::from(vec![
                    Span::styled(line, Style::default().fg(theme.colors.primary))
                ])
            })
            .collect();

        let para = Paragraph::new(lines)
            .alignment(Alignment::Center);

        para.render(area, buf);
    }
}

impl Widget for Logo<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Self::render_to(self.theme, area, buf);
    }
}

// ══════════════════════════════════════════════════════════════════════════
// Stats Bar Widget
// ══════════════════════════════════════════════════════════════════════════

pub struct StatsBar<'a> {
    stats: DeckStats,
    theme: &'a Theme,
}

impl<'a> StatsBar<'a> {
    pub fn new(stats: DeckStats, theme: &'a Theme) -> Self {
        Self { stats, theme }
    }
}

impl Widget for StatsBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::horizontal([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

        // New cards
        let new_text = Line::from(vec![
            Span::styled("● ", self.theme.stats_new()),
            Span::styled("New: ", Style::default().fg(self.theme.colors.text_muted)),
            Span::styled(
                self.stats.new_cards.to_string(),
                self.theme.stats_new(),
            ),
        ]);
        Paragraph::new(new_text)
            .alignment(Alignment::Center)
            .render(chunks[0], buf);

        // Learning cards
        let learning_text = Line::from(vec![
            Span::styled("● ", self.theme.stats_learning()),
            Span::styled("Learning: ", Style::default().fg(self.theme.colors.text_muted)),
            Span::styled(
                self.stats.learning_cards.to_string(),
                self.theme.stats_learning(),
            ),
        ]);
        Paragraph::new(learning_text)
            .alignment(Alignment::Center)
            .render(chunks[1], buf);

        // Due cards
        let due_text = Line::from(vec![
            Span::styled("● ", self.theme.stats_due()),
            Span::styled("Due: ", Style::default().fg(self.theme.colors.text_muted)),
            Span::styled(
                self.stats.due_cards.to_string(),
                self.theme.stats_due(),
            ),
        ]);
        Paragraph::new(due_text)
            .alignment(Alignment::Center)
            .render(chunks[2], buf);

        // Total
        let total_text = Line::from(vec![
            Span::styled("Total: ", Style::default().fg(self.theme.colors.text_muted)),
            Span::styled(
                self.stats.total_cards.to_string(),
                Style::default().fg(self.theme.colors.text_dim),
            ),
        ]);
        Paragraph::new(total_text)
            .alignment(Alignment::Center)
            .render(chunks[3], buf);
    }
}

// ══════════════════════════════════════════════════════════════════════════
// Flashcard Widget
// ══════════════════════════════════════════════════════════════════════════

pub struct FlashcardWidget<'a> {
    content: &'a str,
    is_front: bool,
    theme: &'a Theme,
}

impl<'a> FlashcardWidget<'a> {
    pub fn new(content: &'a str, is_front: bool, theme: &'a Theme) -> Self {
        Self { content, is_front, theme }
    }
}

impl Widget for FlashcardWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (label, label_style, border_style) = if self.is_front {
            ("QUESTION", self.theme.card_front(), Style::default().fg(self.theme.colors.accent))
        } else {
            ("ANSWER", self.theme.card_back(), Style::default().fg(self.theme.colors.success))
        };

        // Outer block with pretty border
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled(label, label_style),
                Span::raw(" "),
            ]))
            .title_alignment(Alignment::Center);

        let inner = block.inner(area);
        block.render(area, buf);

        // Content
        let content_para = Paragraph::new(self.content)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(self.theme.colors.text));

        // Center vertically
        let content_height = self.content.lines().count() as u16;
        let vertical_padding = inner.height.saturating_sub(content_height) / 2;

        let content_area = Rect {
            x: inner.x + 2,
            y: inner.y + vertical_padding,
            width: inner.width.saturating_sub(4),
            height: inner.height.saturating_sub(vertical_padding),
        };

        content_para.render(content_area, buf);
    }
}

// ══════════════════════════════════════════════════════════════════════════
// Rating Buttons Widget
// ══════════════════════════════════════════════════════════════════════════

pub struct RatingButtons<'a> {
    intervals: &'a [(crate::models::ReviewRating, String)],
    enabled: bool,
    theme: &'a Theme,
}

impl<'a> RatingButtons<'a> {
    pub fn new(intervals: &'a [(crate::models::ReviewRating, String)], enabled: bool, theme: &'a Theme) -> Self {
        Self { intervals, enabled, theme }
    }
}

impl Widget for RatingButtons<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::horizontal([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

        for (i, (rating, interval)) in self.intervals.iter().enumerate() {
            let color = if self.enabled {
                rating.color_for_theme(self.theme)
            } else {
                self.theme.colors.text_dim
            };

            let key = (i + 1).to_string();
            let name = rating.name();

            let button = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(color));

            let inner = button.inner(chunks[i]);
            button.render(chunks[i], buf);

            // Key number
            let key_line = Line::from(vec![
                Span::styled(&key, Style::default().fg(color).add_modifier(Modifier::BOLD)),
            ]);
            Paragraph::new(key_line)
                .alignment(Alignment::Center)
                .render(
                    Rect {
                        y: inner.y,
                        ..inner
                    },
                    buf,
                );

            // Rating name
            let name_line = Line::from(vec![
                Span::styled(name, Style::default().fg(color)),
            ]);
            Paragraph::new(name_line)
                .alignment(Alignment::Center)
                .render(
                    Rect {
                        y: inner.y + 1,
                        ..inner
                    },
                    buf,
                );

            // Interval
            if self.enabled {
                let interval_line = Line::from(vec![
                    Span::styled(interval, Style::default().fg(self.theme.colors.text_muted)),
                ]);
                Paragraph::new(interval_line)
                    .alignment(Alignment::Center)
                    .render(
                        Rect {
                            y: inner.y + 2,
                            ..inner
                        },
                        buf,
                    );
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════
// Key Hints Widget
// ══════════════════════════════════════════════════════════════════════════

pub struct KeyHints<'a> {
    hints: &'a [(&'a str, &'a str)],
    theme: &'a Theme,
}

impl<'a> KeyHints<'a> {
    pub fn new(hints: &'a [(&'a str, &'a str)], theme: &'a Theme) -> Self {
        Self { hints, theme }
    }
}

impl Widget for KeyHints<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let spans: Vec<Span> = self
            .hints
            .iter()
            .flat_map(|(key, desc)| {
                vec![
                    Span::styled(*key, self.theme.key_highlight()),
                    Span::styled(format!(" {} ", desc), self.theme.key_hint()),
                    Span::styled("│ ", Style::default().fg(self.theme.colors.text_dim)),
                ]
            })
            .collect();

        let line = Line::from(spans);
        Paragraph::new(line)
            .alignment(Alignment::Center)
            .render(area, buf);
    }
}

// ══════════════════════════════════════════════════════════════════════════
// Completion Screen Widget
// ══════════════════════════════════════════════════════════════════════════

pub struct CompletionScreen<'a> {
    cards_studied: usize,
    duration_mins: u64,
    theme: &'a Theme,
}

impl<'a> CompletionScreen<'a> {
    pub fn new(cards_studied: usize, duration_mins: u64, theme: &'a Theme) -> Self {
        Self {
            cards_studied,
            duration_mins,
            theme,
        }
    }
}

impl Widget for CompletionScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.theme.colors.success))
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("SESSION COMPLETE", self.theme.card_back()),
                Span::raw(" "),
            ]))
            .title_alignment(Alignment::Center);

        let inner = block.inner(area);
        block.render(area, buf);

        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Great job!", Style::default().fg(self.theme.colors.success).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Cards studied: ", Style::default().fg(self.theme.colors.text_muted)),
                Span::styled(
                    self.cards_studied.to_string(),
                    Style::default().fg(self.theme.colors.primary).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Time: ", Style::default().fg(self.theme.colors.text_muted)),
                Span::styled(
                    format!("{} minutes", self.duration_mins),
                    Style::default().fg(self.theme.colors.primary).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(self.theme.colors.text_dim)),
                Span::styled("ESC", self.theme.key_highlight()),
                Span::styled(" to return", Style::default().fg(self.theme.colors.text_dim)),
            ]),
        ];

        Paragraph::new(text)
            .alignment(Alignment::Center)
            .render(inner, buf);
    }
}
