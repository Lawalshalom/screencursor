// lib.rs — Tauri backend.
//
// Hand tracking is delegated to a Python sidecar (track/sidecar.py) that uses
// Google's MediaPipe Tasks API. The sidecar outputs one JSON line per frame to
// stdout; this file reads those lines, emits Tauri events, and simulates input.
//
// The camera module is kept for the raw Calibration preview (no detection overlay).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod camera;
pub mod input;
pub mod tray;
pub mod settings;
pub mod utils;

// Keep hand/gesture modules for compilation; detection is now in Python.
pub mod hand;
pub mod gesture;

use std::sync::{Arc, Mutex, OnceLock};
use std::process::{Child, Command, Stdio};
use std::io::{BufRead, BufReader};
use tauri::{Manager, Emitter};
use opencv::prelude::VectorToVec;

static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

// ── AppState ──────────────────────────────────────────────────────────────────
#[derive(Clone)]
pub struct AppState {
    pub tracking:        Arc<Mutex<bool>>,
    pub camera:          Arc<Mutex<Option<camera::Camera>>>,
    pub sidecar_process: Arc<Mutex<Option<Child>>>,
    pub input_sim:       Arc<Mutex<Option<Box<dyn input::common::InputSimulator + Send>>>>,
    pub prev_left_held:  Arc<Mutex<bool>>,
    pub preview_frame:   Arc<Mutex<Option<String>>>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            tracking:        Arc::new(Mutex::new(false)),
            camera:          Arc::new(Mutex::new(None)),
            sidecar_process: Arc::new(Mutex::new(None)),
            input_sim:       Arc::new(Mutex::new(None)),
            prev_left_held:  Arc::new(Mutex::new(false)),
            preview_frame:   Arc::new(Mutex::new(None)),
        }
    }
}

// ── Tauri setup ───────────────────────────────────────────────────────────────
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            APP_HANDLE.set(app.handle().clone()).expect("APP_HANDLE set");

            let state = app.state::<AppState>();

            // Camera for Calibration preview (best-effort).
            if let Ok(mut cam) = state.camera.lock() {
                match camera::Camera::new() {
                    Ok(c) => { eprintln!("Camera opened at index {}", c.camera_index()); *cam = Some(c); }
                    Err(e) => eprintln!("Camera warning: {e}"),
                }
            }

            if let Ok(mut sim) = state.input_sim.lock() {
                *sim = Some(input::create_simulator());
            }

            if let Some(w) = app.get_webview_window("main") {
                #[cfg(debug_assertions)]   let _ = w.show();
                #[cfg(not(debug_assertions))] let _ = w.hide();
            }

            if let Err(e) = tray::create_tray(&app.handle()) {
                eprintln!("Tray warning: {e}");
            }
            Ok(())
        })
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            start_tracking,
            stop_tracking,
            get_settings,
            update_settings,
            get_camera_preview,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri run failed");
}

// ── Python sidecar path resolution ───────────────────────────────────────────
/// Returns (python_executable, sidecar_script_path).
/// Looks for the venv Python first, then falls back to system python3.
fn find_sidecar() -> Result<(String, String), String> {
    // src-tauri/ → parent = project root
    let manifest     = env!("CARGO_MANIFEST_DIR");
    let project_root = std::path::Path::new(manifest)
        .parent().ok_or("Cannot determine project root")?;

    // sidecar/sidecar.py
    let sidecar = project_root.join("sidecar").join("sidecar.py");
    if !sidecar.exists() {
        return Err(format!(
            "sidecar/sidecar.py not found at {} — run: python3 -m venv sidecar/venv && sidecar/venv/bin/pip install -r sidecar/requirements.txt",
            sidecar.display()
        ));
    }

    // Python search: sidecar/venv (preferred) → system python3
    let venv_py = project_root.join("sidecar").join("venv").join("bin").join("python3");
    let python = if venv_py.exists() {
        venv_py.to_string_lossy().to_string()
    } else {
        eprintln!("[sidecar] WARNING: sidecar/venv not found, falling back to system python3");
        "python3".to_string()
    };

    eprintln!("[sidecar] python={python}  script={}", sidecar.display());
    Ok((python, sidecar.to_string_lossy().to_string()))
}

