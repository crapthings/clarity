# API Documentation

This document describes the Tauri commands (API) available in Clarity.

## Overview

Clarity uses Tauri's IPC (Inter-Process Communication) to communicate between the React frontend and Rust backend. All API calls are made using `invoke()` from `@tauri-apps/api/core`.

## General Patterns

### TypeScript/JavaScript Usage

```typescript
import { invoke } from '@tauri-apps/api/core'

// Example
const result = await invoke('command_name', { param1: value1, param2: value2 })
```

### Error Handling

All commands return `Result<T, String>` in Rust, which translates to:
- Success: Returns the value
- Error: Throws an error with the error message

```typescript
try {
  const result = await invoke('command_name', { ... })
  // Handle success
} catch (error) {
  // Handle error
  console.error('API error:', error)
}
```

## Recording Commands

### `start_recording`

Start capturing screenshots at 1 FPS.

**Parameters**: None

**Returns**: `void`

**Example**:
```typescript
await invoke('start_recording')
```

**Errors**:
- Permission denied (macOS screen recording)
- Storage directory creation failed

---

### `stop_recording`

Stop capturing screenshots.

**Parameters**: None

**Returns**: `void`

**Example**:
```typescript
await invoke('stop_recording')
```

---

### `get_status`

Get current recording status and statistics.

**Parameters**: None

**Returns**: `ScreenshotStatus`
```typescript
{
  isRecording: boolean
  screenshotsCount: number
  storagePath: string
}
```

**Example**:
```typescript
const status = await invoke('get_status')
console.log(`Recording: ${status.isRecording}, Count: ${status.screenshotsCount}`)
```

---

### `get_storage_path`

Get the storage path where screenshots are saved.

**Parameters**: None

**Returns**: `string` (path)

**Example**:
```typescript
const path = await invoke('get_storage_path')
```

---

### `test_screenshot`

Test screenshot capture (for debugging permissions).

**Parameters**: None

**Returns**: `string` (status message)

**Example**:
```typescript
const result = await invoke('test_screenshot')
console.log(result)
```

---

## Data Retrieval Commands

### `get_traces`

Get screenshot traces within a time range.

**Parameters**:
```typescript
{
  startTime?: string  // ISO 8601 format (RFC3339)
  endTime?: string    // ISO 8601 format (RFC3339)
  limit?: number      // Maximum number of results
}
```

**Returns**: `ScreenshotTrace[]`
```typescript
{
  id: number
  timestamp: string    // ISO 8601 format
  filePath: string
  width: number
  height: number
  fileSize: number
}[]
```

**Example**:
```typescript
const traces = await invoke('get_traces', {
  startTime: '2026-01-31T00:00:00Z',
  endTime: '2026-01-31T23:59:59Z',
  limit: 100
})
```

---

### `get_summaries`

Get AI-generated summaries within a time range.

**Parameters**:
```typescript
{
  startTime?: string  // ISO 8601 format
  endTime?: string    // ISO 8601 format
  limit?: number
}
```

**Returns**: `Summary[]`
```typescript
{
  id: number
  startTime: string
  endTime: string
  content: string      // Markdown format
  screenshotCount: number
  createdAt: string
}[]
```

**Example**:
```typescript
const summaries = await invoke('get_summaries', {
  startTime: '2026-01-31T00:00:00Z',
  endTime: '2026-01-31T23:59:59Z'
})
```

---

### `get_today_count`

Get the number of screenshots captured today.

**Parameters**: None

**Returns**: `number`

**Example**:
```typescript
const count = await invoke('get_today_count')
```

---

### `get_today_statistics`

Get comprehensive statistics for today.

**Parameters**: None

**Returns**: `TodayStatistics`
```typescript
{
  screenshotCount: number
  summaryCount: number
  apiStatistics: {
    totalRequests: number
    successfulRequests: number
    failedRequests: number
    totalPromptTokens: number
    totalCompletionTokens: number
    totalTokens: number
    avgDurationMs: number | null
  }
}
```

**Example**:
```typescript
const stats = await invoke('get_today_statistics')
console.log(`Screenshots: ${stats.screenshotCount}`)
console.log(`API Requests: ${stats.apiStatistics.totalRequests}`)
```

