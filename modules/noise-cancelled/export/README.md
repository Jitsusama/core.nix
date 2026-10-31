# The Export

What `model.nix` runs as it builds, to turn DeepFilterNet 3's published
checkpoint into the one-frame-at-a-time model the noise-cancelled program
runs. `export.py` traces grazder's torchDF streaming module to ONNX and
checks what OpenVINO makes of it against PyTorch before keeping it.

- `stand-in/`: small Python stand-ins for the modules DeepFilterNet imports
  but the export never calls, so the build needs neither its Rust bindings
  nor torchaudio.
