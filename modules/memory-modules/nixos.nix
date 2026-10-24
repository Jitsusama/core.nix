# Which memory modules the machine has, for the launcher's Memory to say what
# RAM it is: each one's locator, size, type, speed, maker and part, as the
# firmware lists them in SMBIOS. The kernel lets only root read those, so a
# unit run as root once at boot writes them to /run/memory-modules.json,
# readable by everyone; modules.py says how it reads them. Nothing waits on
# it, since nothing needs it until Joel asks.
{ pkgs, ... }:
let
  modules = pkgs.writers.writePython3Bin "memory-modules" { doCheck = false; } (
    builtins.readFile ./modules.py
  );
  written = "/run/memory-modules.json";
in
{
  systemd.services.memory-modules = {
    description = "Says which memory modules the firmware lists, for the launcher";
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      Type = "oneshot";
      # It writes in a folder of its own, the one place the sandbox lets it
      # write, and the file is then renamed into place, unconfined, so a
      # reader never sees it half written.
      RuntimeDirectory = "memory-modules";
      ExecStart = "${modules}/bin/memory-modules /sys/firmware/dmi/entries %t/memory-modules/modules.json";
      ExecStartPost = "+${pkgs.coreutils}/bin/mv %t/memory-modules/modules.json ${written}";
      # The entries are root's own and readable by their owner, so root
      # reads them with no capability at all. A DynamicUser couldn't.
      CapabilityBoundingSet = "";
      NoNewPrivileges = true;
      PrivateDevices = true;
      PrivateNetwork = true;
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
      RestrictAddressFamilies = "none";
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
      UMask = "0022";
    };
  };
}
