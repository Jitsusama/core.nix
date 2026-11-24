# The camera kept from agents near Joel's meetings: from a quarter hour
# before each until it ends, no process in the agents' slices may open a
# camera, while his own programs in app.slice, Meet and Tuple among them,
# and the system's camera relays may (src/main.rs says how). Agents run as
# his account, so only their cgroups tell them apart, and only root may put
# a device program on a cgroup, so the guard is the system's.
#
# The slices are the account's agents' places, as ../slices names them:
# jobs.slice, where pi's tools, its spawned sessions and the units they
# start with systemd-run run (../slices/systemd-run.sh), and agents.slice,
# pi's hosts and the sessions' terminals. Joel's own work in jobs.slice's
# jobs-awaited.slice is refused a camera too, near a meeting, which only a
# camera read by hand from a terminal would notice.
{
  config,
  lib,
  options,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.camera-guard) account;
  guard = pkgs.callPackage ./package.nix { };
in
{
  options.jitsusama.camera-guard.account = lib.mkOption {
    type = lib.types.nullOr lib.types.str;
    # The machine's own account, which is Joel's where one is named.
    default = (import ../greetd/account.nix { inherit config lib options; }).account;
    defaultText = lib.literalExpression "config.jitsusama.login.account";
    example = "joel";
    description = ''
      The account whose agents are kept off the camera near its meetings,
      read from its google daemon. Null starts no guard.
    '';
  };

  config.systemd.services.camera-guard = lib.mkIf (account != null) {
    description = "Keeps agents from opening a camera near Joel's meetings";
    wantedBy = [ "multi-user.target" ];
    # A guard that stops leaves its program on, refusing agents a camera
    # until it's back and decides afresh, so it's started again at once.
    startLimitIntervalSec = 0;
    serviceConfig = {
      Type = "exec";
      ExecStart = lib.concatStringsSep " " [
        (lib.getExe guard)
        account
        "jobs.slice"
        "agents.slice"
      ];
      Restart = "always";
      RestartSec = 5;
      # The meetings last said, so a restart while the calendar can't be
      # read still knows the meeting under way.
      StateDirectory = "camera-guard";
      # bpf(2) to load the program and put it on a cgroup, and to find an
      # earlier run's by its id; reading past the account's own runtime
      # folder, which is its alone, to ask its google daemon.
      CapabilityBoundingSet = [
        "CAP_BPF"
        "CAP_DAC_OVERRIDE"
        "CAP_NET_ADMIN"
        "CAP_SYS_ADMIN"
      ];
      DevicePolicy = "closed";
      LockPersonality = true;
      MemoryDenyWriteExecute = true;
      NoNewPrivileges = true;
      PrivateNetwork = true;
      PrivateTmp = true;
      # The cgroups are opened to read alone; bpf(2) puts the program on
      # through the descriptor, which a read-only mount allows.
      ProtectControlGroups = true;
      ProtectKernelModules = true;
      ProtectSystem = "strict";
      RestrictAddressFamilies = "AF_UNIX";
      RestrictNamespaces = true;
      RestrictRealtime = true;
      SystemCallArchitectures = "native";
    };
  };
}