---

## Daily Summary Commands

### `get_daily_summary`

Get the daily summary for a specific date.

**Parameters**:
```typescript
{
  date?: string  // YYYY-MM-DD format, defaults to today
}
```

**Returns**: `DailySummary | null`
```typescript
{
  id: number
  date: string           // YYYY-MM-DD
  content: string        // Markdown format
  screenshotCount: number
  summaryCount: number
  totalDurationSeconds: number
  createdAt: string
  updatedAt: string
} | null
```

**Example**:
```typescript
const summary = await invoke('get_daily_summary', { date: '2026-01-31' })
```

---

### `generate_daily_summary`

Generate a daily summary for a specific date.

**Parameters**:
```typescript
{
  date?: string  // YYYY-MM-DD format, defaults to today
}
```

**Returns**: `DailySummary`

**Example**:
```typescript
const summary = await invoke('generate_daily_summary', { date: '2026-01-31' })
```

**Note**: This may take some time as it processes all summaries for the day and calls the AI API.

---

### `get_historical_stats`

Get historical statistics for a date range.

**Parameters**:
```typescript
{
  days: number  // Number of days to retrieve (from today backwards)
}
```

**Returns**: `HistoricalStats[]`
```typescript
{
  date: string              // YYYY-MM-DD
  screenshotCount: number
  summaryCount: number
  totalDurationSeconds: number
}[]
```

**Example**:
```typescript
// Get last 7 days
const stats = await invoke('get_historical_stats', { days: 7 })
// Get last 30 days
const monthlyStats = await invoke('get_historical_stats', { days: 30 })
```

---

## Settings Commands

### `get_gemini_api_key`

Get the stored Google Gemini API key.

**Parameters**: None

**Returns**: `string` (API key or empty string)

**Example**:
```typescript
const apiKey = await invoke('get_gemini_api_key')
```

---

### `set_gemini_api_key`

Set the Google Gemini API key.

**Parameters**:
```typescript
{
  apiKey: string
}
```

**Returns**: `void`

**Example**:
```typescript
await invoke('set_gemini_api_key', { apiKey: 'your-api-key-here' })
```

---

### `get_summary_interval`

Get the summary generation interval in seconds.

**Parameters**: None

**Returns**: `number` (seconds)

**Example**:
```typescript
const interval = await invoke('get_summary_interval')
```

---

### `set_summary_interval`

Set the summary generation interval.

**Parameters**:
```typescript
{
  intervalSeconds: number  // 10-3600 seconds
}
```

**Returns**: `void`

**Example**:
```typescript
await invoke('set_summary_interval', { intervalSeconds: 45 })
```

---

### `get_ai_model`

Get the current AI model identifier.

**Parameters**: None

**Returns**: `string` (model name)

**Example**:
```typescript
const model = await invoke('get_ai_model')
// Returns: "gemini-3-flash-preview"
```

---

### `set_ai_model`

Set the AI model to use.

**Parameters**:
```typescript
{
  model: string  // e.g., "gemini-3-flash-preview"
}
```

**Returns**: `void`

**Example**:
```typescript
await invoke('set_ai_model', { model: 'gemini-3-flash-preview' })
```

---

### `get_ai_prompt`

Get the AI prompt for a specific language.

**Parameters**:
```typescript
{
  language?: string  // "en" or "zh", defaults to "zh"
}
```

**Returns**: `string` (prompt text)

**Example**:
```typescript
const prompt = await invoke('get_ai_prompt', { language: 'en' })
```

---

### `set_ai_prompt`

Set the AI prompt for a specific language.

**Parameters**:
```typescript
{
  prompt: string
  language?: string  // "en" or "zh", defaults to "zh"
}
```

**Returns**: `void`

**Example**:
```typescript
await invoke('set_ai_prompt', {
  prompt: 'Analyze this video and provide insights.',
  language: 'en'
})
```

---

### `reset_ai_prompt`

Reset the AI prompt to default for a language.

**Parameters**:
```typescript
{
  language?: string  // "en" or "zh", defaults to "zh"
}
```

**Returns**: `string` (default prompt)

