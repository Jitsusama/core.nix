# update-to-main on a Mac: root's launchd daemon, on a socket only root and
# the update-ask group write, that updates the Mac to its repository's main
# when asked (system.nix says what it's told). A machine imports this and
# says its repository, its system and whose signatures count.
#
# The daemon's definition names only the running system's path and a fixed
# config path, so it's the same in every system and activate never reloads
# it mid-switch; it starts itself again once a switch is done.
#
# launchd loads it before that path names a system holding it: at boot, and
# in the switch that first brings it. launchd never tries a missing program
# again, so the daemon waits for its program rather than be one.
{ config, lib, ... }:
let
  cfg = config.jitsusama.update-to-main;
  program = "/run/current-system/sw/bin/update-to-main";
  # The machine's own Nix: nix-darwin's when it manages Nix, and otherwise
  # the installer's, as Determinate's is, never another beside it.
  nixBin =
    if config.nix.enable then
      "${lib.getBin config.nix.package}/bin"
    else
      "/nix/var/nix/profiles/default/bin";
in
{
  imports = [ ./system.nix ];

  config = {
    jitsusama.update-to-main.system = {
      kind = "darwin";
      state = "/var/root/.local/state/update-to-main";
      socket = "/var/run/update-to-main.sock";
      ended = "/var/run/update-to-main.last.json";
      profile = "/nix/var/nix/profiles/system";
      # A Mac boots no NixOS system, and says its power through pmset.
      booted = "/var/empty";
      power_supplies = "/var/empty";
      tools = {
        nix = "${nixBin}/nix";
        nix_env = "${nixBin}/nix-env";
        pmset = "/usr/bin/pmset";
      };
    };

    # A group nix-darwin keeps, its id one macOS leaves free.
    users.knownGroups = [ "update-ask" ];
    users.groups.update-ask = {
      gid = 452;
      members = cfg.askers;
      description = "Accounts that may ask update-to-main to update the Mac";
    };

    launchd.daemons.update-to-main.serviceConfig = {
      Label = "org.jitsusama.update-to-main";
      ProgramArguments = [
        "/bin/sh"
        "-c"
        "until [ -x ${program} ]; do sleep 1; done; exec ${program} serve --config /etc/update-to-main/config.json"
      ];
      RunAtLoad = true;
      KeepAlive = true;
      ThrottleInterval = 10;
      # A switch it runs is its child, and launchd leaves it be if the
      # daemon itself stops.
      AbandonProcessGroup = true;
      StandardOutPath = "/var/log/update-to-main.log";
      StandardErrorPath = "/var/log/update-to-main.log";
    };
  };
}
