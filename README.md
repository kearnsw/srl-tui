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
- **Card Browser** - View, edit, and delete cards with full metadata
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
git clone https://github.com/kearnsw/srl-tui.git
cd srl-tui
cargo install --path .
```

## Usage

```bash
# Launch the TUI
srl

# Export backup
srl --export-backup ~/backup.json

# Import backup
srl --import-backup ~/backup.json

# Import CSV
srl --import cards.csv --import-name "My Deck"

# Import folder of CSVs
srl --import-folder ./decks/
```

## Keyboard Shortcuts

### Deck List
| Key | Action |
|-----|--------|
| `j/k` | Navigate |
| `Enter` | Study deck |
| `b` | Browse cards |
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

## Spaced Repetition (SM-2)

The algorithm adjusts intervals based on your recall:

- **Again (1)** - Forgot → Review in 10 minutes
- **Hard (2)** - Struggled → Interval × 1.2
- **Good (3)** - Normal → Interval × ease factor
- **Easy (4)** - Perfect → Interval × ease factor × 1.3

## Backup Format

Backups are JSON files containing all decks with full card data and review history:

```json
{
  "version": 1,
  "created_at": "2025-01-06T12:00:00",
  "decks": [...]
}
```

## CSV Import Format

```csv
front,back
Question 1,Answer 1
Question 2,Answer 2
```

## License

MIT
