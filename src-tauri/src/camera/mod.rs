// Camera abstraction over OpenCV VideoCapture.
// Tries indices 0-4 to find a working camera and sets a 640x480 resolution.
// Provides frame capture and proper Drop cleanup.

use opencv::videoio;
use opencv::core::Mat;
use opencv::prelude::MatTraitConst;
use opencv::prelude::VideoCaptureTraitConst;
use opencv::prelude::VideoCaptureTrait;

pub struct Camera {
    capture: Option<videoio::VideoCapture>,
    camera_index: i32,
}

impl Camera {
    pub fn new() -> Result<Self, String> {
        // Try camera indices 0-5 to find a working camera
        // On macOS, index 0 may be an iPhone (Continuity Camera)
        // so we try several indices
        for camera_index in 0..5 {
            eprintln!("Trying camera index {}", camera_index);
            match videoio::VideoCapture::new(camera_index, videoio::CAP_ANY) {
                Ok(mut capture) => {
                    if capture.is_opened().unwrap_or(false) {
                        // Try to read a test frame to verify it works
                        let mut test_frame = Mat::default();
                        match capture.read(&mut test_frame) {
                            Ok(_) if !test_frame.empty() => {
                                eprintln!("Successfully opened camera at index {}", camera_index);
                                let mut cam = Camera {
                                    capture: Some(capture),
                                    camera_index,
                                };
                                // Set reasonable default settings
                                let _ = cam.set_resolution(640, 480);
                                return Ok(cam);
                            }
                            Ok(_) => eprintln!("Camera {} returned empty frame", camera_index),
                            Err(e) => eprintln!("Camera {} read error: {}", camera_index, e),
                        }
                    } else {
                        eprintln!("Camera {} is not opened", camera_index);
                    }
                }
                Err(e) => eprintln!("Failed to open camera {}: {}", camera_index, e),
            }
        }

        Err("No working camera found. Tried indices 0-4.".to_string())
    }

    pub fn camera_index(&self) -> i32 {
        self.camera_index
    }

    pub fn set_resolution(&mut self, width: i32, height: i32) -> Result<(), String> {
        if let Some(ref mut capture) = self.capture {
            let _ = capture.set(videoio::CAP_PROP_FRAME_WIDTH, width as f64);
            let _ = capture.set(videoio::CAP_PROP_FRAME_HEIGHT, height as f64);
        }
        Ok(())
    }

    pub fn capture_frame(&mut self) -> Result<Mat, String> {
        let capture = self.capture.as_mut().ok_or("Camera not initialized")?;
        let mut frame = Mat::default();
        capture.read(&mut frame).map_err(|e| format!("Failed to read frame: {}", e))?;

        if frame.empty() {
            return Err("Captured frame is empty".to_string());
        }

        Ok(frame)
    }

    pub fn is_opened(&self) -> bool {
        self.capture.as_ref().map_or(false, |c| c.is_opened().unwrap_or(false))
    }
}

impl Drop for Camera {
    fn drop(&mut self) {
        if let Some(mut capture) = self.capture.take() {
            let _ = capture.release();
        }
    }
}
