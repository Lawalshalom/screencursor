# ScreenCursor - Production Deployment Guide

This guide covers building, signing, and distributing ScreenCursor for macOS and Windows.

## Prerequisites

### macOS
- macOS 11.0+ (Big Sur or later)
- Xcode command line tools: `xcode-select --install`
- Apple Developer account (for code signing and notarization)
- Valid Developer ID certificate
- App-specific password for notarization

### Windows
- Windows 10/11
- Visual Studio 2019+ with C++ build tools
- Code signing certificate (optional but recommended)

---

## Building for Production

### macOS (.app and .dmg)

1. **Set environment variables** for signing and notarization:
```bash
export APPLE_ID="your-email@example.com"
export APPLE_ID_PASSWORD="xxxx-xxxx-xxxx-xxxx"  # App-specific password
export APPLE_TEAM_ID="XXXXXXXXXX"  # Your Apple Team ID
export APPLE_CERTIFICATE_NAME="Developer ID Application: Your Name (XXXXXXXXXX)"
```

2. **Update `tauri.conf.json`** with your app details:
```json
{
  "bundle": {
    "identifier": "com.yourcompany.screencursor",
    "publisher": "Your Company",
    "category": "Public.app-category.utilities",
    "longDescription": "Hand gesture mouse control using computer vision",
    "shortDescription": "Hand gesture mouse control",
    "copyright": "Copyright 2026, Your Company",
    "version": "1.0.0"
  }
}
```

3. **Build the app:**
```bash
npm run tauri:build
```

4. **Output files:**
- `src-tauri/target/release/bundle/macos/screencursor.app`
- `src-tauri/target/release/bundle/dmg/screencursor.dmg`

### Windows (.exe and .msi)

1. **Update `tauri.conf.json`** with Windows-specific settings:
```json
{
  "bundle": {
    "windows": {
      "webviewInstallMode": {
        "type": "downloadBootstrapper"
      },
      "nsis": {
        "installMode": "both",
        "displayLanguageSelector": true,
        "languages": ["SimpChinese", "English"]
      }
    }
  }
}
```

2. **(Optional) Set up code signing** in `tauri.conf.json`:
```json
{
  "bundle": {
    "windows": {
      "signCommand": "signtool sign /f \"C:\\path\\to\\certificate.pfx\" /p \"certificate-password\" /tr http://timestamp.digicert.com /td sha256 /fd sha256 \"$1\""
    }
  }
}
```

3. **Build the app:**
```bash
npm run tauri:build
```

4. **Output files:**
- `src-tauri/target/release/bundle/nsis/screencursor.exe` (installer)
- `src-tauri/target/release/bundle/msi/screencursor.msi`

---

## Code Signing

### macOS Code Signing