// ── Tracking commands ─────────────────────────────────────────────────────────
#[tauri::command]
fn start_tracking(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let app_state = state.inner().clone();
    {
        let mut t = app_state.tracking.lock().map_err(|_| "lock error")?;
        if *t { return Ok("Already tracking".to_string()); }
        *t = true;
    }

    // Release the Rust-side camera so the sidecar can open it exclusively.
    if let Ok(mut cam) = app_state.camera.lock() {
        *cam = None;  // drops the Camera, releasing the device
    }

    let (python, script) = find_sidecar()?;
    eprintln!("Spawning sidecar: {python} {script}");

    let mut child = Command::new(&python)
        .arg(&script)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to spawn sidecar: {e}"))?;

    let stdout = child.stdout.take().ok_or("Sidecar has no stdout")?;

    if let Ok(mut proc) = app_state.sidecar_process.lock() {
        *proc = Some(child);
    }

    std::thread::spawn(move || {
        read_sidecar_output(stdout, app_state);
    });

    Ok("Tracking started".to_string())
}

#[tauri::command]
fn stop_tracking(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let s = state.inner();
    {
        let mut t = s.tracking.lock().map_err(|_| "lock error")?;
        if !*t { return Ok("Not tracking".to_string()); }
        *t = false;
    }
    // Kill the sidecar process.
    if let Ok(mut proc) = s.sidecar_process.lock() {
        if let Some(mut child) = proc.take() {
            let _ = child.kill();
            let _ = child.wait(); // reap to free resources
        }
    }
    // Clear cached preview frame.
    if let Ok(mut pf) = s.preview_frame.lock() { *pf = None; }
    // Reopen the Rust-side camera for preview polling.
    std::thread::sleep(std::time::Duration::from_millis(300));
    if let Ok(mut cam) = s.camera.lock() {
        if cam.is_none() {
            match camera::Camera::new() {
                Ok(c) => { *cam = Some(c); }
                Err(e) => eprintln!("Camera reopen warning: {e}"),
            }
        }
    }
    // Notify frontend that tracking has stopped.
    if let Some(h) = APP_HANDLE.get() {
        let _ = h.emit("tracking-stopped", ());
    }
    Ok("Tracking stopped".to_string())
}

/// Same as start/stop but callable without Tauri State (e.g. from tray).
pub fn toggle_tracking(state: &AppState) -> Result<String, String> {
    let tracking = state.tracking.lock().map_err(|_| "lock error")?;
    if *tracking {
        drop(tracking);
        // Re-use stop logic via direct mutation.
        let mut t = state.tracking.lock().map_err(|_| "lock error")?;
        *t = false;
        drop(t);
        if let Ok(mut proc) = state.sidecar_process.lock() {
            if let Some(mut child) = proc.take() { let _ = child.kill(); }
        }
        Ok("Tracking stopped".to_string())
    } else {
        drop(tracking);
        // Clone state so we can call the command logic.
        let state_clone = state.clone();
        let mut t = state_clone.tracking.lock().map_err(|_| "lock error")?;
        *t = true;
        drop(t);

        let (python, script) = find_sidecar()?;
        let mut child = Command::new(&python)
            .arg(&script)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("Spawn failed: {e}"))?;
        let stdout = child.stdout.take().ok_or("no stdout")?;
        if let Ok(mut proc) = state_clone.sidecar_process.lock() { *proc = Some(child); }
        std::thread::spawn(move || read_sidecar_output(stdout, state_clone));
        Ok("Tracking started".to_string())
    }
}

// ── Sidecar output reader ─────────────────────────────────────────────────────
#[derive(serde::Deserialize, Clone)]
struct SidecarEvent {
    state:      String,
    message:    String,
    hint:       String,
    gesture:    Option<String>,
    confidence: Option<f32>,
    /// Normalised cursor position from single-hand mode (0.0–1.0).
    cursor_x:   Option<f32>,
    cursor_y:   Option<f32>,
    /// Base64 JPEG preview frame (every ~4th frame).
    frame:      Option<String>,
}

