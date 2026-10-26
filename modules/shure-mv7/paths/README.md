# The MV7's Mixer Paths

ALSA mixer paths for the Shure MV7, one per direction, which the module's
profile set gives PipeWire so it drives the microphone's own controls rather
than muting and turning it down in software:

- `shure-mv7-microphone.conf`: the control the firmware calls Microphone, its
  switch the mute the touch pad flips and its volume the gain.
- `shure-mv7-headphones.conf`: the control it calls USB Streaming, the
  headphone volume the touch strip slides.
