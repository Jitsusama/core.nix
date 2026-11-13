# The machine's disk guard: the daemon that owns what lies outside every
# home. It collects Nix's garbage once something lets it go, oldest first;
# expires generations, never one that runs, boots
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

  # Nix 2.34's collector takes other programs' temporary roots on a
  # socket, a thread a client, and a client that left before its thread
  # was registered left a stale entry: the next given the same descriptor
  # ended the collector with std::terminate. On 11 November that aborted
  # the guard's deletes at 06:16 and 11:54, each freeing nothing. So the
  # guard's own Nix carries upstream's two fixes (NixOS/nix#16569 and
  # #15963), and the machine's Nix stays as it is, so nothing else is
  # built again. Another version is left as it is, as it may hold them.
  nix =
    let
      machine = config.nix.package;
    in
    if machine ? appendPatches && lib.versions.majorMinor machine.version == "2.34" then
      machine.appendPatches [
        ./nix-gc-roots-client-leaves.patch
        ./nix-gc-roots-client-interrupted.patch
      ]
    else
      machine;
in
{
  imports = [ ./system.nix ];

  config = {
    jitsusama.disk-guard.settings = {
      status_file = lib.mkDefault true;
      boot = lib.mkDefault config.boot.loader.efi.efiSysMountPoint;
      nix.bin = lib.mkDefault (lib.getExe' nix "nix");
      dedupe.systemd_run = lib.mkDefault (lib.getExe' config.systemd.package "systemd-run");
      # A day of garbage stays while the disk is healthy, within the guard's
      # budget, so a switch the next morning finds what the last one fetched:
      # on 7 November the reader's 329 crate tarballs, collected between two
      # switches, cost the second most of 13 minutes to fetch again.
      garbage.keep = lib.mkDefault "1d";
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
        # A stop goes to the guard alone, which ends its commands with SIGKILL
        # (src/commands.rs); sent to the whole unit, it made Nix abort in a
        # delete and dump core. What's left once the guard exits is killed.
        KillMode = "mixed";
        # A stop the guard can't take mid-walk is killed after 5 s, not the
        # default 90, so a switch restarting it holds nothing else for long:
        # the account's guard held the shell's start for 91 s on 10
        # November. What it learns is saved as it goes.
        TimeoutStopSec = "5s";
        # The socket every account's guard asks, and the status file
        # `disk-guard status` reads, both open to every account. It's kept
        # while the guard is down, so its last status, heartbeat and all,
        # says the guard stopped rather than vanishing; the guard replaces
        # the socket left behind as it starts.
        RuntimeDirectory = "disk-guard";
        RuntimeDirectoryMode = "0755";
        RuntimeDirectoryPreserve = "yes";
        # Any account may pass through to the journal, which it reads beside
        # its own guard's, but not list the folder: the guard saves every
        # other file there as root's alone.
        StateDirectory = "disk-guard";
        StateDirectoryMode = "0711";
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
