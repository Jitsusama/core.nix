# Joel's local sites: the mocks, tours and records made just for him, served
# from ~/.local/share/jitsusama/sites on this machine alone, so none goes
# to a host other people reach. Its root serves a home page listing each
# site, written into a folder of its own (homeFolder), and each site is a
# folder under it, reached at
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
# only to its own pages. A document a session wrote is served under a name
# of its own, http://documents.sites.localhost:8484/<site>/, its link under
# the sites' name sent there, so its script runs in the documents' origin
# and can't act as one of the sites' own. It reads what sits beside it, the
# kit and the home's rows among them, but no feed, which only the sites' own
# pages read, and it keeps its theme apart from theirs.
#
# The home is built from sites.json, a row per site, which sessions write:
# home/home.py writes the list, its llms.txt and Markdown twin, and each
# page's strip, and copies the home's own styles in beside them; its
# keys and switch come from the kit, as every site's do. It runs whenever
# sites.json changes and at every switch, so a changed builder reaches
# the home without anyone running it. A live site's row is declared
# here beside its mount instead (rows), so what the home says of it is
# built with it; it stands in for any row sites.json still has at its
# path.
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
# the disk, so the daemon writes it into the documents as it reads, which
# answer for the mount (modules/repos/src/homes.rs). A machine without
# repos has none of it.
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
  # The theme's web kit (modules/theme/web), which every site's page and the
  # home read from the one copy the sites share.
  webKit = config.xdg.dataFile."jitsusama/web".source;
  documents = "${config.xdg.dataHome}/jitsusama/sites";
  # Where the home's builder writes the home's own page and files, under the
  # runtime folder: a fenced session neither sees nor writes it, so the
  # server serves them as the sites' own rather than sandboxed as a document
  # is (src/http.rs says why), and the home can read Joel's feeds and answer
  # what he's asked. A reboot empties it, and the home is built whole from
  # its record before the server serves; a record that won't read then
  # builds it from the last record that read (builtFrom's --kept), or from
  # the declared rows alone where none was kept, until a session mends it,
  # since the last build went with the reboot.
  homeFolder = cfg.homeFolder;
  # Loopback, under the names a browser sends there by itself.
  served = pkgs.writeText "local-sites.json" (
    builtins.toJSON {
      listen = "127.0.0.1:8484";
      names = [
        "sites.localhost"
        "localhost"
        "127.0.0.1"
      ];
      documentsName = "documents.sites.localhost";
      inherit documents;
      home = homeFolder;
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
  # strip is the one its build writes, so the home writes none. The home
  # writes every row it lists into rows.json, these as declared, for the
  # homes' readers (system's, the web-design skill's, Code's). System's and
  # Code's read it first and sites.json only without it, and its live rows
  # stay in sites.json too, so rolling this back means taking rows.json
  # away as well, or they keep reading the last rows it wrote. A session's
  # edit to a live row there no longer reaches the home, since the
  # declaration stands in for it.
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

  # The sites' ask row refuses a choice or act posing as one of the
  # launcher's acts on a look by core's own names for them (kit/ask.ts), so
  # a machine renaming one would let a page offer what For You refuses.
  lookActs = import ./look-acts.nix;
  # Words as the shell's Words.qml reads them: as words.json writes them,
  # by toJSON, which writes a set with an outPath or __toString as that,
  # whether it's a thing or its id. A store path's context, which fromJSON
  # refuses, says nothing of a word, so it goes.
  asRead = words: builtins.fromJSON (builtins.unsafeDiscardStringContext (builtins.toJSON words));
  coreWords = asRead (import ../quickshell/launcher/words.nix).value;
  # The file Words.qml reads, as this machine writes it; none where it has
  # no launcher, whose words are then no one's.
  wordsFile = config.xdg.configFile."quickshell/launcher/words.json" or null;
  # Whether the file is made from its text, as the launcher writes it: a
  # source given in its place wins over the text, which then says words
  # the launcher never reads. Such a source isn't read, since reading it
  # would build it while the machine is evaluated, so it's refused below.
  fromText =
    wordsFile.text or null != null
    && lib.isAttrs wordsFile.source
    && (wordsFile.source.text or null) == wordsFile.text;
  unreadWords = wordsFile != null && !fromText;
  # The machine's words as the file holds them, even where a machine writes
  # that text itself.
  machineWords =
    if fromText then
      builtins.fromJSON (builtins.unsafeDiscardStringContext wordsFile.text)
    else
      coreWords;
  # A thing's id as Words.qml keys it, made a string: a list of one id is
  # that id; no act's id holds a comma or is empty, or reads as an object,
  # so no other id can be one.
  keyOf =
    id:
    if builtins.isString id then
      id
    else if builtins.isList id && builtins.length id == 1 then
      keyOf (builtins.head id)
    else
      null;
  # A word's name as Words.qml names it: by the last thing of its id, so a
  # rename appended after core's own still counts, null where that thing
  # names none.
  nameIn =
    words: id:
    let
      things = words.things or [ ];
      thing = lib.findFirst (thing: keyOf (thing.id or null) == id) null (
        lib.reverseList (if builtins.isList things then things else [ ])
      );
    in
    if thing == null then id else thing.name or null;
  # A name as the refusal writes it, one that isn't words as JSON writes it.
  shownIn =
    words: id:
    let
      name = nameIn words id;
    in
    if name == null then
      "no name"
    else if builtins.isString name then
      name
    else
      builtins.toJSON name;
  renamedActs = lib.filter (id: nameIn machineWords id != nameIn coreWords id) lookActs;
  # Rows whose path isn't plain steps, which the home's builder refuses
  # (home/home.py's ours), named here so the switch never gets that far.
  step = "[A-Za-z0-9_~-][A-Za-z0-9._~-]*";
  astray = lib.filter (path: builtins.match "${step}(/${step})*" path == null) (
    lib.attrNames cfg.rows
  );
  # Kinds two declared sites are home to, patterns the homes' readers would
  # drop, and kinds one site gives two patterns, each of which takes links
  # away (rows-homes.nix says how).
  declaredHomes = import ./rows-homes.nix { inherit lib; } cfg.rows;
  roots = config.jitsusama.repos.roots or null;
  # Whether the map reaches every cell of the realm (modules/mycelium/site/
  # map-check.sh): the map's live-page test, pointed at the page this
  # machine serves, for a reader such as the hourly card to count with
  # --json (map-check.nix).
  mapCheck = import ./map-check.nix { inherit lib pkgs; };
  # The home's builder, then what it builds from; its folder goes between,
  # where the runtime folder is known (systemd's %t, or the activation's).
  builder = lib.escapeShellArgs [
    (lib.getExe pkgs.python3)
    "${./home/home.py}"
  ];
  builtFrom = lib.escapeShellArgs [
    # The last record that read, kept where no session writes and a reboot
    # doesn't reach, to build the home from when sites.json won't read and
    # the runtime folder holds no build to keep.
    "--kept=${config.xdg.stateHome}/local-sites/sites.json"
    "${import ./home-source.nix}"
    "${config.xdg.dataHome}/jitsusama/sites"
    "${webKit}/theme.css"
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
      as a live site's pages are. A page there runs only scripts served
      beside it (script-src 'self'): one written into the page, an
      onclick-like attribute or a javascript: link does nothing.
    '';
  };

  options.jitsusama.local-sites.homeFolder = lib.mkOption {
    type = lib.types.str;
    default = "local-sites/home";
    readOnly = true;
    description = ''
      The folder under the runtime folder the home is built into, where
      it lists its rows, so what reads them reads where they're written.
    '';
  };

  config.jitsusama.local-sites = {
    mounts = {
      local-sites = sites.shared webKit;
      system = sites.localSite {
        name = "system";
        src = ../system/site;
        inherit webKit;
        # Its emphasis, as its styles mark it, in the action's gold, the
        # page's one accent.
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
        name = "Your machines, now";
        what = lib.concatStrings [
          "Every machine you have, drawn as a bridge: how each stands, and what it "
          "carries of your mycelium, your home network and your code, with the way "
          "deeper into each."
        ];
        asks = lib.concatStrings [
          "Step through the machines with j and k; say whether the first screen tells "
          "you which machine is which and how each is doing."
        ];
        # Each machine's address on the page, which every other site links to:
        # opened there, its cursor starts on that machine.
        homes = [
          {
            kind = "machine";
            address = "#machine={name}";
          }
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
        # Each cell's address on the map, which every other site links a
        # cell to: opened there, its drawer is open. The map reads a cell's
        # whole link there too, mycelium:cell/<ADDRESS>@<machine>, as its
        # address alone.
        homes = [
          {
            kind = "cell";
            address = "#cell={name}";
          }
        ];
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
      # cell's purpose, standing, documents, lineage, what its quest
      # waits on its person for, and the needs it serves and relies on:
      # each entry whole as the node sends it, of which the page draws the
      # cell at the other end, the need's own words, escaped, and since
      # when. The map reads, of each forage, only its cell and who started
      # it, from its lineage, though every page is passed what the forage
      # page reads. It reads its networks and cells whole, as `mycelium networks` draws them; of
      # each machine only what the map draws; of each message only who
      # sent it, how and where, never its words; the balance service's
      # latest reading whole, for each machine's share of the work; and of
      # what waits on the mesh, each question waiting and hold standing,
      # who sent it, where and since when, and whom or until when it
      # waits, never its words either. Of each tie between two cells, when
      # they last talked, how much each said and of what kinds, and how
      # strong it stood then, never where it went. Of each feeding, which
      # receptor fed which cell something, when and of what kind, which
      # the node keeps under `source`, never the change itself; `source`
      # passes whole, holding today only that word. Of each murmur the
      # murmurs page reads only what it draws, not the machine it was
      # said on. Of each builder the machine sends its builds to, the map
      # reads how it stands and what it builds, never its host: its `said`
      # is the node's reason, which names the way it's reached (a jump
      # host, a proxy) but never an address.
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
            "waits"
            "serves"
            "relies"
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
          builder.fields = [
            "name"
            "state"
            "systems"
            "jobs"
            "lent"
            "said"
          ];
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
          tie.fields = [
            "heard"
            "hour"
            "today"
            "intents"
            "strength"
          ];
          fed.fields = [
            "receptor"
            "cell"
            "at"
            "source"
          ];
        };
      };
    }
    // lib.optionalAttrs (roots != null) {
      # The repos daemon's reading, repositories, checkouts, trees and
      # changes (modules/repos/src/feed.rs), each told whole: Code draws all
      # of each, and none holds more than git's heads and GitHub's checks.
      # Its using records say how many processes work in each folder and
      # what they take, never which.
      repos = {
        socket = "daemons/repos.sock";
        kinds = {
          reading = { };
          repository = { };
          checkout = { };
          tree = { };
          change = { };
          using = { };
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

  config.home.packages = lib.optional (realm != null) mapCheck;

  # The config the unit serves, where its help's example reads it, so
  # what it serves can be read without finding its path in the store.
  config.xdg.configFile."local-sites/local-sites.json".source = served;

  config.assertions = [
    # A machine renaming the launcher's acts on a look (renamedActs above).
    # A words file local-sites can't read, as a source given in its text's
    # place, could rename them unseen (fromText above).
    {
      assertion = !unreadWords;
      message = lib.concatStrings [
        "the launcher's words.json is given as a source, which local-sites can't read "
        "to hold the launcher's acts on a look to core's names: write it as text"
      ];
    }
    {
      assertion = renamedActs == [ ];
      message = lib.concatStrings [
        "the launcher's words.json, as jitsusama.quickshell.launcher.words or the machine "
        "writes it, renames the launcher's acts on a look ("
        (lib.concatMapStringsSep ", " (
          id: "${id} from ${shownIn coreWords id} to ${shownIn machineWords id}"
        ) renamedActs)
        "), which the sites' ask row knows only by core's names, "
        "so a page would offer a choice For You refuses: keep core's names for them"
      ];
    }
    # A row the home lists with nothing mounted at its path would send Joel
    # to a page that isn't there.
    {
      assertion = unmounted == [ ];
      message = lib.concatStrings [
        "jitsusama.local-sites.rows names sites with no mount: ${toString unmounted}; "
        "remove them from jitsusama.local-sites.rows too"
      ];
    }
    {
      assertion = astray == [ ];
      message = lib.concatStrings [
        "jitsusama.local-sites.rows names sites by paths that lead off the sites: "
        "${toString astray}; name each by plain steps of letters, digits and -._~, "
        "none starting with a dot"
      ];
    }
    {
      assertion = declaredHomes.homedTwice == [ ];
      message = lib.concatStrings [
        "jitsusama.local-sites.rows names more than one site as home to a kind: "
        "${lib.concatStringsSep "; " declaredHomes.homedTwice}; let one site be each kind's home"
      ];
    }
    {
      assertion = declaredHomes.dropped == [ ];
      message = lib.concatStrings [
        "jitsusama.local-sites.rows gives patterns the homes' readers drop: "
        "${lib.concatStringsSep "; " declaredHomes.dropped}; give each a kind and an "
        "address that is a fragment (#) on its own page holding {name}, in plain bytes"
      ];
    }
    {
      assertion = declaredHomes.patternedTwice == [ ];
      message = lib.concatStrings [
        "jitsusama.local-sites.rows gives a kind more than one pattern on one site: "
        "${lib.concatStringsSep "; " declaredHomes.patternedTwice}; keep one, since the "
        "homes' readers take the first and never draw the others' links (lib.mkForce "
        "replaces the homes a site already declares rather than adding to them)"
      ];
    }
  ];

  # A switch builds the home too, since a changed builder or page changes
  # no file the path unit watches. A run that changes nothing writes nothing.
  # Only where the runtime folder stands, as it does wherever the server
  # runs; its unit builds the home at login on a machine that has none yet.
  config.home.activation.localSitesHome = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    localSitesRuntime="''${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
    if [ -d "$localSitesRuntime" ]; then
      run ${builder} "--into=$localSitesRuntime/${homeFolder}" ${builtFrom}
    fi
  '';

  config.systemd.user.services = {
    local-sites-home = {
      Unit.Description = "Builds the home of Joel's local sites from its record";
      Service = {
        Type = "oneshot";
        ExecStart = "${builder} --into=%t/${homeFolder} ${builtFrom}";
        Slice = "background.slice";
      };
      # At login too, since a reboot empties the runtime folder the home is in.
      Install.WantedBy = [ "default.target" ];
    };

    local-sites = {
      Unit = {
        Description = "Serves Joel's local sites, on loopback only";
        # The home is built first, or its page answers that it isn't ready.
        Wants = [ "local-sites-home.service" ];
        After = [ "local-sites-home.service" ];
      };
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
    # The daemon holds the record and hears the disk move, so it writes
    # Code's homes as it reads rather than anything reading every repository
    # again to write them. They're the code site's, which is declared here,
    # so here is where the daemon is told where they go.
    repos.Service.Environment = [ ''"REPOS_HOMES=${documents}/code/homes.json"'' ];
  };
}