#[derive(Clone, serde::Serialize)]
struct TrackingStatus {
    state:      String,
    message:    String,
    hint:       String,
    gesture:    Option<String>,
    confidence: Option<f32>,
}

fn read_sidecar_output(stdout: std::process::ChildStdout, state: AppState) {
    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        // Stop if tracking was cancelled.
        if let Ok(t) = state.tracking.lock() {
            if !*t { break; }
        }

        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let event: SidecarEvent = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(e) => { eprintln!("Sidecar parse error: {e} — raw: {line}"); continue; }
        };

        // Forward as tracking-status event to frontend.
        let status = TrackingStatus {
            state:      event.state.clone(),
            message:    event.message.clone(),
            hint:       event.hint.clone(),
            gesture:    event.gesture.clone(),
            confidence: event.confidence,
        };
        if let Some(h) = APP_HANDLE.get() {
            let _ = h.emit("tracking-status", &status);
        }

        // Move mouse if cursor coordinates provided (single-hand mode).
        if let (Some(cx), Some(cy)) = (event.cursor_x, event.cursor_y) {
            let (sw, sh) = screen_size();
            let sx = (cx as f64 * sw) as i32;
            let sy = (cy as f64 * sh) as i32;
            if let Ok(mut g) = state.input_sim.lock() {
                if let Some(sim) = g.as_mut() {
                    sim.mouse_move(sx, sy);
                }
            }
        }

        // Cache + emit annotated preview frame when present.
        if let Some(ref f) = event.frame {
            if let Ok(mut pf) = state.preview_frame.lock() {
                *pf = Some(f.clone());
            }
            if let Some(h) = APP_HANDLE.get() {
                let _ = h.emit("preview-frame", f.as_str());
            }
        }

        // If a gesture was recognised, also emit gesture-detected and simulate.
        if let Some(ref g) = event.gesture {
            if let Some(h) = APP_HANDLE.get() {
                let _ = h.emit("gesture-detected", g.as_str());
            }
            simulate_gesture_str(&state, g);
        }
    }
    eprintln!("Sidecar output stream ended");
}

// ── Input simulation (gesture name → OS action) ───────────────────────────────
fn simulate_gesture_str(state: &AppState, gesture: &str) {
    use input::common::{MouseButton, ClickAction, VirtualKey};

    let mut sim_guard = match state.input_sim.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    let sim = match sim_guard.as_mut() {
        Some(s) => s,
        None => return,
    };

    match gesture {
        "Left Click" => {
            sim.mouse_click(MouseButton::Left,  ClickAction::Down);
            sim.mouse_click(MouseButton::Left,  ClickAction::Up);
        }
        "Double Click" => {
            sim.mouse_click(MouseButton::Left,  ClickAction::Down);
            sim.mouse_click(MouseButton::Left,  ClickAction::Up);
            sim.mouse_click(MouseButton::Left,  ClickAction::Down);
            sim.mouse_click(MouseButton::Left,  ClickAction::Up);
        }
        "Left Down" => {
            sim.mouse_click(MouseButton::Left,  ClickAction::Down);
        }
        "Left Up" => {
            sim.mouse_click(MouseButton::Left,  ClickAction::Up);
        }
        "Right Click" => {
            sim.mouse_click(MouseButton::Right, ClickAction::Down);
            sim.mouse_click(MouseButton::Right, ClickAction::Up);
        }
        "Scroll Up"    => sim.scroll(0, -3),
        "Scroll Down"  => sim.scroll(0,  3),
        "Scroll Left"  => sim.scroll(-3, 0),
        "Scroll Right" => sim.scroll( 3, 0),

        "Swipe Left"  => {
            #[cfg(target_os = "macos")]
            sim.key_sequence(&[VirtualKey::Meta, VirtualKey::A]); // placeholder
        }
        "Swipe Right" => {
            #[cfg(target_os = "macos")]
            sim.key_sequence(&[VirtualKey::Meta, VirtualKey::E]); // placeholder
        }
        "Swipe Up" => {
            #[cfg(target_os = "macos")]
            sim.key_sequence(&[VirtualKey::Meta, VirtualKey::F]);
        }
        "Swipe Down" => {
            #[cfg(target_os = "macos")]
            sim.key_sequence(&[VirtualKey::Meta, VirtualKey::D]);
        }
        "Zoom In" => {
            #[cfg(target_os = "macos")]
            sim.key_sequence(&[VirtualKey::Meta, VirtualKey::Equal]);
            #[cfg(target_os = "windows")]
            sim.key_sequence(&[VirtualKey::Control, VirtualKey::Equal]);
        }
        "Zoom Out" => {
            #[cfg(target_os = "macos")]
            sim.key_sequence(&[VirtualKey::Meta, VirtualKey::Minus]);
            #[cfg(target_os = "windows")]
            sim.key_sequence(&[VirtualKey::Control, VirtualKey::Minus]);
        }
        "App Switch" => {
            #[cfg(target_os = "macos")]
            sim.key_sequence(&[VirtualKey::Meta, VirtualKey::Tab]);
            #[cfg(target_os = "windows")]
            sim.key_sequence(&[VirtualKey::Alt, VirtualKey::Tab]);
        }
        "Screenshot" => {
            #[cfg(target_os = "macos")]
            sim.key_sequence(&[VirtualKey::Meta, VirtualKey::Shift, VirtualKey::Key4]);
            #[cfg(target_os = "windows")]
            sim.key_sequence(&[VirtualKey::Meta, VirtualKey::Shift, VirtualKey::S]);
        }
        _ => {} // Swipes and unknowns: no OS action yet
    }
}

