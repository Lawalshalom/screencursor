use opencv::{dnn, core};
use opencv::prelude::*;
fn main() {
    let mut net = dnn::read_net_from_onnx("models/palm_detection_mediapipe_2023feb.onnx").unwrap();
    let blob = dnn::blob_from_image_def(&core::Mat::default()).unwrap();
    net.set_input(&blob, "", 1.0, core::Scalar::default()).unwrap();
    let res = net.forward_def().unwrap();
    println!("Rows: {}, Cols: {}", res.rows(), res.cols());
}
