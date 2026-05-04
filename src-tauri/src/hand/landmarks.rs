// Landmark extraction — handpose_estimation_mediapipe_2023feb.onnx
//
// Model I/O (verified via onnxruntime):
//   Input  "input_1": [1, 224, 224, 3]  NHWC float32 [0,1]
//   Output "Identity":   [1, 63]   21 keypoints × 3 (x, y, z) in 224px space
//   Output "Identity_1": [1, 1]    hand-presence probability
//   Output "Identity_2": [1, 1]    handedness score
//   Output "Identity_3": [1, 63]   world-space keypoints (ignored)
//
// We use only "Identity" (pixel coords) and "Identity_1" (hand present score).
// Keypoint i: x = data[i*3], y = data[i*3+1]  in 0..224 range.
// Scale back to frame coords using the crop bbox.

use opencv::dnn;
use opencv::core::{self, Mat, Point2f, Size};
use opencv::imgproc;
use opencv::prelude::{NetTrait, NetTraitConst, MatTraitConst};
use std::path::Path;

#[derive(Default, Clone)]
pub struct HandLandmarks {
    pub points: Vec<Point2f>,
}

impl HandLandmarks {
    pub fn new() -> Self { HandLandmarks { points: Vec::new() } }

    pub fn get_point(&self, index: usize) -> Option<Point2f> {
        self.points.get(index).cloned()
    }

    pub fn len(&self) -> usize { self.points.len() }
}

pub struct LandmarkExtractor {
    net: dnn::Net,
}

impl LandmarkExtractor {
    pub fn new<P: AsRef<Path>>(model_path: P) -> Result<Self, String> {
        let net = dnn::read_net_from_onnx(model_path.as_ref().to_str().unwrap())
            .map_err(|e| format!("Failed to load landmark model: {}", e))?;
        Ok(LandmarkExtractor { net })
    }

    pub fn extract(
        &mut self,
        frame: &Mat,
        bbox:  &super::detection::PalmDetection,
    ) -> Result<HandLandmarks, String> {
        const MODEL_SIZE: i32 = 224;
        const PRESENCE_THRESH: f32 = 0.5;

        // ── 1. Crop palm region ───────────────────────────────────────────────
        let crop = bbox.bbox;
        let roi = frame.roi(crop)
            .map_err(|e| format!("Failed to crop: {}", e))?;

        // ── 2. Resize to 224×224 + BGR→RGB + float32 [0,1] ───────────────────
        let size = Size::new(MODEL_SIZE, MODEL_SIZE);
        let mut roi_resized = Mat::default();
        imgproc::resize(&roi, &mut roi_resized, size, 0.0, 0.0, imgproc::INTER_LINEAR)
            .map_err(|e| format!("Resize: {}", e))?;
        let mut rgb = Mat::default();
        opencv::imgproc::cvt_color(&roi_resized, &mut rgb,
            opencv::imgproc::COLOR_BGR2RGB, 0,
            opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT)
            .map_err(|e| format!("cvtColor: {}", e))?;
        let mut float_mat = Mat::default();
        rgb.convert_to(&mut float_mat, opencv::core::CV_32F, 1.0 / 255.0, 0.0)
            .map_err(|e| format!("convert_to: {}", e))?;

        // ── 3. Reshape to NHWC [1,224,224,3] blob ────────────────────────────
        let blob = float_mat.reshape_nd(1, &[1i32, MODEL_SIZE, MODEL_SIZE, 3])
            .map_err(|e| format!("reshape_nd: {}", e))?;

        // ── 4. Forward — retrieve "Identity" (keypoints) + "Identity_1" (presence) ─
        self.net.set_input(&blob, "", 1.0, core::Scalar::default())
            .map_err(|e| format!("set_input: {}", e))?;

        let out_names = self.net.get_unconnected_out_layers_names()
            .map_err(|e| format!("get_output_names: {}", e))?;
        let mut out_vecs: core::Vector<core::Vector<Mat>> = core::Vector::new();
        self.net.forward_and_retrieve(&mut out_vecs, &out_names)
            .map_err(|e| format!("forward: {}", e))?;

        if out_vecs.len() < 2 { return Ok(HandLandmarks::new()); }

        let kp_list  = out_vecs.get(0).map_err(|e| e.to_string())?;
        let pr_list  = out_vecs.get(1).map_err(|e| e.to_string())?;
        if kp_list.is_empty() || pr_list.is_empty() { return Ok(HandLandmarks::new()); }

        let kp_mat = kp_list.get(0).map_err(|e| e.to_string())?;
        let pr_mat = pr_list.get(0).map_err(|e| e.to_string())?;

        // Hand-presence check (already sigmoid'd by the model)
        let presence = *pr_mat.at_2d::<f32>(0, 0).unwrap_or(&0.0);
        if presence < PRESENCE_THRESH {
            return Ok(HandLandmarks::new());
        }

        // ── 5. Parse 21 keypoints from flat [1,63] tensor ────────────────────
        // Values are pixel coords in [0, 224] range relative to the crop.
        let mut landmarks = HandLandmarks::new();

        // Scale from model-space (0..224) back to frame-space
        let scale_x = crop.width  as f32 / MODEL_SIZE as f32;
        let scale_y = crop.height as f32 / MODEL_SIZE as f32;

        for i in 0..21 {
            let col_x = i * 3;
            let col_y = i * 3 + 1;
            if let (Ok(xv), Ok(yv)) = (
                kp_mat.at_2d::<f32>(0, col_x),
                kp_mat.at_2d::<f32>(0, col_y),
            ) {
                let x_frame = crop.x as f32 + *xv * scale_x;
                let y_frame = crop.y as f32 + *yv * scale_y;
                landmarks.points.push(Point2f::new(x_frame, y_frame));
            }
        }

        Ok(landmarks)
    }
}
