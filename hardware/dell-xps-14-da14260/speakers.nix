# The XPS 14's speaker tuning, which speakers.conf describes, run as a
# PipeWire client of its own in every account. It follows the sound server:
# it starts when PipeWire does, after WirePlumber, which links it, and stops
# and restarts with it.
{ config, pkgs, ... }:
{
  imports = [ ../../modules/pipewire/nixos.nix ];

  systemd.user.services.speaker-tuning = {
    description = "The XPS 14's speaker tuning";
    after = [
      "pipewire.service"
      "wireplumber.service"
    ];
    requires = [ "pipewire.service" ];
    wants = [ "wireplumber.service" ];
    partOf = [ "pipewire.service" ];
    wantedBy = [ "pipewire.service" ];
    # The limiter is an LV2 plugin, which PipeWire looks for here.
    environment.LV2_PATH = "${pkgs.lsp-plugins}/lib/lv2";
    serviceConfig = {
      ExecStart = "${config.services.pipewire.package}/bin/pipewire -c ${./speakers.conf}";
      Restart = "on-failure";
      RestartSec = 2;
    };
  };
}
