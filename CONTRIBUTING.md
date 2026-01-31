# Contributing to Clarity

Thank you for your interest in contributing to Clarity! This document provides guidelines and instructions for contributing.

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check the issue list as you might find out that you don't need to create one. When you are creating a bug report, please include as many details as possible:

- **Clear title and description**
- **Steps to reproduce** the behavior
- **Expected behavior** vs **actual behavior**
- **Screenshots** if applicable
- **Environment details**: OS, Node.js version, Rust version
- **Error messages** or logs

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. When creating an enhancement suggestion, please include:

- **Clear title and description**
- **Use case**: Why is this feature useful?
- **Proposed solution** (if you have one)
- **Alternatives considered** (if any)

### Pull Requests

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Ensure your code follows the project's style guidelines
5. Test your changes thoroughly
6. Commit your changes (`git commit -m 'Add some amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

## Development Setup

### Prerequisites

- Node.js (v18 or higher)
- pnpm
- Rust (latest stable)
- Tauri CLI v2: `cargo install tauri-cli@^2`

### Getting Started

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/clarity.git
cd clarity

# Install dependencies
pnpm install

# Run in development mode
pnpm tauri dev
```

## Coding Standards

### Rust

- Follow Rust naming conventions (snake_case for functions/variables, PascalCase for types)
- Run `cargo fmt` before committing
- Run `cargo clippy` to check for common issues
- Add comments for complex logic
- Use meaningful variable and function names

### JavaScript/React

- Follow the existing code style
- Use functional components with hooks
- Prefer `const` over `let`
- Use meaningful variable and function names
- Add JSDoc comments for complex functions

### Code Style

- Use 2 spaces for indentation in JavaScript/TypeScript
- Use 4 spaces for indentation in Rust
- Maximum line length: 100 characters
- Use single quotes for JavaScript strings (when possible)

## Testing

Before submitting a PR, please ensure:

- [ ] Your code compiles without errors
- [ ] All existing tests pass (if applicable)
- [ ] You've tested your changes on your target platform(s)
- [ ] You've tested edge cases

## Commit Messages

Write clear commit messages:

- Use the present tense ("Add feature" not "Added feature")
- Use the imperative mood ("Move cursor to..." not "Moves cursor to...")
- Limit the first line to 72 characters or less
- Reference issues and pull requests liberally after the first line

Example:
```
Add video resolution setting

- Add resolution option in settings page
- Update backend to support resolution parameter
- Fix issue #123
```

## Project Structure

- `src/` - React frontend code
- `src-tauri/src/` - Rust backend code
- `src-tauri/src/lib.rs` - Main application logic
- `src-tauri/src/db.rs` - Database operations
- `src-tauri/src/video_summary.rs` - Video processing and AI integration

## Areas for Contribution

- 🐛 Bug fixes
- ✨ New features
- 📚 Documentation improvements
- 🎨 UI/UX improvements
- ⚡ Performance optimizations
- 🌍 Translations (additional languages)
- 🧪 Tests

## Questions?

Feel free to open an issue for any questions or reach out to the maintainers.

Thank you for contributing to Clarity! 🎉
