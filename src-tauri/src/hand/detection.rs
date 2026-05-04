// Palm detection — palm_detection_mediapipe_2023feb.onnx
//
// Model I/O (verified via onnxruntime):
//   Input  "input_1": [1, 192, 192, 3]  NHWC float32 [0,1]
//   Output "Identity":   [1, 2016, 18]  raw box regressions
//   Output "Identity_1": [1, 2016,  1]  raw logit scores  (sigmoid → probability)
//
// Post-processing mirrors the Python MediaPipe demo:
//   1. Generate 2016 SSD anchors (strides [8,16,16,16])
//   2. sigmoid(score) > threshold → candidate
//   3. Decode box: cx = anchor_cx + reg[0]/192,  cy = anchor_cy + reg[1]/192
//                  w  = reg[2]/192,               h  = reg[3]/192   (all normalised)
//   4. IoU-NMS → keep best

use opencv::dnn;
use opencv::core::{self, Mat, Rect, Size};
use opencv::prelude::{NetTrait, NetTraitConst, MatTraitConst};
use std::path::Path;

pub struct PalmDetection {
    pub bbox:       Rect,
    pub confidence: f32,
}

pub struct HandDetector {
    net:     dnn::Net,
    anchors: Vec<[f32; 2]>,   // (cx, cy) normalised [0,1]
}

// ── Anchor generation ────────────────────────────────────────────────────────
// MediaPipe palm-detection config:  strides=[8,16,16,16], 2 anchors/cell,
// fixed_anchor_size=true, anchor_offset=0.5, input=192.
fn generate_anchors() -> Vec<[f32; 2]> {
    const INPUT: f32 = 192.0;
    let strides: &[u32] = &[8, 16, 16, 16];
    let mut anchors = Vec::with_capacity(2016);
    for &stride in strides {
        let cells = (INPUT / stride as f32).ceil() as u32;
        for y in 0..cells {
            for x in 0..cells {
                for _ in 0..2 {
                    let cx = (x as f32 + 0.5) / cells as f32;
                    let cy = (y as f32 + 0.5) / cells as f32;
                    anchors.push([cx, cy]);
                }
            }
        }
    }
    anchors
}

// ── Sigmoid ──────────────────────────────────────────────────────────────────
#[inline]
fn sigmoid(x: f32) -> f32 { 1.0 / (1.0 + (-x).exp()) }

// ── IoU ─────────────────────────────────────────────────────────────────────
fn iou(a: &[f32; 4], b: &[f32; 4]) -> f32 {
    // a/b = [x1, y1, x2, y2] normalised
    let ix1 = a[0].max(b[0]);
    let iy1 = a[1].max(b[1]);
    let ix2 = a[2].min(b[2]);
    let iy2 = a[3].min(b[3]);
    let inter = (ix2 - ix1).max(0.0) * (iy2 - iy1).max(0.0);
    let area_a = (a[2] - a[0]) * (a[3] - a[1]);
    let area_b = (b[2] - b[0]) * (b[3] - b[1]);
    inter / (area_a + area_b - inter + 1e-6)
}

impl HandDetector {
    pub fn new<P: AsRef<Path>>(model_path: P) -> Result<Self, String> {
        let net = dnn::read_net_from_onnx(model_path.as_ref().to_str().unwrap())
            .map_err(|e| format!("Failed to load palm model: {}", e))?;
        let anchors = generate_anchors();
        eprintln!("Palm detector: {} anchors generated", anchors.len());
        Ok(HandDetector { net, anchors })
    }

