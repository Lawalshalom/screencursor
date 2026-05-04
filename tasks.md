# ScreenCursor - Task Tracking

> **Project:** Hand gesture mouse control using computer vision (Tauri v2 + OpenCV + Vue 3)
> **Last Updated:** 2026-05-01
> **Status:** Active Development

## Quick Context for New Sessions

- **What it does:** Desktop app that uses webcam + computer vision to track hand gestures and convert them to mouse/keyboard input
- **Tech Stack:** Tauri v2 (Rust backend), OpenCV (hand detection via ONNX models), Vue 3 + TypeScript + Tailwind CSS (frontend)
- **Platform:** macOS (primary dev), Windows (target)
- **Key directories:**
  - `src-tauri/src/` - Rust backend (camera, hand detection, gestures, input simulation)
  - `src/` - Vue frontend (settings, calibration, about views)
  - `src-tauri/models/` - ONNX models (already downloaded)
  - `src-tauri/icons/` - App icons (basic icons exist)

## Completed Tasks

### Project Setup
- [x] Initialize Tauri v2 project with Vue 3 + TypeScript template
- [x] Configure `Cargo.toml` with required crates (tauri, opencv, tokio, serde, etc.)
- [x] Configure `package.json` with frontend dependencies
- [x] Configure `tauri.conf.json` (app name, identifier, window settings)
- [x] Create directory structure for all modules
- [x] Download ONNX models to `src-tauri/models/`
  - `palm_detection_mediapipe_2023feb.onnx` (3.9MB)
  - `handpose_estimation_mediapipe_2023feb.onnx` (4.1MB)
- [x] Create basic app icons in `src-tauri/icons/`
- [x] Create design.md with full implementation plan

### Backend (Rust)
- [x] **Camera Module** (`src-tauri/src/camera/mod.rs`)
  - Basic camera access via OpenCV VideoCapture
  - Frame capture functionality
  - Camera state management (open/close)

- [x] **Hand Detection** (`src-tauri/src/hand/detection.rs`)
  - Load ONNX model using OpenCV DNN
  - Detect palms in frame (192x192 input)
  - Parse detections and return bounding boxes
  - Basic confidence filtering (0.5 threshold)
  - Simple NMS (truncate to best detection)

- [x] **Landmark Extraction** (`src-tauri/src/hand/landmarks.rs`)
  - Load landmark ONNX model
  - Crop palm region and resize to 224x224
  - Extract 21 hand keypoints
  - Scale coordinates back to frame dimensions

- [x] **Gesture Recognition** (`src-tauri/src/gesture/`)
  - Define Gesture enum (12 gesture types)
  - Implement GestureTracker with position history
  - **Working gestures:**
    - [x] LeftClick (thumb + index pinch)
    - [x] RightClick (thumb + middle pinch)
    - [x] ScrollUp/Down/Left/Right (hand movement)
    - [x] SwipeUp/Down/Left/Right (velocity-based)
  - **Not implemented:**
    - [ ] ZoomIn / ZoomOut
    - [ ] AppSwitch
    - [ ] Screenshot

- [x] **Settings Module** (`src-tauri/src/settings/mod.rs`)
  - Settings struct with tracking_enabled, scroll_sensitivity, swipe_threshold, pinch_threshold
  - load_settings() and save_settings() functions
  - Settings file path in platform config directory

- [x] **Input Simulation - Windows** (`src-tauri/src/input/windows.rs`) - FULLY IMPLEMENTED
  - [x] `mouse_move` - normalized coordinates to 0..65535
  - [x] `mouse_click` - left/right/middle down/up
  - [x] `scroll` - vertical and horizontal scroll
  - [x] `key_sequence` - keyboard input with proper virtual key mapping
  - [x] AppSwitch: Alt+Tab mapping
  - [x] Screenshot: Win+Shift+S mapping

- [x] **Input Simulation - macOS** (`src-tauri/src/input/macos.rs`) - PARTIALLY IMPLEMENTED
  - [x] `mouse_click` - using CGEvent (CGEventType::LeftMouseDown/Up, etc.)
  - [x] `mouse_move` - using CGEvent::new_mouse_event
  - [x] `key_sequence` - using CGEvent::new_keyboard_event with CGKeyCode mapping
  - [x] AppSwitch: Cmd+Tab mapping
  - [x] Screenshot: Cmd+Shift+4 mapping
  - [ ] `scroll` - CGEventCreateScrollWheelEvent not directly exposed in core-graphics 0.24 (STUBBED)

