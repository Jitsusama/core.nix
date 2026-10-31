# update-to-main on NixOS: root's service, on a socket only root and the
# update-ask group write, that updates the machine to its repository's main
# when asked (system.nix says what it's told). A machine imports this and
# says its repository, its system and whose signatures count.
#
# The service starts from the running system's path with the config at a
# fixed one, and a switch never restarts it: it's what's switching, and it
# starts itself again once the switch is done.
{ config, lib, ... }:
let
  cfg = config.jitsusama.update-to-main;
in
{
  imports = [ ./system.nix ];

  config = {
    jitsusama.update-to-main.system = {
      kind = "nixos";
      state = "/var/lib/update-to-main";
      socket = "/run/update-to-main/ask.sock";
      profile = "/nix/var/nix/profiles/system";
      booted = "/run/booted-system";
      power_supplies = "/sys/class/power_supply";
      tools = {
        nix = lib.getExe' config.nix.package "nix";
        nix_env = lib.getExe' config.nix.package "nix-env";
        pmset = null;
      };
    };

    users.groups.update-ask.members = cfg.askers;

    systemd.services.update-to-main = {
      description = "Updates the machine to its repository's main when asked";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      restartIfChanged = false;
      stopIfChanged = false;
      serviceConfig = {
        ExecStart = "/run/current-system/sw/bin/update-to-main serve --config /etc/update-to-main/config.json";
        Restart = "always";
        RestartSec = 5;
        StateDirectory = "update-to-main";
        StateDirectoryMode = "0700";
        RuntimeDirectory = "update-to-main";
        RuntimeDirectoryMode = "0755";
        # A switch it runs may restart much of the machine, but never what
        # it started: the switch is its child.
        KillMode = "process";
      };
    };
  };
}
