# Privacy Policy

**Last Updated**: January 31, 2026

## Overview

Clarity is designed with privacy as a core principle. This document explains how Clarity handles your data and what information is collected, stored, or transmitted.

## Data Storage

### Local Storage Only

**All data is stored locally on your device:**

- **Screenshots**: Stored in platform-specific directories:
  - macOS: `~/Library/Application Support/clarity/recordings/`
  - Windows: `%LOCALAPPDATA%/clarity/recordings/`
  - Linux: `~/.local/share/clarity/recordings/`

- **Database**: SQLite database stored locally:
  - macOS: `~/Library/Application Support/clarity/clarity.db`
  - Windows: `%LOCALAPPDATA%/clarity/clarity.db`
  - Linux: `~/.local/share/clarity/clarity.db`

- **Settings**: All application settings stored in local database

**No data is uploaded to any cloud service or external server.**

## External Services

### Google Gemini API

**What is sent:**
- Video files (created from your screenshots) for AI analysis
- Your custom AI prompts
- API requests for generating summaries

**What is NOT sent:**
- Raw screenshots
- Personal files or documents
- Browsing history
- Any other personal data

**Your Control:**
- You provide your own Google Gemini API key
- You can disable AI features entirely
- You can customize prompts to exclude sensitive information
- You control which videos are sent for processing

**Data Handling:**
- Videos are uploaded to Google Gemini File API
- Google processes videos according to their [Privacy Policy](https://policies.google.com/privacy)
- Processed videos may be stored by Google according to their retention policies
- You can delete files via Google AI Studio if needed

**Important**: When you use Google Gemini API, you are subject to Google's privacy policy and terms of service. Clarity does not have access to or control over how Google handles your data.

## Data Collection

### What We Collect

**Nothing.** Clarity does not collect, transmit, or store any data on external servers.

### What You Store Locally

- Screenshots of your screen activity
- AI-generated summaries
- Application settings
- API request logs (for debugging)

### Analytics

**Clarity does not include any analytics or tracking code.**

## Data Sharing

**We do not share your data with anyone.**

- No third-party analytics
- No advertising networks
- No data brokers
- No cloud sync (unless you explicitly configure it in the future)

## Data Security

### Local Security

- Screenshots stored as regular files (protected by OS file permissions)
- Database encrypted by SQLite (if encryption enabled)
- API keys stored in local database (encrypted by Tauri)

### Best Practices

1. **Secure Your Device**: Use strong passwords and encryption
2. **Control API Keys**: Use your own Google Gemini API key
3. **Review Settings**: Regularly review your application settings
4. **Delete Data**: You can delete screenshots and database at any time

## Your Rights

### Access

You have full access to all your data:
- Screenshots: Browse files in storage directory
- Database: Use SQLite tools to inspect database
- Settings: View and modify in Settings page

### Deletion

You can delete your data at any time:
- Delete screenshots: Remove files from storage directory
- Delete database: Delete `clarity.db` file
- Reset settings: Use "Reset to Default" options

### Export

Currently, data export features are planned for future releases. Until then, you can:
- Copy screenshot files manually
- Export database using SQLite tools
- Copy summary text from the UI

## Open Source

Clarity is open source. You can:
- Review all source code
- Verify privacy claims
- Modify the code to suit your needs
- Report privacy concerns via GitHub Issues

## Changes to This Policy

We will notify users of any material changes to this privacy policy by:
- Updating this document
- Posting in GitHub repository
- Including in release notes

## Contact

For privacy concerns or questions:
- **GitHub Issues**: [Create an issue](https://github.com/crapthings/clarity/issues)
- **Security Advisories**: [Report security issues](https://github.com/crapthings/clarity/security/advisories/new)

## Compliance

### GDPR

If you are in the EU:
- All data stored locally (no cross-border transfers)
- You have full control over your data
- You can delete data at any time
- No third-party data sharing

### CCPA

If you are in California:
- No sale of personal information
- No sharing of personal information
- Full access to your data
- Right to deletion

## Third-Party Services

### Google Gemini API

When you use AI features, you interact with Google's services:
- [Google Privacy Policy](https://policies.google.com/privacy)
- [Google AI Terms of Service](https://ai.google.dev/terms)
- [Google AI Data Usage](https://ai.google.dev/gemini-api/docs/data-usage)

**We recommend reviewing Google's privacy policy before using AI features.**

## Recommendations

1. **Use Your Own API Key**: Never share API keys
2. **Review AI Prompts**: Ensure prompts don't include sensitive information
3. **Monitor API Usage**: Check API request logs regularly
4. **Secure Storage**: Use encrypted storage for sensitive screenshots
5. **Regular Cleanup**: Delete old screenshots periodically

## Disclaimer

While Clarity is designed to be privacy-first:
- **You are responsible** for securing your device
- **You are responsible** for managing your API keys
- **You are responsible** for what data you choose to analyze with AI
- **We cannot guarantee** absolute privacy if your device is compromised

## Summary

**Clarity's Privacy Promise:**
- ✅ All data stored locally
- ✅ No cloud sync or backup
- ✅ No analytics or tracking
- ✅ No data sharing
- ✅ Open source (verifiable)
- ⚠️ AI features require Google Gemini API (you control)

**Your Privacy, Your Control.**
