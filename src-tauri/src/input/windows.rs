use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::Foundation::*;
use crate::input::common::*;
use std::mem;

pub struct WindowsInputSimulator;

impl WindowsInputSimulator {
    pub fn new() -> Self {
        WindowsInputSimulator
    }
}

impl InputSimulator for WindowsInputSimulator {
    fn mouse_move(&self, x: i32, y: i32) {
        unsafe {
            // Get screen dimensions
            let screen_width = GetSystemMetrics(SM_CXSCREEN) as i32;
            let screen_height = GetSystemMetrics(SM_CYSCREEN) as i32;

            // Normalize to 0..65535 (required for ABSOLUTE positioning)
            let nx = ((x as f64 / screen_width as f64) * 65535.0) as i32;
            let ny = ((y as f64 / screen_height as f64) * 65535.0) as i32;

            let mut input = INPUT::default();
            input.r#type = INPUT_MOUSE;
            input.Anonymous.mi.dwFlags = MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE;
            input.Anonymous.mi.dx = nx;
            input.Anonymous.mi.dy = ny;
            SendInput(&[input], mem::size_of::<INPUT>() as i32);
        }
    }

    fn mouse_click(&self, button: MouseButton, action: ClickAction) {
        let flags = match (button, action) {
            (MouseButton::Left, ClickAction::Down) => MOUSEEVENTF_LEFTDOWN,
            (MouseButton::Left, ClickAction::Up) => MOUSEEVENTF_LEFTUP,
            (MouseButton::Right, ClickAction::Down) => MOUSEEVENTF_RIGHTDOWN,
            (MouseButton::Right, ClickAction::Up) => MOUSEEVENTF_RIGHTUP,
            (MouseButton::Middle, ClickAction::Down) => MOUSEEVENTF_MIDDLEDOWN,
            (MouseButton::Middle, ClickAction::Up) => MOUSEEVENTF_MIDDLEUP,
            _ => return,
        };

        unsafe {
            let mut input = INPUT::default();
            input.r#type = INPUT_MOUSE;
            input.Anonymous.mi.dwFlags = flags;
            SendInput(&[input], mem::size_of::<INPUT>() as i32);
        }
    }

    fn scroll(&self, dx: i32, dy: i32) {
        unsafe {
            if dy != 0 {
                let mut input = INPUT::default();
                input.r#type = INPUT_MOUSE;
                input.Anonymous.mi.dwFlags = MOUSEEVENTF_WHEEL;
                input.Anonymous.mi.mouseData = dy as u32;
                SendInput(&[input], mem::size_of::<INPUT>() as i32);
            }

            if dx != 0 {
                let mut input = INPUT::default();
                input.r#type = INPUT_MOUSE;
                input.Anonymous.mi.dwFlags = MOUSEEVENTF_HWHEEL;
                input.Anonymous.mi.mouseData = dx as u32;
                SendInput(&[input], mem::size_of::<INPUT>() as i32);
            }
        }
    }

    fn key_sequence(&self, keys: &[VirtualKey]) {
        for key in keys {
            if let Some(vk) = Self::virtual_key_for(*key) {
                unsafe {
                    // Key down
                    let mut input_down = INPUT::default();
                    input_down.r#type = INPUT_KEYBOARD;
                    input_down.Anonymous.ki.wVk = vk;
                    input_down.Anonymous.ki.dwFlags = KEYEVENTF_NONE;
                    SendInput(&[input_down], mem::size_of::<INPUT>() as i32);

                    // Key up
                    let mut input_up = INPUT::default();
                    input_up.r#type = INPUT_KEYBOARD;
                    input_up.Anonymous.ki.wVk = vk;
                    input_up.Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;
                    SendInput(&[input_up], mem::size_of::<INPUT>() as i32);
                }
            }
        }
    }
}

impl WindowsInputSimulator {
    fn virtual_key_for(key: VirtualKey) -> Option<u16> {
        match key {
            VirtualKey::Space => Some(VK_SPACE.0),
            VirtualKey::Enter => Some(VK_RETURN.0),
            VirtualKey::Escape => Some(VK_ESCAPE.0),
            VirtualKey::Tab => Some(VK_TAB.0),
            VirtualKey::Control => Some(VK_CONTROL.0),
            VirtualKey::Alt => Some(VK_MENU.0),
            VirtualKey::Shift => Some(VK_SHIFT.0),
            VirtualKey::Meta => Some(VK_LWIN.0),
            VirtualKey::F1 => Some(VK_F1.0),
            VirtualKey::F2 => Some(VK_F2.0),
            VirtualKey::F3 => Some(VK_F3.0),
            VirtualKey::F4 => Some(VK_F4.0),
            VirtualKey::F5 => Some(VK_F5.0),
            VirtualKey::F6 => Some(VK_F6.0),
            VirtualKey::F7 => Some(VK_F7.0),
            VirtualKey::F8 => Some(VK_F8.0),
            VirtualKey::F9 => Some(VK_F9.0),
            VirtualKey::F10 => Some(VK_F10.0),
            VirtualKey::F11 => Some(VK_F11.0),
            VirtualKey::F12 => Some(VK_F12.0),
            VirtualKey::Key1 => Some(VK_1.0),
            VirtualKey::Key2 => Some(VK_2.0),
            VirtualKey::Key3 => Some(VK_3.0),
            VirtualKey::Key4 => Some(VK_4.0),
            VirtualKey::Minus => Some(VK_OEM_MINUS.0),
            VirtualKey::Equal => Some(VK_OEM_PLUS.0),
            VirtualKey::A => Some(VK_A.0),
            VirtualKey::B => Some(VK_B.0),
            VirtualKey::C => Some(VK_C.0),
            VirtualKey::D => Some(VK_D.0),
            VirtualKey::E => Some(VK_E.0),
            VirtualKey::F => Some(VK_F.0),
            VirtualKey::G => Some(VK_G.0),
            VirtualKey::H => Some(VK_H.0),
            VirtualKey::I => Some(VK_I.0),
            VirtualKey::J => Some(VK_J.0),
            VirtualKey::K => Some(VK_K.0),
            VirtualKey::L => Some(VK_L.0),
            VirtualKey::M => Some(VK_M.0),
            VirtualKey::N => Some(VK_N.0),
            VirtualKey::O => Some(VK_O.0),
            VirtualKey::P => Some(VK_P.0),
            VirtualKey::Q => Some(VK_Q.0),
            VirtualKey::R => Some(VK_R.0),
            VirtualKey::S => Some(VK_S.0),
            VirtualKey::T => Some(VK_T.0),
            VirtualKey::U => Some(VK_U.0),
            VirtualKey::V => Some(VK_V.0),
            VirtualKey::W => Some(VK_W.0),
            VirtualKey::X => Some(VK_X.0),
            VirtualKey::Y => Some(VK_Y.0),
            VirtualKey::Z => Some(VK_Z.0),
            _ => None,
        }
    }
}
