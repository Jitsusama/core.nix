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
# changed builder reaches the home without anyone running it. A live
# site's row is declared here beside its mount instead (rows), so what the
# home says of it is built with it; it stands in for any row sites.json
# still has at its path.
#
# A live site is built by Nix from its TypeScript on the kit (site.nix
# says how) and mounted here with the feed it watches. system/ is the
# first: how the whole system stands, watched live from the system
# daemon's socket; network/ maps the home network, watched live from
# net's (modules/net). Their sources are modules/system/site and
# modules/net/site, declared here rather than in their daemons' modules,
# so a machine with a daemon and no local sites still evaluates.
#
# mycelium/ and forage/ are watched live from the machine's mycelium
# node: the realm's map (modules/mycelium/site) and what each cell is for
# (modules/mycelium/forage). A machine in no realm has no node to read, so
# it has neither.
#
# code/ draws every repository on the disk, watched live from the repos
# daemon (modules/repos), and the cells at work in them from the node. Its
# homes.json, where each repository and tree is home on it, changes with
# the disk, so a timer writes it from repos's record into the documents
# (code-site.sh), which answer for the mount. A machine without repos has
# none of it.
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
  # What the home says of a live site, as a row of sites.json says it.
  row = lib.types.submodule {
    options = {
      name = lib.mkOption {
        type = lib.types.str;
        description = "The site's name, as the home lists it.";
      };
      what = lib.mkOption {
        type = lib.types.str;
        description = "What the site shows, in a sentence.";
      };
      asks = lib.mkOption {
        type = lib.types.str;
        default = "";
        description = "What the site asks of Joel, if anything.";
      };
      homes = lib.mkOption {
        type = lib.types.listOf (lib.types.either lib.types.str (lib.types.attrsOf lib.types.str));
        default = [ ];
        description = ''
          The kinds of thing the site is home to, each a kind's name or a kind
          with its address pattern, as a row of sites.json says them.
        '';
      };
    };
  };
  # Each declared row as home.py reads it: an app, growing, and live. Its
  # strip is the one its build writes, so the home writes none. The homes'
  # readers (system's, the web-design skill's, Code's) still read sites.json,
  # so its live rows stay there, and rolling this back stays safe, until
  # they read these. A session's edit to a live row there no longer reaches
  # the home, since the declaration stands in for it.
  declared = pkgs.writeText "local-sites-rows.json" (
    builtins.toJSON {
      sites = lib.mapAttrsToList (
        path: row:
        row
        // {
          path = "${path}/";
          kind = "app";
          when = "live";
        }
      ) cfg.rows;
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
  # Rows naming a site nothing is mounted at.
  unmounted = lib.attrNames (removeAttrs cfg.rows (lib.attrNames cfg.mounts));
  roots = config.jitsusama.repos.roots or null;
  build = lib.escapeShellArgs [
    (lib.getExe pkgs.python3)
    "${./home/home.py}"
    "${./home}"
    "${config.xdg.dataHome}/jitsusama/sites"
    "${declared}"
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

  options.jitsusama.local-sites.rows = lib.mkOption {
    type = lib.types.attrsOf row;
    default = { };
    description = ''
      What the home says of each live site, by the name it's mounted at,
      in place of a row of sites.json at its path.
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
      network = sites.localSite {
        name = "network";
        src = ../net/site;
        inherit webKit;
        # Everything on its map is a machine, so it wears their hue too.
        tint = "machine";
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
      # The realm drawn as a map, live from its node (modules/mycelium/site).
      mycelium = sites.localSite {
        name = "mycelium";
        src = ../mycelium/site;
        inherit webKit;
        # Its mark, in the hue the theme gives what programs are.
        tint = "program";
      };
      # What the quests' sessions murmur into their webs, live from the
      # node's murmur records (modules/mycelium/murmurs). It is served at
      # web/, where the home has always listed it; the copy once written to
      # the state folder there stays behind the mount, unread.
      web = sites.localSite {
        name = "web";
        src = ../mycelium/murmurs;
        inherit webKit;
        tint = "program";
      };
    }
    // lib.optionalAttrs (roots != null) {
      # Every repository on the disk as a grove, live from the repos daemon
      # (modules/repos/site). Its trees are leaves, so it wears the hue the
      # theme gives what's well and growing.
      code = sites.localSite {
        name = "code";
        src = ../repos/site;
        inherit webKit;
        tint = "success";
      };
    };
    # What the home says of each mount, a row beside it, so a site's row
    # is there exactly when the site is.
    rows = {
      system = {
        name = "Your system, now";
        what = lib.concatStrings [
          "How your whole system stands at once: every machine on your mycelium, the "
          "mycelium itself, your home network and your code, each judged by its own "
          "owner, with the way deeper into each."
        ];
        asks = lib.concatStrings [
          "Step through the four parts with j and k; say whether the sentence at the "
          "top tells you enough at a glance."
        ];
      };
      network = {
        name = "Your home network, now";
        what = lib.concatStrings [
          "A live map of your home network, floor by floor: each eero's radios and "
          "how busy they are, every device with its address and signal, and each "
          "one's route to the internet."
        ];
        asks = lib.concatStrings [
          "Press Return on this machine to see its route to the internet; say what "
          "you'd want it to show."
        ];
      };
    }
    // lib.optionalAttrs (realm != null) {
      forage = {
        name = "What each cell is for";
        what = lib.concatStrings [
          "Every agent session on this machine with its forage: what it's for, what "
          "it found and planned, and which sessions it started."
        ];
        asks = "Open the orchestrator's session; say whether its purpose reads at a glance.";
      };
      mycelium = {
        name = "Your mycelium, now";
        what = lib.concatStrings [
          "A live map of every session and machine at work for you, each hanging "
          "from the networks it has joined, moving the moment one changes."
        ];
        asks = "Keep it open a while; say what you'd want to see that it doesn't show.";
      };
      web = {
        name = "What your sessions murmur";
        what = lib.concatStrings [
          "Small things your sessions notice and tie to what they're about, gathered "
          "from every machine sharing the quest and the same everywhere within seconds."
        ];
      };
    }
    // lib.optionalAttrs (roots != null) {
      code = {
        name = "Code";
        what = lib.concatStrings [
          "Every repository on this machine drawn as a tree, its worktrees as leaves "
          "coloured by when each was last written, with its open pull requests and "
          "the agents at work in each, moving the moment git writes."
        ];
        asks = "Press ? for its keys; say whether it reads at a glance.";
        homes = [
          "repository"
          "tree"
        ];
      };
    };
    feeds = {
      # The system daemon's summary, its parts and their things
      # (modules/system), each told whole: a page shows all of each.
      system = {
        socket = "daemons/system.sock";
        kinds = {
          summary = { };
          part = { };
          machine = { };
        };
      };
      # net's devices, the links between them, the sources it read them from,
      # the internet and the concerns worth a look (modules/net/src/serve.rs),
      # each told whole: the map draws all of each. The upload's use, which
      # net reads only while it's watched, isn't the page's to ask for.
      net = {
        socket = "daemons/net.sock";
        kinds = {
          device = { };
          link = { };
          source = { };
          internet = { };
          concern = { };
        };
      };
    }
    // lib.optionalAttrs (realm != null) {
      # The realm's node on this machine. The forage page reads only each
      # cell's purpose, standing, documents and lineage. The map reads its
      # networks and cells whole, as `mycelium networks` draws them; of
      # each machine only what the map draws; of each message only who
      # sent it, how and where, never its words; the balance service's
      # latest reading whole, for each machine's share of the work; and of
      # what waits on the mesh, each question waiting and hold standing,
      # who sent it, where and since when, and whom or until when it
      # waits, never its words either. Of each murmur the murmurs page
      # reads only what it draws, not the machine it was said on.
      node = {
        socket = "mycelium/${realm}.sock";
        kinds = {
          forage.fields = [
            "cell"
            "machine"
            "quest"
            "purpose"
            "status"
            "documents"
            "lineage"
          ];
          network = { };
          cell = { };
          machine.fields = [
            "name"
            "state"
            "model_name"
            "os"
            "system"
            "cpu"
            "memory"
            "disk"
            "disk_free"
            "disk_band"
            "power"
            "room"
            "declared"
            "measured_at"
          ];
          message.fields = [
            "at"
            "from"
            "intent"
            "groups"
          ];
          murmur.fields = [
            "writer"
            "at"
            "text"
            "tied_to"
            "purpose"
            "settled"
          ];
          balance = { };
          unanswered.fields = [
            "id"
            "from"
            "intent"
            "groups"
            "since"
            "re"
            "waiting_on"
            "until"
          ];
        };
      };
    }
    // lib.optionalAttrs (roots != null) {
      # The repos daemon's reading, repositories, checkouts, trees and
      # changes (modules/repos/src/feed.rs), each told whole: Code draws all
      # of each, and none holds more than git's heads and GitHub's checks.
      repos = {
        socket = "daemons/repos.sock";
        kinds = {
          reading = { };
          repository = { };
          checkout = { };
          tree = { };
          change = { };
        };
      };
    };
  };

  config.systemd.user.paths = {
    local-sites-home = {
      Unit.Description = "Watches the sites' record for the home";
      Path.PathChanged = "${root}/sites.json";
      Install.WantedBy = [ "paths.target" ];
    };
  };

  # The config the unit serves, where its help's example reads it, so
  # what it serves can be read without finding its path in the store.
  config.xdg.configFile."local-sites/local-sites.json".source = served;

  # A row the home lists with nothing mounted at its path would send Joel
  # to a page that isn't there.
  config.assertions = [
    {
      assertion = unmounted == [ ];
      message = lib.concatStrings [
        "jitsusama.local-sites.rows names sites with no mount: ${toString unmounted}; "
        "remove them from jitsusama.local-sites.rows too"
      ];
    }
  ];

  # A switch builds the home too, since a changed builder or page changes
  # no file the path unit watches. A run that changes nothing writes nothing.
  config.home.activation.localSitesHome = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    run ${build}
  '';

  config.systemd.user.services = {
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
  // lib.optionalAttrs (roots != null) {
    # repos reads git's own files alone, taking no lock and running no git, so
    # a reading never gets in the way of an agent's work in a checkout.
    code-site = {
      Unit.Description = "Writes where each repository and tree is home on the code site";
      Service = {
        Type = "oneshot";
        Environment = [
          "REPOS_ROOTS=${lib.concatStringsSep ":" roots}"
          "PATH=${
            lib.makeBinPath [
              (pkgs.callPackage ../repos/package.nix { })
              pkgs.coreutils
              pkgs.jq
            ]
          }"
        ];
        ExecStart = lib.escapeShellArgs [
          (lib.getExe pkgs.bash)
          "${./code-site.sh}"
          "${config.xdg.dataHome}/jitsusama/sites/code"
        ];
        # A reading takes about 60 ms, so one still going at the next minute
        # is stuck, and ending it says so in the journal while the last homes
        # stay.
        TimeoutStartSec = "50s";
        Slice = "background.slice";
      };
    };
  };

  # Every minute: a reading takes about 60 ms, and a link to a tree cut a
  # minute ago landing on Code before its home is said is no harm, since
  # Code itself draws live.
  config.systemd.user.timers = lib.optionalAttrs (roots != null) {
    code-site = {
      Unit.Description = "Rewrites the code site's homes every minute";
      Timer = {
        OnActiveSec = "5s";
        OnUnitActiveSec = "60s";
        AccuracySec = "5s";
      };
      Install.WantedBy = [ "timers.target" ];
    };
  };
}
