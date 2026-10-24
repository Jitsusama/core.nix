# Dictation's chord held: dictation-chord says when Super and D are let go,
# and after how long, at /run/dictation-chord/socket, so holding the chord
# listens until it's let go. niri says nothing of a chord's release, and
# the keys are the input group's, which an account mustn't join, so it
# runs as a user of its own in that group, and the kernel hands it those
# three keys alone. It can't reach the network, a home or another device.
{ pkgs, ... }:
let
  dictation = pkgs.callPackage ./package.nix { };
in
{
  users.users.dictation-chord = {
    isSystemUser = true;
    group = "dictation-chord";
  };
  users.groups.dictation-chord = { };

  systemd.services.dictation-chord = {
    description = "Says when dictation's chord is let go";
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      ExecStart = "${dictation}/bin/dictation-chord /run/dictation-chord/socket";
      Restart = "on-failure";
      User = "dictation-chord";
      Group = "dictation-chord";
      SupplementaryGroups = [ "input" ];
      RuntimeDirectory = "dictation-chord";
      RuntimeDirectoryMode = "0755";
      RemoveIPC = true;
      DevicePolicy = "closed";
      DeviceAllow = [ "char-input r" ];
      CapabilityBoundingSet = "";
      NoNewPrivileges = true;
      PrivateNetwork = true;
      RestrictAddressFamilies = "AF_UNIX";
      IPAddressDeny = "any";
      ProtectSystem = "strict";
      ProtectHome = true;
      PrivateTmp = true;
      ProtectKernelTunables = true;
      ProtectKernelModules = true;
      ProtectKernelLogs = true;
      ProtectControlGroups = true;
      ProtectClock = true;
      ProtectHostname = true;
      ProtectProc = "invisible";
      ProcSubset = "pid";
      LockPersonality = true;
      MemoryDenyWriteExecute = true;
      RestrictNamespaces = true;
      RestrictRealtime = true;
      RestrictSUIDSGID = true;
      SystemCallArchitectures = "native";
      SystemCallFilter = [
        "@system-service"
        "~@privileged @resources"
      ];
      UMask = "0022";
    };
  };
}
