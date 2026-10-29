# The Surface Laptop 4's Mixer Paths

ALSA mixer paths that the laptop's profile set gives PipeWire in place of
its own:

- `analog-input-internal-mic.conf`: the built-in microphones, with the
  codec's boost held at 0 dB and the capture gain stopped at its 0 dB step,
  so the desktop's full volume no longer clips them.
