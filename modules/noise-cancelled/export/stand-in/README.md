# The Stand-Ins

Put first on the export's Python path, so DeepFilterNet imports these
rather than the real modules. Each holds only what loading the model
asks of it, and refuses anything else, so a call the export never made
before fails loudly rather than quietly doing nothing.

- `libdf.py`: DeepFilterNet's Rust bindings, its settings and its one
  model-building call.
- `torchaudio/`: torchaudio, which DeepFilterNet imports to read and write
  audio files the export never touches.
