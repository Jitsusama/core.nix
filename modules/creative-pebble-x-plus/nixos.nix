# Creative's Pebble X Plus speakers, tuned to a published measurement, which
# tuning.conf describes. The tuning runs as a PipeWire client of its own in
# every account: it starts when PipeWire does, after WirePlumber, which links
# it to the speakers whenever they're plugged in, and stops and restarts with
# it.
{ config, ... }:
{
  imports = [ ../pipewire/nixos.nix ];

  systemd.user.services.pebble-x-plus-tuning = {
    description = "The Pebble X Plus speakers' tuning";
    after = [
      "pipewire.service"
      "wireplumber.service"
    ];
    requires = [ "pipewire.service" ];
    wants = [ "wireplumber.service" ];
    partOf = [ "pipewire.service" ];
    wantedBy = [ "pipewire.service" ];
    serviceConfig = {
      ExecStart = "${config.services.pipewire.package}/bin/pipewire -c ${./tuning.conf}";
      Restart = "on-failure";
      RestartSec = 2;
    };
  };
}
