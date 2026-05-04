use std::collections::VecDeque;
use std::time::{Duration, Instant};
use crate::hand::landmarks::HandLandmarks;
use super::gestures::Gesture;

const WRIST_IDX: usize = 0;
const THUMB_TIP_IDX: usize = 4;
const INDEX_TIP_IDX: usize = 8;
const MIDDLE_TIP_IDX: usize = 12;
#[allow(dead_code)]
const RING_TIP_IDX: usize = 16;
#[allow(dead_code)]
const PINKY_TIP_IDX: usize = 20;

// Default constants
const DEFAULT_PINCH_THRESHOLD: f32 = 30.0;
#[allow(dead_code)]
const DEFAULT_SWIPE_THRESHOLD: f32 = 50.0;
const DEFAULT_SWIPE_VELOCITY_THRESHOLD: f32 = 100.0;
const DEFAULT_SCROLL_SENSITIVITY: f32 = 2.0;
const DEFAULT_ZOOM_THRESHOLD: f32 = 25.0;
const DEFAULT_ZOOM_TIME_WINDOW: f32 = 0.5;

#[derive(Default)]
pub struct GestureTracker {
    last_wrist_pos: Option<(f32, f32)>,
    last_update: Option<Instant>,
    position_history: VecDeque<(f32, f32, Instant)>,
    pinch_start_time: Option<Instant>,
    scroll_accumulator: (f32, f32),
    last_pinch_distance: Option<f32>,
    pinch_distance_history: VecDeque<(f32, Instant)>,
    // Configurable thresholds
    pinch_threshold: f32,
    scroll_sensitivity: f32,
    swipe_velocity_threshold: f32,
    zoom_threshold: f32,
    zoom_time_window: f32,
}

impl GestureTracker {
    pub fn new() -> Self {
        GestureTracker {
            last_wrist_pos: None,
            last_update: None,
            position_history: VecDeque::with_capacity(10),
            pinch_start_time: None,
            scroll_accumulator: (0.0, 0.0),
            last_pinch_distance: None,
            pinch_distance_history: VecDeque::with_capacity(10),
            pinch_threshold: DEFAULT_PINCH_THRESHOLD,
            scroll_sensitivity: DEFAULT_SCROLL_SENSITIVITY,
            swipe_velocity_threshold: DEFAULT_SWIPE_VELOCITY_THRESHOLD,
            zoom_threshold: DEFAULT_ZOOM_THRESHOLD,
            zoom_time_window: DEFAULT_ZOOM_TIME_WINDOW,
        }
    }

    pub fn update_settings(&mut self, settings: &crate::settings::Settings) {
        self.pinch_threshold = settings.pinch_threshold;
        self.scroll_sensitivity = settings.scroll_sensitivity;
        self.swipe_velocity_threshold = settings.swipe_threshold.max(1.0);
        self.zoom_threshold = settings.zoom_threshold.max(1.0);
        self.zoom_time_window = settings.zoom_time_window.max(0.1);
    }

    pub fn update(&mut self, landmarks: &HandLandmarks) -> Option<Gesture> {
        let now = Instant::now();

        // Get key landmarks
        let wrist = landmarks.get_point(WRIST_IDX)?;
        let thumb_tip = landmarks.get_point(THUMB_TIP_IDX)?;
        let index_tip = landmarks.get_point(INDEX_TIP_IDX)?;
        let middle_tip = landmarks.get_point(MIDDLE_TIP_IDX)?;

        // Calculate distances
        let pinch_distance = Self::distance(&thumb_tip, &index_tip);

        // Check for pinch (left click)
        if pinch_distance < self.pinch_threshold {
            if self.pinch_start_time.is_none() {
                self.pinch_start_time = Some(now);
            }
            // Reset zoom tracking during pinch
            self.last_pinch_distance = None;
            self.pinch_distance_history.clear();
            return Some(Gesture::LeftClick);
        } else {
            self.pinch_start_time = None;
        }

        // Check for right click (thumb + middle)
        let right_pinch_distance = Self::distance(&thumb_tip, &middle_tip);
        if right_pinch_distance < self.pinch_threshold {
            // Reset zoom tracking during right click
            self.last_pinch_distance = None;
            self.pinch_distance_history.clear();
            return Some(Gesture::RightClick);
        }

        // Track pinch distance for zoom detection
        if let Some(last_dist) = self.last_pinch_distance {
            let delta = pinch_distance - last_dist;
            self.pinch_distance_history.push_back((delta, now));

            // Calculate total delta over zoom_time_window
            let total_delta: f32 = self.pinch_distance_history.iter()
                .filter(|(_, t)| now.duration_since(*t).as_secs_f32() < self.zoom_time_window)
                .map(|(d, _)| *d)
                .sum();

            if total_delta > self.zoom_threshold {
                // Zoom in (fingers spreading)
                self.last_pinch_distance = None;
                self.pinch_distance_history.clear();
                return Some(Gesture::ZoomIn);
            } else if total_delta < -self.zoom_threshold {
                // Zoom out (fingers pinching)
                self.last_pinch_distance = None;
                self.pinch_distance_history.clear();
                return Some(Gesture::ZoomOut);
            }
        }
        self.last_pinch_distance = Some(pinch_distance);

        // Keep only recent pinch distance history
        while let Some((_, time)) = self.pinch_distance_history.front() {
            if now.duration_since(*time).as_secs_f32() > self.zoom_time_window {
                self.pinch_distance_history.pop_front();
            } else {
                break;
            }
        }

        // Track position for swipe/scroll
        if let Some((last_x, last_y)) = self.last_wrist_pos {
            let dx = wrist.x - last_x;
            let dy = wrist.y - last_y;

            // Calculate velocity
            if let Some(last_time) = self.last_update {
                let dt = now.duration_since(last_time).as_secs_f32();
                if dt > 0.0 {
                    let vx = dx / dt;
                    let vy = dy / dt;

                    // Swipe detection (high velocity)
                    if vx.abs() > self.swipe_velocity_threshold && vx.abs() > vy.abs() {
                        if vx > 0.0 {
                            return Some(Gesture::SwipeRight);
                        } else {
                            return Some(Gesture::SwipeLeft);
                        }
                    } else if vy.abs() > self.swipe_velocity_threshold && vy.abs() > vx.abs() {
                        if vy > 0.0 {
                            return Some(Gesture::SwipeDown);
                        } else {
                            return Some(Gesture::SwipeUp);
                        }
                    }

                    // Scroll (slow movement)
                    self.scroll_accumulator.0 += dx * self.scroll_sensitivity;
                    self.scroll_accumulator.1 += dy * self.scroll_sensitivity;

                    if self.scroll_accumulator.1 > self.scroll_sensitivity {
                        self.scroll_accumulator.1 = 0.0;
                        return Some(Gesture::ScrollDown);
                    } else if self.scroll_accumulator.1 < -self.scroll_sensitivity {
                        self.scroll_accumulator.1 = 0.0;
                        return Some(Gesture::ScrollUp);
                    } else if self.scroll_accumulator.0 > self.scroll_sensitivity {
                        self.scroll_accumulator.0 = 0.0;
                        return Some(Gesture::ScrollRight);
                    } else if self.scroll_accumulator.0 < -self.scroll_sensitivity {
                        self.scroll_accumulator.0 = 0.0;
                        return Some(Gesture::ScrollLeft);
                    }
                }
            }
        }

        // Update history
        self.last_wrist_pos = Some((wrist.x, wrist.y));
        self.last_update = Some(now);
        self.position_history.push_back((wrist.x, wrist.y, now));

        // Keep only recent positions
        while let Some((_, _, time)) = self.position_history.front() {
            if now.duration_since(*time) > Duration::from_secs(1) {
                self.position_history.pop_front();
            } else {
                break;
            }
        }

        None
    }

