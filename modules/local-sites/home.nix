# Joel's local sites: the mocks, tours and records made just for him, served
# from ~/.local/share/jitsusama/sites on this machine alone, so none goes
# to a host other people reach. Its root holds a home page listing each
# site, and each site is a folder under it, reached at
# http://sites.localhost:8484/<site>/ (a browser sends any .localhost name
# to the machine itself). The sites are what sessions write, so they're
# state, never a Nix value; only the server is declared.
#
# One site is live: mycelium/ maps the realm as it stands, from a now.json
# beside it that a timer rewrites every 20 seconds from what the machine's
# node says (mycelium-map.sh, shaped by mycelium-map.jq). A machine in no
# realm has no node to read, so it runs no timer.
{ mycelium }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  root = "%h/.local/share/jitsusama/sites";
  realm = config.jitsusama.mycelium.realm or null;
in
{
  systemd.user.services = {
    local-sites = {
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
  // lib.optionalAttrs (realm != null) {
    mycelium-map = {
      Unit.Description = "Writes the mycelium site's map from what the node says now";
      Service = {
        Type = "oneshot";
        Environment = [
          "MYCELIUM_REALM=${realm}"
          "PATH=${
            lib.makeBinPath [
              mycelium.packages.${pkgs.stdenv.hostPlatform.system}.default
              pkgs.coreutils
              pkgs.jq
            ]
          }"
        ];
        ExecStart = lib.escapeShellArgs [
          (lib.getExe pkgs.bash)
          "${./mycelium-map.sh}"
          "${./mycelium-map.jq}"
          "${config.xdg.dataHome}/jitsusama/sites/mycelium"
        ];
        Slice = "background.slice";
      };
    };
  };

  # Every 20 seconds, as the page reads it; a run takes about a tenth of a
  # second.
  systemd.user.timers = lib.optionalAttrs (realm != null) {
    mycelium-map = {
      Unit.Description = "Rewrites the mycelium site's map every 20 seconds";
      Timer = {
        OnActiveSec = "1s";
        OnUnitActiveSec = "20s";
        AccuracySec = "1s";
      };
      Install.WantedBy = [ "timers.target" ];
    };
  };
}
