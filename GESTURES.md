# Screencursor — Gesture Implementation Reference

## Overview

| Layer | File | Responsibility |
|---|---|---|
| **Sidecar (Python)** | `sidecar/sidecar.py` | Camera, MediaPipe inference, gesture logic, frame encoding |
| **Backend (Rust)** | `src-tauri/src/lib.rs` | Sidecar lifecycle, JSON parse, event emission, cursor move |
| **Input (Rust)** | `src-tauri/src/input/macos.rs` | CoreGraphics mouse/scroll/keyboard calls |

---

## MediaPipe Model

| Parameter | Value |
|---|---|
| Model | `hand_landmarker.task` (float16, ~7.5 MB) |
| Path | `sidecar/hand_landmarker.task` (auto-downloaded) |
| `num_hands` | 2 |
| `min_hand_detection_confidence` | 0.65 |
| `min_hand_presence_confidence` | 0.65 |
| `min_tracking_confidence` | 0.65 |
| Running mode | `VIDEO` |

**Key landmarks used:**

| Landmark | Index | Role |
|---|---|---|
| Wrist | 0 | 2-hand centroid swipe |
| Thumb tip | 4 | Pinch detection |
| Index tip | **8** | **Cursor control + pinch** |

---

## Gestures: 1 Hand — Cursor Mode

**Class:** `SingleHandTracker` in `sidecar/sidecar.py`

### Cursor Movement
| Property | Value |
|---|---|
| Tracking point | Index finger tip (landmark **8**) |
| X axis | Mirrored: `cursor_x = 1.0 − tip.x` |
| Y axis | Direct: `cursor_y = tip.y` |
| Smoothing | EMA factor **0.40** |
| Cursor frozen? | Yes, while in scroll mode (pinch held ≥ 3 frames) |
| Screen mapping | `CGDisplay::main().bounds()` in `lib.rs → screen_size()` |
| OS call | `sim.mouse_move(x, y)` → `CGEvent MouseMoved` |

### Left Click (quick tap)
| Property | Value |
|---|---|
| Trigger | Thumb tip (4) + Index tip (8) pinch held **< 3 frames**, then released |
| Threshold | `dist(tip4, tip8) < 0.04` (normalised) |
| Debounce | Fires on release, not on press |
| OS calls | `LeftMouseDown` + `LeftMouseUp` at current cursor position |
| Click position | `CGEventCreate(NULL)` → `CGEventGetLocation` |
| File | `sidecar.py → SingleHandTracker.update()` + `lib.rs → simulate_gesture_str()` |

### Scroll (pinch + hold + move)
| Property | Value |
|---|---|
| Trigger | Pinch held **≥ 3 consecutive frames** (`CLICK_FRAMES = 3`), then move |
| Enter threshold | 3 frames (`~100 ms at 30 fps`) |
| Displacement threshold | **0.03** accumulated (3% of frame per scroll tick) |
| Cooldown between ticks | **0.12 s** (max ~8 scroll events/s) |
| Direction | Whichever axis (dx or dy) is larger |
| Cursor during scroll | **Frozen** — cursor does not move |
| Visual cue | Hand skeleton turns **orange** in preview |
| OS calls | `sim.scroll(0, ±3)` or `sim.scroll(±3, 0)` |
| Exit | Release pinch |
| File | `sidecar.py → SingleHandTracker.update()` + `lib.rs → simulate_gesture_str()` |

### Zoom In / Zoom Out
| Property | Value |
|---|---|
| Status | 🔴 **DISABLED** (`ZOOM_ENABLED = False`) |
| Re-enable | Set `SingleHandTracker.ZOOM_ENABLED = True` in `sidecar.py` |

---

## Gestures: 2 Hands — Gesture Mode

**Class:** `TwoHandDetector` in `sidecar/sidecar.py`

> Cursor control is **suspended** when 2 hands are visible.

### Swipe (Left / Right / Up / Down)
| Property | Value |
|---|---|
| Detection | Centroid of **both wrists** (landmark 0) over last **6 frames** (~200 ms) |
| Threshold | Centroid displacement > **0.08** (8% of frame) |
| Direction | Larger axis (dx vs dy) determines direction |
| Cooldown | **1.5 s** between swipes |
| OS calls | Key sequences in `simulate_gesture_str` (configurable) |
| File | `sidecar.py → TwoHandDetector.update()` + `lib.rs → simulate_gesture_str()` |

### Right Click
| Property | Value |
|---|---|
| Trigger | **Both** hands pinch thumb (4) + index (8) simultaneously |
| Threshold | `dist(tip4, tip8) < 0.05` on **each** hand |
| Debounce | One event per combined press |
| OS calls | `RightMouseDown` + `RightMouseUp` |
| File | `sidecar.py → TwoHandDetector.update()` + `lib.rs → simulate_gesture_str()` |

---

## Event Pipeline

```
sidecar.py stdout (JSON lines)
   cursor_x/y  ──► lib.rs → sim.mouse_move(sx, sy)          [every frame, 1-hand]
   gesture     ──► lib.rs → h.emit("gesture-detected")       [on gesture]
                          → simulate_gesture_str(g)
   frame       ──► lib.rs → h.emit("preview-frame", b64)     [every 4th frame]
   always      ──► lib.rs → h.emit("tracking-status", s)     [every frame]
```

### Tauri Events

| Event | Payload | When |
|---|---|---|
| `tracking-status` | `{state, message, hint, gesture?, confidence?}` | Every frame |
| `gesture-detected` | gesture name string | On gesture fire |
| `preview-frame` | base64 JPEG | Every 4th frame (~7–8 fps) |
| `tracking-stopped` | `()` | After `stop_tracking` completes |

### Tauri Commands

| Command | Effect |
|---|---|
| `start_tracking` | Closes Rust camera, spawns sidecar |
| `stop_tracking` | Kills sidecar, reopens Rust camera, clears preview cache |
| `get_camera_preview` | Cached sidecar frame (tracking) or raw JPEG (idle) |

---

## File Map

```
screencursor/
├── sidecar/
│   ├── sidecar.py            ← all gesture logic
│   ├── hand_landmarker.task  ← MediaPipe model (~7.5 MB, auto-downloaded)
│   ├── requirements.txt      ← mediapipe, opencv-python, numpy
│   └── venv/                 ← isolated Python environment
└── src-tauri/src/
    ├── lib.rs                ← sidecar lifecycle, event emission, cursor move
    └── input/
        ├── common.rs         ← InputSimulator trait
        ├── macos.rs          ← CGEvent calls
        └── windows.rs        ← Windows input stub
```