    pub fn detect(&mut self, frame: &Mat) -> Result<Vec<PalmDetection>, String> {
        const MODEL_SIZE: i32 = 192;
        const SCORE_THRESH: f32 = 0.5;
        const NMS_THRESH:   f32 = 0.3;

        let frame_h = frame.rows() as f32;
        let frame_w = frame.cols() as f32;
        if frame_h <= 0.0 || frame_w <= 0.0 { return Ok(vec![]); }

        // ── 1. Build NHWC blob [1,192,192,3] ──────────────────────────────────
        let size = Size::new(MODEL_SIZE, MODEL_SIZE);
        let mut resized = Mat::default();
        opencv::imgproc::resize(frame, &mut resized, size, 0.0, 0.0,
            opencv::imgproc::INTER_LINEAR)
            .map_err(|e| format!("Resize: {}", e))?;
        let mut rgb = Mat::default();
        opencv::imgproc::cvt_color(&resized, &mut rgb,
            opencv::imgproc::COLOR_BGR2RGB, 0,
            opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT)
            .map_err(|e| format!("cvtColor: {}", e))?;
        let mut float_mat = Mat::default();
        rgb.convert_to(&mut float_mat, opencv::core::CV_32F, 1.0 / 255.0, 0.0)
            .map_err(|e| format!("convert_to: {}", e))?;
        let blob = float_mat.reshape_nd(1, &[1i32, MODEL_SIZE, MODEL_SIZE, 3])
            .map_err(|e| format!("reshape_nd: {}", e))?;

        // ── 2. Forward — retrieve both named output layers ────────────────────
        self.net.set_input(&blob, "", 1.0, core::Scalar::default())
            .map_err(|e| format!("set_input: {}", e))?;

        // Get all output layer names (Identity=boxes, Identity_1=scores in the right order)
        let out_names = self.net.get_unconnected_out_layers_names()
            .map_err(|e| format!("get_output_names: {}", e))?;
        let mut out_vecs: core::Vector<core::Vector<Mat>> = core::Vector::new();
        self.net.forward_and_retrieve(&mut out_vecs, &out_names)
            .map_err(|e| format!("forward: {}", e))?;

        if out_vecs.len() < 2 {
            return Err(format!("Expected 2 output layers, got {}", out_vecs.len()));
        }

        // out_vecs[0] → boxes  [1, 2016, 18]
        // out_vecs[1] → scores [1, 2016,  1]
        let boxes_list  = out_vecs.get(0).map_err(|e| e.to_string())?;
        let scores_list = out_vecs.get(1).map_err(|e| e.to_string())?;
        if boxes_list.is_empty() || scores_list.is_empty() {
            return Ok(vec![]);
        }
        let boxes_mat  = boxes_list.get(0).map_err(|e| e.to_string())?;
        let scores_mat = scores_list.get(0).map_err(|e| e.to_string())?;

        // ── 3. Decode candidates ──────────────────────────────────────────────
        // boxes_mat  is [1, 2016, 18]  — reshape to access [anchor][value]
        // scores_mat is [1, 2016, 1]
        // We flatten by accessing raw data via at_3d.

        let num_anchors = self.anchors.len() as i32;
        let mut candidates: Vec<(f32, [f32; 4])> = Vec::new(); // (score, [x1,y1,x2,y2] norm)

        for i in 0..num_anchors {
            let raw_score = *scores_mat.at_3d::<f32>(0, i, 0)
                .unwrap_or(&-999.0);
            let score = sigmoid(raw_score);
            if score < SCORE_THRESH { continue; }

            let [acx, acy] = self.anchors[i as usize];

            // Decode box (MediaPipe fixed_anchor_size encoding)
            let dx  = *boxes_mat.at_3d::<f32>(0, i, 0).unwrap_or(&0.0);
            let dy  = *boxes_mat.at_3d::<f32>(0, i, 1).unwrap_or(&0.0);
            let dw  = *boxes_mat.at_3d::<f32>(0, i, 2).unwrap_or(&0.0);
            let dh  = *boxes_mat.at_3d::<f32>(0, i, 3).unwrap_or(&0.0);

            let cx = acx + dx / MODEL_SIZE as f32;
            let cy = acy + dy / MODEL_SIZE as f32;
            let w  = dw  / MODEL_SIZE as f32;
            let h  = dh  / MODEL_SIZE as f32;

            let x1 = (cx - w / 2.0).clamp(0.0, 1.0);
            let y1 = (cy - h / 2.0).clamp(0.0, 1.0);
            let x2 = (cx + w / 2.0).clamp(0.0, 1.0);
            let y2 = (cy + h / 2.0).clamp(0.0, 1.0);

            if x2 > x1 && y2 > y1 {
                candidates.push((score, [x1, y1, x2, y2]));
            }
        }

        // ── 4. IoU-NMS ───────────────────────────────────────────────────────
        candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut kept: Vec<(f32, [f32; 4])> = Vec::new();
        'outer: for cand in &candidates {
            for k in &kept {
                if iou(&cand.1, &k.1) > NMS_THRESH { continue 'outer; }
            }
            kept.push(*cand);
        }

        // ── 5. Convert to pixel Rect ─────────────────────────────────────────
        let detections = kept.into_iter().map(|(score, bb)| {
            let x = (bb[0] * frame_w) as i32;
            let y = (bb[1] * frame_h) as i32;
            let w = ((bb[2] - bb[0]) * frame_w) as i32;
            let h = ((bb[3] - bb[1]) * frame_h) as i32;
            PalmDetection {
                bbox: Rect::new(
                    x.max(0),
                    y.max(0),
                    w.min(frame_w as i32 - x),
                    h.min(frame_h as i32 - y),
                ),
                confidence: score,
            }
        }).collect();

        Ok(detections)
    }
}
