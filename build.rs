use std::env;
use std::path::PathBuf;
use burn_import::onnx::ModelGen;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Path to your ONNX file
    let onnx = "src/games/resources/model_11_joker_op16.onnx";

    ModelGen::new()
        .input(onnx)
        .out_dir(&out_dir.to_str().unwrap())
        .run_from_script();

    println!("cargo:rerun-if-changed={onnx}");
}
