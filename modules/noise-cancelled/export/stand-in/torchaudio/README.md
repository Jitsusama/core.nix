# torchaudio's Stand-In

The names DeepFilterNet imports from torchaudio, laid out as torchaudio
lays them out so the same import lines find them.

- `functional.py`: the resampler, which refuses, since nothing is
  resampled.
- `backend/`: torchaudio's backends, of which DeepFilterNet names one
  audio file description in its types alone.
