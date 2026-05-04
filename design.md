# ScreenCursor Implementation Plan

## Context
The user wants to build a cross-platform (macOS/Windows) desktop application called ScreenCursor that uses computer vision to track hand gestures for mouse and system input control. The core need is to enable hands-free computer interaction via webcam-based hand tracking, replacing or augmenting traditional mouse/keyboard input with gestures (click, swipe, zoom, scroll, app switch, screenshot). The app must be compileable to native executables for both platforms using Tauri, with OpenCV + custom pre-trained models for hand detection/landmark extraction, a system tray-based UI with a separate settings window, and no main dashboard. This plan addresses the user's explicit tech stack choices (Tauri, OpenCV + Custom Models, System Tray + Settings) and required features, providing a phased implementation approach to deliver the app.

## 1. Tech Stack Breakdown

### Rust Crates (src-tauri/Cargo.toml)

**Core Tauri & Runtime:**
- `tauri` v2.0+ (features: `["tray-icon", "image-png", "devtools"]`)
- `tauri-build` (build dependencies)
- `tokio` (async runtime for camera frame processing)
- `serde` + `serde_json` (settings serialization)

**Computer Vision:**
- `opencv` v4.x (with DNN module enabled; features: `["dnn", "imgproc", "videoio"]`)

**Platform-Specific Input Simulation:**
- **Windows:** `windows` v0.56+ (features: `["Win32_UI_Input_KeyboardAndMouse", "Win32_Foundation"]`)
- **macOS:** `core-graphics` v0.24+ (for CGEvent), `core-foundation` v0.9+

**Utilities:**
- `directories` (platform-specific config/data directories)
- `once_cell` or `lazy_static` (global state for camera/hand detector)
- `thiserror` (error handling)

### npm Packages (package.json)

- `typescript` v5+
- `vite` v5+ (bundler)
- `vue` v3+ or `svelte` v4+ (lightweight UI framework for settings window)
- `tailwindcss` v3+ (styling)
- `@tauri-apps/cli` v2+ (Tauri CLI)
- `@tauri-apps/api` v2+ (frontend API bindings)

### External Models (ONNX Format)

