# The switch's reader, which net's daemon reads the home's TP-Link Easy
# Smart switch through (AGENTS.md, The Switch's Reader). The switch shows
# its ports only once signed in, and its admin password lets its holder
# mirror ports or change VLANs, so the password never reaches an account:
# every agent runs as Joel's. A system service of its own, under a user
# made for it alone, holds the password as an encrypted credential, signs
# in, and writes /run/net-switch/report.json, which any account may read
# and which holds only what the switch's status pages show.
#
# The credential is written by hand, never as a Nix value:
#   systemd-creds encrypt --name=switch-password - /etc/credstore.encrypted/switch-password
# Until it is, the report says the switch is signed out and nothing else
# changes. A machine with no switch leaves the address unset and runs no
# reader; net then says its report isn't there.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.net.switch;
  net = pkgs.callPackage ./package.nix { };
in
{
  options.jitsusama.net.switch.address = lib.mkOption {
    type = lib.types.nullOr lib.types.str;
    default = null;
    example = "10.0.0.30";
    description = ''
      The IPv4 address the home's TP-Link Easy Smart switch answers at,
      the only address its reader may reach; or null where there's no
      switch to read.
    '';
  };

  config = lib.mkIf (cfg.address != null) {
    systemd.services.net-switch = {
      description = "Reads the home switch's status pages for net";
      wants = [ "network-online.target" ];
      after = [ "network-online.target" ];
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        # Background work, behind everything Joel uses, as net's daemon is.
        Slice = "background.slice";
        ExecStart = "${lib.getExe net} switch-reader ${cfg.address}";
        Restart = "on-failure";
        RestartSec = 60;
        # A user made for this service alone, so no account, Joel's
        # included, can read the password it's handed.
        DynamicUser = true;
        # Given by name only, it's looked for in /etc/credstore.encrypted/
        # and a missing one isn't fatal, so the reader starts signed out.
        LoadCredentialEncrypted = "switch-password";
        # The report's folder, readable by every account, the one place it
        # writes. Its files are written whole and renamed into place.
        RuntimeDirectory = "net-switch";
        RuntimeDirectoryMode = "0755";
        UMask = "0022";
        # It reaches the switch and nothing else: no other address, no name
        # lookup, no other kind of socket.
        IPAddressDeny = "any";
        IPAddressAllow = cfg.address;
        RestrictAddressFamilies = [
          "AF_INET"
          "AF_UNIX"
        ];
        CapabilityBoundingSet = "";
        NoNewPrivileges = true;
        PrivateDevices = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectKernelLogs = true;
        ProtectControlGroups = true;
        ProtectClock = true;
        ProtectHostname = true;
        ProtectProc = "invisible";
        ProcSubset = "pid";
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        LockPersonality = true;
        MemoryDenyWriteExecute = true;
        SystemCallArchitectures = "native";
        SystemCallFilter = [
          "@system-service"
          "~@privileged @resources"
        ];
      };
    };
  };
}
