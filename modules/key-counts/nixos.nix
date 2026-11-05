# Counts the keys Joel presses, so the ErgoDox's layout can be weighed
# against what he really uses (Joel, 2026-11-05: yes to "a count-only key
# counter"). key-counts keeps, for each keyboard, layer, modifiers held and
# key, how many presses and how many held, in /var/lib/key-counts/counts.tsv,
# and never the order keys came in, when, or a word typed.
#
# The keys are the input group's, which an account mustn't join, or anything
# it runs could read every key, so it runs as a user of its own in that
# group, as dictation-chord does. It can't reach the network, a home or any
# device but the input devices, so the counts stay on the machine. They're
# readable by every account, since a count says nothing of what was typed.
{ pkgs, ... }:
let
  key-counts = pkgs.callPackage ./package.nix { };
in
{
  users.users.key-counts = {
    isSystemUser = true;
    group = "key-counts";
  };
  users.groups.key-counts = { };

  systemd.services.key-counts = {
    description = "Counts each key pressed, and nothing typed";
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      ExecStart = "${key-counts}/bin/key-counts /var/lib/key-counts/counts.tsv";
      Restart = "on-failure";
      User = "key-counts";
      Group = "key-counts";
      SupplementaryGroups = [ "input" ];
      StateDirectory = "key-counts";
      StateDirectoryMode = "0755";
      RemoveIPC = true;
      DevicePolicy = "closed";
      DeviceAllow = [ "char-input r" ];
      CapabilityBoundingSet = "";
      NoNewPrivileges = true;
      PrivateNetwork = true;
      RestrictAddressFamilies = "none";
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
