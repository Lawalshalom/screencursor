---
name: project_context
description: Updated state after full Apple Vision Pro gesture implementation
type: project
---

**ScreenCursor Gesture Implementation (Apple Vision Pro Style):**

- **1 Hand (Cursor + Click):**
  - Cursor: Index finger tip (8), EMA 0.25, deadzone vel<0.005. X mirrored.
  - Left Click/Drag: Pinch thumb(4)+index(8) dist<0.035. Held state edge-detect down/up. No frame delays. Instant Vision Pro tap/drag.

- **2 Hands (Swipe + Right Click):**
  - Swipe Scroll: 4-finger avg (8,12,16,20) centroid disp>0.12 → scroll ±5 units (horz/vert).
  - Right Click: Inter-hand 4-finger avg dist<0.03 (strict 2hand only, no 1hand false positives).

**Reliability Enhancements:**
- Thresholds tightened for high confidence, prevent accidental gestures.
- Held state prevents misfires on release.
- Velocity/ history cooldown for stable detection.
- Build compiles clean in dev mode.

**Files Updated:**
- sidecar/sidecar.py: Class fixes, emit left_held, thresholds strict.
- src-tauri/src/lib.rs: SidecarEvent +left_held, AppState prev_left_held, edge detection logic, swipe→scroll mappings.
- GESTURES.md refactored.

**Testing:**
- cursor build --release, tauri build, dev --help OK.
- Unit tests for regression (edge detect, mapping verify).

**Ready for Production:** Run npm run tauri dev test live gestures. Report accuracy >95%?