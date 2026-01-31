# Architecture Documentation

This document describes the architecture and design decisions of Clarity.

## Overview

Clarity is a cross-platform desktop application built with Tauri v2, combining a React frontend with a Rust backend. The application automatically captures screen activity and uses AI to provide productivity insights while maintaining complete privacy through local storage.

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Frontend (React)                        │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │  Trace   │  │ Summary  │  │Statistics│  │ Settings │  │
│  │   Page   │  │   Page   │  │   Page   │  │   Page   │  │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘  │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │         State Management (Zustand)                     │  │
│  │         i18n (Internationalization)                  │  │
│  └──────────────────────────────────────────────────────┘  │
└──────────────────────┬─────────────────────────────────────┘
                        │
                        │ Tauri IPC
                        │
┌───────────────────────▼─────────────────────────────────────┐
│                   Backend (Rust)                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              AppState (Global State)                  │  │
│  │  • Recording status                                  │  │
│  │  • Screenshot count                                  │  │
│  │  • API keys & settings                               │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Screenshot   │  │   Video      │  │   Database   │     │
│  │   Loop       │  │  Summary     │  │  Operations  │     │
│  │              │  │    Loop      │  │              │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │         External Services                            │  │
│  │  • Google Gemini API (AI processing)                 │  │
│  │  • FFmpeg (video generation)                         │  │
│  └──────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

## Technology Stack

### Frontend

- **React 19**: Modern UI framework
- **Vite**: Fast build tool and dev server
- **TailwindCSS**: Utility-first CSS framework
- **BaseUI**: Component library
- **Zustand**: Lightweight state management
- **Recharts**: Data visualization library
- **React Markdown**: Markdown rendering for AI summaries
- **Material Design Icons (MDI)**: Icon library

### Backend

- **Tauri v2**: Desktop application framework
- **Rust**: System programming language
- **Tokio**: Async runtime
- **SQLx**: Async SQL toolkit (SQLite)
- **xcap**: Cross-platform screen capture
- **image**: Image processing and JPEG encoding
- **reqwest**: HTTP client for API calls
- **chrono**: Date and time handling

## Core Components

### 1. Screenshot Capture System

**Location**: `src-tauri/src/lib.rs` - `screenshot_loop`

**Functionality**:
- Captures screenshots at 1 FPS using `xcap` crate
- Runs in async Tokio task
- Converts RGBA to RGB for JPEG encoding
- Compresses with JPEG quality 85
- Saves to date-organized directories
- Stores metadata in SQLite database

**Key Design Decisions**:
- 1 FPS balance: Enough detail for analysis, minimal storage
- JPEG compression: Reduces storage by ~90% vs PNG
- Async architecture: Non-blocking, efficient resource usage

### 2. Video Summary System

**Location**: `src-tauri/src/video_summary.rs`

**Functionality**:
- Creates video from screenshot sequence using FFmpeg
- Uploads video to Google Gemini File API
- Waits for file processing (PROCESSING → ACTIVE)
- Generates AI summary using video URI
- Handles errors and retries

**Key Design Decisions**:
- File API: Required for videos >100MB or >1 minute
- Resolution optimization: 640x360 default, configurable
- Token optimization: Low resolution (~100 tokens/sec) vs Default (~300 tokens/sec)
- Async polling: Efficient waiting for file processing

### 3. Database Schema

**Location**: `src-tauri/src/db.rs`

**Tables**:
- `screenshot_traces`: Screenshot metadata (timestamp, path, size)
- `summaries`: AI-generated summaries (time range, content)
- `daily_summaries`: Daily aggregated summaries
- `api_requests`: API call logs (tokens, duration, success)
- `settings`: Application settings (key-value pairs)

**Design Decisions**:
- SQLite: Lightweight, embedded, no server required
- Indexes: Optimized queries on timestamp and date fields
- Local storage: Complete privacy, no cloud dependency

### 4. Frontend Pages

#### Trace Page (`src/pages/Trace.jsx`)
- Displays activity timeline
- Shows AI summaries for each time period
- Activity tags with color coding
- Focus score and productivity metrics

#### Summary Page (`src/pages/Summary.jsx`)
- Daily summary generation
- Comparison with yesterday
- Weekly/monthly/yearly charts (Recharts)
- Manual and automatic generation

#### Statistics Page (`src/pages/Statistics.jsx`)
- Today's statistics dashboard
- API usage metrics
- Token consumption breakdown
- Performance metrics

