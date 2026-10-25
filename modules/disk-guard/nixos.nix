# The machine's disk guard: the daemon that owns what lies outside every
# home. It collects Nix's garbage by its own policy, oldest first and within
# a budget for what's young; expires generations, never one that runs, boots
# or has a boot entry; links the store's identical files weekly in a spare
# moment; and takes old cores, the build directories Nix left and root's
# caches. Each account's guard asks it to collect when they share a
# filesystem, and to vouch for processes the account can't read.
# AGENTS.md beside this says how it decides.
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
  podman = config.virtualisation.podman;
in
{
  imports = [ ./system.nix ];

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

    # The machine's chores yield to what Joel uses, as the account's guard
    # does in its own background.slice. In system.slice the guard had the
    # weight of every account together, and used 6,554 CPU-s in 17 h against
    # niri's 1,743, a tenth of a core. At the builds' weight, with every other
    # slice busy, it still keeps about 0.7 of a core of sixteen, so a disk
    # filling under a build is still caught; its passes wait only while what
    # Joel runs wants the CPU.
    systemd.slices.background.sliceConfig.CPUWeight = 10;

    systemd.services.disk-guard = {
      description = "Keeps the machine's disks from filling";
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        ExecStart = "${lib.getExe package} system --config ${configFile}";
        Slice = "background.slice";
        # The socket every account's guard asks, and the status file
        # `disk-guard status` reads, both open to every account. It's kept
        # while the guard is down, so its last status, heartbeat and all,
        # says the guard stopped rather than vanishing; the guard replaces
        # the socket left behind as it starts.
        RuntimeDirectory = "disk-guard";
        RuntimeDirectoryMode = "0755";
        RuntimeDirectoryPreserve = "yes";
        StateDirectory = "disk-guard";
        StateDirectoryMode = "0700";
        # The guard holds 35 MiB itself, but root's Nix lists and describes
        # the dead paths in this cgroup: on a workstation's store of 100,000
        # paths the listing held about 400 MiB, and under a 256 MiB high mark
        # it swapped and crawled for minutes where it takes one. The high
        # mark leaves it that room twice over, and still reclaims the page
        # cache the walks fill. A command that overruns the ceiling is killed
        # and journaled, never the guard.
        MemoryHigh = "1G";
        MemoryMax = "2G";
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
        # Not RestrictSUIDSGID: its filter can't read openat2's mode, so it
        # refuses every openat2 as unimplemented, and the guard opens
        # everything it removes with openat2 so nothing on the way can lead
        # it out of its root. It never makes a set-id file to restrict.
        RestrictAddressFamilies = [ "AF_UNIX" ];
        SystemCallArchitectures = "native";
      };
    };
  };
}
