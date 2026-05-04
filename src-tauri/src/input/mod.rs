// Platform-specific input simulation.
// Routes to macos.rs or windows.rs to generate OS-level mouse/keyboard/scroll events.

pub mod common;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

use common::*;

pub fn create_simulator() -> Box<dyn InputSimulator + Send> {
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacOSInputSimulator::new());

    #[cfg(target_os = "windows")]
    return Box::new(windows::WindowsInputSimulator::new());

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    panic!("Unsupported platform");
}
