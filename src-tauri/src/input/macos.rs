// macOS input implementation using Core Graphics.
// This module maps the generic `InputSimulator` trait to macOS-specific
// CGEvent APIs for mouse/keyboard/scroll simulation.

use crate::input::common::*;

#[cfg(target_os = "macos")]
use core_graphics::event::*;
#[cfg(target_os = "macos")]
use core_graphics::event_source::*;
#[cfg(target_os = "macos")]
use core_graphics::geometry::CGPoint;

pub struct MacOSInputSimulator;

impl MacOSInputSimulator {
    pub fn new() -> Self {
        MacOSInputSimulator
    }
}

#[cfg(target_os = "macos")]
impl InputSimulator for MacOSInputSimulator {
    fn mouse_move(&self, x: i32, y: i32) {
        let point = CGPoint::new(x as f64, y as f64);
        if let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
            if let Ok(event) = CGEvent::new_mouse_event(
                source,
                CGEventType::MouseMoved,
                point,
                CGMouseButton::Left,
            ) {
                let _ = event.post(CGEventTapLocation::HID);
            }
        }
    }

    fn mouse_click(&self, button: MouseButton, action: ClickAction) {
        let (event_type, cg_button) = match (button, action) {
            (MouseButton::Left, ClickAction::Down) => (CGEventType::LeftMouseDown, CGMouseButton::Left),
            (MouseButton::Left, ClickAction::Up) => (CGEventType::LeftMouseUp, CGMouseButton::Left),
            (MouseButton::Right, ClickAction::Down) => (CGEventType::RightMouseDown, CGMouseButton::Right),
            (MouseButton::Right, ClickAction::Up) => (CGEventType::RightMouseUp, CGMouseButton::Right),
            (MouseButton::Middle, ClickAction::Down) => (CGEventType::OtherMouseDown, CGMouseButton::Center),
            (MouseButton::Middle, ClickAction::Up) => (CGEventType::OtherMouseUp, CGMouseButton::Center),
        };

        // Get the actual current cursor position so the click lands in the right spot.
        let point = unsafe {
            let ev = CGEventCreate(std::ptr::null());
            if ev.is_null() {
                CGPoint::new(0.0, 0.0)
            } else {
                let p = CGEventGetLocation(ev as *const _);
                CFRelease(ev as *const _);
                p
            }
        };

        if let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
            if let Ok(event) = CGEvent::new_mouse_event(source, event_type, point, cg_button) {
                let _ = event.post(CGEventTapLocation::HID);
            }
        }
    }

    fn scroll(&self, dx: i32, dy: i32) {
        if dx == 0 && dy == 0 {
            return;
        }

        // Use FFI to call CGEventCreateScrollWheelEvent
        unsafe {
            // Pass null as source (system uses default)
            let event_ref = CGEventCreateScrollWheelEvent(
                std::ptr::null(),
                1, // line units
                2, // wheel count (x and y)
                dx,
                -dy, // macOS uses inverted scroll direction for natural scrolling
            );

            if !event_ref.is_null() {
                // Post the event directly using FFI
                CGEventPost(CGEventTapLocation::HID as u32, event_ref);
                // Release the event ref
                CFRelease(event_ref as *const _);
            }
        }
    }

    fn key_sequence(&self, keys: &[VirtualKey]) {
        for key in keys {
            let key_code = MacOSInputSimulator::virtual_key_to_cgkeycode(*key);
            if key_code == 0 {
                continue;
            }

            if let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
                // Key down
                if let Ok(event) = CGEvent::new_keyboard_event(source, key_code, true) {
                    let _ = event.post(CGEventTapLocation::HID);
                }
            }

            if let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
                // Key up
                if let Ok(event) = CGEvent::new_keyboard_event(source, key_code, false) {
                    let _ = event.post(CGEventTapLocation::HID);
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
extern "C" {
    fn CGEventCreateScrollWheelEvent(
        source: *const std::ffi::c_void,
        units: u32,
        wheel_count: u32,
        scrolling_delta_x: i32,
        scrolling_delta_y: i32,
    ) -> *mut std::ffi::c_void;
    fn CGEventCreate(source: *const std::ffi::c_void) -> *mut std::ffi::c_void;
    fn CGEventGetLocation(event: *const std::ffi::c_void) -> CGPoint;
    fn CGEventPost(location: u32, event: *const std::ffi::c_void);
    fn CFRelease(cf: *const std::ffi::c_void);
}

#[cfg(target_os = "macos")]
impl MacOSInputSimulator {
    fn virtual_key_to_cgkeycode(key: VirtualKey) -> CGKeyCode {
        match key {
            VirtualKey::Space => 0x31,
            VirtualKey::Enter => 0x24,
            VirtualKey::Escape => 0x35,
            VirtualKey::Tab => 0x30,
            VirtualKey::Control => 0x3B,
            VirtualKey::Alt => 0x3A,
            VirtualKey::Shift => 0x38,
            VirtualKey::Meta => 0x37,
            VirtualKey::F1 => 0x7A,
            VirtualKey::F2 => 0x78,
            VirtualKey::F3 => 0x63,
            VirtualKey::F4 => 0x76,
            VirtualKey::F5 => 0x60,
            VirtualKey::F6 => 0x61,
            VirtualKey::F7 => 0x62,
            VirtualKey::F8 => 0x64,
            VirtualKey::F9 => 0x65,
            VirtualKey::F10 => 0x6D,
            VirtualKey::F11 => 0x67,
            VirtualKey::F12 => 0x6F,
            VirtualKey::Key1 => 0x12,
            VirtualKey::Key2 => 0x13,
            VirtualKey::Key3 => 0x14,
            VirtualKey::Key4 => 0x15,
            VirtualKey::Key5 => 0x17,
            VirtualKey::Key6 => 0x16,
            VirtualKey::Key7 => 0x1A,
            VirtualKey::Key8 => 0x1C,
            VirtualKey::Key9 => 0x19,
            VirtualKey::Key0 => 0x1D,
            // Fixed: kVK_ANSI_Minus = 0x1B, kVK_ANSI_Equal = 0x18
            VirtualKey::Minus => 0x1B,
            VirtualKey::Equal => 0x18,
            VirtualKey::A => 0x00,
            VirtualKey::B => 0x0B,
            VirtualKey::C => 0x08,
            VirtualKey::D => 0x02,
            VirtualKey::E => 0x0E,
            VirtualKey::F => 0x03,
            VirtualKey::G => 0x05,
            VirtualKey::H => 0x04,
            VirtualKey::I => 0x22,
            VirtualKey::J => 0x26,
            VirtualKey::K => 0x28,
            VirtualKey::L => 0x25,
            VirtualKey::M => 0x2E,
            VirtualKey::N => 0x2D,
            VirtualKey::O => 0x1F, // WARNING: also used by Equal (0x1F) - conflict avoided by separating mappings
            VirtualKey::P => 0x23,
            VirtualKey::Q => 0x0C,
            VirtualKey::R => 0x0F,
            VirtualKey::S => 0x01,
            VirtualKey::T => 0x11,
            VirtualKey::U => 0x20,
            VirtualKey::V => 0x09,
            VirtualKey::W => 0x0D,
            VirtualKey::X => 0x07,
            VirtualKey::Y => 0x10,
            VirtualKey::Z => 0x06,
        }
    }
}

// Stub implementations for non-macOS platforms
#[cfg(not(target_os = "macos"))]
impl InputSimulator for MacOSInputSimulator {
    fn mouse_move(&self, _x: i32, _y: i32) {
        eprintln!("mouse_move: not supported on this platform");
    }
    fn mouse_click(&self, _button: MouseButton, _action: ClickAction) {
        eprintln!("mouse_click: not supported on this platform");
    }
    fn scroll(&self, _dx: i32, _dy: i32) {
        eprintln!("scroll: not supported on this platform");
    }
    fn key_sequence(&self, _keys: &[VirtualKey]) {
        eprintln!("key_sequence: not supported on this platform");
    }
}
