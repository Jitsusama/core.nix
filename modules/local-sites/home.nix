# Joel's local sites: the mocks, tours and records made just for him, served
# from ~/.local/share/jitsusama/sites on this machine alone, so none goes
# to a host other people reach. Its root holds a home page listing each
# site, and each site is a folder under it, reached at
# http://sites.localhost:8484/<site>/ (a browser sends any .localhost name
# to the machine itself). The sites are what sessions write, so they're
# state, never a Nix value; only the server and the home's builder are
# declared.
#
# The server is local-sites (src/main.rs says how), which serves the
# documents, the folders a module mounts at their own paths, and a live
# feed at /feed/<name> from each daemon a module declares, with the kinds
# and fields a page may read of it, so the server enforces what a page
# never sees. It answers only its own names on loopback and opens a feed
# only to its own pages.
#
# The home is built from sites.json, a row per site, which sessions write:
# home/home.py writes the list, its llms.txt and Markdown twin, and each
# page's strip, and copies the home's own scripts and styles in beside
# them. It runs whenever sites.json changes and at every switch, so a
# changed builder reaches the home without anyone running it.
#
# A live site is built by Nix from its TypeScript on the kit (site.nix
# says how) and mounted here with the feed it watches. system/ is the
# first: how the whole system stands, watched live from the system
# daemon's socket. Its source is modules/system/site, declared here
# rather than in the system module, so a machine with the system daemon
# and no local sites still evaluates.
#
# Three sites still read files written beside them. mycelium/ maps the
# realm as it stands, from a now.json beside it that a timer rewrites every
# 20 seconds from what the machine's node says (mycelium-map.sh, shaped by
# mycelium-map.jq). A machine in no realm has no node to read, so it runs
# no timer. network/ maps the home network from net's record (modules/net),
# copied in each time net rewrites it, since the server serves nothing
# outside its root and won't follow a link out of it; on a machine that
# runs no net, the record never appears and nothing is copied. code/ lists
# every repository on the disk from a repos.json beside it that a timer
# rewrites every minute from repos's record (code-site.sh), until Code
# reads a feed of its own; a machine without repos (modules/repos) runs no
# timer.
{ mycelium }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.local-sites;
  local-sites = pkgs.callPackage ./package.nix { };
  sites = import ./site.nix { inherit lib pkgs; };
  # The theme's web kit (modules/theme/web), which every site's page reads.
  webKit = config.xdg.dataFile."jitsusama/web".source;
  documents = "${config.xdg.dataHome}/jitsusama/sites";
  # Loopback, under the names a browser sends there by itself.
  served = pkgs.writeText "local-sites.json" (
    builtins.toJSON {
      listen = "127.0.0.1:8484";
      names = [
        "sites.localhost"
        "localhost"
        "127.0.0.1"
      ];
      inherit documents;
      inherit (cfg) mounts feeds;
    }
  );
  kind = lib.types.submodule {
    options = {
      fields = lib.mkOption {
        type = lib.types.nullOr (lib.types.listOf lib.types.str);
        default = null;
        description = ''
          The only fields of the kind's records a page is shown, besides
          the key, and the only ones the daemon is asked for; null shows
          them all.
        '';
      };
      deep = lib.mkOption {
        type = lib.types.nullOr (lib.types.listOf lib.types.str);
        default = null;
        description = ''
          Fields the daemon is asked to say deep, so a change inside one
          says only what moved in it, as the mycelium node's `deep` does.
        '';
      };
    };
  };
  feed = lib.types.submodule {
    options = {
      socket = lib.mkOption {
        type = lib.types.str;
        example = "daemons/net.sock";
        description = ''
          The daemon's socket, absolute or under the user's runtime folder.
        '';
      };
      kinds = lib.mkOption {
        type = lib.types.attrsOf kind;
        description = ''
          The kinds a page may read, by name; a kind not named here is
          refused, so a feed shows no more than its module declared.
        '';
      };
    };
  };
  root = "%h/.local/share/jitsusama/sites";
  realm = config.jitsusama.mycelium.realm or null;
  roots = config.jitsusama.repos.roots or null;
  build = lib.escapeShellArgs [
    (lib.getExe pkgs.python3)
    "${./home/home.py}"
    "${./home}"
    "${config.xdg.dataHome}/jitsusama/sites"
  ];
