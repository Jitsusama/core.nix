# PipeWire plays and records every sound, for ALSA and PulseAudio programs
# alike, with WirePlumber choosing where each stream goes. rtkit lets it run
# at realtime priority, so audio doesn't crackle under a build.
#
# A mute or volume changed now and then went unsaid: the shell kept showing
# the old one until something else changed, about one change in sixteen
# on the desktop test's microphone. PipeWire's converter used up a pending
# change whenever it was asked for its params, so the next change undid
# the flag that says one happened. The patch
# (replay-only-to-info-listeners.patch) replays a node's state only to a
# listener that takes it, as the adapter already does.
{ pkgs, ... }:
let
  # Music at 44.1 kHz is resampled for outputs running at 48 kHz, which
  # Bluetooth headphones always do. PipeWire's best resampler costs a few
  # percent of one core while it resamples, and nothing otherwise.
  resampling."stream.properties"."resample.quality" = 10;
in
{
  services.pipewire = {
    enable = true;
    package = pkgs.pipewire.overrideAttrs (old: {
      patches = (old.patches or [ ]) ++ [ ./replay-only-to-info-listeners.patch ];
    });
    alsa.enable = true;
    pulse.enable = true;
    extraConfig.client."10-resampling" = resampling;
    extraConfig.pipewire-pulse."10-resampling" = resampling;

    # Until somebody picks an input, WirePlumber takes the one it ranks
    # highest, and it ranks a webcam's microphone above a USB microphone,
    # which somebody plugged in to talk into. So a webcam's yields to one,
    # though it still comes before the laptop's own, which a shut lid
    # covers at a desk.
    wireplumber.extraConfig."50-webcam-microphones-yield"."monitor.alsa.rules" = [
      {
        matches = [
          {
            "media.class" = "Audio/Source";
            "device.form-factor" = "webcam";
          }
        ];
        actions.update-props."priority.session" = 1900;
      }
    ];
  };
  security.rtkit.enable = true;
}
