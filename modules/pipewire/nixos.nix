# PipeWire plays and records every sound, for ALSA and PulseAudio programs
# alike, with WirePlumber choosing where each stream goes. rtkit lets it run
# at realtime priority, so audio doesn't crackle under a build.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  # Music at 44.1 kHz is resampled to 48 kHz for every output, since
  # PipeWire's clock runs at 48 kHz alone unless it's allowed others. Its
  # best resampler costs a few percent of one core while it resamples, and
  # nothing otherwise.
  resampling."stream.properties"."resample.quality" = 10;
  cat = "${pkgs.coreutils}/bin/cat";
  # The account's sound services: PipeWire's own three, and each service
  # that runs a PipeWire of its own, as a tuning chain does, so a chain a
  # module adds is granted without a list here to keep.
  pipewire = lib.getExe' config.services.pipewire.package "pipewire";
  chains = lib.attrNames (
    lib.filterAttrs (
      _: service: lib.hasPrefix "${pipewire} " (toString (service.serviceConfig.ExecStart or ""))
    ) config.systemd.user.services
  );
  services = map lib.escapeRegex (
    [
      "pipewire"
      "pipewire-pulse"
      "wireplumber"
    ]
    ++ chains
  );
in
{
  services.pipewire = {
    enable = true;
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
  # rtkit grants only an active session, and PipeWire asks before the
  # seat has one, so the account's sound services are granted by place
  # (realtime.rules says why), reading it with cat.
  security.polkit.extraConfig =
    builtins.replaceStrings [ "@cat@" "@services@" ] [ cat (lib.concatStringsSep "|" services) ]
      (builtins.readFile ./realtime.rules);
}