| Model | Source | Purpose | Input Size |
|-------|--------|---------|------------|
| `palm_detection_mediapipe_2023feb.onnx` | [OpenCV Zoo - Palm Detection](https://github.com/opencv/opencv_zoo/tree/main/models/palm_detection_mediapipe) or [Hugging Face](https://huggingface.co/opencv/palm_detection_mediapipe) | Detect palms in frame | 192x192 |
| `handpose_estimation_mediapipe_2023feb.onnx` | [OpenCV Zoo - Hand Pose](https://github.com/opencv/opencv_zoo/tree/main/models/hand_pose_estimation_mediapipe) | 21-point landmark extraction | 224x224 |

**Model Download Commands:**
```bash
# Using git lfs to clone OpenCV Zoo
git clone https://github.com/opencv/opencv_zoo.git
# Models located in:
# opencv_zoo/models/palm_detection_mediapipe/
# opencv_zoo/models/hand_pose_estimation_mediapipe/
```

---

## 2. Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Tauri Shell (Rust)                       │
│  ┌──────────┐  ┌───────────┐  ┌──────────┐  ┌─────────┐  │
│  │  Camera  │─▶│  Hand     │─▶│ Gesture  │─▶│  Input  │  │
│  │  Module  │  │  Detection│  │ Recognizer│  │ Simulator│ │
│  └──────────┘  └───────────┘  └──────────┘  └─────────┘  │
│        │               │              │               │     │
│        └───────────────┴──────────────┴───────────────┘     │
│                       │                                     │
│              ┌────────▼────────┐                            │
│              │  Tauri Commands │                            │
│              └─────────────────┘                            │
│                       │                                     │
│        ┌──────────────┴──────────────┐                     │
│        │         System Tray          │                     │
│        └─────────────────────────────┘                     │
└─────────────────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│              Web Frontend (Settings Window)                  │
│  ┌──────────┐  ┌───────────┐  ┌──────────┐                │
│  │ Calibra- │  │  Gesture  │  │  Camera  │                │
│  │ tion UI  │  │  Settings │  │  Preview │                │
│  └──────────┘  └───────────┘  └──────────┘                │
└─────────────────────────────────────────────────────────────┘
```

### Module Descriptions

| Module | File Path | Responsibility |
|--------|-----------|----------------|
| **Camera Module** | `src-tauri/src/camera/mod.rs` | Webcam access via OpenCV VideoCapture, frame capture loop |
| **Hand Detection** | `src-tauri/src/hand/detection.rs` | Load ONNX model, run inference, return palm bounding boxes |
| **Landmark Extraction** | `src-tauri/src/hand/landmarks.rs` | Load landmark ONNX model, extract 21 hand keypoints |
| **Gesture Recognition** | `src-tauri/src/gesture/mod.rs` | Analyze landmark sequences, classify gestures |
| **Input Simulation** | `src-tauri/src/input/mod.rs` | Platform-specific mouse/keyboard event synthesis |
| **Tauri Backend** | `src-tauri/src/lib.rs` | Commands, state management, tray setup |
| **Web Frontend** | `src/` | Settings window (Vue/Svelte + Tailwind) |
| **System Tray** | `src-tauri/src/tray.rs` | Tray icon, context menu, show/hide settings |

---

## 3. Implementation Phases

### Phase 1: Project Scaffolding
**Dependencies:** None

1. Initialize Tauri project in current directory:
   ```bash
   npm create tauri-app@latest . -- --template vue-ts
   ```
2. Configure `src-tauri/Cargo.toml` with required crates
3. Configure `tauri.conf.json` (app name, identifier, window settings)
4. Set up directory structure for modules
5. Download ONNX models to `src-tauri/models/`

### Phase 2: Camera Module
**Dependencies:** Phase 1, OpenCV installed

1. Create `src-tauri/src/camera/mod.rs` with:
   - `Camera` struct wrapping `opencv::videoio::VideoCapture`
   - `new()` → open default camera (index 0)
   - `capture_frame()` → returns `Mat` (frame)
   - `is_opened()` → check camera availability
2. Implement frame streaming using Tokio task
3. Test: Print frame dimensions to verify camera access

### Phase 3: Hand Detection (OpenCV DNN)
**Dependencies:** Phase 2, ONNX models downloaded

1. Create `src-tauri/src/hand/detection.rs`:
   - `HandDetector` struct with `opencv::dnn::Net`
   - `new(model_path: &str)` → load ONNX via `opencv::dnn::read_net_from_onnx()`
   - `detect(&self, frame: &Mat)` → returns `Vec<PalmDetection>`
   - Preprocess: resize to 192x192, convert to blob via `blob_from_image()`
   - Postprocess: parse detection output, apply NMS (non-maximum suppression)
2. Define `PalmDetection` struct (bbox, confidence)
3. Test with static image first, then with camera frames

### Phase 4: Landmark Extraction
**Dependencies:** Phase 3

1. Create `src-tauri/src/hand/landmarks.rs`:
   - `LandmarkExtractor` struct with `opencv::dnn::Net`
   - `new(model_path: &str)` → load landmark ONNX model
   - `extract(&self, frame: &Mat, palm: &PalmDetection)` → returns `HandLandmarks`
   - Crop and resize palm region to 224x224, create blob, run forward pass
   - Parse output: 21 keypoints (x, y) normalized to frame coordinates
2. Define `HandLandmarks` struct with 21 `Point2f` values
3. Test: visualize landmarks on frame (draw circles)

### Phase 5: Gesture Recognition
**Dependencies:** Phase 4

1. Create `src-tauri/src/gesture/mod.rs`:
   - `GestureRecognizer` struct with gesture state tracking
   - `recognize(landmarks: &HandLandmarks)` → returns `Option<Gesture>`

2. Define `Gesture` enum:
   ```rust
   pub enum Gesture {
       LeftClick,         // Thumb-index pinch
       RightClick,        // Thumb-middle pinch (or hold)
       ScrollUp,          // Hand moving up
       ScrollDown,        // Hand moving down
       ScrollLeft,        // Hand moving left
       ScrollRight,       // Hand moving right
       SwipeUp,           // Quick upward motion
       SwipeDown,         // Quick downward motion
       SwipeLeft,         // Quick left motion
       SwipeRight,        // Quick right motion
       ZoomIn,            // Two-finger spread (or pinch out)
       ZoomOut,           // Two-finger pinch
       AppSwitch,         // Three-finger swipe up
       Screenshot,        // Specific gesture (e.g., fist then open)
   }
   ```

3. Implement gesture detection logic:
   - **Pinch detection:** Distance between thumb tip (4) and index tip (8) < threshold
   - **Scroll:** Track landmark[0] (wrist) movement over time
   - **Swipe:** Velocity of wrist movement exceeds threshold
   - **Zoom:** Distance between two hand landmarks changing (or two hands)
   - **App Switch / Screenshot:** Combination of landmarks or timed gestures

4. Add calibration settings (thresholds stored in config)

### Phase 6: Input Simulation (Platform-Specific)
**Dependencies:** Phase 5

1. Create `src-tauri/src/input/mod.rs` with platform conditional compilation:
   ```rust
   #[cfg(target_os = "macos")]
   mod macos;
   #[cfg(target_os = "windows")]
   mod windows;
   ```

2. **macOS Implementation** (`src-tauri/src/input/macos.rs`):
   - Use `core-graphics` crate for `CGEvent`
   - `mouse_move(x, y)` → `CGEvent::new_mouse_event(MouseMoved)`
   - `mouse_click(button: MouseButton, action: ClickAction)` → `LeftMouseDown`/`LeftMouseUp`
   - `scroll(dx, dy)` → `CGEvent::new_scroll_wheel`
   - `key_sequence(keys: &[KeyCode])` → `CGEvent::new_keyboard()` for shortcuts
   - App Switch: Cmd+Tab → use `CGEvent` with appropriate key codes
   - Screenshot: Cmd+Shift+4

3. **Windows Implementation** (`src-tauri/src/input/windows.rs`):
   - Use `windows` crate (`Windows::Win32::UI::Input::KeyboardAndMouse`)
   - `mouse_move(x, y)` → `SendInput()` with `MOUSEINPUT` struct
   - `mouse_click(button, action)` → `SendInput()` with `MOUSEEVENTF_LEFTDOWN/UP`
   - `scroll(dx, dy)` → `SendInput()` with `MOUSEEVENTF_WHEEL`
   - `key_sequence(keys)` → `SendInput()` with `KEYBDINPUT` for shortcuts
   - App Switch: Alt+Tab → `VK_MENU` + `VK_TAB`
   - Screenshot: Win+Shift+S → `VK_LWIN` + `VK_SHIFT` + `VK_S`

4. Define common trait:
   ```rust
   pub trait InputSimulator {
       fn mouse_move(&self, x: i32, y: i32);
       fn mouse_click(&self, button: MouseButton, action: ClickAction);
       fn scroll(&self, dx: i32, dy: i32);
       fn key_sequence(&self, keys: &[VirtualKey]);
   }
   ```

### Phase 7: Tauri Backend Integration
**Dependencies:** Phases 2-6

1. Update `src-tauri/src/lib.rs`:
   - Define application state struct:
     ```rust
     struct AppState {
         camera: Mutex<Option<Camera>>,
         detector: Mutex<Option<HandDetector>>,
         landmarker: Mutex<Option<LandmarkExtractor>>,
         recognizer: Mutex<GestureRecognizer>,
         input_sim: Mutex<Box<dyn InputSimulator + Send>>,
         settings: Mutex<Settings>,
     }
     ```
   - Implement Tauri commands:
     - `start_tracking()` → spawns Tokio task for camera loop
     - `stop_tracking()` → sets stop flag, joins task
     - `get_settings()` → returns current settings
     - `update_settings(settings: Settings)` → saves to file
     - `calibrate()` → runs calibration routine
     - `get_camera_preview()` → returns base64 encoded frame with landmarks

2. Set up state management in `run()` function

### Phase 8: System Tray
**Dependencies:** Phase 7

1. Create `src-tauri/src/tray.rs`:
   - Use `TrayIconBuilder` to create tray icon
   - Menu items: "Show Settings", "Start/Stop Tracking", "Quit"
   - Load tray icon from `icons/tray-icon.png`
   - Handle menu events:
     - Show Settings → `app.handle().emit("show-settings", ())`
     - Start/Stop → toggle tracking state
   - Handle tray icon events (left click → toggle settings window)

2. Update `tauri.conf.json` to hide main window on startup:
   ```json
   "windows": [{
     "label": "main",
     "title": "ScreenCursor",
     "visible": false,
     "width": 800,
     "height": 600
   }]
   ```

### Phase 9: Web Frontend (Settings Window)
**Dependencies:** Phase 8

1. Create settings UI in `src/`:
   - **Settings Page** (`src/views/Settings.vue`):
     - Gesture toggle switches (enable/disable each gesture)
     - Sensitivity sliders (scroll speed, swipe threshold, pinch threshold)
     - Camera selection dropdown
   - **Calibration Page** (`src/views/Calibration.vue`):
     - Camera preview with landmark overlay
     - Calibration wizard (guide user through gestures)
     - Test area to verify gesture recognition
   - **About Page** (`src/views/About.vue`):
     - Version info, model info

2. Use `@tauri-apps/api` for backend communication:
   - `invoke("start_tracking")`
   - `invoke("get_settings")` / `invoke("update_settings", { settings })`
   - `event.listen("gesture-detected", callback)` for real-time feedback

3. Style with Tailwind CSS (dark theme recommended for utility app)

### Phase 10: Build & Distribution
**Dependencies:** All previous phases

1. **macOS Build:**
   ```bash
   cd src-tauri
   cargo tauri build
   # Output: src-tauri/target/release/bundle/
   # - .app (application bundle)
   # - .dmg (disk image for distribution)
   ```

2. **Windows Build** (on Windows machine or CI):
   ```bash
   cd src-tauri
   cargo tauri build
   # Output: src-tauri/target/release/bundle/
   # - .exe (NSIS installer)
   # - .msi (WiX installer)
   ```

3. **CI/CD for Cross-Platform:**
   - Use GitHub Actions with `tauri-action`
   - Matrix build for macOS and Windows

---

## 4. File Structure

```
screencursor/
├── design.md                     # Project design document
├── src/                          # Web frontend
│   ├── views/
│   │   ├── Settings.vue
│   │   ├── Calibration.vue
│   │   └── About.vue
│   ├── components/
│   │   ├── GestureCard.vue
│   │   ├── CameraPreview.vue
│   │   └── ThresholdSlider.vue
│   ├── App.vue
│   ├── main.ts
│   └── style.css
├── src-tauri/                    # Rust backend
│   ├── models/                   # ONNX models
│   │   ├── palm_detection_mediapipe_2023feb.onnx
│   │   └── handpose_estimation_mediapipe_2023feb.onnx
│   ├── icons/                    # App icons + tray icon
│   │   ├── tray-icon.png
│   │   ├── 32x32.png
│   │   └── ...
│   ├── src/
│   │   ├── lib.rs                # Tauri commands, state, app setup
│   │   ├── main.rs               # Entry point
│   │   ├── camera/
│   │   │   ├── mod.rs            # Camera module
│   │   │   └── frame.rs          # Frame processing
│   │   ├── hand/
│   │   │   ├── mod.rs
│   │   │   ├── detection.rs      # Palm detection (OpenCV DNN)
│   │   │   └── landmarks.rs      # 21-point landmark extraction
│   │   ├── gesture/
│   │   │   ├── mod.rs            # Gesture recognizer
│   │   │   ├── gestures.rs       # Gesture definitions & logic
│   │   │   └── tracker.rs        # Landmark history tracking
│   │   ├── input/
│   │   │   ├── mod.rs            # Platform dispatch
│   │   │   ├── macos.rs          # macOS input (CGEvent)
│   │   │   ├── windows.rs        # Windows input (Win32)
│   │   │   └── common.rs         # Shared types & traits
│   │   ├── tray.rs               # System tray setup
│   │   ├── settings/
│   │   │   ├── mod.rs
│   │   │   └── config.rs         # Settings persistence
│   │   └── utils/
│   │       ├── mod.rs
│   │       └── constants.rs      # Thresholds, defaults
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── build.rs
├── package.json
├── tsconfig.json
├── vite.config.ts
├── tailwind.config.js
└── README.md
```

---

## 5. Model Sourcing

### Palm Detection Model
- **Repository:** [opencv_zoo - palm_detection_mediapipe](https://github.com/opencv/opencv_zoo/tree/main/models/palm_detection_mediapipe)
- **Model File:** `palm_detection_mediapipe_2023feb.onnx`
- **Alternative:** [Hugging Face - palm_detection_mediapipe](https://huggingface.co/opencv/palm_detection_mediapipe)
- **Quantized Version:** `palm_detection_mediapipe_2023feb_int8bq.onnx` (smaller, faster)

### Hand Landmark Model
- **Repository:** [opencv_zoo - hand_pose_estimation_mediapipe](https://github.com/opencv/opencv_zoo/tree/main/models/hand_pose_estimation_mediapipe)
- **Model File:** `handpose_estimation_mediapipe_2023feb.onnx`
- **Output:** 21 keypoints (3D: x, y, z per point)

### Download Script
Create `download_models.sh` in `src-tauri/`:
```bash
#!/bin/bash
cd "$(dirname "$0")/models"
# Download palm detection model
curl -L -o palm_detection_mediapipe_2023feb.onnx \
  "https://huggingface.co/opencv/palm_detection_mediapipe/resolve/main/palm_detection_mediapipe_2023feb.onnx"
# Download hand landmark model
curl -L -o handpose_estimation_mediapipe_2023feb.onnx \
  "https://huggingface.co/opencv/handpose_estimation_mediapipe/resolve/main/handpose_estimation_mediapipe_2023feb.onnx"
echo "Models downloaded successfully."
```

---

## 6. Platform-Specific Code

### macOS Input Simulation (`src-tauri/src/input/macos.rs`)

```rust
#[cfg(target_os = "macos")]
use core_graphics::event::{
    CGEvent, CGEventFlags, CGKeyCode, CGMouseButton,
    EventType, CGEventSource, CGEventSourceStateID
};

pub struct MacOSInputSimulator;

impl InputSimulator for MacOSInputSimulator {
    fn mouse_click(&self, button: MouseButton, action: ClickAction) {
        let event_type = match (button, action) {
            (MouseButton::Left, ClickAction::Down) => EventType::LeftMouseDown,
            (MouseButton::Left, ClickAction::Up) => EventType::LeftMouseUp,
            (MouseButton::Right, ClickAction::Down) => EventType::RightMouseDown,
            (MouseButton::Right, ClickAction::Up) => EventType::RightMouseUp,
            _ => return,
        };
        // Create CGEvent and post to HID event tap
    }

    fn key_sequence(&self, keys: &[VirtualKey]) {
        // Cmd+Tab for app switch:
        // VK_COMMAND down, VK_TAB down, VK_TAB up, VK_COMMAND up
        // For screenshot: VK_COMMAND down, VK_SHIFT down, VK_4 down, ...
    }
}
```

### Windows Input Simulation (`src-tauri/src/input/windows.rs`)

```rust
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::*;

pub struct WindowsInputSimulator;

impl InputSimulator for WindowsInputSimulator {
    fn mouse_click(&self, button: MouseButton, action: ClickAction) {
        let flags = match (button, action) {
            (MouseButton::Left, ClickAction::Down) => MOUSEEVENTF_LEFTDOWN,
            (MouseButton::Left, ClickAction::Up) => MOUSEEVENTF_LEFTUP,
            (MouseButton::Right, ClickAction::Down) => MOUSEEVENTF_RIGHTDOWN,
            (MouseButton::Right, ClickAction::Up) => MOUSEEVENTF_RIGHTUP,
            _ => return,
        };
        unsafe {
            let mut input = INPUT::default();
            input.r#type = INPUT_MOUSE;
            input.Anonymous.mi.dwFlags = flags;
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }

    fn key_sequence(&self, keys: &[VirtualKey]) {
        // Alt+Tab for app switch:
        // VK_MENU down, VK_TAB down, VK_TAB up, VK_MENU up
        // For screenshot: VK_LWIN + VK_SHIFT + VK_S
    }
}
```

---

## 7. Build & Distribute

### Prerequisites

**macOS:**
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Install OpenCV (via Homebrew)
brew install opencv
# Set environment variables
export OPENCV_LINK_LIBS="opencv_world"
export OPENCV_INCLUDE_PATHS="$(brew --prefix opencv)/include"
export OPENCV_LIBS_PATHS="$(brew --prefix opencv)/lib"
# Install Tauri CLI
cargo install tauri-cli --version "^2"
```

**Windows:**
```powershell
# Install Rust (from rustup.rs)
# Install OpenCV: download from https://opencv.org/releases/ and set OPENCV_DIR
# Install Visual Studio with C++ desktop development workload
# Install Tauri CLI
cargo install tauri-cli --version "^2"
```

### Build Commands

```bash
# Development mode (with dev tools)
npm run tauri dev

# Production build
npm run tauri build

# Output locations:
# macOS: src-tauri/target/release/bundle/macos/ScreenCursor.app
#         src-tauri/target/release/bundle/dmg/ScreenCursor.dmg
# Windows: src-tauri/target/release/bundle/nsis/ScreenCursor.exe
#          src-tauri/target/release/bundle/msi/ScreenCursor.msi
```

### CI/CD (GitHub Actions)

Create `.github/workflows/build.yml`:
```yaml
name: Build Release
on:
  push:
    tags: ['v*']

jobs:
  release:
    strategy:
      fail-fast: false
      matrix:
        platform: [macos-latest, windows-latest]
    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: tauri-apps/tauri-action@v0
        with:
          tagName: v__VERSION__
          releaseName: "ScreenCursor v__VERSION__"
```

---

## 8. Verification

### Module Testing

| Module | Test Method | Expected Result |
|--------|-------------|-----------------|
| Camera | Run `camera::test::capture_frame()` | Frame dimensions printed, no errors |
| Hand Detection | Run with static test image containing hand | Bounding box coordinates returned |
| Landmarks | Run on detected palm region | 21 points visualized on image |
| Gesture Recognition | Perform gestures in front of camera | Correct gesture enum returned |
| Input Simulation (macOS) | Call `mouse_click(Left, Down)` | Left click registered by OS |
| Input Simulation (Windows) | Call `key_sequence([Alt, Tab])` | App switcher appears |
| Settings Persistence | Save/load settings JSON | Values persist across restarts |
| Tray Icon | Start app | Icon appears in system tray |
| Settings Window | Click "Show Settings" | Window appears with UI |

### End-to-End Testing Checklist

1. **Launch App:** System tray icon appears, no main window
2. **Start Tracking:** Click tray menu → Start Tracking
3. **Gesture Tests:**
   - [ ] Pinch thumb+index → Left click at cursor position
   - [ ] Hand moving up/down → Vertical scroll
   - [ ] Hand moving left/right → Horizontal scroll
   - [ ] Quick swipe up → Swipe up action
   - [ ] Spread two fingers → Zoom in
   - [ ] Pinch two fingers → Zoom out
   - [ ] Three-finger gesture → App switch (Alt+Tab / Cmd+Tab)
   - [ ] Specific gesture → Screenshot (Win+Shift+S / Cmd+Shift+4)
4. **Settings Window:** All toggles and sliders work, settings persist
5. **Stop Tracking:** Click tray menu → Stop Tracking
6. **Quit:** Click tray menu → Quit

### Camera Preview Verification

The settings window should show:
- Live camera feed
- Detected hand with 21 landmark points drawn
- Current gesture recognized displayed in real-time
- FPS counter to monitor performance

---

### Critical Files for Implementation
- `/Users/mac/Downloads/screencursor/src-tauri/src/lib.rs` (Tauri commands, state management, app entry point)
- `/Users/mac/Downloads/screencursor/src-tauri/src/hand/detection.rs` (OpenCV DNN palm detection with ONNX model)
- `/Users/mac/Downloads/screencursor/src-tauri/src/input/mod.rs` (Platform dispatch for macOS/Windows input simulation)
- `/Users/mac/Downloads/screencursor/src-tauri/src/gesture/gestures.rs` (Gesture recognition logic mapping landmarks to actions)
- `/Users/mac/Downloads/screencursor/src-tauri/src/tray.rs` (System tray icon and context menu setup)

Sources:
- [OpenCV Zoo - Palm Detection](https://github.com/opencv/opencv_zoo/tree/main/models/palm_detection_mediapipe)
- [OpenCV Zoo - Hand Pose Estimation](https://github.com/opencv/opencv_zoo/tree/main/models/hand_pose_estimation_mediapipe)
- [Hugging Face - OpenCV Models](https://huggingface.co/opencv)
- [Tauri v2 Documentation](https://tauri.app/v2/)
- [opencv-rust Crate](https://crates.io/crates/opencv)
- [core-graphics Rust Bindings](https://crates.io/crates/core-graphics)
- [windows-rs Crate](https://crates.io/crates/windows)
