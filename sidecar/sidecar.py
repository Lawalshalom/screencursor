#!/usr/bin/env python3
"""
screencursor sidecar — ported from Gesture-Controlled-Virtual-Mouse
(github.com/Yadav-Soham/Gesture-Controlled-Virtual-Mouse)

Exact algorithms ported:
  HandRecog.set_finger_state()  — signed-dist ratio (tip→pip / pip→mcp)
  HandRecog.get_gesture()       — binary encoding + 4-frame debounce
  Controller.get_position()     — landmark 9, dead-zone + proportional + fast damping
  Controller.pinch_control()    — scroll via 5-frame quantized pinch+move

Gesture map (single hand):
  V_GEST              → cursor move  (landmark 9, flag=True)
  FIST                → drag         (Left Down + move)
  MID   (after flag)  → Left Click
  INDEX (after flag)  → Right Click
  TWO_FINGER_CLOSED   → Double Click
  PINCH               → Scroll U/D/L/R (pinch+move, 5-frame quantize)
  PALM                → idle

Two hands:
  both pinch          → Right Click
  wrist centroid move → Swipe L/R/U/D
"""

import sys, os, json, time, math, shutil, base64, urllib.request, collections
from enum import IntEnum
import cv2, mediapipe as mp

SCRIPT_DIR      = os.path.dirname(os.path.abspath(__file__))
MODEL_FILE      = os.path.join(SCRIPT_DIR, 'hand_landmarker.task')
MODEL_URL       = ('https://storage.googleapis.com/mediapipe-models/'
                   'hand_landmarker/hand_landmarker/float16/latest/hand_landmarker.task')
MODEL_MIN_BYTES = 5_000_000

HAND_CONNECTIONS = [
    (0,1),(1,2),(2,3),(3,4),(0,5),(5,6),(6,7),(7,8),
    (5,9),(9,10),(10,11),(11,12),(9,13),(13,14),(14,15),(15,16),
    (13,17),(17,18),(18,19),(19,20),(0,17),
]

# ── Gesture encodings (binary, exactly as reference) ──────────────────────────
class Gest(IntEnum):
    FIST             = 0
    PINKY            = 1
    RING             = 2
    MID              = 4
    LAST3            = 7
    INDEX            = 8
    FIRST2           = 12
    LAST4            = 15
    THUMB            = 16
    PALM             = 31
    V_GEST           = 33
    TWO_FINGER_CLOSED= 34
    PINCH            = 35   # merged PINCH_MAJOR / PINCH_MINOR

# ── JSON output ────────────────────────────────────────────────────────────────
def emit(state, message, hint, gesture=None, confidence=None,
         cursor_x=None, cursor_y=None, frame=None):
    p = {"state": state, "message": message, "hint": hint,
         "gesture": gesture, "confidence": confidence}
    if cursor_x is not None: p["cursor_x"] = round(cursor_x, 4)
    if cursor_y is not None: p["cursor_y"] = round(cursor_y, 4)
    if frame:                p["frame"]    = frame
    print(json.dumps(p), flush=True)

# ── Model download ─────────────────────────────────────────────────────────────
def ensure_model():
    if os.path.exists(MODEL_FILE) and os.path.getsize(MODEL_FILE) >= MODEL_MIN_BYTES:
        return
    if os.path.exists(MODEL_FILE): os.remove(MODEL_FILE)
    emit("loading", "Downloading hand model…", "First-run (~8 MB)")
    tmp = MODEL_FILE + '.tmp'
    try:
        urllib.request.urlretrieve(MODEL_URL, tmp)
        if os.path.getsize(tmp) < MODEL_MIN_BYTES:
            os.remove(tmp); emit("error", "Download too small", ""); sys.exit(1)
        shutil.move(tmp, MODEL_FILE)
    except Exception as e:
        if os.path.exists(tmp): os.remove(tmp)
        emit("error", f"Download failed: {e}", ""); sys.exit(1)


