# The mesh reader at %t/pi-mesh-reader/reader.sock: one process that
# follows each folder of pi's mesh state for every pi process that asks,
# rather than each of seventy listing and reading the same folder itself
# (mesh-reader/src/lib.rs says why). It reads only under
# ~/.local/state/pi, answers only the account, and is started by the first
# pi process that asks. Should it be down, pi reads the folder itself at
# its slower backstop, so nothing waits on it.
{ lib, pkgs, ... }:
let
  meshReader = pkgs.callPackage ./mesh-reader/package.nix { };
in
{
  config = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
    systemd.user.sockets.pi-mesh-reader = {
      Unit.Description = "Reads each folder of pi's mesh once for every pi process that asks";
      Socket = {
        ListenStream = "%t/pi-mesh-reader/reader.sock";
        SocketMode = "0600";
        DirectoryMode = "0700";
        Service = "pi-mesh-reader.service";
      };
      Install.WantedBy = [ "sockets.target" ];
    };
    systemd.user.services.pi-mesh-reader = {
      Unit = {
        Description = "Reads each folder of pi's mesh once for every pi process that asks";
        Requires = [ "pi-mesh-reader.socket" ];
        After = [ "pi-mesh-reader.socket" ];
      };
      Service = {
        ExecStart = "${lib.getExe meshReader} --root %h/.local/state/pi";
        SyslogIdentifier = "pi-mesh-reader";
        # Outside agents.slice, so the agents' ceiling never slows the one
        # reader all of them read through; its own bound keeps it small.
        # It holds each followed folder's files once, about a megabyte.
        MemoryHigh = "256M";
        MemoryMax = "512M";
        Restart = "on-failure";
      };
    };
  };
}
