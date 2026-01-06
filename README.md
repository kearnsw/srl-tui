# SRL - Spaced Repetition Learning

A beautiful, fast Anki-style spaced repetition flashcard app for the terminal.

```
  ███████╗██████╗ ██╗
  ██╔════╝██╔══██╗██║
  ███████╗██████╔╝██║
  ╚════██║██╔══██╗██║
  ███████║██║  ██║███████╗
  ╚══════╝╚═╝  ╚═╝╚══════╝
     spaced repetition learning
```

Built with [Ratatui](https://ratatui.rs) for a modern TUI experience.

## Features

- **SM-2 Spaced Repetition** - Optimal review scheduling based on recall quality
- **Anki Compatibility** - Import/export `.apkg` files with full scheduling data
- **Card Browser** - View, edit, and delete cards with full metadata
- **Statistics Dashboard** - Track total reviews, daily/weekly streaks, cards by difficulty
- **10 Themes** - Beautiful color schemes including Kanagawa Wave
- **Backup System** - Export/import all decks as JSON
- **CSV Import** - Bulk import from spreadsheets
- **Keyboard-Driven** - Fast, efficient studying

## Installation

### Homebrew (macOS)

```bash
brew tap kearnsw/tap
brew install srl
```

### From Source

```bash
git clone https://github.com/kearnsw/flashcards.git
cd flashcards
cargo install --path .
```

## Usage

```bash
# Launch the TUI
srl

# Export backup (JSON)
srl --export-backup ~/backup.json

# Import backup (JSON)
srl --import-backup ~/backup.json

# Import CSV
srl --import cards.csv --import-name "My Deck"

# Import folder of CSVs
srl --import-folder ./decks/

# Import from Anki (.apkg or .txt)
srl --import-anki deck.apkg
srl --import-anki vocab.txt --import-anki-name "Spanish"

# Export to Anki (preserves scheduling progress)
srl --export-anki my_decks.apkg
```

## Keyboard Shortcuts

### Deck List
| Key | Action |
|-----|--------|
| `j/k` | Navigate |
| `Enter` | Study deck |
| `b` | Browse cards |
| `s` | Statistics |
| `n` | New deck |
| `d` | Delete deck |
| `x` | Export backup |
| `t` | Cycle theme |
| `q` | Quit |

### Study Mode
| Key | Action |
|-----|--------|
| `Space` | Show answer |
| `1-4` | Rate recall (Again/Hard/Good/Easy) |
| `a` | Add card |
| `b` | Browse cards |
| `Esc` | Back to decks |

### Card Browser
| Key | Action |
|-----|--------|
| `j/k` | Navigate cards |
| `e` | Edit card |
| `d` | Delete card (press twice) |
| `a` | Add card |
| `Esc` | Back |

### Edit Mode
| Key | Action |
|-----|--------|
| `Tab` | Switch field |
| `Enter` | Save |
| `Esc` | Cancel |

## Spaced Repetition

### What is Spaced Repetition?

Spaced repetition is a learning technique that incorporates increasing intervals of time between subsequent reviews of previously learned material. This method exploits the psychological spacing effect, which demonstrates that learning is more effective when study sessions are spaced out over time rather than massed together.

The core insight is that memories have a "forgetting curve" - we forget information at an exponential rate after learning it. By reviewing material just before we're about to forget it, we can reset and strengthen the memory with optimal efficiency.

### The SM-2 Algorithm

SRL implements the **SM-2 (SuperMemo 2) algorithm**, developed by Piotr Wozniak in 1987. SM-2 is the foundation of modern spaced repetition software, including Anki.

#### How It Works

Each card tracks:
- **Ease Factor (EF)**: A multiplier (starting at 2.5) indicating how easy the card is
- **Interval**: Days until the next review
- **Repetitions**: Number of successful reviews in a row

When you review a card, you rate your recall from 0-5 (SRL uses 1-4):

| Rating | Meaning | Effect |
|--------|---------|--------|
| **Again (1)** | Complete blackout | Reset to beginning, EF decreases |
| **Hard (2)** | Significant difficulty | Interval × 1.2, EF decreases slightly |
| **Good (3)** | Correct with hesitation | Interval × EF |
| **Easy (4)** | Perfect, instant recall | Interval × EF × 1.3, EF increases |

#### The Formula

For successful recalls (rating ≥ 3):
```
new_interval = old_interval × ease_factor
ease_factor = max(1.3, EF + (0.1 - (5-q) × (0.08 + (5-q) × 0.02)))
```

Where `q` is the quality of response (0-5 scale).

#### References

- Wozniak, P. A. (1990). *Optimization of repetition spacing in the practice of learning*. Master's thesis, University of Technology in Poznan.
  - [Full thesis (SuperMemo)](https://super-memory.com/english/ol.htm)
- Wozniak, P. A., & Gorzelanczyk, E. J. (1994). Optimization of repetition spacing in the practice of learning. *Acta Neurobiologiae Experimentalis*, 54, 59-62.
- [SM-2 Algorithm Description (SuperMemo)](https://super-memory.com/english/ol/sm2.htm)
- [Anki's Implementation of SM-2](https://docs.ankiweb.net/studying.html#spaced-repetition)

### Why Spaced Repetition Works

Research has consistently shown that spaced repetition can:
- Reduce study time by 50% or more compared to massed practice
- Improve long-term retention from ~20% to ~80%+
- Enable learning of large amounts of material (10,000+ items)

Key studies:
- Cepeda, N. J., et al. (2006). Distributed practice in verbal recall tasks: A review and quantitative synthesis. *Psychological Bulletin*, 132(3), 354-380.
- Karpicke, J. D., & Roediger, H. L. (2008). The critical importance of retrieval for learning. *Science*, 319(5865), 966-968.

## Anki Compatibility

SRL supports full round-trip compatibility with Anki:

### Import from Anki

```bash
# Import .apkg package (preserves all scheduling data)
srl --import-anki my_deck.apkg

# Import tab-separated text export
srl --import-anki vocab.txt --import-anki-name "Vocabulary"
```

**Preserved on import:**
- Card content (front/back)
- Interval (days until next review)
- Ease factor
- Repetition count
- Lapse count
- Tags

### Export to Anki

```bash
# Export all decks to Anki format
srl --export-anki my_decks.apkg
```

**Preserved on export:**
- All card content
- Full scheduling state
- Deck names and descriptions
- Tags

The exported `.apkg` file can be imported directly into Anki Desktop or AnkiMobile.

## Import Formats

### CSV Format

```csv
front,back
Question 1,Answer 1
Question 2,Answer 2
```

### Anki Text Export

Tab-separated format with optional tags:
```
front<TAB>back
front<TAB>back<TAB>tag1 tag2
```

## Backup Format

JSON backups contain all decks with full card data and review history:

```json
{
  "version": 1,
  "created_at": "2025-01-06T12:00:00",
  "decks": [...]
}
```

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Quick Start

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make your changes
4. Run tests: `cargo test`
5. Submit a pull request

## License

MIT - see [LICENSE](LICENSE) for details.