# ── Hand Recognition (ported from HandRecog) ──────────────────────────────────
class HandRecog:
    """
    Converts MediaPipe landmarks to Gest enum value.
    Uses signed-distance ratio approach from the reference repo.
    """
    def __init__(self):
        self.finger       = 0
        self.ori_gesture  = Gest.PALM
        self.prev_gesture = Gest.PALM
        self.frame_count  = 0
        self.hand         = None   # list of NormalizedLandmark

    def update(self, hand):
        self.hand = hand

    # ── Signed / unsigned euclidean dist between two landmark indices ──────────
    def _signed_dist(self, i, j):
        a, b = self.hand[i], self.hand[j]
        sign = 1 if a.y < b.y else -1
        return sign * math.hypot(a.x - b.x, a.y - b.y)

    def _dist(self, i, j):
        a, b = self.hand[i], self.hand[j]
        return math.hypot(a.x - b.x, a.y - b.y)

    def _dz(self, i, j):
        return abs(self.hand[i].z - self.hand[j].z)

    # ── set_finger_state: binary encode 4 fingers via tip/pip/mcp ratio ───────
    def set_finger_state(self):
        if self.hand is None:
            return
        # [tip, pip, mcp] for index, middle, ring, pinky
        points = [[8,5,0], [12,9,0], [16,13,0], [20,17,0]]
        self.finger = 0            # thumb bit = 0 (not used in ratio)
        for point in points:
            d1 = self._signed_dist(point[0], point[1])  # tip → pip
            d2 = self._signed_dist(point[1], point[2])  # pip → mcp
            try:
                ratio = round(d1 / d2, 1)
            except ZeroDivisionError:
                ratio = round(d1 / 0.01, 1)
            self.finger <<= 1
            if ratio > 0.5:
                self.finger |= 1

    # ── get_gesture: classify + 4-frame debounce ──────────────────────────────
    def get_gesture(self):
        if self.hand is None:
            return Gest.PALM

        current = Gest.PALM

        # PINCH: ring+pinky+middle folded, index folded, thumb close to index tip
        if self.finger in (Gest.LAST3, Gest.LAST4) and self._dist(8, 4) < 0.05:
            current = Gest.PINCH

        elif self.finger == Gest.FIRST2:       # index + middle up
            tip_spread  = self._dist(8, 12)    # index tip ↔ middle tip
            mcp_spread  = self._dist(5,  9)    # index mcp ↔ middle mcp
            ratio       = tip_spread / mcp_spread
            if ratio > 1.7:
                current = Gest.V_GEST           # fingers spread → cursor
            else:
                if self._dz(8, 12) < 0.1:
                    current = Gest.TWO_FINGER_CLOSED  # fingers close in depth
                else:
                    current = Gest.MID          # only middle effectively up

        else:
            current = self.finger               # raw binary

        # 4-frame debounce (from reference: frame_count > 4)
        if current == self.prev_gesture:
            self.frame_count += 1
        else:
            self.frame_count = 0
        self.prev_gesture = current

        if self.frame_count > 4:
            self.ori_gesture = current
        return self.ori_gesture