- [x] **Tauri Commands** (`src-tauri/src/lib.rs`)
  - [x] start_tracking command - spawns background thread for continuous tracking
  - [x] stop_tracking command - sets flag to stop tracking loop
  - [x] get_settings command
  - [x] update_settings command (doesn't actually save yet)
  - [x] get_camera_preview command (returns base64 JPEG with landmarks drawn)
  - [x] Added input_sim to AppState and initialized it
  - [x] Connected gesture recognition to input simulation (simulate_gesture function)
  - [x] Implemented tracking_loop function that runs in separate thread
  - [x] Fixed build errors (base64, state borrowing, etc.)

### Frontend (Vue 3)
- [x] **App.vue** - Main app shell with navigation
  - Navigation between Main, Settings, Calibration, About views
  - Start/Stop tracking button
  - Gesture event listener (displays last gesture)

- [x] **Settings.vue** - Settings page
  - Gesture toggle switches (all 13 gestures)
  - Sensitivity sliders (scroll speed, swipe threshold, pinch threshold)
  - Save/Reset buttons

- [x] **Calibration.vue** - Calibration page
  - Camera preview canvas
  - Start/Stop preview button
  - Gesture test display
  - Calibration instructions

- [x] **About.vue** - About page
  - Version info
  - Description
  - Supported gestures list
  - Technology stack

- [x] **Styling** - Tailwind CSS configured
  - Dark theme (gray-900 background)
  - Responsive grid layouts

## Pending Tasks

### High Priority (Core Functionality)

#### 1. Fix Tray Icon (Tauri v2 API) - COMPLETED ✅
- [x] Uncomment `pub mod tray;` in `lib.rs`
- [x] Implement tray icon using `TrayIconBuilder` in `setup()` function
- [x] Create tray menu with items: "Start/Stop Tracking", "Show Settings", "Quit"
- [x] Handle tray icon click events
- [x] Handle menu events (emit events to frontend or call commands directly)

#### 2. Complete Remaining Gestures in `src-tauri/src/gesture/tracker.rs` - PARTIALLY COMPLETED
- [x] **ZoomIn / ZoomOut** - detect pinch distance change over time (implemented)
- [ ] **AppSwitch** - three-finger gesture or keyboard shortcut simulation (Cmd+Tab / Alt+Tab)
- [ ] **Screenshot** - specific gesture (fist then open) or keyboard shortcut (Cmd+Shift+4 / Win+Shift+S)
- [ ] Add gesture cooldown/debouncing in `simulate_gesture`
- [x] Make gesture thresholds configurable from settings

#### 3. Fix Build Warnings
- [ ] Remove unused constants in `src-tauri/src/gesture/tracker.rs` (RING_TIP_IDX, PINKY_TIP_IDX, SWIPE_THRESHOLD)
- [ ] Fix base64 deprecation warning - use `base64::Engine::encode` properly
- [ ] Remove unused import `base64::Engine` in `lib.rs`

#### 4. Fix Settings Integration - COMPLETED ✅
- [x] **update_settings command** (`src-tauri/src/lib.rs`)
  - Actually parse and save settings (currently just returns "Settings updated")
  - Apply settings to running state (update gesture thresholds in GestureTracker)
- [ ] **Frontend settings loading**
  - Fix `loadSettings()` in Settings.vue to actually apply loaded settings
  - Sync gesture toggles with backend
  - Show saved confirmation message properly
- [x] **Settings Config** (`src-tauri/src/settings/config.rs`)
  - Replace placeholder with actual GestureConfig struct
  - Add all gesture toggle settings
  - Add camera selection setting

#### 5. Unit Tests (Critical for Preventing Regressions) - PARTIALLY COMPLETED
- [ ] **Camera Module Tests**
  - Test camera initialization (mock or skip if no camera)
  - Test frame capture returns valid Mat
  - Test camera release on drop

- [ ] **Hand Detection Tests**
  - Test ONNX model loading (with test model path)
  - Test detect() with known test image (include test fixture)
  - Test bounding box coordinates are valid
  - Test confidence filtering
  - Test NMS logic

- [ ] **Landmark Extraction Tests**
  - Test ONNX model loading
  - Test extract() returns 21 points
  - Test coordinate scaling back to frame
  - Test with known palm crop

- [x] **Gesture Recognition Tests**
  - Test LeftClick detection (pinch distance < threshold)
  - Test RightClick detection
  - Test Scroll detection (movement accumulation)
  - Test Swipe detection (velocity calculation)
  - Test gesture state tracking (no false positives)
  - Add test fixtures (mock HandLandmarks)

- [x] **Input Simulation Tests**
  - Test InputSimulator trait methods (mock or dry-run)
  - Test platform selection (cfg macros)

- [x] **Settings Tests**
  - Test load_settings with no file (returns default)
  - Test save_settings writes valid JSON
  - Test load_settings reads saved JSON
  - Test settings round-trip

- [ ] **Frontend Tests**
  - Setup Vitest for Vue components
  - Test Settings.vue (toggle gestures, adjust sliders)
  - Test Calibration.vue (start/stop preview)
  - Test App.vue (navigation, tracking toggle)

### Medium Priority (Polish & Testing)

#### 6. Improve macOS Input Simulation - COMPLETED ✅
- [x] Implement `scroll` using CGEventCreateScrollWheelEvent or alternative method
- [ ] Get current mouse position for click events (instead of using (0,0))
- [ ] Test on macOS - verify clicks, scrolls, keys work

#### 7. Frontend Components
- [ ] **Create reusable components** (`src/components/`)
  - GestureCard.vue (gesture toggle card)
  - CameraPreview.vue (reusable camera preview component)
  - ThresholdSlider.vue (reusable slider component)
- [ ] **Improve Calibration.vue**
  - Show real-time FPS
  - Show landmark points on canvas
  - Add gesture confidence indicator
  - Add calibration wizard

#### 8. Camera Improvements
- [ ] Add camera selection dropdown (support multiple cameras)
- [ ] Add camera resolution settings
- [ ] Handle camera disconnection/reconnection
- [ ] Add FPS counter to preview

#### 9. Integration Tests
- [ ] Test full tracking pipeline (camera → detection → landmarks → gesture → input)
- [ ] Test Tauri commands end-to-end
- [ ] Test settings persistence across app restarts

#### 10. Build & CI/CD
- [ ] **GitHub Actions Workflow** (`.github/workflows/build.yml`)
  - Build on macOS (for .app and .dmg)
  - Build on Windows (for .exe and .msi)
  - Run tests in CI
  - Create releases on tags
- [ ] **Build Documentation**
  - Add build prerequisites to README
  - Document OpenCV installation steps for each platform
  - Add troubleshooting section

#### 11. Documentation
- [ ] Create README.md
  - Project description
  - Features list
  - Installation instructions
  - Usage guide (how to perform gestures)
  - Build from source instructions
  - Troubleshooting section
- [ ] Add inline code documentation (rustdoc/jsdoc)
- [ ] Create user manual (screenshots of settings, calibration)

## File Status Quick Reference

| File | Status | Notes |
|------|--------|-------|
| `src-tauri/src/lib.rs` | **Mostly Done** | Tracking loop implemented, input connected, needs tray |
| `src-tauri/src/camera/mod.rs` | Done | Works |
| `src-tauri/src/hand/detection.rs` | Done | Works |
| `src-tauri/src/hand/landmarks.rs` | Done | Works |
| `src-tauri/src/gesture/mod.rs` | Done | Structure complete |
| `src-tauri/src/gesture/gestures.rs` | Done | All 12 gestures defined |
| `src-tauri/src/gesture/tracker.rs` | Partial | 4/12 gestures implemented, warnings |
| `src-tauri/src/input/mod.rs` | Done | Platform dispatch works |
| `src-tauri/src/input/common.rs` | Done | Trait and types defined |
| `src-tauri/src/input/macos.rs` | **Mostly Done** | mouse_click, key_sequence work; scroll stubbed |
| `src-tauri/src/input/windows.rs` | **Done** | Full implementation |
| `src-tauri/src/tray.rs` | **Broken** | Needs Tauri v2 API update |
| `src-tauri/src/settings/mod.rs` | Partial | Load/save works, config stub |
| `src-tauri/src/settings/config.rs` | **TODO** | Just a placeholder |
| `src-tauri/src/utils/constants.rs` | Done | All constants defined |
| `src-tauri/src/main.rs` | Done | Calls run() |
| `src/App.vue` | Done | Navigation and tracking toggle |
| `src/views/Settings.vue` | Partial | UI done, save not connected |
| `src/views/Calibration.vue` | Partial | Preview works, no landmarks drawn |
| `src/views/About.vue` | Done | Static content |
| `src/components/` | **Empty** | No reusable components |
| `src/main.ts` | Done | Imports App.vue |
| `src/style.css` | Done | Tailwind directives |

## Next Session Action Plan

1. **Fix Tray Icon** - Use Tauri v2 `TrayIconBuilder` API in `lib.rs` setup function
2. **Complete Remaining Gestures** - Implement ZoomIn/Out, AppSwitch, Screenshot in tracker.rs
3. **Fix Build Warnings** - Clean up dead code, unused imports, deprecation warnings
4. **Implement macOS scroll** - Find proper API for scroll wheel events
5. **Write Unit Tests** - Start with camera and detection tests
6. **Update frontend** - Connect settings save to backend, show confirmation

## Test Commands

```bash
# Run Rust tests
cd src-tauri && cargo test

# Run frontend tests (once Vitest is set up)
npm run test

# Build for development
npm run tauri:dev

# Build for production
npm run tauri:build

# Build only Rust (faster iteration)
cd src-tauri && cargo build
```

## Key Reminders

- ONNX models are already downloaded in `src-tauri/models/`
- Input simulation works for Windows (full) and macOS (partial - scroll stubbed)
- Gesture recognition is connected to input simulation via `simulate_gesture()` in lib.rs
- Tracking loop spawns a background thread that processes frames continuously
- Tray icon is commented out - needs Tauri v2 API implementation
- Build has warnings (dead code constants, base64 deprecation) but compiles successfully
- No unit tests exist anywhere in the project yet
- Windows input module (`windows.rs`) is fully implemented
- macOS input module (`macos.rs`) compiles and partially works
