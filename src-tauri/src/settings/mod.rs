// Settings persistence.
//
// Loaded/saved as JSON under the platform config dir
// (macOS: ~/Library/Application Support/screencursor/settings.json).
//
// `#[serde(rename_all = "camelCase")]` makes the on-the-wire names match the
// Vue frontend (`scrollSensitivity`, `pinchThreshold`, ...) while keeping
// idiomatic snake_case on the Rust side.

pub mod config;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub tracking_enabled: bool,
    pub scroll_sensitivity: f32,
    pub swipe_threshold: f32,
    pub pinch_threshold: f32,
    pub zoom_threshold: f32,
    pub zoom_time_window: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            tracking_enabled: false,
            scroll_sensitivity: 1.0,
            swipe_threshold: 50.0,
            pinch_threshold: 30.0,
            zoom_threshold: 25.0,
            zoom_time_window: 0.5,
        }
    }
}

pub fn load_settings() -> Settings {
    let path = get_settings_path();
    if path.exists() {
        if let Ok(contents) = fs::read_to_string(&path) {
            if let Ok(settings) = serde_json::from_str(&contents) {
                return settings;
            }
        }
    }
    Settings::default()
}

pub fn save_settings(settings: &Settings) -> Result<(), String> {
    let path = get_settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

fn get_settings_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("screencursor");
    path.push("settings.json");
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert!(!settings.tracking_enabled);
        assert_eq!(settings.scroll_sensitivity, 1.0);
        assert_eq!(settings.swipe_threshold, 50.0);
        assert_eq!(settings.pinch_threshold, 30.0);
        assert_eq!(settings.zoom_threshold, 25.0);
        assert_eq!(settings.zoom_time_window, 0.5);
    }

    #[test]
    fn test_settings_serialization() {
        let settings = Settings {
            tracking_enabled: true,
            scroll_sensitivity: 2.5,
            swipe_threshold: 75.0,
            pinch_threshold: 32.0,
            zoom_threshold: 28.0,
            zoom_time_window: 0.6,
        };

        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: Settings = serde_json::from_str(&json).unwrap();

        assert!(deserialized.tracking_enabled);
        assert_eq!(deserialized.scroll_sensitivity, 2.5);
        assert_eq!(deserialized.swipe_threshold, 75.0);
        assert_eq!(deserialized.pinch_threshold, 32.0);
        assert_eq!(deserialized.zoom_threshold, 28.0);
        assert_eq!(deserialized.zoom_time_window, 0.6);
    }

    #[test]
    fn test_save_and_load_settings() {
        // Use a temporary file for testing
        let temp_dir = std::env::temp_dir().join("screencursor_test");
        let temp_path = temp_dir.join("settings.json");

        // Clean up before test
        if temp_path.exists() {
            std::fs::remove_file(&temp_path).unwrap();
        }

        let settings = Settings {
            tracking_enabled: true,
            scroll_sensitivity: 2.0,
            swipe_threshold: 60.0,
            pinch_threshold: 35.0,
            zoom_threshold: 30.0,
            zoom_time_window: 0.6,
        };

        // Save to temp path
        let json = serde_json::to_string_pretty(&settings).unwrap();
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(&temp_path, json).unwrap();

        // Load from temp path
        let contents = std::fs::read_to_string(&temp_path).unwrap();
        let loaded: Settings = serde_json::from_str(&contents).unwrap();

        assert!(loaded.tracking_enabled);
        assert_eq!(loaded.scroll_sensitivity, 2.0);
        assert_eq!(loaded.swipe_threshold, 60.0);
        assert_eq!(loaded.pinch_threshold, 35.0);
        assert_eq!(loaded.zoom_threshold, 30.0);
        assert_eq!(loaded.zoom_time_window, 0.6);

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
