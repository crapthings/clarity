# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-01-31

### Added

- Initial release of Clarity
- **Screen Recording**
  - 1 FPS automatic screenshot capture
  - Cross-platform support (macOS, Windows, Linux)
  - JPEG compression with quality 85
  - Organized storage by date (YYYY-MM-DD)
  
- **AI-Powered Analysis**
  - Google Gemini API integration
  - Video-based activity summarization
  - Customizable AI prompts (English and Chinese)
  - Configurable video resolution (low/default)
  - Support for multiple AI models
  
- **Activity Timeline (Trace)**
  - Visual timeline of daily activities
  - AI-generated summaries for each time period
  - Activity tags with color coding
  - Focus score and productivity metrics
  - Date navigation
  
- **Daily Summary**
  - Comprehensive daily reports
  - AI-generated insights and recommendations
  - Comparison with previous day
  - Weekly, monthly, and yearly statistics
  - Beautiful charts using Recharts
  
- **Statistics Dashboard**
  - Real-time statistics tracking
  - Screenshot count and summary count
  - API request statistics
  - Token usage tracking
  - Success rate and performance metrics
  
- **Settings**
  - Google Gemini API key configuration
  - AI model selection
  - Summary interval configuration
  - Custom AI prompts (bilingual support)
  - Video resolution settings
  - Language selection (English/Chinese)
  
- **Privacy & Security**
  - All data stored locally
  - No external data transmission (except AI API calls)
  - SQLite database for local storage
  - Platform-specific secure storage directories
  
- **Internationalization**
  - Full English and Chinese (中文) support
  - Language-specific AI prompts
  - Localized UI elements

### Technical Details

- Built with Tauri v2 + React + Rust
- Frontend: React 19, Vite, TailwindCSS, BaseUI, Recharts
- Backend: Rust, Tokio, SQLx, xcap, image
- Database: SQLite with SQLx
- AI Integration: Google Gemini File API
- State Management: Zustand with persistence

### Known Issues

- None at initial release

---

## [Unreleased]

### Planned Features

- Export functionality (JSON, CSV)
- More AI model support
- Plugin system
- Additional language support
- Cloud sync (optional)
- Mobile app companion

[0.1.0]: https://github.com/crapthings/clarity/releases/tag/v0.1.0
