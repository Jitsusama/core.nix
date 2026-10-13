# The machine's disk guard: the daemon that owns what lies outside every
# home. It collects Nix's garbage by its own policy, oldest first and within
# a budget for what's young; expires generations, never one that runs, boots
# or has a boot entry; links the store's identical files weekly in a spare
# moment; and takes old cores, the build directories Nix left and root's
# caches. Each account's guard asks it to collect when they share a
# filesystem, and to vouch for processes the account can't read.
# AGENTS.md beside this says how it decides.
#
# jitsusama.disk-guard.settings is the guard's own config, rendered as it is,
# and the build refuses one check-config refuses. Each source core gives is a
# default, replaced whole by a machine's own, or dropped by setting it null.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  package = pkgs.callPackage ./package.nix { };
  json = pkgs.formats.json { };
  settings = config.jitsusama.disk-guard.settings;
  podman = config.virtualisation.podman;

  rendered = settings // {
    version = 1;
    scope = "system";
    sources = lib.filterAttrs (_: source: source != null) settings.sources;
  };

  # The config as the guard reads it, refused at build by the guard itself.
  configFile =
    pkgs.runCommandLocal "disk-guard-system.json"
      {
        text = builtins.toJSON rendered;
        passAsFile = [ "text" ];
      }
      ''
        ${lib.getExe package} check-config "$textPath"
        cp "$textPath" $out
      '';
in
{
  options.jitsusama.disk-guard.settings = lib.mkOption {
    type = lib.types.submodule { freeformType = json.type; };
    default = { };
    description = ''
      The machine's guard's config: its bands, its sources, and its policy
      for Nix's garbage, generations and linking. A source set to null is
      dropped.
    '';
  };

  config = {
    jitsusama.disk-guard.settings = {
      status_file = lib.mkDefault true;
      boot = lib.mkDefault config.boot.loader.efi.efiSysMountPoint;
      nix.bin = lib.mkDefault (lib.getExe' config.nix.package "nix");
      dedupe.systemd_run = lib.mkDefault (lib.getExe' config.systemd.package "systemd-run");
      sources = {
        coredumps = lib.mkDefault {
          kind = "coredumps";
          path = "/var/lib/systemd/coredump";
        };
        nix-builds = lib.mkDefault {
          kind = "nix-builds";
          path = config.nix.settings.build-dir or "/nix/var/nix/builds";
        };
        nix = lib.mkDefault {
          kind = "cache";
          path = "/root/.cache/nix";
          rank = 20;
        };
        tmp = lib.mkDefault {
          kind = "temp";
          path = "/tmp";
        };
        var-tmp = lib.mkDefault {
          kind = "temp";
          path = "/var/tmp";
        };
        podman = lib.mkIf podman.enable (
          lib.mkDefault {
            kind = "podman";
            command = lib.getExe podman.package;
            path = "/var/lib/containers/storage";
          }
        );
      };
    };

    environment.systemPackages = [ package ];
    environment.etc."disk-guard/config.json".source = configFile;

    systemd.services.disk-guard = {
      description = "Keeps the machine's disks from filling";
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        ExecStart = "${lib.getExe package} system --config ${configFile}";
        # The socket every account's guard asks, and the status file
        # `disk-guard status` reads, both open to every account.
        RuntimeDirectory = "disk-guard";
        RuntimeDirectoryMode = "0755";
        StateDirectory = "disk-guard";
        StateDirectoryMode = "0700";
        # The guard holds little itself and stays under the high mark, which
        # reclaims the page cache its walks fill. Root's Nix lists the dead
        # paths in this cgroup, so the ceiling leaves it room. A command
        # that still overruns it is killed and journaled, never the guard.
        MemoryHigh = "256M";
        MemoryMax = "1G";
        OOMPolicy = "continue";
        Restart = "on-failure";
        RestartSec = "30s";

        # Nothing that changes what the guard sees: it judges processes in
        # every home, takes leftovers in the real /tmp and runs root's Nix,
        # which remounts the store to write it. What it never needs goes.
        ProtectSystem = "full";
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectKernelLogs = true;
        ProtectClock = true;
        ProtectHostname = true;
        LockPersonality = true;
        NoNewPrivileges = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        RestrictAddressFamilies = [ "AF_UNIX" ];
        SystemCallArchitectures = "native";
      };
    };
  };
}
