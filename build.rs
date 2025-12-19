use std::env;
use std::path::PathBuf;
use burn_import::onnx::ModelGen;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Path to your ONNX file (5 dice per player, joker rules)
    let onnx = "src/games/resources/model_55_joker_op16.onnx";

    ModelGen::new()
        .input(onnx)
        .out_dir(&out_dir.to_str().unwrap())
        .run_from_script();

    println!("cargo:rerun-if-changed={onnx}");
}
