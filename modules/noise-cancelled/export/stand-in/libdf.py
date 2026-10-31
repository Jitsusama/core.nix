"""A stand-in for DeepFilterNet's Rust bindings, enough to load its model.

The bindings pin pyo3 0.19, which can't build for this Python, and the
export needs only what building the model asks of them: its bands' widths
and the first value of its unit norm. Everything else they hold is the
signal processing torchDF does itself in PyTorch, so it's refused here.
"""

import numpy as np

# What DeepFilterNet 3 runs at: 48 kHz, a 960-point FFT and 32 bands.
RATE = 48000
FFT = 960
BANDS = 32
# Its bands' widths in FFT bins at those, as torchDF holds them.
WIDTHS = [2] * 13 + [5, 5, 7, 7, 8, 10, 12, 13, 15, 18, 20, 24, 28, 31, 37, 42, 50, 56, 67]
# The unit norm's first value, falling across the bins, as libDF's lib.rs.
UNIT_NORM_INIT = (0.001, 0.0001)


class UnknownSettingsError(ValueError):
    """Settings other than DeepFilterNet 3's, whose widths aren't known."""

    def __init__(self) -> None:
        """Say which settings are known."""
        super().__init__(f"only {RATE} Hz, {FFT} bins and {BANDS} bands are known")


class DF:
    """The settings DeepFilterNet's model reads from its Rust state."""

    def __init__(
        self,
        sr: int,
        fft_size: int,
        hop_size: int,
        nb_bands: int,
        min_nb_erb_freqs: int = 1,
    ) -> None:
        """Take only DeepFilterNet 3's own settings, the widths' only ones.

        The fewest bins a band may hold is already in the widths, which
        are torchDF's for these settings, so it's taken and not used.
        """
        del hop_size, min_nb_erb_freqs
        if (sr, fft_size, nb_bands) != (RATE, FFT, BANDS):
            raise UnknownSettingsError
        self.rate = sr

    def sr(self) -> int:
        """Give the sample rate."""
        return self.rate

    def erb_widths(self) -> np.ndarray:
        """Give each band's width in FFT bins."""
        return np.array(WIDTHS, dtype=np.uint64)


def unit_norm_init(bins: int) -> np.ndarray:
    """Give the unit norm's first value for each bin."""
    return np.linspace(*UNIT_NORM_INIT, bins, dtype=np.float32).reshape(1, bins)


def refused(*_args: object, **_kwargs: object) -> None:
    """Refuse the signal processing torchDF does itself."""
    why = "the export never runs DeepFilterNet's own signal processing"
    raise NotImplementedError(why)


erb = erb_inv = erb_norm = unit_norm = refused
