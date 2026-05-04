# ScreenCursor

> Hand gesture mouse control powered by computer vision — control your cursor with nothing but your hands.

ScreenCursor is a cross-platform desktop app that uses your webcam to track hand landmarks and translate them into real mouse and keyboard events. The gesture engine runs on a Python MediaPipe sidecar, communicating in real-time with a native Rust/Tauri backend.

## Features

- 🖱️ **Cursor Control** — Index fingertip drives the cursor with EMA smoothing and deadzone filtering
- 👆 **Left Click / Drag** — Thumb + index pinch (Apple Vision Pro style, instant tap & drag)
- 🖱️ **Right Click** — Two-hand close-pinch gesture (strict two-hand only, no false positives)
- 📜 **Swipe Scroll** — Two-hand 4-finger centroid displacement for vertical/horizontal scrolling
- ⚙️ **System Tray** — Start/stop tracking, open settings, or quit from the menu bar
- 🔴 **Real-time Preview** — Live camera feed with hand landmark overlay
- 🎛️ **Configurable Settings** — Sensitivity, thresholds, and gesture toggles per-user
- 🖥️ **Cross-platform** — macOS (primary), Windows (planned)

## Tech Stack

| Layer | Technology |
|-------|-----------|
| **Frontend** | Vue 3, TypeScript, Tailwind CSS, Vite |
| **Backend** | Tauri v2 (Rust) |
| **Gesture Engine** | Python 3 sidecar — MediaPipe, OpenCV, NumPy |
| **ML Model** | MediaPipe Hand Landmarker (`.task` format) |
| **IPC** | Tauri sidecar stdio (JSON events) |
| **Platform** | macOS (primary), Windows (target) |

## Project Structure

```
screencursor/
├── src/                        # Vue 3 frontend
│   ├── views/                  # Settings, Calibration, About views
│   ├── components/             # Reusable Vue components
│   ├── App.vue                 # Main app shell
│   └── main.ts                 # Entry point
├── src-tauri/                  # Rust/Tauri backend
│   ├── src/
│   │   ├── camera/             # OpenCV camera access
│   │   ├── hand/               # Hand detection & landmark processing
│   │   ├── gesture/            # Gesture classification logic
│   │   ├── input/              # Mouse & keyboard event simulation
│   │   ├── settings/           # Persistent settings management
│   │   ├── tray.rs             # System tray integration
│   │   └── lib.rs              # Tauri commands & tracking loop
│   ├── models/                 # ONNX models (not committed — see setup)
│   ├── icons/                  # App & tray icons
│   └── Cargo.toml              # Rust dependencies
├── sidecar/                    # Python gesture engine
│   ├── sidecar.py              # MediaPipe hand landmark processor
│   ├── hand_landmarker.task    # MediaPipe model (not committed — see setup)
│   └── requirements.txt        # Python dependencies
├── GESTURES.md                 # Full gesture reference
└── deploy.md                   # Production deployment guide
```

## Prerequisites

### macOS

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Node.js 18+
brew install node

# Python 3.10+
brew install python
```

### Windows

```bash
# Rust — https://www.rust-lang.org/tools/install

# Node.js 18+ — https://nodejs.org/

# Python 3.10+ — https://www.python.org/downloads/
```

## Installation

### 1. Clone the repo

```bash
git clone https://github.com/<your-username>/screencursor.git
cd screencursor
```

### 2. Install frontend dependencies

```bash
npm install
```

### 3. Set up the Python sidecar

```bash
cd sidecar
python3 -m venv venv
source venv/bin/activate        # Windows: venv\Scripts\activate
pip install -r requirements.txt
```

### 4. Download the MediaPipe model

```bash
# Inside the sidecar/ directory:
curl -L -o hand_landmarker.task \
  https://storage.googleapis.com/mediapipe-models/hand_landmarker/hand_landmarker/float16/latest/hand_landmarker.task
```

### 5. (Optional) Download ONNX fallback models

```bash
mkdir -p src-tauri/models
# palm_detection_mediapipe_2023feb.onnx  (~3.9 MB)
# handpose_estimation_mediapipe_2023feb.onnx  (~4.1 MB)
```

## Running the App

### Development

```bash
npm run tauri:dev
```

Starts Tauri with hot-reload for the Vue frontend.

### Production Build

```bash
npm run tauri:build
```

Output:
- **macOS:** `src-tauri/target/release/bundle/dmg/screencursor.dmg`
- **Windows:** `src-tauri/target/release/bundle/nsis/screencursor_setup.exe`

## Gesture Reference

| Gesture | Action |
|---------|--------|
| Index fingertip movement | Move cursor |
| Thumb + index pinch (1 hand) | Left click / drag |
| Two hands close together | Right click |
| Two-hand 4-finger swipe up/down | Scroll vertical |
| Two-hand 4-finger swipe left/right | Scroll horizontal |

See [GESTURES.md](./GESTURES.md) for the full reference including thresholds and tuning.

## Settings

Access via the system tray → Settings, or the in-app Settings tab:

| Setting | Description |
|---------|-------------|
| Tracking Enabled | Toggle hand tracking on/off |
| Scroll Sensitivity | Scrolling speed multiplier (0.1–10.0) |
| Swipe Threshold | Min velocity for swipe detection (px/s) |
| Pinch Threshold | Distance threshold for pinch (normalised units) |
| Zoom Threshold | Distance delta required for zoom (px) |

## Testing

```bash
# Rust unit tests
cd src-tauri && cargo test

# Frontend type check
npm run build
```

## Troubleshooting

### Camera not detected
- Ensure your webcam isn't in use by another app
- macOS: System Settings → Privacy & Security → Camera → enable ScreenCursor

### Gestures not recognised
- Ensure good lighting and a plain background
- Keep your hand within the camera frame
- Adjust sensitivity in Settings

### Build errors
- **Rust:** Run `rustup update` to get the latest toolchain
- **Node:** Delete `node_modules/` and re-run `npm install`
- **Python sidecar:** Ensure the virtual env is activated and `hand_landmarker.task` is present in `sidecar/`

## Contributing

Contributions are welcome! Please open an issue first to discuss what you'd like to change, then submit a pull request against the `main` branch.

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Commit your changes: `git commit -m 'feat: add my feature'`
4. Push and open a PR

## License

MIT License — see [LICENSE](./LICENSE) for details.
