# The Mac's disk guard: the daemon that owns what lies outside every home,
# as nixos.nix's does on Linux. It collects Nix's garbage by its own policy
# beside whatever collects it already, expires generations, links the
# store's identical files in a spare moment, takes root's caches and what's
# left in the temporary directories, and from urgent on has Time Machine
# thin its local snapshots. Each account's guard asks it to collect when
# they share a filesystem, and to vouch for processes the account can't
# read. AGENTS.md beside this says how it decides.
#
# jitsusama.disk-guard.settings is the guard's own config, declared and
# rendered in system.nix. Each source core gives is a default, replaced whole
# by a machine's own, or dropped by setting it null.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  package = pkgs.callPackage ./package.nix { };
  inherit (config.jitsusama.disk-guard) configFile;

  # The machine's own Nix: nix-darwin's when it manages Nix, and otherwise
  # the installer's, as Determinate's is, never another beside it.
  nix =
    if config.nix.enable then
      lib.getExe' config.nix.package "nix"
    else
      "/nix/var/nix/profiles/default/bin/nix";
in
{
  imports = [ ./system.nix ];

  config = {
    jitsusama.disk-guard.settings = {
      status_file = lib.mkDefault true;
      # A Mac boots no generation from a partition of entries, so none is
      # kept for one.
      boot = lib.mkDefault "/var/empty";
      nix.bin = lib.mkDefault nix;
      # A Mac has no scopes to confine a run to, and no cgroups to limit.
      dedupe.systemd_run = lib.mkDefault null;
      sources = {
        nix = lib.mkDefault {
          kind = "cache";
          path = "/var/root/.cache/nix";
          rank = 20;
        };
        # /tmp and /var/tmp are links into /private.
        tmp = lib.mkDefault {
          kind = "temp";
          path = "/private/tmp";
        };
        var-tmp = lib.mkDefault {
          kind = "temp";
          path = "/private/var/tmp";
        };
        # Local snapshots hold what's been deleted since the last backup;
        # thinning them frees it, most urgently first, and loses only
        # restore points the backup disk has anyway.
        time-machine = lib.mkDefault {
          kind = "producer";
          command = [
            "/usr/bin/tmutil"
            "thinlocalsnapshots"
            "/"
            "21474836480"
            "4"
          ];
          path = "/";
          from = "urgent";
          timeout = "10m";
        };
      };
    };

    environment.systemPackages = [ package ];
    environment.etc."disk-guard/config.json".source = configFile;

    launchd.daemons.disk-guard.serviceConfig = {
      Label = "org.jitsusama.disk-guard";
      ProgramArguments = [
        (lib.getExe package)
        "system"
        "--config"
        "${configFile}"
      ];
      RunAtLoad = true;
      # Started again whenever it stops, as systemd's on-failure does, but
      # not more than every 30 s.
      KeepAlive = true;
      ThrottleInterval = 30;
      StandardOutPath = "/var/log/disk-guard.log";
      StandardErrorPath = "/var/log/disk-guard.log";
    };
  };
}