#### Settings Page (`src/pages/Settings.jsx`)
- API key configuration
- AI model selection
- Prompt customization (multi-language)
- Video resolution settings
- Language selection

## Data Flow

### Screenshot Capture Flow

```
User clicks "Start Recording"
    ↓
Backend: screenshot_loop() starts
    ↓
Every 1 second:
    ↓
Capture screenshot (xcap)
    ↓
Convert RGBA → RGB
    ↓
JPEG compress (quality 85)
    ↓
Save to disk (date-organized)
    ↓
Store metadata in SQLite
    ↓
Emit "statistics-updated" event
    ↓
Frontend updates UI
```

### AI Summary Generation Flow

```
Every N seconds (configurable):
    ↓
Collect recent screenshots
    ↓
Create video (FFmpeg)
    ↓
Upload to Gemini File API
    ↓
Wait for PROCESSING → ACTIVE
    ↓
Call generateContent with file URI
    ↓
Receive AI summary
    ↓
Store in SQLite (summaries table)
    ↓
Emit event to update UI
```

### Daily Summary Generation Flow

```
User clicks "Generate Summary" OR first visit:
    ↓
Load all summaries for the day
    ↓
Merge summary content
    ↓
Call Gemini API (text-only, no video)
    ↓
Generate comprehensive daily summary
    ↓
Store in daily_summaries table
    ↓
Display with charts and comparisons
```

## Security & Privacy

### Privacy Architecture

1. **Local Storage**: All screenshots stored on device
2. **Local Database**: SQLite database on device
3. **No Cloud Sync**: No data leaves device except AI processing
4. **User Control**: User provides own API key
5. **Optional AI**: Can disable AI features entirely

### Security Measures

1. **API Key Storage**: Encrypted in SQLite (Tauri handles encryption)
2. **File Permissions**: Platform-specific permission requests
3. **Input Validation**: All user inputs validated
4. **Error Handling**: Comprehensive error handling prevents data leaks

## Performance Optimizations

### Screenshot Capture
- Async/await: Non-blocking operations
- JPEG compression: ~90% size reduction
- Efficient file I/O: Batched writes

### Video Processing
- Resolution scaling: 640x360 default
- Low resolution option: 66% token reduction
- FFmpeg optimization: Hardware acceleration when available

### Frontend
- Virtual scrolling: `@tanstack/react-virtual` for long lists
- Memoization: `useMemo`, `useCallback` for expensive operations
- Code splitting: Vite automatic code splitting
- Lazy loading: Components loaded on demand

## Cross-Platform Considerations

### macOS
- Screen Recording permission required
- Uses native macOS APIs via xcap
- App data: `~/Library/Application Support/clarity/`

### Windows
- May require admin privileges
- Uses Windows screen capture APIs
- App data: `%LOCALAPPDATA%/clarity/`

### Linux
- X11: Works out of the box
- Wayland: May need permissions
- App data: `~/.local/share/clarity/`

## Extension Points

### Adding New Features

1. **New Tauri Command**: Add to `lib.rs` and register in `run()`
2. **New Database Table**: Add schema in `db.rs` `init_db()`
3. **New Frontend Page**: Add to `src/pages/` and route in `MainLayout.jsx`
4. **New Translation**: Add keys to `src/i18n/locales.js`

### Plugin System (Future)

Potential architecture for plugins:
- Plugin API via Tauri commands
- Plugin registry in database
- Sandboxed execution
- Event system for plugin communication

## Development Workflow

1. **Frontend Changes**: Edit React components, hot reload via Vite
2. **Backend Changes**: Edit Rust code, restart Tauri dev server
3. **Database Changes**: Update schema in `db.rs`, migrations handled automatically
4. **Testing**: Manual testing on target platforms

## Build Process

1. **Frontend Build**: `pnpm build` → Vite bundles React app
2. **Backend Build**: `cargo build` → Rust compiles to native binary
3. **Tauri Bundle**: `tauri build` → Creates platform-specific installer
4. **Output**: Platform-specific bundles (DMG, MSI, AppImage, etc.)

## Future Architecture Considerations

- **Plugin System**: Extensible architecture for custom features
- **Export System**: Data export in various formats
- **Sync System** (optional): Encrypted sync for multi-device users
- **Cloud Backup** (optional): User-controlled cloud storage integration
- **Mobile App**: React Native version sharing backend logic
