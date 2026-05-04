// Gesture recognition layer.
// - gestures.rs: Gesture enum and human-readable names.
// - tracker.rs: GestureTracker (position/velocity/pinch state, thresholds) for per-frame detection.

pub mod gestures;
pub mod tracker;

pub use gestures::Gesture;
pub use tracker::GestureTracker;

use crate::hand::landmarks::HandLandmarks;

pub struct GestureRecognizer {
    tracker: GestureTracker,
}

impl GestureRecognizer {
    pub fn new() -> Self {
        GestureRecognizer {
            tracker: GestureTracker::new(),
        }
    }

    pub fn recognize(&mut self, landmarks: &HandLandmarks) -> Option<Gesture> {
        self.tracker.update(landmarks)
    }
}
