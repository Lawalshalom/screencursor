#!/usr/bin/env python3
"""
screencursor sidecar — MediaPipe hand tracking + gesture recognition.
Outputs one JSON line per frame to stdout for the Tauri Rust backend.
"""

import sys, os, json, time, math, shutil, urllib.request, urllib.error, collections
import cv2
import mediapipe as mp

# ── Model path ─────────────────────────────────────────────────────────────────
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
MODEL_FILE = os.path.join(SCRIPT_DIR, 'hand_landmarker.task')
MODEL_URL  = ('https://storage.googleapis.com/mediapipe-models/'
              'hand_landmarker/hand_landmarker/float16/latest/hand_landmarker.task')
MODEL_MIN_BYTES = 20_000_000  # ~29 MB expected; reject anything < 20 MB

def emit(state, message, hint, gesture=None, confidence=None):
    print(json.dumps({
        "state": state, "message": message, "hint": hint,
        "gesture": gesture, "confidence": confidence
    }), flush=True)

def ensure_model():
    """Download the model if missing or corrupt (< 20 MB)."""
    if os.path.exists(MODEL_FILE):
        size = os.path.getsize(MODEL_FILE)
        if size >= MODEL_MIN_BYTES:
            return  # looks good
        sys.stderr.write(f"[sidecar] Model file is only {size:,} bytes (corrupt / partial) — re-downloading.\n")
        sys.stderr.flush()
        os.remove(MODEL_FILE)

    emit("loading", "Downloading hand tracking model…", "First-run setup, please wait (~29 MB)")

    tmp = MODEL_FILE + '.tmp'
    try:
        def progress(count, block, total):
            if total > 0:
                pct = min(100, count * block * 100 // total)
                sys.stderr.write(f'\r  Downloading {pct}% …  ')
                sys.stderr.flush()

        urllib.request.urlretrieve(MODEL_URL, tmp, reporthook=progress)
        sys.stderr.write('\n')

        size = os.path.getsize(tmp)
        if size < MODEL_MIN_BYTES:
            os.remove(tmp)
            emit("error", "Model download too small — check internet connection", "")
            sys.exit(1)

        shutil.move(tmp, MODEL_FILE)
        sys.stderr.write(f"[sidecar] Model saved ({size:,} bytes)\n")
    except Exception as e:
        if os.path.exists(tmp):
            os.remove(tmp)
        emit("error", f"Model download failed: {e}", "Check your internet connection")
        sys.exit(1)


# ── Gesture recogniser ─────────────────────────────────────────────────────────
class GestureRecogniser:
    PINCH_THR  = 0.06   # fraction of image width
    SWIPE_VEL  = 600    # px/s
    SCROLL_PX  = 18     # px accumulation threshold

    def __init__(self):
        self._last_pos   = None
        self._last_time  = None
        self._scroll     = [0.0, 0.0]
        self._pinch_prev = None
        self._zoom_hist  = collections.deque(maxlen=10)

    @staticmethod
    def _dist(a, b):
        return math.hypot(a.x - b.x, a.y - b.y)

    def update(self, lm, fw, fh):
        thumb, index, middle = lm[4], lm[8], lm[12]

        pl = self._dist(thumb, index)
        pr = self._dist(thumb, middle)

        if pl < self.PINCH_THR:
            self._pinch_prev = None; self._zoom_hist.clear(); return "Left Click"
        if pr < self.PINCH_THR:
            self._pinch_prev = None; self._zoom_hist.clear(); return "Right Click"

        # Zoom via pinch spread
        if self._pinch_prev is not None:
            self._zoom_hist.append(pl - self._pinch_prev)
            total = sum(self._zoom_hist)
            if total > 0.08:
                self._zoom_hist.clear(); self._pinch_prev = None; return "Zoom In"
            if total < -0.08:
                self._zoom_hist.clear(); self._pinch_prev = None; return "Zoom Out"
        self._pinch_prev = pl

        now = time.time()
        wx, wy = lm[0].x * fw, lm[0].y * fh

        if self._last_pos and self._last_time:
            dt = now - self._last_time
            if dt > 0:
                vx = (wx - self._last_pos[0]) / dt
                vy = (wy - self._last_pos[1]) / dt
                if abs(vx) > self.SWIPE_VEL and abs(vx) > abs(vy):
                    self._reset(); self._last_pos=(wx,wy); self._last_time=now
                    return "Swipe Right" if vx > 0 else "Swipe Left"
                if abs(vy) > self.SWIPE_VEL and abs(vy) > abs(vx):
                    self._reset(); self._last_pos=(wx,wy); self._last_time=now
                    return "Swipe Down" if vy > 0 else "Swipe Up"
                self._scroll[0] += wx - self._last_pos[0]
                self._scroll[1] += wy - self._last_pos[1]
                s = self.SCROLL_PX
                if   self._scroll[1] >  s: self._scroll[1]=0; return "Scroll Down"
                elif self._scroll[1] < -s: self._scroll[1]=0; return "Scroll Up"
                elif self._scroll[0] >  s: self._scroll[0]=0; return "Scroll Right"
                elif self._scroll[0] < -s: self._scroll[0]=0; return "Scroll Left"

        self._last_pos  = (wx, wy)
        self._last_time = now
        return None

    def _reset(self): self._scroll = [0.0, 0.0]


# ── Main ───────────────────────────────────────────────────────────────────────
def main():
    ensure_model()

    BaseOptions    = mp.tasks.BaseOptions
    HandLandmarker = mp.tasks.vision.HandLandmarker
    Opts           = mp.tasks.vision.HandLandmarkerOptions
    RunMode        = mp.tasks.vision.RunningMode

    opts = Opts(
        base_options=BaseOptions(model_asset_path=MODEL_FILE),
        running_mode=RunMode.VIDEO,
        num_hands=1,
        min_hand_detection_confidence=0.8,
        min_hand_presence_confidence=0.8,
        min_tracking_confidence=0.8,
    )

    cap = cv2.VideoCapture(0)
    if not cap.isOpened():
        emit("error", "Cannot open camera", "Check camera permissions"); sys.exit(1)

    recogniser = GestureRecogniser()
    ts = 0

    with HandLandmarker.create_from_options(opts) as lm:
        while True:
            ok, frame = cap.read()
            if not ok:
                emit("error", "Camera read failed", "Check camera connection"); break

            fw, fh = frame.shape[1], frame.shape[0]
            rgb    = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
            img    = mp.Image(image_format=mp.ImageFormat.SRGB, data=rgb)
            ts    += 33
            res    = lm.detect_for_video(img, ts)

            if not res.hand_landmarks:
                emit("no_hand", "No hand detected",
                     "Show your open hand clearly to the camera"); continue

            hand = res.hand_landmarks[0]
            xs   = [l.x for l in hand]; ys = [l.y for l in hand]
            bb   = (max(xs)-min(xs)) * (max(ys)-min(ys))

            if bb > 0.35:
                emit("too_close", "Hand too close",
                     "Move your hand further from the camera", confidence=0.9); continue

            gesture = recogniser.update(hand, fw, fh)
            if gesture:
                emit("gesture", f"Gesture: {gesture}", "Action performed!",
                     gesture=gesture, confidence=0.95)
            else:
                emit("hand_detected", "Hand detected — tracking active",
                     "Pinch thumb+index to click · move hand to scroll", confidence=0.9)

    cap.release()

if __name__ == "__main__":
    main()
