# The processor's own telemetry, read for the shell: the temperatures no
# hwmon chip shows (the NPU's, the image processor's, the media and display
# engines') and what holds the processor back, by which limit, each second
# while someone asks (src/main.rs says how). The kernel shows the region to
# root alone, so the reader runs as root with nothing else it could use, and
# says only those readings to whoever asks.
{ pkgs, ... }:
let
  reader = pkgs.callPackage ./package.nix { };
in
{
  # systemd holds the socket, and starts the reader the first time anything
  # connects, so a machine nobody asks never runs it.
  systemd.sockets.processor-telemetry = {
    description = "Where the shell asks for the processor's own telemetry";
    wantedBy = [ "sockets.target" ];
    listenStreams = [ "/run/processor-telemetry/io.jitsusama.ProcessorTelemetry" ];
    socketConfig = {
      # Where varlinkctl list-registry finds it, beside systemd's own.
      Symlinks = [ "/run/varlink/registry/io.jitsusama.ProcessorTelemetry" ];
      FileDescriptorName = "varlink";
      # Anyone may ask, as anyone may read hwmon's temperatures: a reading
      # a second of temperatures and limits says no more than those do, and
      # a group would reach Joel's shell only after he next signs in.
      SocketMode = "0666";
      DirectoryMode = "0755";
    };
  };

  systemd.services.processor-telemetry = {
    description = "Read the processor's own temperatures and limits";
    requires = [ "processor-telemetry.socket" ];
    after = [ "processor-telemetry.socket" ];
    serviceConfig = {
      ExecStart = "${reader}/bin/processor-telemetry";
      Restart = "on-failure";
      # Root only to read a file root owns, so no capability at all: the
      # file's owner reads it without one.
      CapabilityBoundingSet = "";
      NoNewPrivileges = true;
      PrivateNetwork = true;
      PrivateTmp = true;
      ProtectHome = true;
      ProtectSystem = "strict";
      # Reading /sys is all it does there.
      ProtectKernelTunables = true;
      ProtectKernelModules = true;
      ProtectKernelLogs = true;
      ProtectControlGroups = true;
      ProtectClock = true;
      ProtectHostname = true;
      RestrictAddressFamilies = [ "AF_UNIX" ];
      RestrictNamespaces = true;
      RestrictRealtime = true;
      LockPersonality = true;
      MemoryDenyWriteExecute = true;
      SystemCallArchitectures = "native";
      SystemCallFilter = [ "@system-service" ];
    };
  };
}
