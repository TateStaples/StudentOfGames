import onnx
from onnx import version_converter, shape_inference
import os

# Input path
in_path = 'src/games/resources/model_11_joker.onnx'

# Load the ONNX model
model = onnx.load(in_path)

# Convert the model to opset version 16
upgraded_model = version_converter.convert_version(model, 16)

# Apply shape inference to the upgraded model
inferred_model = shape_inference.infer_shapes(upgraded_model)

# Build output path by appending "_op16" before ".onnx"
base, ext = os.path.splitext(in_path)
out_path = base + "_op16" + ext

# Save the converted model
onnx.save(inferred_model, out_path)

print(f"Saved upgraded model to {out_path}")
