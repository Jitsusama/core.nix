"""The resampler DeepFilterNet imports, which the export never calls."""


def resample(*_args: object, **_kwargs: object) -> None:
    """Refuse, since nothing is resampled."""
    why = "the export never resamples"
    raise NotImplementedError(why)