in
{
  # Every live site wears the theme's web kit.
  imports = [ ../theme/home.nix ];

  options.jitsusama.local-sites.feeds = lib.mkOption {
    type = lib.types.attrsOf feed;
    default = { };
    description = ''
      The daemons a page may read live, each at /feed/<name>, with the
      kinds and fields of each it may read. A page reads and never acts:
      the server passes only hello, get, list, watch, unwatch and health.
    '';
  };

  options.jitsusama.local-sites.mounts = lib.mkOption {
    type = lib.types.attrsOf lib.types.path;
    default = { };
    description = ''
      Folders Nix built, each served at /<name>/ ahead of the documents,
      as a live site's pages are.
    '';
  };

  config.jitsusama.local-sites = {
    mounts = {
      local-sites = sites.kit;
      system = sites.localSite {
        name = "system";
        src = ../system/site;
        inherit webKit;
        # Its emphasis, as its styles mark it, in the action's gold: the
        # page's world is the weather over everything, and gold is its sun.
        tint = "action";
      };
    }
    // lib.optionalAttrs (realm != null) {
      # What each of this machine's cells is for, from its node's forage
      # records (modules/mycelium/forage), in the programs' hue.
      forage = sites.localSite {
        name = "forage";
        src = ../mycelium/forage;
        inherit webKit;
        tint = "program";
      };
    };
    # The system daemon's summary, its parts and their things
    # (modules/system), each told whole: a page shows all of each.
    feeds = {
      system = {
        socket = "daemons/system.sock";
        kinds = {
          summary = { };
          part = { };
          machine = { };
        };
      };
    }
    // lib.optionalAttrs (realm != null) {
      # The realm's node on this machine, of which a page reads only each
      # cell's forage: its purpose, standing, documents and lineage, and
      # never what passes between cells.
      node = {
        socket = "mycelium/${realm}.sock";
        kinds.forage.fields = [
          "cell"
          "machine"
          "quest"
          "purpose"
          "status"
          "documents"
          "lineage"
        ];
      };
    };
  };

  config.systemd.user.paths = {
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

  # The config the unit serves, where its help's example reads it, so
  # what it serves can be read without finding its path in the store.
  config.xdg.configFile."local-sites/local-sites.json".source = served;

  # A switch builds the home too, since a changed builder or page changes
  # no file the path unit watches. A run that changes nothing writes nothing.
  config.home.activation.localSitesHome = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    run ${build}
  '';

  config.systemd.user.services = {
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
        # Its command line says its name, as a daemon's does (docs/daemons.md).
        ExecStart = "@${lib.getExe local-sites} local-sites --config ${served}";
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
  }
  // lib.optionalAttrs (roots != null) {
    # repos reads git's own files alone, taking no lock and running no git, so
    # a reading never gets in the way of an agent's work in a checkout.
    code-site = {
      Unit.Description = "Writes the code site's record from what repos reads on the disk";
      Service = {
        Type = "oneshot";
        Environment = [
          "REPOS_ROOTS=${lib.concatStringsSep ":" roots}"
          "PATH=${
            lib.makeBinPath [
              (pkgs.callPackage ../repos/package.nix { })
              pkgs.coreutils
            ]
          }"
        ];
        ExecStart = lib.escapeShellArgs [
          (lib.getExe pkgs.bash)
          "${./code-site.sh}"
          "${config.xdg.dataHome}/jitsusama/sites/code"
        ];
        Slice = "background.slice";
      };
    };
  };

  # Every 20 seconds, as the page reads it; a run takes about a tenth of a
  # second.
  config.systemd.user.timers =
    lib.optionalAttrs (realm != null) {
      mycelium-map = {
        Unit.Description = "Rewrites the mycelium site's map every 20 seconds";
        Timer = {
          OnActiveSec = "1s";
          OnUnitActiveSec = "20s";
          AccuracySec = "1s";
        };
        Install.WantedBy = [ "timers.target" ];
      };
    }
    # Every minute: a reading takes about 60 ms, and the page says how old
    # its record is, so a minute keeps it current without the disk being read
    # more than it changes.
    // lib.optionalAttrs (roots != null) {
      code-site = {
        Unit.Description = "Rewrites the code site's record every minute";
        Timer = {
          OnActiveSec = "5s";
          OnUnitActiveSec = "60s";
          AccuracySec = "5s";
        };
        Install.WantedBy = [ "timers.target" ];
      };
    };
}
