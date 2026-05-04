// Generic input interface used by the tracking loop.
// Platform-specific implementations are in macos.rs and windows.rs.

pub trait InputSimulator {
    fn mouse_move(&self, _x: i32, _y: i32) {}
    fn mouse_click(&self, _button: MouseButton, _action: ClickAction) {}
    fn scroll(&self, _dx: i32, _dy: i32) {}
    fn key_sequence(&self, _keys: &[VirtualKey]) {}
}

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy)]
pub enum ClickAction {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy)]
pub enum VirtualKey {
    // Common keys
    Space,
    Enter,
    Escape,
    Tab,
    // Modifier keys
    Control,
    Alt,
    Shift,
    Meta, // Cmd on macOS, Win key on Windows
    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    // Number keys
    Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9, Key0,
    // Symbol keys
    Minus, Equal,
    // Letter keys
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_key_variants() {
        // Ensure all variants are coverable
        let keys = vec![
            VirtualKey::Space,
            VirtualKey::Enter,
            VirtualKey::Escape,
            VirtualKey::Tab,
            VirtualKey::Control,
            VirtualKey::Alt,
            VirtualKey::Shift,
            VirtualKey::Meta,
            VirtualKey::F1,
            VirtualKey::Minus,
            VirtualKey::Equal,
        ];
        for key in keys {
            // Just ensure they can be cloned and debug-formatted
            let _ = format!("{:?}", key);
            let _ = key.clone();
        }
    }

    #[test]
    fn test_mouse_button_debug() {
        let button = MouseButton::Left;
        let _ = format!("{:?}", button);
    }

    #[test]
    fn test_click_action_debug() {
        let action = ClickAction::Down;
        let _ = format!("{:?}", action);
    }
}
