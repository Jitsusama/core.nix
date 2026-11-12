# Joel's local sites: the mocks, tours and records made just for him, served
# from ~/.local/share/jitsusama/sites on this machine alone, so none goes
# to a host other people reach. Its root holds a home page listing each
# site, and each site is a folder under it, reached at
# http://sites.localhost:8484/<site>/ (a browser sends any .localhost name
# to the machine itself). The sites are what sessions write, so they're
# state, never a Nix value; only the server is declared.
{ lib, pkgs, ... }:
let
  root = "%h/.local/share/jitsusama/sites";
in
{
  systemd.user.services.local-sites = {
    Unit.Description = "Serves Joel's local sites, on loopback only";
    Service = {
      ExecStartPre = "${lib.getExe' pkgs.coreutils "mkdir"} -p ${root}";
      ExecStart = lib.concatStringsSep " " [
        (lib.getExe pkgs.static-web-server)
        "--host 127.0.0.1"
        "--port 8484"
        "--root ${root}"
        # A site changes while he reads it, so a reload shows the change.
        "--cache-control-headers false"
        "--compression true"
      ];
      # Whatever Joel runs comes first; a page waits a moment at most.
      Slice = "background.slice";
      Restart = "on-failure";
      RestartSec = "5s";
    };
    Install.WantedBy = [ "default.target" ];
  };
}
