# Security Policy

## Supported Versions

We release patches for security vulnerabilities. Which versions are eligible for receiving such patches depends on the CVSS v3.0 Rating:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability, please follow these steps:

1. **Do not** open a public GitHub issue
2. Create a private security advisory at https://github.com/crapthings/clarity/security/advisories/new
3. Provide a detailed description of the vulnerability
4. Include steps to reproduce the issue (if applicable)
5. Wait for a response before disclosing publicly

We will respond within 48 hours and work with you to address the vulnerability.

## Security Best Practices

### Privacy

Clarity is designed with privacy as a core principle:

- **All data is stored locally** on your device
- **No data is sent to external servers** except for AI processing (Google Gemini API)
- **Screenshots are stored** in platform-specific application data directories
- **Database is local** (SQLite) and never transmitted
- **API keys are stored locally** and encrypted by the operating system

### Data Storage

- Screenshots: Stored in `~/Library/Application Support/clarity/recordings/` (macOS) or equivalent
- Database: Stored in `~/Library/Application Support/clarity/clarity.db` (macOS) or equivalent
- All data remains on your local machine

### API Usage

- Clarity uses Google Gemini API for AI-powered summaries
- Video files are uploaded to Google Gemini File API for processing
- You must provide your own API key (stored locally)
- API keys are never shared or transmitted except to Google's API

### Permissions

Clarity requires the following permissions:

- **Screen Recording** (macOS/Windows/Linux): Required to capture screenshots
- **File System Access**: Required to save screenshots and database locally
- **Network Access**: Required only for Google Gemini API calls (if AI features are enabled)

### Recommendations

1. **Keep your API keys secure**: Never share your Google Gemini API key
2. **Review your data**: Regularly check the storage directory to understand what data is being stored
3. **Use strong system passwords**: Protect your device to prevent unauthorized access to local data
4. **Keep the app updated**: Security patches are included in updates

## Known Security Considerations

### Screen Recording

- Screenshots may contain sensitive information (passwords, personal data, etc.)
- Screenshots are stored as JPEG files on your local disk
- Ensure your device is secure and encrypted

### AI Processing

- Video files are sent to Google Gemini API for processing
- Review Google's privacy policy: https://ai.google.dev/terms
- Consider using low-resolution mode to minimize data sent

### Local Storage

- Database and screenshots are stored in plain text/files
- Ensure your device's disk encryption is enabled
- Be cautious when sharing your device

## Security Updates

Security updates will be released as patches to the latest version. We recommend:

- Enabling automatic updates (when available)
- Regularly checking for updates
- Reviewing release notes for security-related changes

## Contact

For security concerns, please create a private security advisory at:
https://github.com/crapthings/clarity/security/advisories/new
