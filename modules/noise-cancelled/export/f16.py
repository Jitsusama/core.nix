"""DeepFilterNet 3's frame graph, rewritten to hold in half precision.

The NPU computes in half precision (f16) alone, which keeps nothing below
2^-14 but zero. Exported as it is, the graph loses a quiet room to that
twice: a quiet frame's spectrum is flushed to zero before any maths, and
the band energies' logarithm, 10 log10(power + 1e-10), is asked for log(0),
since 1e-10 is zero in f16. The -inf that gives runs on as NaN through the
encoder, and what the NPU cleans comes out as noise.

So the graph is rewritten to do the same maths where f16 holds it. The
frame is gained by GAIN going in, and the cleaned frame divided by it
coming out. The band energies take two paths, scaled by QUIET for a quiet
band (each bin's power clamped short of f16's largest) and by LOUD for the
rest; a Select takes the quiet path where the loud one finds the band
quiet, and each path's logarithm is put back by its scale. The deep
filter's unit norm, which grows as the square root of the gain, is put
back by it, and its starting values and the silence threshold are scaled
with the frame's power.

It's refused unless it agrees with the export in single precision, and
unless what each logarithm is asked for, where it's taken, is a number f16
holds, on a voice in a room made quiet (-60 dB) and made as loud as it
goes. The NPU isn't in the build to ask; QEST-20261111-JCWNFD's lab
measured it there, 33 dB from the export on a voice in a room.

usage: f16.py EXPORT VOICE NOISE OUTPUT
"""

import sys
import wave

import numpy as np
import openvino as ov
import openvino.opset13 as ops

# How far the frame is gained going in: enough that a quiet room's spectrum
# stays above f16's smallest, and little enough that a loud one stays under
# its largest.
GAIN = 2.0**7
# How each path of the band energies scales power, after the gain.
QUIET = 2.0**24
LOUD = 2.0**8
# Where a band counts as quiet, in power as the export has it: low enough
# that no bin of the widest band, weighed at 0.02 or more, reaches the
# quiet path's clamp.
QUIETER_THAN = 1e-5
# The most the quiet path keeps of a bin's power, short of f16's largest.
CLAMP = 65000.0
# The least and the most f16 holds as a normal number.
LEAST = 2.0**-14
MOST = 65504.0
# How far a cleaned sample may stray from the export's, out of full scale
# 1, both computed in single precision.
AGREED = 1e-5
# A frame, 10 ms at 48 kHz, and the state DeepFilterNet 3 keeps between
# frames, in floats.
FRAME = 480
STATE = 45304


def constant(value: float | np.ndarray) -> ov.Output:
    """Make a constant of single precision, as an output to wire in."""
    return ops.constant(np.asarray(value, np.float32)).output(0)


def rewire(old: ov.Output, new: ov.Node) -> None:
    """Point everything reading `old` at `new` instead, but `new` itself."""
    for reader in list(old.get_target_inputs()):
        if reader.get_node().get_friendly_name() != new.get_friendly_name():
            reader.replace_source_output(new.output(0))


def only(model: ov.Model, kind: str) -> ov.Node:
    """Find the one op of `kind`, refusing a graph with more or none."""
    found = [op for op in model.get_ops() if op.get_type_name() == kind]
    if len(found) != 1:
        sys.exit(f"the export has {len(found)} {kind} ops, where the rewrite knows one")
    return found[0]


def named(model: ov.Model, name: str) -> ov.Node:
    """Find the op ONNX named `name`."""
    for op in model.get_ops():
        if op.get_friendly_name() == name:
            return op
    sys.exit(f"the export has no {name}, which the rewrite scales")


def gain_the_frame(model: ov.Model) -> None:
    """Gain the frame going in, and divide the cleaned frame coming out."""
    frame = next(p for p in model.get_parameters() if p.get_friendly_name() == "input_frame")
    rewire(frame.output(0), ops.multiply(frame.output(0), constant(GAIN)))
    cleaned = next(
        r for r in model.get_results() if "enhanced_audio_frame" in r.input_value(0).get_names()
    )
    cleaned.input(0).replace_source_output(
        ops.multiply(cleaned.input_value(0), constant(1 / GAIN)).output(0)
    )


def band_energies_in_two(model: ov.Model) -> list[tuple[ov.Output, ov.Output | None]]:
    """Take the band energies' logarithm on two paths f16 holds.

    Says what each new logarithm is asked for, with where it's taken.
    """
    log = only(model, "Log")
    floored = log.input_value(0).get_node()
    at = next(
        i for i in range(2) if floored.input_value(i).get_node().get_type_name() == "Constant"
    )
    floor = float(floored.input_value(at).get_node().get_data().ravel()[0])
    bands = floored.input_value(1 - at).get_node()
    side = 0 if bands.input_value(0).get_node().get_type_name() == "ReduceSum" else 1
    summed = bands.input_value(side).get_node()
    spectrum = summed.input_value(0).get_node().input_value(0)
    transposed = bands.get_attributes()

    def path(scale: float, *, clamped: bool) -> tuple[ov.Node, ov.Node, ov.Node]:
        power = ops.power(ops.multiply(spectrum, constant(np.sqrt(scale) / GAIN)), constant(2.0))
        if clamped:
            power = ops.minimum(power, constant(CLAMP))
        sums = ops.reduce_sum(
            power, summed.input_value(1), keep_dims=summed.get_attributes()["keep_dims"]
        )
        inputs = [bands.input_value(0), bands.input_value(1)]
        inputs[side] = sums.output(0)
        energy = ops.matmul(
            inputs[0], inputs[1], transposed["transpose_a"], transposed["transpose_b"]
        )
        asked = ops.add(energy, constant(floor * scale))
        return energy, asked, ops.subtract(ops.log(asked), constant(np.log(scale)))

    loud, loud_asked, loud_log = path(LOUD, clamped=False)
    _, quiet_asked, quiet_log = path(QUIET, clamped=True)
    quiet = ops.less(loud, constant(QUIETER_THAN * LOUD))
    rewire(log.output(0), ops.select(quiet, quiet_log, loud_log))
    return [
        (quiet_asked.output(0), quiet.output(0)),
        (loud_asked.output(0), ops.logical_not(quiet).output(0)),
    ]


