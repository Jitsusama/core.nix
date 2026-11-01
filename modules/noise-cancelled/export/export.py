"""DeepFilterNet 3 exported to ONNX one 10 ms frame at a time.

grazder's torchDF holds the whole denoiser as one streaming PyTorch module:
a frame of 480 samples at 48 kHz and every state it keeps, flattened into
one vector, go in; the cleaned frame and the new state come out. Traced
rather than scripted, so no branch on a value is left in the graph for a
device to refuse, with the attenuation limit at zero, which is none, as the
microphone wants. The export is then run by OpenVINO on the processor
against a fresh model, since exporting changes the module it traces, and
refused unless every frame agrees.

usage: export.py CHECKPOINT OUTPUT
"""

import sys

import numpy as np
import openvino as ov
import torch
from torch_df_streaming import TorchDFPipeline

# A frame, 10 ms at 48 kHz.
FRAME = 480
# How far a cleaned sample may stray from PyTorch's, out of full scale 1;
# the processor computes in single precision as PyTorch does.
AGREED = 1e-4


def pipeline(checkpoint: str) -> TorchDFPipeline:
    """Load DeepFilterNet 3's streaming pipeline from its checkpoint."""
    return TorchDFPipeline(device="cpu", model_base_dir=checkpoint)


def speech_in_noise() -> np.ndarray:
    """Make three seconds of a voiced tone, broken as speech is, in hiss."""
    rate = 48000
    at = np.arange(rate * 3) / rate
    voiced = 0.3 * np.sin(2 * np.pi * 220 * at) * (np.sin(2 * np.pi * 2 * at) > 0)
    hiss = 0.05 * np.random.default_rng(7).standard_normal(at.size)
    return np.asarray(voiced + hiss, dtype=np.float32).reshape(-1, FRAME)


def main() -> None:
    """Export the model, then refuse it unless it agrees with PyTorch."""
    checkpoint, output = sys.argv[1:]
    traced = pipeline(checkpoint)
    model = traced.torch_streaming_model
    inputs = (torch.zeros(FRAME), traced.states.clone(), traced.atten_lim_db)
    torch.onnx.export(
        model,
        inputs,
        output,
        dynamo=False,
        input_names=["input_frame", "states", "atten_lim_db"],
        output_names=["enhanced_audio_frame", "out_states", "lsnr"],
        opset_version=14,
    )

    fresh = pipeline(checkpoint)
    request = ov.Core().compile_model(output, "CPU").create_infer_request()
    wanted_state = fresh.states.clone()
    state = np.zeros(wanted_state.numel(), dtype=np.float32)
    worst = 0.0
    for frame in speech_in_noise():
        with torch.no_grad():
            wanted, wanted_state, _ = fresh.torch_streaming_model(
                torch.from_numpy(frame), wanted_state, fresh.atten_lim_db
            )
        got = request.infer({"input_frame": frame, "states": state})
        state = got["out_states"]
        worst = max(worst, float(np.abs(got["enhanced_audio_frame"] - wanted.numpy()).max()))
    print(f"{state.size} floats of state; at worst {worst:.1e} from PyTorch")
    if worst > AGREED:
        sys.exit(f"the export strays {worst:.1e} from PyTorch, past {AGREED:.0e}")


if __name__ == "__main__":
    main()