# ── Controller (ported from Controller) ───────────────────────────────────────
class GestureController:
    """
    Translates Gest enum → (cursor_x, cursor_y, gesture_string, mode).

    get_position(): landmark 9, dead-zone + proportional + fast damping.
    pinch_control(): 5-frame quantized pinch+move → scroll direction.
    """
    PINCH_THRESHOLD = 0.3   # step size for scroll quantization (×10 scaled)

    def __init__(self, fw=640, fh=480):
        self._fw = fw; self._fh = fh
        # cursor position in pixel space (like original)
        self._cx_px = fw / 2
        self._cy_px = fh / 2
        self._prev_lm9 = None          # previous lm9 pixel position

        # gesture state flags
        self._flag      = False        # V_GEST seen → ready for click
        self._grabflag  = False        # FIST drag active
        self._pinchflag = False

        # pinch_control state
        self._pinch_sx  = None        # pinch start x (normalised lm8.x)
        self._pinch_sy  = None
        self._pinch_dir = None        # True=horizontal False=vertical
        self._prev_pv   = 0           # prevpinchlv
        self._pinch_lv  = 0
        self._pfc       = 0           # framecount

    # ── get_position: landmark 9, dead-zone + proportional damping ─────────
    def get_position(self, hand):
        """Returns (cx_norm, cy_norm). Mirrors X (no frame flip)."""
        lm = hand[9]
        # Mirror X since we don't flip the frame
        x_px = (1.0 - lm.x) * self._fw
        y_px = lm.y * self._fh

        if self._prev_lm9 is None:
            self._prev_lm9 = (x_px, y_px)
            return self._cx_px / self._fw, self._cy_px / self._fh

        dx = x_px - self._prev_lm9[0]
        dy = y_px - self._prev_lm9[1]
        self._prev_lm9 = (x_px, y_px)

        distsq = dx*dx + dy*dy
        if distsq <= 25:              # dead-zone: within 5 px radius
            ratio = 0.0
        elif distsq <= 900:           # smooth zone: 5–30 px
            ratio = 0.07 * math.sqrt(distsq)
        else:                         # fast zone: > 30 px
            ratio = 2.1

        self._cx_px = max(0.0, min(self._fw, self._cx_px + dx * ratio))
        self._cy_px = max(0.0, min(self._fh, self._cy_px + dy * ratio))
        return self._cx_px / self._fw, self._cy_px / self._fh

    # ── pinch_control: 5-frame quantized scroll ────────────────────────────
    def pinch_control_init(self, hand):
        self._pinch_sx = hand[8].x
        self._pinch_sy = hand[8].y
        self._pinch_lv = 0; self._prev_pv = 0; self._pfc = 0

    def pinch_control(self, hand):
        """Returns scroll gesture string or None."""
        result = None
        if self._pfc == 5:
            self._pfc = 0
            self._pinch_lv = self._prev_pv
            if self._pinch_dir is True:
                result = "Scroll Right" if self._pinch_lv > 0 else "Scroll Left"
            elif self._pinch_dir is False:
                result = "Scroll Up" if self._pinch_lv > 0 else "Scroll Down"

        lvx = round((hand[8].x - self._pinch_sx) * 10, 1)
        lvy = round((self._pinch_sy - hand[8].y) * 10, 1)

        if abs(lvy) > abs(lvx) and abs(lvy) > self.PINCH_THRESHOLD:
            self._pinch_dir = False
            if abs(self._prev_pv - lvy) < self.PINCH_THRESHOLD:
                self._pfc += 1
            else:
                self._prev_pv = lvy; self._pfc = 0
        elif abs(lvx) > self.PINCH_THRESHOLD:
            self._pinch_dir = True
            if abs(self._prev_pv - lvx) < self.PINCH_THRESHOLD:
                self._pfc += 1
            else:
                self._prev_pv = lvx; self._pfc = 0

        return result

    # ── handle_controls: map Gest → action ────────────────────────────────
    def handle(self, gesture, hand):
        """Returns (cx, cy, gesture_str_or_None, mode_str)."""
        cx = self._cx_px / self._fw
        cy = self._cy_px / self._fh

        # Compute position for all non-PALM gestures
        if gesture != Gest.PALM:
            cx, cy = self.get_position(hand)

        # Reset drag if we left FIST
        if gesture != Gest.FIST and self._grabflag:
            self._grabflag = False
            return cx, cy, "Left Up", "cursor"

        # Reset pinch if we left PINCH
        if gesture != Gest.PINCH and self._pinchflag:
            self._pinchflag = False

        # ── gesture actions ──────────────────────────────────────────────
        if gesture == Gest.V_GEST:
            self._flag = True
            return cx, cy, None, "cursor"

        elif gesture == Gest.FIST:
            if not self._grabflag:
                self._grabflag = True
                return cx, cy, "Left Down", "drag"
            return cx, cy, None, "drag"

        elif gesture == Gest.MID and self._flag:
            self._flag = False
            return cx, cy, "Left Click", "cursor"

        elif gesture == Gest.INDEX and self._flag:
            self._flag = False
            return cx, cy, "Right Click", "cursor"

        elif gesture == Gest.TWO_FINGER_CLOSED and self._flag:
            self._flag = False
            return cx, cy, "Double Click", "cursor"

        elif gesture == Gest.PINCH:
            if not self._pinchflag:
                self.pinch_control_init(hand)
                self._pinchflag = True
            scroll_g = self.pinch_control(hand)
            return cx, cy, scroll_g, "scroll"

        elif gesture == Gest.PALM:
            return cx, cy, None, "idle"

        return cx, cy, None, "idle"