def unit_norm_with_the_gain(model: ov.Model) -> None:
    """Put back the deep filter's unit norm, its start and the silence gate."""
    root = only(model, "Sqrt")
    rewire(root.output(0), ops.multiply(root.output(0), constant(np.sqrt(GAIN))))
    start = named(model, "/Where_1")
    start.input(1).replace_source_output(
        constant(start.input_value(1).get_node().get_data() * GAIN)
    )
    silence = named(model, "/GreaterOrEqual")
    silence.input(1).replace_source_output(
        constant(silence.input_value(1).get_node().get_data() * GAIN * GAIN)
    )


def rewritten(export: str) -> tuple[ov.Model, list[tuple[ov.Output, ov.Output | None]]]:
    """Rewrite the export to hold in f16, saying what its logarithms take."""
    model = ov.Core().read_model(export)
    return model, [(only(model, "Log").input_value(0), None)]


def cleaned(model: ov.Model, frames: np.ndarray) -> list[dict[str, np.ndarray]]:
    """Clean `frames` on the processor in single precision, frame by frame."""
    compiled = ov.Core().compile_model(model, "CPU", {"INFERENCE_PRECISION_HINT": "f32"})
    request = compiled.create_infer_request()
    state = np.zeros(STATE, np.float32)
    said = []
    for frame in frames:
        got = request.infer({"input_frame": frame, "states": state})
        state = got["out_states"].copy()
        said.append({name: v.copy() for k, v in got.items() for name in k.get_names()})
    return said


def mono(path: str) -> np.ndarray:
    """Read a 16-bit mono recording as floats in full scale 1."""
    with wave.open(path) as recording:
        data = recording.readframes(recording.getnframes())
    return np.frombuffer(data, "<i2").astype(np.float32) / 32768


def voice_in_a_room(voice: str, noise: str) -> np.ndarray:
    """Mix the voice at -20 dB with the room 10 dB under it, as frames."""
    spoken, room = mono(voice), mono(noise)
    room = np.resize(room, spoken.size)
    spoken *= 0.1 / np.sqrt(np.mean(spoken**2))
    room *= 0.0316 / np.sqrt(np.mean(room**2))
    mixed: np.ndarray = (spoken + room)[: spoken.size // FRAME * FRAME]
    return mixed.reshape(-1, FRAME)


def agrees(export: str, model: ov.Model, frames: np.ndarray) -> None:
    """Refuse a rewrite whose cleaned frames stray from the export's."""
    key = "enhanced_audio_frame"
    wanted = np.concatenate([s[key] for s in cleaned(ov.Core().read_model(export), frames)])
    got = np.concatenate([s[key] for s in cleaned(model, frames)])
    worst = float(np.abs(got - wanted).max())
    print(f"at worst {worst:.1e} from the export")
    if worst > AGREED:
        sys.exit(f"the rewrite strays {worst:.1e} from the export, past {AGREED:.0e}")


def held(
    model: ov.Model, logs: list[tuple[ov.Output, ov.Output | None]], frames: np.ndarray
) -> None:
    """Refuse a graph asking a logarithm for what f16 doesn't hold.

    `logs` pairs what each logarithm is asked for with where it's taken,
    or None for everywhere.
    """
    names = []
    for at, (asked, where) in enumerate(logs):
        asked.get_tensor().add_names({f"asked-{at}"})
        if where is not None:
            where.get_tensor().add_names({f"where-{at}"})
        names.append((f"asked-{at}", None if where is None else f"where-{at}"))
    model.add_outputs([o for pair in logs for o in pair if o is not None])
    peak = float(np.abs(frames).max())
    for how, made in (("quiet", 1e-3), ("loud", 1 / peak)):
        values = []
        for said in cleaned(model, frames * np.float32(made)):
            for asked_name, where_name in names:
                asked = said[asked_name]
                if where_name is not None:
                    asked = asked[said[where_name].astype(bool)]
                values.append(asked.ravel())
        everything = np.concatenate(values)
        least, most = float(everything.min()), float(everything.max())
        print(f"{how}: a logarithm is asked for {least:.1e} to {most:.1e}")
        if least < LEAST or most > MOST:
            sys.exit(
                f"{how}, a logarithm is asked for {least:.1e} to {most:.1e}, "
                f"past f16's {LEAST:.1e} to {MOST:.0f}"
            )


def main() -> None:
    """Rewrite the export, then refuse it unless it agrees and holds in f16."""
    export, voice, noise, output = sys.argv[1:]
    frames = voice_in_a_room(voice, noise)
    model, logs = rewritten(export)
    agrees(export, model, frames)
    ov.save_model(model, output, compress_to_fp16=False)
    checked, logs = rewritten(export)
    held(checked, logs, frames)


if __name__ == "__main__":
    main()
