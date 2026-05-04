#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesture {
    LeftClick,
    RightClick,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
    SwipeUp,
    SwipeDown,
    SwipeLeft,
    SwipeRight,
    ZoomIn,
    ZoomOut,
    AppSwitch,
    Screenshot,
}

impl Gesture {
    pub fn as_str(&self) -> &'static str {
        match self {
            Gesture::LeftClick => "Left Click",
            Gesture::RightClick => "Right Click",
            Gesture::ScrollUp => "Scroll Up",
            Gesture::ScrollDown => "Scroll Down",
            Gesture::ScrollLeft => "Scroll Left",
            Gesture::ScrollRight => "Scroll Right",
            Gesture::SwipeUp => "Swipe Up",
            Gesture::SwipeDown => "Swipe Down",
            Gesture::SwipeLeft => "Swipe Left",
            Gesture::SwipeRight => "Swipe Right",
            Gesture::ZoomIn => "Zoom In",
            Gesture::ZoomOut => "Zoom Out",
            Gesture::AppSwitch => "App Switch",
            Gesture::Screenshot => "Screenshot",
        }
    }
}