# ── Two-hand detector (unchanged, wrist centroid swipe) ───────────────────────
class TwoHandDetector:
    HISTORY   = 6;  MIN_DISP = 0.08;  COOLDOWN = 1.5;  PINCH_THR = 0.05

    def __init__(self):
        self._c = collections.deque(maxlen=self.HISTORY)
        self._p0 = False; self._p1 = False; self._t = 0.0

    def update(self, hands):
        h0, h1 = hands[0], hands[1]
        cx = (h0[0].x + h1[0].x) / 2
        cy = (h0[0].y + h1[0].y) / 2
        self._c.append((cx, cy))

        def pinch(h): return math.hypot(h[4].x-h[8].x, h[4].y-h[8].y) < self.PINCH_THR
        p0, p1 = pinch(h0), pinch(h1)
        if p0 and p1 and not (self._p0 and self._p1):
            self._p0 = True; self._p1 = True; self._c.clear()
            return "Right Click"
        if not p0: self._p0 = False
        if not p1: self._p1 = False
        if p0 or p1: return None

        if len(self._c) < self.HISTORY: return None
        if time.time() - self._t < self.COOLDOWN: return None
        dx = self._c[-1][0] - self._c[0][0]
        dy = self._c[-1][1] - self._c[0][1]
        if max(abs(dx), abs(dy)) < self.MIN_DISP: return None
        self._t = time.time(); self._c.clear()
        if abs(dx) >= abs(dy): return "Swipe Right" if dx > 0 else "Swipe Left"
        return "Swipe Down" if dy > 0 else "Swipe Up"