1. **Request a Developer ID certificate:**
   - Go to [Apple Developer Portal](https://developer.apple.com)
   - Certificates > + > Developer ID Application
   - Download and install the certificate in Keychain Access

2. **Sign the app** (automatic with Tauri if configured):
```bash
# Manual signing if needed
codesign --force --deep --sign "Developer ID Application: Your Name (XXXXXXXXXX)" \
  "src-tauri/target/release/bundle/macos/screencursor.app"
```

3. **Verify signing:**
```bash
codesign --verify --deep --strict "src-tauri/target/release/bundle/macos/screencursor.app"
codesign --display --verbose=2 "src-tauri/target/release/bundle/macos/screencursor.app"
```

### macOS Notarization

Required for Gatekeeper to allow your app to run without warnings:

```bash
# Submit for notarization
xcrun notarytool submit \
  "src-tauri/target/release/bundle/dmg/screencursor.dmg" \
  --apple-id "$APPLE_ID" \
  --team-id "$APPLE_TEAM_ID" \
  --password "$APPLE_ID_PASSWORD" \
  --wait

# Staple the ticket to the DMG
xcrun stapler staple "src-tauri/target/release/bundle/dmg/screencursor.dmg"
```

### Windows Code Signing

1. **Get a code signing certificate** from a CA (DigiCert, Sectigo, etc.)
2. **Sign the executables:**
```powershell
# Sign the installer
signtool sign /f "C:\path\to\certificate.pfx" /p "password" \
  /tr http://timestamp.digicert.com /td sha256 /fd sha256 \
  "src-tauri/target/release/bundle/nsis/screencursor.exe"

# Sign the MSI
signtool sign /f "C:\path\to\certificate.pfx" /p "password" \
  /tr http://timestamp.digicert.com /td sha256 /fd sha256 \
  "src-tauri/target/release/bundle/msi/screencursor.msi"
```

---

## Distribution

### Option 1: GitHub Releases (Recommended)

1. **Create a GitHub repository** and push your code.

2. **Create a release:**
```bash
# Tag the release
git tag -a v1.0.0 -m "Initial release"
git push origin v1.0.0
```

3. **Upload build artifacts** to GitHub Releases:
   - `screencursor.dmg` (macOS)
   - `screencursor.exe` (Windows installer)
   - `screencursor.msi` (Windows MSI)

4. **Update `tauri.conf.json`** for updater (optional):
```json
{
  "updater": {
    "active": true,
    "endpoints": [
      "https://github.com/yourusername/screencursor/releases/latest/download/latest.json"
    ]
  }
}
```

### Option 2: Self-Hosted

Host the files on your own server:

1. **Upload files** to your server:
```
https://yourdomain.com/downloads/screencursor-v1.0.0.dmg
https://yourdomain.com/downloads/screencursor-v1.0.0.exe
https://yourdomain.com/downloads/screencursor-v1.0.0.msi
```

2. **Create a download page** with links to the files.

### Option 3: Homebrew Cask (macOS only)

Create a Homebrew cask for easy installation:

1. **Fork** [homebrew-cask](https://github.com/Homebrew/homebrew-cask).

2. **Create the cask file** `Casks/screencursor.rb`:
```ruby
cask "screencursor" do
  version "1.0.0"
  sha256 "SHA256_OF_YOUR_DMG_FILE"

  url "https://github.com/yourusername/screencursor/releases/download/v#{version}/screencursor.dmg"
  name "ScreenCursor"
  desc "Hand gesture mouse control using computer vision"
  homepage "https://github.com/yourusername/screencursor"

  app "screencursor.app"
end
```

3. **Submit a pull request** to homebrew-cask.

---

## GitHub Actions CI/CD (Optional)

Create `.github/workflows/build.yml` for automated builds:

```yaml
name: Build and Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '18'

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install OpenCV
        run: brew install opencv

      - name: Install dependencies
        run: npm install

      - name: Build Tauri app
        env:
          APPLE_ID: ${{ secrets.APPLE_ID }}
          APPLE_ID_PASSWORD: ${{ secrets.APPLE_ID_PASSWORD }}
          APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
        run: npm run tauri:build

      - name: Upload DMG
        uses: actions/upload-artifact@v4
        with:
          name: screencursor-dmg
          path: src-tauri/target/release/bundle/dmg/*.dmg

  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '18'

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install dependencies
        run: npm install

      - name: Build Tauri app
        run: npm run tauri:build

      - name: Upload EXE
        uses: actions/upload-artifact@v4
        with:
          name: screencursor-exe
          path: src-tauri/target/release/bundle/nsis/*.exe

      - name: Upload MSI
        uses: actions/upload-artifact@v4
        with:
          name: screencursor-msi
          path: src-tauri/target/release/bundle/msi/*.msi

  release:
    needs: [build-macos, build-windows]
    runs-on: ubuntu-latest
    steps:
      - name: Download all artifacts
        uses: actions/download-artifact@v4

      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          files: |
            screencursor-dmg/*.dmg
            screencursor-exe/*.exe
            screencursor-msi/*.msi
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

---

## Checklist for First Release

- [ ] Update version in `package.json` and `src-tauri/Cargo.toml`
- [ ] Update version in `tauri.conf.json`
- [ ] Test the app on clean macOS and Windows systems
- [ ] Set up code signing certificates
- [ ] Build for both platforms
- [ ] Sign and notarize macOS build
- [ ] Sign Windows build (optional)
- [ ] Create GitHub release with all artifacts
- [ ] Update download links in README.md
- [ ] Test installation from the release artifacts
- [ ] Announce the release

---

## Important Notes

1. **macOS Gatekeeper:** Without notarization, users will see a warning and need to right-click > Open to run the app.

2. **Windows SmartScreen:** Without code signing, users will see a "Windows protected your PC" warning.

3. **Camera Permissions:** The app needs camera access. On macOS, the user may need to grant permission in System Settings > Privacy & Security > Camera.

4. **OpenCV Dependency:** The built app bundles OpenCV, but ensure the `opencv` crate is configured correctly for static linking in `Cargo.toml`.

5. **Testing:** Always test the production build (not just dev mode) before releasing.
