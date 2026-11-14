# The Nix daemon, which runs every build on the machine. Builds yield the CPU
# to whatever Joel is doing, so a build on every core never slows the desktop:
# each build runs at SCHED_IDLE, which the scheduler takes off a core the
# moment any normal task there wakes, whatever cgroups either is in, and a
# core running only idle work counts as free when one looks for a core.
# Batch, which they ran at before, only never preempts as it wakes; a batch
# build kept its turn, and niri waited for it. With a sixteen-thread Nix build
# at batch beside the agents' work, niri waited for a core over 3 ms 47 and
# 76 times in 30 s and missed 4 and 7 of 3,600 refreshes; at idle it waited
# 10 and 11 times and missed 4 and 1. The build did 32% less work, giving way
# to the agents' cargo builds as well as to Joel. An idle slice did as well
# for niri but starved VM checks, whose threads vm-first puts back to the
# normal policy below; at SCHED_IDLE they keep build.slice's weight.
#
# A policy only orders processes when they meet on a core, though, and the
# daemon's group, system.slice, weighs the same as Joel's user.slice, so a
# build took half the CPU from his programs whatever its policy. In a slice of its own at
# a tenth of their weight it takes what they leave. On the XPS 14, with a
# kernel build in an app's scope, a sixteen-thread Nix build and a virtual
# machine booting at once, a wakeup in another app waited 280 to 377 us at
# the 99th percentile over two runs, against 802 to 1,534 us in system.slice;
# the kernel build took 202 s, against 233 s; and the Nix build kept 58% of
# the throughput it had. An idle slice did no better for the apps, and since
# it yields to everything in user.slice, a build in a terminal would stop
# Nix's builds altogether. The slice's name has no dash, since systemd reads
# one as nesting: nix-build.slice would sit in a nix.slice at the default
# weight, and its own weight would rank it only against its siblings.
#
# Builds also ask for idle I/O, which a disk scheduler that honours priority
# would serve only when nothing else wants the disk. NVMe drives run without
# one, as the kernel sets them up and as measured fastest on the XPS 14, so
# there it's a statement of intent rather than a limit.
#
# A build that keeps the machine short of memory is stopped by systemd-oomd
# after 30 s, at the same pressure as the user slices, rather than left to the
# kernel's OOM killer, which picks by size and could pick the browser instead.
# oomd stops whichever of the daemon's cgroups reclaims most, which with a
# cgroup for each build is most often one build. vm-first marks the daemon's
# own cgroup for oomd to pass over while a build can go instead, since the
# daemon's going takes every connection with it; when it does go, it starts
# again with the next connection.
#
# A build that brings free space under 10 GiB has the daemon collect garbage
# until 24 GiB is free, so Nix never fills the disk by itself, whatever else
# looks after it. The daemon checks every 5 s while it builds, and collects
# only garbage, nothing a profile or a root still needs. Nix's default of 0
# never collects: the XPS 14 made 238 GiB of garbage in its first three days,
# most of it disk images of up to 9 GiB each. 10 GiB leaves room for one of
# those to finish while the collection runs, and 14 GiB more keeps it from
# running again with the next build. It's the last resort beneath the disk
# guard, so it starts past the guard's urgent band, a tenth free, on a disk
# as small as 120 GB. At 24 GiB, on such a disk the guard had cleared, every
# path added began a collection for garbage that wasn't there, and on
# 9 November one held the store's lock inside its add for half an hour, every
# other add and the guard waiting behind it.
#
# Each build runs in a cgroup of its own, which Nix makes under the daemon's
# as nix-build-uid-<its user>, having moved the daemon itself into a
# nix-daemon cgroup beside them. So a build's processes end with it, none
# left running, each build's CPU, memory and disk use can be read from its
# cgroup, and the CPU is shared by build rather than by thread, so a build
# on sixteen threads no longer crowds out one on a single thread. They all
# stay in build.slice, under its weight and its limits. Nix 3.21 makes the
# cgroups without handing them any controllers, and systemd handed the
# daemon none for the CPU until asked, so vm-first hands on what systemd
# gives, once the daemon has moved aside, since systemd can't start a new
# daemon in a cgroup that hands controllers on; Nix 2.35 does that itself.
# The daemon stops only its own process when it restarts, so a switch that
# turns this on lets the builds already running finish where they are.
# Before each start, systemd takes the hand-off back so the new daemon can
# start, which would fail, and the restart with it, while a build had handed
# controllers further down itself. None can: Nix gives a build its cgroup
# and the right to move its processes, but not its cgroup.subtree_control,
# so a systemd-nspawn container in a uid-range build makes cgroups with no
# controllers, and the daemon restarts beside it. A Nix that hands that on
# too fails vm-checks-go-first before a switch can find it out. As
# each build ends, vm-first says in its journal what the build wrote and
# read, and for which command and quest, so the builds that wear the disk
# most can be found: journalctl -u vm-first.
#
# A check's virtual machine is a build too, and on a busy machine it shared
# each CPU evenly with the compilers beside it, as batch, which never
# preempts as it wakes, and at idle it would wait behind every program
# and the agents' work besides: its guest's timers fired late and the check's
# timeouts failed, so other sessions held their builds while one ran. So
# vm-first puts every qemu thread back to the normal policy at nice -10
# within a second of it starting, and weighs the build holding it at 1000
# against every other build's 100, as nice -10 is to 0. On one CPU, a
# spinner at nice -10 took 81% of it against one at 0. Everything outside
# the daemon, what Joel uses, keeps its lead.
#
# What needs that is a check with deadlines, not qemu itself: a check run in
# containers or in a nested compositor keeps time the same way. So a build
# running NixOS's test driver, systemd-nspawn, niri or sway is put ahead as
# a VM's is, every thread in it and in its containers' cgroups, and weighed
# at 1000. Only the daemon's builds are looked in, never Joel's own niri.
#
# A check can run its machines as systemd-nspawn containers rather than
# virtual machines, which start in a second or two rather than booting a
# kernel. A container needs a range of user ids of its own, which a build
# asking for the uid-range feature gets once the daemon allocates build users
# itself rather than taking nixbld's.
#
# The daemon trusts whoever its trusted-users name by their uid, and a trusted
# user can change its substituters, turn its sandbox off, or add a path to the
# store unsigned, which makes it root. A sandbox that hands an agent the
# daemon's socket hands it root with it, since the agent keeps Joel's uid. So
# every account also gets a door of its own, nix-untrusted.sock in its runtime
# folder, where a daemon of its own takes each connection as an untrusted
# user's and passes on only what such a user may ask. Builds, evaluations and
# fetches from the configured caches work through it as before; a setting only
# trusted users may change is refused with a warning. The experimental
# features are what Nix 2.34 asks for `nix daemon`, --force-untrusted and
# --process-ops, named here so the door opens whatever nix.conf says.
#
# The daemon signs every path it builds with a key of the machine's own, so
# another machine can take what this one built as it takes a cache's, by
# trusting the key in its trusted-public-keys, rather than by trusting the
# account that copies it, which would make that account root there too. The
# key is made once, by root, as the machine first activates: the secret half
# in /etc/nix/signing-key, root's alone, and the public half beside it in
# signing-key.pub for a machine repository to hand the others. It's named
# for the machine, as caches' keys are, and kept across switches, since the
# others trust it by that name. Paths built before it, and those copied in
# from other builders, stay as they came. Activation runs before the daemon
# starts, at boot and at a switch, so the daemon never looks for a key that
# isn't there yet, which would fail every build.
#
# Tools that fetch private sources hand Nix their credentials as impure-env
# entries, which a fixed-output build reads from its environment, and stock
# Nix takes impure-env only from a trusted user. So an account left out of
# trusted-users, or a sandbox at the door, couldn't fetch them at all. The
# daemon here takes, from a client that isn't trusted, only the variables a
# machine names in untrusted-impure-env-names, and drops the rest, naming
# each but never its value (untrusted-named-impure-env.patch). That gives
# such a client nothing over others: the variables reach only fixed-output
# builds, whose outputs are checked against their hashes, so a client hands
# its own credentials to its own downloads. Each connection has a daemon
# process of its own, so one client's values never reach another's builds.
# A machine that names none gets stock Nix's answer.
#
# The names reach the daemon and the door through NIX_CONFIG in their units,
# never nix.conf, which every client reads too. A Nix other than this one,
# such as the one tec puts first on the path, doesn't know the setting and
# warned "unknown setting 'untrusted-impure-env-names'" on every command.
# Both need them: the door filters what a sandbox hands it, and the daemon
# filters again what the door, as an account it doesn't trust, passes on.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  gib = 1024 * 1024 * 1024;
  # Only 2.34's daemon is patched, so a newer Nix fails
  # an-untrusted-client-passes-only-named-env until the patch is carried to it.
  nix =
    if pkgs.nix ? appendPatches && lib.versions.majorMinor pkgs.nix.version == "2.34" then
      pkgs.nix.appendPatches [ ./untrusted-named-impure-env.patch ]
    else
      pkgs.nix;
  vmFirst = pkgs.callPackage ./vm-first/package.nix { };
  untrusted = builtins.concatStringsSep " " [
    "${config.nix.package}/bin/nix"
    "--extra-experimental-features nix-command"
    "--extra-experimental-features daemon-trust-override"
    "--extra-experimental-features mounted-ssh-store"
    "daemon --stdio --force-untrusted --process-ops"
  ];
  cfg = config.jitsusama.nix-daemon;
  # Set in the daemon's and the door's environment alone, after nix.conf.
  namedEnv = lib.optionalAttrs (cfg.untrustedImpureEnvNames != [ ]) {
    NIX_CONFIG = "untrusted-impure-env-names = ${lib.concatStringsSep " " cfg.untrustedImpureEnvNames}";
  };