# ── Drawing helpers ───────────────────────────────────────────────────────────
def draw_hand(vis, hand, fw, fh, colour=(0,230,0)):
    pts = [(int(l.x*fw), int(l.y*fh)) for l in hand]
    xs, ys = [p[0] for p in pts], [p[1] for p in pts]
    cv2.rectangle(vis, (max(0,min(xs)-10), max(0,min(ys)-10)),
                  (min(fw-1,max(xs)+10), min(fh-1,max(ys)+10)), colour, 2)
    for a, b in HAND_CONNECTIONS:
        cv2.line(vis, pts[a], pts[b], (colour[0]//2,colour[1]//2,colour[2]//2), 1)
    for i, (x, y) in enumerate(pts):
        r = 7 if i in (4, 8, 9, 12) else 4
        cv2.circle(vis, (x, y), r, colour, -1)
        cv2.circle(vis, (x, y), r, (255,255,255), 1)

def encode_frame(frame, quality=60):
    h, w = frame.shape[:2]
    scale = min(320/w, 240/h)
    if scale < 1.0:
        frame = cv2.resize(frame, (int(w*scale), int(h*scale)))
    rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
    ok, buf = cv2.imencode('.jpg', rgb, [cv2.IMWRITE_JPEG_QUALITY, quality])
    return base64.b64encode(buf.tobytes()).decode() if ok else None

MODE_COLOUR = {"cursor":(0,230,0), "drag":(0,100,255),
               "scroll":(0,165,255), "idle":(100,100,100)}
MODE_HINT   = {
    "cursor": "V-sign to move · middle-fold = click · index-fold = right-click",
    "drag":   "Fist to drag · open hand to release",
    "scroll": "Pinch + move to scroll",
    "idle":   "Make V-sign (index+middle spread) to start",
}

# ── Main ──────────────────────────────────────────────────────────────────────
def main():
    ensure_model()

    opts = mp.tasks.vision.HandLandmarkerOptions(
        base_options=mp.tasks.BaseOptions(model_asset_path=MODEL_FILE),
        running_mode=mp.tasks.vision.RunningMode.VIDEO,
        num_hands=2,
        min_hand_detection_confidence=0.60,
        min_hand_presence_confidence=0.60,
        min_tracking_confidence=0.50,
    )

    cap = cv2.VideoCapture(0)
    if not cap.isOpened():
        emit("error", "Cannot open camera", "Check camera permissions"); sys.exit(1)
    cap.set(cv2.CAP_PROP_FRAME_WIDTH, 640)
    cap.set(cv2.CAP_PROP_FRAME_HEIGHT, 480)

    fw = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    fh = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

    recog   = HandRecog()
    ctrl    = GestureController(fw, fh)
    two     = TwoHandDetector()
    ts = 0; frame_idx = 0; SKIP = 4

    with mp.tasks.vision.HandLandmarker.create_from_options(opts) as lm:
        while True:
            ok, frame = cap.read()
            if not ok:
                emit("error", "Camera read failed", "Check camera"); break

            rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
            img = mp.Image(image_format=mp.ImageFormat.SRGB, data=rgb)
            ts += 33; frame_idx += 1
            res  = lm.detect_for_video(img, ts)
            send = (frame_idx % SKIP == 0)

            hands = res.hand_landmarks
            n     = len(hands)

            if n == 0:
                ctrl._prev_lm9 = None   # reset damping on hand loss
                enc = encode_frame(frame) if send else None
                emit("no_hand", "No hand detected",
                     "Make V-sign (index+middle) to start", frame=enc)
                continue

            vis = frame.copy()
            cv2.line(vis, (fw//2-15,fh//2),(fw//2+15,fh//2),(60,60,60),1)
            cv2.line(vis, (fw//2,fh//2-15),(fw//2,fh//2+15),(60,60,60),1)

            if n == 1:
                hand = hands[0]
                recog.update(hand)
                recog.set_finger_state()
                gest = recog.get_gesture()

                cx, cy, gesture_str, mode = ctrl.handle(gest, hand)

                col = MODE_COLOUR.get(mode, (150,150,150))
                if gesture_str: col = (0,200,255)
                draw_hand(vis, hand, fw, fh, colour=col)

                # show landmark 9 (cursor anchor)
                lm9 = hand[9]
                cv2.drawMarker(vis,
                    (int((1-lm9.x)*fw), int(lm9.y*fh)),   # mirrored
                    col, cv2.MARKER_CROSS, 18, 2)

                # gesture label
                gest_label = gest.name if hasattr(gest,'name') else str(gest)
                cv2.putText(vis, gest_label, (8, fh-8),
                    cv2.FONT_HERSHEY_SIMPLEX, 0.4, col, 1, cv2.LINE_AA)

                enc = encode_frame(vis) if send else None

                if gesture_str:
                    emit("gesture", f"Gesture: {gesture_str}",
                         MODE_HINT.get(mode,""),
                         gesture=gesture_str, confidence=0.95,
                         cursor_x=cx, cursor_y=cy, frame=enc)
                elif mode == "scroll":
                    emit("scroll", "Scroll mode", MODE_HINT["scroll"],
                         cursor_x=cx, cursor_y=cy, frame=enc)
                elif mode == "drag":
                    emit("drag", "Drag mode", MODE_HINT["drag"],
                         cursor_x=cx, cursor_y=cy, frame=enc)
                else:
                    emit("cursor", f"Mode: {mode} ({gest_label})",
                         MODE_HINT.get(mode, ""),
                         cursor_x=cx, cursor_y=cy, frame=enc)

            else:
                gesture_str = two.update(hands)
                for h in hands:
                    draw_hand(vis, h, fw, fh,
                              colour=(0,200,255) if gesture_str else (180,100,0))
                enc = encode_frame(vis) if send else None
                if gesture_str:
                    emit("gesture", f"Gesture: {gesture_str}", "Action!",
                         gesture=gesture_str, confidence=0.95, frame=enc)
                else:
                    emit("two_hands","Two hands — gesture mode",
                         "Both pinch=right click · move together=swipe", frame=enc)

    cap.release()

if __name__ == "__main__":
    main()
