# Contributing to SRL

Thank you for your interest in contributing to SRL! This document provides guidelines and information for contributors.

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Git

### Setup

1. Fork the repository on GitHub
2. Clone your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/flashcards.git
   cd flashcards
   ```
3. Add the upstream remote:
   ```bash
   git remote add upstream https://github.com/kearnsw/flashcards.git
   ```
4. Build the project:
   ```bash
   cargo build
   ```

## Development Workflow

### Creating a Branch

Always create a feature branch for your work:

```bash
git checkout main
git pull upstream main
git checkout -b feature/your-feature-name
```

Use descriptive branch names:
- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation changes
- `refactor/` - Code refactoring

### Making Changes

1. Write clear, concise commit messages
2. Keep commits focused and atomic
3. Add tests for new functionality
4. Update documentation as needed

### Code Style

- Follow standard Rust conventions
- Run `cargo fmt` before committing
- Run `cargo clippy` and address warnings
- Keep functions small and focused

### Testing

Run the test suite before submitting:

```bash
cargo test
```

For manual testing:

```bash
cargo run
```

## Submitting Changes

### Pull Request Process

1. Ensure your branch is up to date:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. Push your branch:
   ```bash
   git push origin feature/your-feature-name
   ```

3. Open a Pull Request on GitHub

4. In your PR description:
   - Describe the changes made
   - Reference any related issues
   - Include screenshots for UI changes
   - Note any breaking changes

### PR Requirements

- All tests must pass
- Code must be formatted (`cargo fmt`)
- No new clippy warnings
- Documentation updated if needed

## Project Structure

```
flashcards/
├── src/
│   ├── main.rs        # Entry point, CLI handling
│   ├── models.rs      # Card, Deck, and related types
│   ├── storage.rs     # Persistence and import/export
│   ├── sm2.rs         # SM-2 spaced repetition algorithm
│   ├── config.rs      # Configuration handling
│   └── ui/
│       ├── mod.rs     # UI module
│       ├── app.rs     # Main application state
│       ├── theme.rs   # Color themes
│       └── widgets.rs # Custom widgets
├── bundled_decks/     # Default decks for new users
├── Cargo.toml         # Dependencies
└── README.md
```

## Areas for Contribution

### Good First Issues

Look for issues labeled `good first issue` for beginner-friendly tasks.

### Feature Ideas

- Additional import/export formats
- New themes
- Statistics visualizations
- Keyboard shortcut customization
- Search and filtering
- Cloud sync

### Documentation

- Improve README
- Add inline documentation
- Create tutorials
- Translate to other languages

## Code of Conduct

### Our Standards

- Be respectful and inclusive
- Accept constructive criticism gracefully
- Focus on what's best for the community
- Show empathy towards others

### Unacceptable Behavior

- Harassment or discriminatory language
- Personal attacks
- Publishing others' private information
- Other unprofessional conduct

## Questions?

Feel free to open an issue for:
- Bug reports
- Feature requests
- Questions about the codebase

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