// ── Settings commands ─────────────────────────────────────────────────────────
#[tauri::command]
fn get_settings() -> Result<String, String> {
    serde_json::to_string(&settings::load_settings())
        .map_err(|e| format!("Serialise error: {e}"))
}

#[tauri::command]
fn update_settings(settings_json: String, _state: tauri::State<'_, AppState>) -> Result<String, String> {
    let s: settings::Settings = serde_json::from_str(&settings_json)
        .map_err(|e| format!("Parse error: {e}"))?;
    settings::save_settings(&s)
        .map_err(|e| format!("Save error: {e}"))?;
    Ok("Settings updated".to_string())
}

// ── Camera preview (raw frame, no detection overlay) ──────────────────────────



#[tauri::command]
fn get_camera_preview(state: tauri::State<'_, AppState>) -> Result<String, String> {
    if let Ok(t) = state.tracking.lock() {
        if *t {
            // While tracking the sidecar owns the camera — return the latest
            // annotated frame it streamed, or empty string if not yet available.
            return Ok(state.preview_frame
                .lock().ok()
                .and_then(|g| g.clone())
                .unwrap_or_default());
        }
    }
    capture_preview_jpeg(state.inner())
}

/// Returns screen (width, height) in points for cursor coordinate mapping.
#[cfg(target_os = "macos")]
fn screen_size() -> (f64, f64) {
    use core_graphics::display::CGDisplay;
    let b = CGDisplay::main().bounds();
    (b.size.width, b.size.height)
}
#[cfg(not(target_os = "macos"))]
fn screen_size() -> (f64, f64) { (1920.0, 1080.0) }

fn capture_preview_jpeg(state: &AppState) -> Result<String, String> {
    let frame = {
        let mut cam = state.camera.lock().map_err(|_| "camera lock")?;
        match cam.as_mut() {
            Some(c) => c.capture_frame().map_err(|e| format!("Capture: {e}"))?,
            None    => return Ok(String::new()),
        }
    };

    // Convert BGR → RGB so the browser renders correct colours.
    let mut rgb = opencv::core::Mat::default();
    opencv::imgproc::cvt_color(&frame, &mut rgb,
        opencv::imgproc::COLOR_BGR2RGB, 0,
        opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT)
        .map_err(|e| format!("cvtColor: {e}"))?;

    let mut buf    = opencv::core::Vector::<u8>::new();
    let     params = opencv::core::Vector::<i32>::new();
    opencv::imgcodecs::imencode(".jpg", &rgb, &mut buf, &params)
        .map_err(|e| format!("imencode: {e}"))?;

    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &buf.to_vec(),
    ))
}
