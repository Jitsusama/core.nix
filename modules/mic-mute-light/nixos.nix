# A laptop's mic-mute light, lit while the default microphone is muted.
# The kernel's audio-micmute trigger follows only a sound card's capture
# switch, and microphones reached through a DSP have none, so on such a
# laptop the light would never light. mic-mute-light.rs follows PipeWire's
# mute instead, run as the person at the seat, and sets the light through
# logind, which lets that person set a light without root. It starts only
# where the light is.
{ pkgs, ... }:
let
  # The standard library alone, so it builds with rustc and no crates.
  micMuteLight = pkgs.runCommandCC "mic-mute-light" { nativeBuildInputs = [ pkgs.rustc ]; } ''
    mkdir -p $out/bin
    rustc --edition 2024 -C opt-level=3 -D warnings \
      -o $out/bin/mic-mute-light ${./mic-mute-light.rs}
  '';
in
{
  systemd.user.services.mic-mute-light = {
    description = "Lights the mic-mute light while the microphone is muted";
    wantedBy = [ "pipewire.service" ];
    after = [ "wireplumber.service" ];
    bindsTo = [ "pipewire.service" ];
    unitConfig.ConditionPathExists = "/sys/class/leds/platform::micmute";
    path = [
      pkgs.brightnessctl
      pkgs.pipewire
      pkgs.wireplumber
    ];
    serviceConfig = {
      ExecStart = "${micMuteLight}/bin/mic-mute-light";
      Restart = "on-failure";
      RestartSec = 2;
      NoNewPrivileges = true;
      PrivateNetwork = true;
      RestrictAddressFamilies = "AF_UNIX";
      ProtectSystem = "strict";
      ProtectHome = "read-only";
      PrivateTmp = true;
      CapabilityBoundingSet = "";
      RestrictNamespaces = true;
      RestrictRealtime = true;
      LockPersonality = true;
      MemoryDenyWriteExecute = true;
      SystemCallArchitectures = "native";
      # PipeWire's clients ask for real-time scheduling as they start, which
      # a filter on @resources would kill them for; RestrictRealtime makes
      # the ask fail instead, as they expect it may.
      SystemCallFilter = [
        "@system-service"
        "~@privileged"
      ];
    };
  };
}
