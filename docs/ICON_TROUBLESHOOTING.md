# Icon Troubleshooting Guide

If you're seeing the default Tauri icon instead of your custom icon in dev mode, try these steps:

## Quick Fix

1. **Stop the dev server** (Ctrl+C)

2. **Clean build artifacts**:
   ```bash
   rm -rf src-tauri/target/debug/bundle
   rm -rf src-tauri/target/debug/clarity.app
   ```

3. **Clear macOS icon cache** (macOS only):
   ```bash
   rm -rf ~/Library/Caches/com.apple.iconservices.store
   killall Finder
   ```

4. **Restart dev server**:
   ```bash
   pnpm tauri dev
   ```

## Verify Icon Files

Make sure all required icon files exist:
```bash
ls -lh src-tauri/icons/icon.* src-tauri/icons/*.png
```

Required files:
- `icon.png` (512x512)
- `icon.icns` (macOS)
- `icon.ico` (Windows)
- `32x32.png`
- `128x128.png`
- `128x128@2x.png` (256x256)

## Regenerate Icons

If icons are missing or incorrect, regenerate them:
```bash
npx @tauri-apps/cli icon public/eye.png --output src-tauri/icons
```

## macOS Specific Issues

### Icon Cache
macOS caches application icons. After updating icons:
1. Delete the app from Applications (if installed)
2. Clear icon cache: `rm -rf ~/Library/Caches/com.apple.iconservices.store`
3. Restart Finder: `killall Finder`
4. Rebuild the app

### Dock Icon
If the Dock still shows the old icon:
1. Quit the app completely
2. Remove it from Dock (right-click → Options → Remove from Dock)
3. Rebuild and relaunch

## Development vs Production

- **Dev mode**: Uses icons from `src-tauri/icons/` directory
- **Production build**: Icons are bundled into the app package

If icons work in production but not in dev, it's likely a cache issue.

## Verify Configuration

Check `src-tauri/tauri.conf.json`:
```json
{
  "bundle": {
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

Paths are relative to `src-tauri/` directory.
