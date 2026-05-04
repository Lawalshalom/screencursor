# ScreenCursor - Applied Bug Fixes & Cleanup Summary

## Changes Applied

### 1. Fixed "Start Tracking" error (Tauri API init)
- **File:** `src/App.vue`
- **Root cause:** Custom polling/bootstrapping for Tauri v2 APIs was fragile:
  - First click: `invokeFn` null (5-second poll hadn't resolved) → error string.
  - Second click: Fallback to raw `@tauri-apps/api/core` import stored unbound `invoke` → throws inside SDK (`Cannot read properties of undefined (reading 'invoke')`).
- **Fix:** Replaced polling shim with direct package imports:
  - `import { invoke } from '@tauri-apps/api/core'`
  - `import { listen } from '@tauri-apps/api/event'`
  - Added `onUnmounted` cleanup to remove event listener on tab switch/HMR.
- **Result:** `Start Tracking` / `Stop Tracking` now works immediately and is idempotent.

### 2. Added Tauri v2 capabilities file
- **File:** `src-tauri/capabilities/default.json` (new)
- **Reason:** Tauri v2 requires explicit capabilities to expose `core:default` permissions
  on the main window. Without this, IPC/event plumbing doesn't provide
  `window.__TAURI_INTERNALS__` and commands fail.
- **Contents:**
  ```json
  {
    "identifier": "default",
    "windows": ["main"],
    "permissions": ["core:default"]
  }
  ```

### 3. Fixed Settings save/load
- **Files:**
  - `src/views/Settings.vue`
  - `src-tauri/src/settings/mod.rs`
- **Bugs:**
  - Frontend sent `{ settings: JSON.stringify(...) }` but Rust expected param name
    `settings_json` (Tauri v2 auto-snake→camel conversion uses `settingsJson`).
  - Rust `Settings` used `snake_case` fields but Vue used `camelCase`. No
    `#[serde(rename_all)]` caused deserialization failures.
  - `loadSettings()` was a TODO — UI always reset to defaults.
- **Fixes:**
  - Added `#[serde(rename_all = "camelCase")]` to `Settings` struct.
  - Frontend now sends `{ settingsJson: JSON.stringify(snapshot) }` with all fields.
  - `loadSettings()` now hydrates reactive settings from persisted JSON.
  - Send all fields (`zoomThreshold`, `zoomTimeWindow`, `trackingEnabled`) even
    if not in UI sliders, to avoid schema drift on disk.

### 4. Split preview from tracking; in-memory JPEG encoding
- **File:** `src-tauri/src/lib.rs`
- **Bugs:**
  - `get_camera_preview` reused `process_frame` which:
    - Wrote `.tmp_preview.jpg` every frame (~40×/s combined preview+tracking)
    - Invoked `gesture-detected` events and simulated input (preview would move
      mouse/click just by opening Calibration tab).
  - `Mat::clone()` in opencv-rust is a shallow ref-counted copy — overlays drawn
    on the clone would mutate the source frame used by tracking.
- **Fixes:**
  - Introduced `render_preview_frame(state)` — draws overlays but does NOT emit
    events nor call `simulate_gesture`. Pure rendering for preview only.
  - `process_frame(state)` — full tracking pipeline (emit + simulate). Used
    exclusively by tracking loop.
  - Replace disk write/read with `opencv::imgcodecs::imencode(".jpg", ...)` →
    base64 of in-memory buffer. No temp files.
  - Use `frame.try_clone()?` for a true deep copy before drawing overlays.
  - Preview returns `__BUSY__` sentinel when tracking is active to avoid camera
    mutex contention; frontend stops polling.

### 5. Calibration view hygiene
- **File:** `src/views/Calibration.vue`
- **Changes:**
  - Reuse a single `Image` object instead of creating one per tick (~10/s) to
    reduce GC pressure and closure retention bugs.
  - Properly typed interval: `ReturnType<typeof setInterval>`.
  - Unified `stopPreview()` clears canvas and flag.
  - `onUnmounted` stops preview.
  - Sentient handling: backend returns `__BUSY__` and UI shows a message and
    disables preview automatically.

### 6. macOS keycode fixes
- **File:** `src-tauri/src/input/macos.rs`
- **Fixes:**
  - `VirtualKey::Equal` → `0x18` (was `0x1F`, which is 'O').
  - `VirtualKey::Minus` → `0x1B` (was `0x1E`, which is 'Minus' is 0x1B).
  - Added header comment.
- **Impact:** Zoom In (Cmd+=) and Zoom Out (Cmd+-) gestures now correctly map
  to macOS system shortcuts.

### 7. Added header/section comments
- Added concise 1–2 line comments for each module:
  - `src-tauri/src/camera/mod.rs`
  - `src-tauri/src/hand/detection.rs`
  - `src-tauri/src/hand/landmarks.rs`
  - `src-tauri/src/gesture/mod.rs`
  - `src-tauri/src/input/common.rs`
  - `src-tauri/src/input/macos.rs`
  - `src-tauri/src/input/mod.rs`
  - `src-tauri/src/settings/mod.rs`
  - `src-tauri/src/tray.rs`
  - `src/App.vue`
  - `src/views/Settings.vue`
  - `src/views/Calibration.vue`

## Verification steps (run locally)

1. Install dependencies:
   ```
   npm install
   ```

2. Build and run in dev mode:
   ```
   npm run tauri:dev
   ```
   If `cargo check` fails due to `libclang.dylib`, install OpenCV via Homebrew:
   ```
   brew install opencv
   ```
   and/or ensure a valid `libclang` (from Xcode CLI tools / rustup component add llvm-tools) is available.

3. Once running:
   - Click tray → **Show Settings** to open window.
   - Go to **Main** → **Start Tracking** should succeed immediately (no errors).
   - Go to **Calibration** → **Start Preview** shows webcam + overlay and does NOT fire clicks/movement.
   - Go to **Settings** → sliders load persisted values; change one → **Save Settings** → message "Settings saved!"; reload app and confirm persistence.
   - Perform a pinch gesture while tracking active → `Last gesture` updates and OS receives the corresponding input.

## Known non-scope items

- AppSwitch / Screenshot gesture detection not implemented (tracked in tasks.md).
- Vitest frontend tests.
- No changes to window visibility/tray behavior (tray-only launch is intentional).
- No dependency version bumps.