    fn distance(p1: &opencv::core::Point2f, p2: &opencv::core::Point2f) -> f32 {
        let dx = p1.x - p2.x;
        let dy = p1.y - p2.y;
        (dx * dx + dy * dy).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hand::landmarks::HandLandmarks;
    use opencv::core::Point2f;

    fn create_mock_landmarks(
        wrist: (f32, f32),
        thumb_tip: (f32, f32),
        index_tip: (f32, f32),
        middle_tip: (f32, f32),
    ) -> HandLandmarks {
        let mut landmarks = HandLandmarks::new();
        // Push points up to index 12 (middle_tip)
        for i in 0..=12 {
            let point = match i {
                0 => Point2f::new(wrist.0, wrist.1),
                4 => Point2f::new(thumb_tip.0, thumb_tip.1),
                8 => Point2f::new(index_tip.0, index_tip.1),
                12 => Point2f::new(middle_tip.0, middle_tip.1),
                _ => Point2f::new(0.0, 0.0),
            };
            landmarks.points.push(point);
        }
        landmarks
    }

    #[test]
    fn test_tracker_new() {
        let tracker = GestureTracker::new();
        assert_eq!(tracker.pinch_threshold, DEFAULT_PINCH_THRESHOLD);
        assert_eq!(tracker.scroll_sensitivity, DEFAULT_SCROLL_SENSITIVITY);
    }

    #[test]
    fn test_update_settings() {
        let mut tracker = GestureTracker::new();
        let settings = crate::settings::Settings {
            tracking_enabled: true,
            scroll_sensitivity: 3.0,
            swipe_threshold: 60.0,
            pinch_threshold: 35.0,
            zoom_threshold: 30.0,
            zoom_time_window: 0.6,
        };
        tracker.update_settings(&settings);
        assert_eq!(tracker.pinch_threshold, 35.0);
        assert_eq!(tracker.scroll_sensitivity, 3.0);
        assert_eq!(tracker.swipe_velocity_threshold, 60.0);
        assert_eq!(tracker.zoom_threshold, 30.0);
        assert_eq!(tracker.zoom_time_window, 0.6);
    }

    #[test]
    fn test_left_click_detection() {
        let mut tracker = GestureTracker::new();
        // Thumb and index are close (distance ~14.14 < 30.0)
        let landmarks = create_mock_landmarks(
            (100.0, 100.0), // wrist
            (150.0, 150.0), // thumb_tip
            (164.0, 164.0), // index_tip (distance to thumb: sqrt(14²+14²) ≈ 19.8 < 30)
            (200.0, 200.0), // middle_tip
        );
        let gesture = tracker.update(&landmarks);
        assert_eq!(gesture, Some(Gesture::LeftClick));
    }

    #[test]
    fn test_no_gesture_when_far() {
        let mut tracker = GestureTracker::new();
        // All points far apart (no gesture)
        let landmarks = create_mock_landmarks(
            (100.0, 100.0),
            (150.0, 150.0),
            (200.0, 200.0), // distance to thumb: ~70.7 > 30
            (250.0, 250.0),
        );
        let gesture = tracker.update(&landmarks);
        // First call may return None (no gesture)
        assert!(gesture.is_none() || gesture == Some(Gesture::ScrollDown));
    }
}
