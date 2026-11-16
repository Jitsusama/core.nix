# Joel's local sites: the mocks, tours and records made just for him, served
# from ~/.local/share/jitsusama/sites on this machine alone, so none goes
# to a host other people reach. Its root holds a home page listing each
# site, and each site is a folder under it, reached at
# http://sites.localhost:8484/<site>/ (a browser sends any .localhost name
# to the machine itself). The sites are what sessions write, so they're
# state, never a Nix value; only the server and the home's builder are
# declared.
#
# The home is built from sites.json, a row per site, which sessions write:
# home/home.py writes the list, its llms.txt and Markdown twin, and each
# page's strip, and copies the home's own scripts and styles in beside
# them. It runs whenever sites.json changes and at every switch, so a
# changed builder reaches the home without anyone running it.
#
# Two sites are live. mycelium/ maps the realm as it stands, from a
# now.json beside it that a timer rewrites every 20 seconds from what the
# machine's node says (mycelium-map.sh, shaped by mycelium-map.jq). A
# machine in no realm has no node to read, so it runs no timer. network/
# maps the home network from net's record (modules/net), copied in each
# time net rewrites it, since the server serves nothing outside its root
# and won't follow a link out of it; on a machine that runs no net, the
# record never appears and nothing is copied.
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
  build = lib.escapeShellArgs [
    (lib.getExe pkgs.python3)
    "${./home/home.py}"
    "${./home}"
    "${config.xdg.dataHome}/jitsusama/sites"
  ];
in
{
  systemd.user.paths = {
    network-site = {
      Unit.Description = "Watches net's record for the network site";
      Path.PathChanged = "%t/net/network.json";
      Install.WantedBy = [ "paths.target" ];
    };
    local-sites-home = {
      Unit.Description = "Watches the sites' record for the home";
      Path.PathChanged = "${root}/sites.json";
      Install.WantedBy = [ "paths.target" ];
    };
  };

  # A switch builds the home too, since a changed builder or page changes
  # no file the path unit watches. A run that changes nothing writes nothing.
  home.activation.localSitesHome = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    run ${build}
  '';

  systemd.user.services = {
    # A copy put in place by a rename, so the page never reads half of one.
    network-site = {
      Unit.Description = "Copies net's record into the network site";
      Service = {
        Type = "oneshot";
        ExecStart = [
          "${lib.getExe' pkgs.coreutils "mkdir"} -p ${root}/network"
          "${lib.getExe' pkgs.coreutils "cp"} %t/net/network.json ${root}/network/network.json.new"
          "${lib.getExe' pkgs.coreutils "mv"} -f ${root}/network/network.json.new ${root}/network/network.json"
        ];
        Slice = "background.slice";
      };
    };

    local-sites-home = {
      Unit.Description = "Builds the home of Joel's local sites from its record";
      Service = {
        Type = "oneshot";
        ExecStart = build;
        Slice = "background.slice";
      };
    };

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