in
{
  options.jitsusama.nix-daemon.untrustedImpureEnvNames = lib.mkOption {
    type = lib.types.listOf (lib.types.strMatching "[A-Za-z_][A-Za-z0-9_]*");
    default = [ ];
    example = [ "NIX_NPM_REGISTRY_PASSWORD" ];
    description = ''
      The impure-env variables the daemon takes from an account it doesn't
      trust, for fixed-output builds alone: the credentials the machine's
      tools hand Nix for private downloads. None, as stock Nix takes.
    '';
  };

  config = {
    nix.package = lib.mkDefault nix;
    nix.daemonCPUSchedPolicy = "idle";
    nix.daemonIOSchedClass = "idle";

    nix.settings.auto-allocate-uids = true;
    nix.settings.use-cgroups = true;
    # Every machine here is built from a flake and rebuilds from one, and core's
    # own programs, `nix-offload there` and changed-checks among them, build
    # flakes by their attributes. Stock Nix still calls both commands and
    # flakes experimental, so a machine whose own config didn't add them
    # refused every `nix build github:...` sent to it.
    nix.settings.experimental-features = [
      "auto-allocate-uids"
      "cgroups"
      "flakes"
      "nix-command"
    ];
    nix.settings.extra-system-features = [ "uid-range" ];

    nix.settings.secret-key-files = [ "/etc/nix/signing-key" ];

    # After /etc, which /etc/nix belongs to.
    system.activationScripts.nix-signing-key = {
      deps = [ "etc" ];
      text = builtins.concatStringsSep " " [
        "${pkgs.bash}/bin/bash ${./signing-key.sh}"
        "${config.nix.package}/bin/nix"
        "${config.networking.hostName}-1"
        "/etc/nix/signing-key /etc/nix/signing-key.pub"
      ];
    };

    nix.settings.min-free = 10 * gib;
    nix.settings.max-free = 24 * gib;
    # A collection keeps what is in use, not everything that built it. With
    # keep-outputs every live derivation kept the outputs of all that built it,
    # and with keep-derivations every live path kept the derivation that built
    # it, and with it every source that went in. So each system generation and
    # dev shell held its whole build closure, and Nix called none of it garbage:
    # on 10 November about 200 GB of a 344 GiB store, with the disk at 90% and
    # the disk guard able to free nothing. A 5 GB copy of a work tree was held
    # only because a derivation named it as a source. What a rebuild would
    # fetch again, like the reader's crate tarballs, is kept as young garbage
    # by the disk guard instead, within its budget and only while the disk is
    # healthy, and an evaluation writes again any derivation it needs.
    nix.settings.keep-outputs = false;
    nix.settings.keep-derivations = false;

    systemd.slices.build.sliceConfig.CPUWeight = 10;

    systemd.services.nix-daemon.environment = namedEnv;

    systemd.services.nix-daemon.serviceConfig = {
      Slice = "build.slice";
      # Nix's own unit says `Delegate=`, which delegates but asks for no
      # controllers, so the daemon got only those systemd already ran for its
      # slice, and no unit there asks for the CPU's. Named here, after it, they
      # are asked for. (DelegateControllers is the property systemd reports,
      # not a key it reads.)
      Delegate = "cpu io memory pids";
      # A restart stops only the daemon that listens, so each client's own
      # daemon and its build finish under the new one, and a switch that
      # restarts it cuts nothing (vm-checks-go-first). Nix's unit says so too;
      # said here, it stays so.
      KillMode = "process";
      ManagedOOMMemoryPressure = "kill";
      ManagedOOMMemoryPressureLimit = "80%";
      # When memory runs out, a build dies before anything Joel uses: its
      # builders inherit this, and a Chromium app sets its own processes to
      # 300, so at the daemon's 0 the kernel chose Slack over a build. Among
      # builds alike, the largest goes.
      OOMScoreAdjust = 1000;
      # A VM check's guest syncs the folders it shares with the host, and
      # virtiofsd forwards each as syncfs() of the host's whole filesystem,
      # read-only shares too. On btrfs each one commits a transaction, which
      # every program's saves then wait behind: 133 of 160 commits in ten
      # minutes of VM checks came from there. It's answered as done without
      # being run, and a check's files are thrown away anyway. Answered
      # ENOSYS, as until 4 November, a build's `sync -f` failed with "Function
      # not implemented", and with it 2 tests of a Rust crate whose shims
      # sync. sync, fsync and fdatasync still run, and Nix itself calls no
      # syncfs(). A test run straight from its driver runs outside the daemon,
      # so core's VM tests also start virtiofsd through without-syncfs
      # (checks.nix).
      SystemCallFilter = [ "~syncfs:0" ];
    };

    systemd.services.vm-first = {
      description = "Puts the VMs of Nix's checks ahead of its other builds";
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        ExecStart = "${vmFirst}/bin/vm-first";
        Restart = "on-failure";
        RestartSec = 5;
        # As root, which owns the daemon's cgroup and its builds' weights. It
        # holds no capability but the one below, and ProtectSystem leaves it
        # nothing to write outside /sys, /proc and /dev.
        # The capability, only to change another user's threads' policy and
        # nice.
        AmbientCapabilities = [ "CAP_SYS_NICE" ];
        CapabilityBoundingSet = [ "CAP_SYS_NICE" ];
        NoNewPrivileges = true;
        PrivateNetwork = true;
        ProtectHome = true;
        ProtectSystem = "strict";
        MemoryDenyWriteExecute = true;
        SystemCallArchitectures = "native";
      };
    };

    systemd.user.sockets.nix-untrusted = {
      description = "The Nix daemon's door for sandboxes, which trusts no one";
      wantedBy = [ "sockets.target" ];
      listenStreams = [ "%t/nix-untrusted.sock" ];
      socketConfig = {
        Accept = true;
        SocketMode = "0600";
      };
    };

    systemd.user.services."nix-untrusted@" = {
      description = "The Nix daemon, for one untrusted connection";
      # One unit a connection, gone once it ends, even failed: a client that
      # hangs up before speaking ends its daemon with "unexpected end-of-file",
      # and on 13 November one such was left a failed unit for every sweep
      # to find. What it said stays in the journal.
      unitConfig.CollectMode = "inactive-or-failed";
      environment = namedEnv;
      serviceConfig = {
        ExecStart = untrusted;
        StandardInput = "socket";
        StandardOutput = "socket";
        StandardError = "journal";
      };
    };
  };
}
