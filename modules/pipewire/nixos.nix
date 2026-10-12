# PipeWire plays and records every sound, for ALSA and PulseAudio programs
# alike, with WirePlumber choosing where each stream goes. rtkit lets it run
# at realtime priority, so audio doesn't crackle under a build.
let
  # Music at 44.1 kHz is resampled for outputs running at 48 kHz, which
  # Bluetooth headphones always do. PipeWire's best resampler costs a few
  # percent of one core while it resamples, and nothing otherwise.
  resampling."stream.properties"."resample.quality" = 10;
in
{
  services.pipewire = {
    enable = true;
    alsa.enable = true;
    pulse.enable = true;
    extraConfig.client."10-resampling" = resampling;
    extraConfig.pipewire-pulse."10-resampling" = resampling;
  };
  security.rtkit.enable = true;
}