**Example**:
```typescript
const defaultPrompt = await invoke('reset_ai_prompt', { language: 'en' })
```

---

### `get_video_resolution`

Get the video resolution setting.

**Parameters**: None

**Returns**: `string` ("low" or "default")

**Example**:
```typescript
const resolution = await invoke('get_video_resolution')
```

---

### `set_video_resolution`

Set the video resolution for AI processing.

**Parameters**:
```typescript
{
  resolution: string  // "low" or "default"
}
```

**Returns**: `void`

**Example**:
```typescript
await invoke('set_video_resolution', { resolution: 'default' })
```

**Resolution Options**:
- `"low"`: ~100 tokens/second, cost-effective
- `"default"`: ~300 tokens/second, better text recognition

---

### `get_language`

Get the current application language.

**Parameters**: None

**Returns**: `string` ("en" or "zh")

**Example**:
```typescript
const lang = await invoke('get_language')
```

---

### `set_language`

Set the application language.

**Parameters**:
```typescript
{
  language: string  // "en" or "zh"
}
```

**Returns**: `void`

**Example**:
```typescript
await invoke('set_language', { language: 'en' })
```

---

## API Statistics Commands

### `get_api_statistics`

Get API request statistics for a time range.

**Parameters**:
```typescript
{
  startTime?: string  // ISO 8601 format
  endTime?: string    // ISO 8601 format
}
```

**Returns**: `ApiStatistics`
```typescript
{
  totalRequests: number
  successfulRequests: number
  failedRequests: number
  totalPromptTokens: number
  totalCompletionTokens: number
  totalTokens: number
  avgDurationMs: number | null
}
```

**Example**:
```typescript
const stats = await invoke('get_api_statistics', {
  startTime: '2026-01-31T00:00:00Z',
  endTime: '2026-01-31T23:59:59Z'
})
```

---

## Testing Commands

### `test_video_summary`

Test video summary generation (for debugging).

**Parameters**: None

**Returns**: `string` (result message)

**Example**:
```typescript
const result = await invoke('test_video_summary')
```

**Note**: This command requires an API key to be set.

---

## Event System

Clarity uses Tauri events for reactive updates:

### `statistics-updated`

Emitted when statistics change (screenshot captured, summary generated, etc.).

**Listen**:
```typescript
import { listen } from '@tauri-apps/api/event'

const unlisten = await listen('statistics-updated', () => {
  // Refresh statistics
  loadStatistics()
})
```

**Cleanup**:
```typescript
unlisten() // Stop listening
```

---

## Error Handling

All commands may throw errors. Common error patterns:

```typescript
try {
  await invoke('command_name', { ... })
} catch (error) {
  if (error.includes('Database error')) {
    // Database issue
  } else if (error.includes('API error')) {
    // API issue
  } else if (error.includes('Permission')) {
    // Permission issue
  }
}
```

## Type Definitions

For TypeScript users, you can create type definitions:

```typescript
// types/clarity.d.ts
declare module '@tauri-apps/api/core' {
  export function invoke<T = any>(
    cmd: string,
    args?: Record<string, any>
  ): Promise<T>
}

// Usage
interface ScreenshotTrace {
  id: number
  timestamp: string
  filePath: string
  width: number
  height: number
  fileSize: number
}

const traces: ScreenshotTrace[] = await invoke('get_traces', { ... })
```

## Best Practices

1. **Error Handling**: Always wrap `invoke()` calls in try-catch
2. **Loading States**: Show loading indicators for async operations
3. **Caching**: Cache results when appropriate to reduce API calls
4. **Event Listeners**: Clean up event listeners in React `useEffect` cleanup
5. **Type Safety**: Use TypeScript for better type safety

## Rate Limiting

Currently, there are no rate limits on Tauri commands. However:
- AI API calls are limited by Google Gemini API quotas
- Database operations are optimized but may be slow with large datasets
- Screenshot capture runs at 1 FPS maximum

## Future API Additions

Planned API additions:
- `export_data`: Export data in various formats
- `import_data`: Import data from backups
- `delete_screenshots`: Bulk delete screenshots
- `get_storage_usage`: Get storage statistics
- `cleanup_old_data`: Remove old data automatically
