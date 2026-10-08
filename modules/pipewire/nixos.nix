# PipeWire plays and records every sound, for ALSA and PulseAudio programs
# alike, with WirePlumber choosing where each stream goes. rtkit lets it run
# at realtime priority, so audio doesn't crackle under a build.
{
  services.pipewire = {
    enable = true;
    alsa.enable = true;
    pulse.enable = true;
  };
  security.rtkit.enable = true;
}
