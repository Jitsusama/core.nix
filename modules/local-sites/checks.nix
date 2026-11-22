# The checks of local-sites, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  self,
  workstation,
  ...
}:
let
  sites = import ./site.nix { inherit lib pkgs; };

  # The theme's own web kit, as a machine writes it, which the checks that
  # read pages live build on; the -holds checks build on an empty one.
  webKit = (account workstation).xdg.dataFile."jitsusama/web".source;

  # An account on a machine in a realm. Its node and net feeds are what the
  # mycelium, murmurs, forage and network live checks hand their servers,
  # and what forage's feed passes is what
  # forage-feed-passes-what-its-page-reads holds to the page, on every
  # system. Each call of bareNixos evaluates a whole system, so the five
  # share this one.
  inRealm = account (bareNixos [
    self.nixosModules.workstation
    {
      home-manager.users.someone = {
        imports = [ self.homeModules.local-sites ];
        jitsusama.mycelium.realm = "harbour";
      };
    }
  ]);

  # The Python the checks that drive a page in headless Chromium run in.
  python = pkgs.python3.withPackages (python: [
    python.playwright
    python.pytest
  ]);

  # What every live site shares, as a machine mounts it at /local-sites/:
  # the kit and the theme's own web kit, which each page reads.
  shared = sites.shared webKit;

  # The system page, built as the server serves it, which the strip's
  # check reads as well as holding it.
  systemSite = sites.localSite {
    name = "system";
    src = ../system/site;
    webKit = pkgs.emptyDirectory;
    tint = "action";
    looks = {
      dark = {
        background = "black";
        action = "white";
      };
      light = {
        background = "white";
        action = "black";
      };
    };
  };
in
{
  # The kit typechecks and its tests pass as it builds (kit/tests).
  local-sites-kit-holds = sites.kit;

  # The system page typechecks on the kit and builds, its tests passing
  # (modules/system/site/tests). Its page's theme is no part of what it
  # proves, so it's built on an empty one, its looks plain black and white.
  system-site-holds = systemSite;

  # Every live page opens with the strip localSite writes, once, after its
  # skip link, saying only that it's an app, and reads the theme from the
  # sites' one copy with no kit/ of its own; and a page that carries its own
  # strip, reads no theme from that copy, or whose styles read a kit/ of
  # their own, fails its build, saying so; and so does a sheet esbuild
  # refuses, warns on or crashes on, or a sheet or a script keeping a
  # comment, saying which; and the sheets and scripts a site, the kit and
  # the theme build reach a page without their comments
  # (tests/each-live-page-opens-with-the-strip.sh).
  each-live-page-opens-with-the-strip =
    pkgs.runCommandLocal "each-live-page-opens-with-the-strip" { }
      ''
        ${lib.getExe pkgs.bash} ${./tests/each-live-page-opens-with-the-strip.sh} ${
          lib.escapeShellArgs [
            "${systemSite}"
            ./site.sh
            (lib.getExe' pkgs.typescript "tsc")
            (lib.getExe pkgs.nodejs)
            (lib.getExe pkgs.esbuild)
            ../theme/web/bare-sheet.sh
            ./kept-comments.sh
            "${./kit}"
            "${../system/site}"
            "system"
            "${sites.kit}/kit"
            "${webKit}"
          ]
        }
        touch $out
      '';

  # The forage page typechecks on the kit and builds, its tests passing
  # (modules/mycelium/forage/tests), on the same plain looks.
  forage-site-holds = sites.localSite {
    name = "forage";
    src = ../mycelium/forage;
    webKit = pkgs.emptyDirectory;
    tint = "program";
    looks = {
      dark = {
        background = "black";
        program = "white";
      };
      light = {
        background = "white";
        program = "black";
      };
    };
  };

  # The node's feed passes every field the forage page reads off a forage
  # record (modules/mycelium/forage/model.ts), as a machine in a realm
  # declares it, so nothing the page draws goes missing live. The page's
  # own tests hand it records whole, so they can't see a field the feed
  # drops. A record's key is always passed.
  forage-feed-passes-what-its-page-reads =
    let
      passed = [ "key" ] ++ inRealm.jitsusama.local-sites.feeds.node.kinds.forage.fields;
      read = lib.unique (
        lib.concatMap (lib.filter builtins.isString) (
          lib.filter builtins.isList (
            builtins.split ''\(record, "([a-z_]+)"\)|record\["([a-z_]+)"]'' (
              builtins.readFile ../mycelium/forage/model.ts
            )
          )
        )
      );
      dropped = lib.subtractLists passed read;
    in
    pkgs.runCommandLocal "forage-feed-passes-what-its-page-reads" { } (
      if read == [ ] then
        ''
          echo "no field read off a forage record was found in model.ts"
          exit 1
        ''
      else if dropped != [ ] then
        ''
          echo "the forage page reads ${lib.concatStringsSep ", " dropped}, which its feed drops"
          exit 1
        ''
      else
        "touch $out"
    );

  # The murmurs page typechecks on the kit and builds, its tests passing
  # (modules/mycelium/murmurs/tests), on the same plain looks.
  murmurs-site-holds = sites.localSite {
    name = "web";
    src = ../mycelium/murmurs;
    webKit = pkgs.emptyDirectory;
    tint = "program";
    looks = {
      dark = {
        background = "black";
        program = "white";
      };
      light = {
        background = "white";
        program = "black";
      };
    };
  };

  # The network page typechecks on the kit and builds, its tests passing
  # (modules/net/site/tests), on the same plain theme as the system page's.
  network-site-holds = sites.localSite {
    name = "network";
    src = ../net/site;
    webKit = pkgs.emptyDirectory;
    tint = "machine";
    looks = {
      dark = {
        background = "black";
        machine = "white";
      };
      light = {
        background = "white";
        machine = "black";
      };
    };
  };

  # The mycelium map typechecks on the kit and builds, its tests passing
  # (modules/mycelium/site/tests), on the same plain looks.
  mycelium-site-holds = sites.localSite {
    name = "mycelium";
    src = ../mycelium/site;
    webKit = pkgs.emptyDirectory;
    tint = "program";
    looks = {
      dark = {
        background = "black";
        program = "white";
      };
      light = {
        background = "white";
        program = "black";
      };
    };
  };

  # The code page typechecks on the kit and builds, its tests passing
  # (modules/repos/site/tests), on the same plain theme as the system page's.
  code-site-holds = sites.localSite {
    name = "code";
    src = ../repos/site;
    webKit = pkgs.emptyDirectory;
    tint = "success";
    looks = {
      dark = {
        background = "black";
        success = "white";
      };
      light = {
        background = "white";
        success = "black";
      };
    };
  };

  # The home's builder lists each site from its row, settled ones apart,
  # puts each row Nix declares in its recorded row's place, says each row in
  # rows.json as written, strips each page that asks, names documents with
  # scripts of their own, draws its mark in each look's tint or the other's
  # where one is missing, takes both from the theme it's handed rather than a
  # kit/ among the documents, leaves out and says a tint or ground that isn't
  # a colour, leaves out and says a recorded row whose path leads off the
  # sites and fails on a declared one, and changes nothing on a second run
  # (tests/home-builds-from-its-rows.sh).
  home-builds-from-its-rows = pkgs.runCommandLocal "home-builds-from-its-rows" { } ''
    ${lib.getExe pkgs.bash} ${./tests/home-builds-from-its-rows.sh} ${
      lib.escapeShellArgs [
        (lib.getExe pkgs.python3)
        ./home
      ]
    }
    touch $out
  '';

  # What a switch refuses in the declared rows' homes (rows-homes.nix), read
  # in evaluation alone, so the rules cost no machine of their own.
  local-sites-rows-homes-read-as-readers-do =
    let
      homes = import ./rows-homes.nix { inherit lib; };
      machines = {
        kind = "machine";
        address = "#machine={name}";
      };
      # The rows core declares, a row giving its kind both ways, and one
      # writing its pattern from the sites' root, all of which stand.
      standing = homes {
        system.homes = [
          "machine"
          machines
        ];
        code.homes = [
          "repository"
          "tree"
        ];
        network.homes = [ ];
        health.homes = [
          {
            kind = "part";
            address = "health/#part={name}";
          }
        ];
      };
      # A second site home to machines, and a dropped pattern beside them,
      # which claims nothing and so isn't a second home.
      twice = homes {
        system.homes = [ machines ];
        network.homes = [ "machine" ];
        health.homes = [
          {
            kind = "machine";
            address = "#m";
          }
        ];
      };
      # Every way a pattern is dropped: no name, no fragment, another page,
      # each byte that could break a link or markup, one that isn't visible
      # ASCII, no kind and no address.
      dropped = homes {
        network.homes =
          map
            (address: {
              kind = "device";
              inherit address;
            })
            [
              "#device"
              "device={name}"
              "system/#device={name}"
              "#device=<{name}>"
              "#device=\\{name}"
              "#device=\"{name}"
              "#device=`{name}"
              "#device={name} x"
              "#device={name}é"
              "#device=\t{name}"
            ]
          ++ [
            { address = "#device={name}"; }
            { kind = "device"; }
          ];
      };
      # A row giving one kind three patterns, of which the readers keep the
      # first alone, named once in their order, and a pattern the readers
      # drop anyway, which leaves the first standing. Another kind's pattern
      # between them hides none, nor does the kind given by name; the first
      # given again, from the page or from the sites' root, draws the same
      # links and is no second; and a dropped pattern ahead of a kept one is
      # no first.
      again = homes {
        code.homes = [
          {
            kind = "tree";
            address = "#tree={name}";
          }
          {
            kind = "tree";
            address = "code/#tree={name}";
          }
        ];
        network.homes = [
          {
            kind = "device";
            address = "#d";
          }
          {
            kind = "device";
            address = "#device={name}";
          }
        ];
        system.homes = [
          machines
          {
            kind = "part";
            address = "#part={name}";
          }
          "machine"
          {
            kind = "machine";
            address = "#host={name}";
          }
          {
            kind = "machine";
            address = "#m";
          }
          {
            kind = "machine";
            address = "system/#machine={name}";
          }
          machines
          {
            kind = "machine";
            address = "#third={name}";
          }
        ];
      };
      # What the switch says of each, so its words are read as well.
      droppedSaid = [
        ''network's device, "#device"''
        ''network's device, "device={name}"''
        ''network's device, "system/#device={name}"''
        ''network's device, "#device=<{name}>"''
        ''network's device, "#device=\\{name}"''
        ''network's device, "#device=\"{name}"''
        ''network's device, "#device=`{name}"''
        ''network's device, "#device={name} x"''
        ''network's device, "#device={name}é"''
        ''network's device, "#device=\t{name}"''
        ''network's pattern with no kind, "#device={name}"''
        "network's device, with no address"
      ];
      found = builtins.toJSON {
        inherit
          standing
          twice
          dropped
          again
          ;
      };
    in
    holds "local-sites-rows-homes-read-as-readers-do"
      "The declared rows' homes aren't refused as their readers would read them: ${found}."
      (
        standing == {
          homedTwice = [ ];
          dropped = [ ];
          patternedTwice = [ ];
        }
        && twice.homedTwice == [ "machine (network, system)" ]
        && twice.dropped == [ ''health's machine, "#m"'' ]
        && dropped.dropped == droppedSaid
        && dropped.homedTwice == [ ]
        &&
          again.patternedTwice == [
            ''system's machine, "#machine={name}", "#host={name}" and "#third={name}"''
          ]
        &&
          again.dropped == [
            ''network's device, "#d"''
            ''system's machine, "#m"''
          ]
        && again.homedTwice == [ ]
        && twice.patternedTwice == [ ]
        && dropped.patternedTwice == [ ]
      );
}
# The server is a Linux user service, and builds only there.
// onLinux {
  # Its tests run as it builds, the server started as its unit starts it
  # beside a stand-in daemon (tests/serve.rs): the documents and a mount
  # served, a file the browser holds answered by its tag alone and one
  # written since sent again, a stranger's name and a stranger's page
  # refused, two pages sharing one watch on the daemon and each told every
  # change trimmed to what the feed shows, an act refused before it reaches
  # the daemon, and a daemon that isn't there said broken with why.
  local-sites-serves-its-sites = pkgs.callPackage ./package.nix { };
  # The system page, built on the theme's own web kit as a machine builds
  # it, read in headless Chromium through the server beside a stand-in
  # system daemon (modules/system/site/tests/reads-live-test.py): live with
  # every part drawn and the whole counted, a part turning and a machine
  # going, broken while the daemon is away and live once it's back, stale
  # once the server goes, the web-design skill's page checks passing at two
  # widths in both looks, and nothing logged. Chromium builds only on Linux.
  system-site-reads-live =
    let
      page = sites.localSite {
        name = "system";
        src = ../system/site;
        inherit webKit;
        tint = "action";
      };
    in
    pkgs.runCommandLocal "system-site-reads-live"
      {
        env = {
          LOCAL_SITES = lib.getExe' (pkgs.callPackage ./package.nix { }) "local-sites";
          SYSTEM_SITE = "${page}";
          LOCAL_SITES_SHARED = "${shared}";
          STRIP = "${./home/strip.css}";
          WEB_SKILL = "${../theme/web/skill}";
          PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
          PYTHONPATH = "${./tests}";
          PYTHONDONTWRITEBYTECODE = "1";
        };
      }
      ''
        HOME=$TMPDIR ${python}/bin/python -m pytest -p no:cacheprovider -p live_page \
          --import-mode=importlib --basetemp="$TMPDIR/pytest" ${../system/site/tests}/reads-live-test.py
        touch $out
      '';
  # The home, built by its own builder from three recorded sites and served
  # with the kit, read in headless Chromium (tests/home-answers-its-keys-test.py):
  # j, k and the arrows step its cursor and hold at the ends, g twice but not
  # once and G reach the first and the last, Return opens the site under the
  # cursor, ? and F1 show its keys and Escape hides them, its shown keys
  # answer a tap, an arrow on the theme switch chooses a look and leaves the
  # cursor, the look holds to the next visit, its mark takes each look's tint, a
  # document under a hero answers a tap on its keys, and the web-design
  # skill's page checks pass at two widths in both looks with nothing logged.
  # Chromium builds only on Linux.
  home-answers-its-keys =
    pkgs.runCommandLocal "home-answers-its-keys"
      {
        env = {
          LOCAL_SITES = lib.getExe' (pkgs.callPackage ./package.nix { }) "local-sites";
          LOCAL_SITES_SHARED = "${shared}";
          STRIP = "${./home/strip.css}";
          WEB_SKILL = "${../theme/web/skill}";
          WEB_KIT = "${webKit}";
          HOME_SOURCES = "${./home}";
          PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
          PYTHONPATH = "${./tests}";
          PYTHONDONTWRITEBYTECODE = "1";
        };
      }
      ''
        HOME=$TMPDIR ${python}/bin/python -m pytest -p no:cacheprovider -p live_page \
          --import-mode=importlib --basetemp="$TMPDIR/pytest" ${./tests}/home-answers-its-keys-test.py
        touch $out
      '';
  # The map, built on the theme's own web kit as a machine builds it, read in
  # headless Chromium through the server beside a stand-in node
  # (modules/mycelium/site/tests/map-reads-live-test.py): live with the realm
  # drawn, a cell moving and another going, broken while the node is away
  # with the realm kept and live once it's back, stale once the server goes
  # with the realm kept, a cell's title drawn as words and never as markup,
  # what each spent today, what waits on a cell and what holds stand as the
  # drawer says them, every cell drawn until too small to read, every line
  # thinned as the map steps out past its own size, the talk between two
  # networks one line until it or a cell it touches opens, the builders
  # standing under the machines in a shape of their own, the
  # web-design skill's page checks and the map's own (site-checks.js)
  # passing at two widths in both looks, nothing logged but what a test
  # causes, and every cell reached (reach.ts) there and on a twin of 10,000.
  # Chromium builds only on Linux.
  # The node's feed is the one home.nix declares for a machine in a realm, so
  # the server under test passes the page exactly what a machine's does.
  mycelium-site-reads-live =
    let
      map = sites.localSite {
        name = "mycelium";
        src = ../mycelium/site;
        inherit webKit;
        tint = "program";
      };
    in
    pkgs.runCommandLocal "mycelium-site-reads-live"
      {
        env = {
          LOCAL_SITES = lib.getExe' (pkgs.callPackage ./package.nix { }) "local-sites";
          MYCELIUM_SITE = "${map}";
          NODE_KINDS = builtins.toJSON inRealm.jitsusama.local-sites.feeds.node.kinds;
          LOCAL_SITES_SHARED = "${shared}";
          STRIP = "${./home/strip.css}";
          WEB_SKILL = "${../theme/web/skill}";
          # The browsers mycelium-map-check carries, so the map's tests
          # launch what a machine runs it with.
          PLAYWRIGHT_BROWSERS_PATH = "${pkgs.callPackage ./headless-browsers.nix { }}";
          PYTHONPATH = "${./tests}";
          PYTHONDONTWRITEBYTECODE = "1";
        };
      }
      ''
        HOME=$TMPDIR ${python}/bin/python -m pytest -p no:cacheprovider -p live_page \
          --import-mode=importlib --basetemp="$TMPDIR/pytest" \
          ${../mycelium/site/tests}/map-reads-live-test.py
        touch $out
      '';
  # The murmurs page, built on the theme's own web kit as a machine builds it,
  # read in headless Chromium through the server beside a stand-in node
  # (modules/mycelium/murmurs/tests/murmurs-reads-live-test.py): live with
  # every murmur drawn newest first and counted, one said, settled or gone,
  # one it can't read said, broken while the node is away with the murmurs
  # kept and live once it's back, stale once the server goes with them kept,
  # words drawn as words and never as markup, its keys and a tap on those its
  # head shows stepping the cursor, a tab whose renderer dies before its first
  # reading opened once more, and the web-design skill's page checks
  # passing at two widths in both looks with nothing logged. Chromium builds
  # only on Linux. The node's feed is the one home.nix declares for a machine
  # in a realm, so the server under test passes the page what a machine's does.
  murmurs-site-reads-live =
    let
      murmurs = sites.localSite {
        name = "web";
        src = ../mycelium/murmurs;
        inherit webKit;
        tint = "program";
      };
    in
    pkgs.runCommandLocal "murmurs-site-reads-live"
      {
        env = {
          LOCAL_SITES = lib.getExe' (pkgs.callPackage ./package.nix { }) "local-sites";
          MURMURS_SITE = "${murmurs}";
          NODE_KINDS = builtins.toJSON inRealm.jitsusama.local-sites.feeds.node.kinds;
          LOCAL_SITES_SHARED = "${shared}";
          STRIP = "${./home/strip.css}";
          WEB_SKILL = "${../theme/web/skill}";
          PLAYWRIGHT_BROWSERS_PATH = "${pkgs.callPackage ./headless-browsers.nix { }}";
          PYTHONPATH = "${./tests}";
          PYTHONDONTWRITEBYTECODE = "1";
        };
      }
      ''
        HOME=$TMPDIR ${python}/bin/python -m pytest -p no:cacheprovider -p live_page \
          --import-mode=importlib --basetemp="$TMPDIR/pytest" \
          ${../mycelium/murmurs/tests}/murmurs-reads-live-test.py
        touch $out
      '';
  # The forage page, built on the theme's own web kit as a machine builds it,
  # read in headless Chromium through the server beside a stand-in node
  # (modules/mycelium/forage/tests/forage-reads-live-test.py): live with
  # every cell drawn in order, a started or split cell under the one it grew
  # from, what waits on its person and the plan furthest to go in its lede,
  # plans, quest, needs and splits in its panel, its keys, a purpose written
  # followed and drawn as words, a wait answered, a cell going, broken while
  # the node is away and live once it's back, stale once the server goes
  # with its cells kept, within its budget, and the web-design skill's page
  # checks passing at two widths in both looks with nothing logged. The
  # node's feed is the one home.nix declares for a machine in a realm, so
  # dropping a field the page draws (cell, machine, quest, purpose, status,
  # documents, lineage, waits, serves or relies) fails here. Chromium builds
  # only on Linux.
  forage-site-reads-live =
    let
      forage = sites.localSite {
        name = "forage";
        src = ../mycelium/forage;
        inherit webKit;
        tint = "program";
      };
    in
    pkgs.runCommandLocal "forage-site-reads-live"
      {
        env = {
          LOCAL_SITES = lib.getExe' (pkgs.callPackage ./package.nix { }) "local-sites";
          FORAGE_SITE = "${forage}";
          NODE_KINDS = builtins.toJSON inRealm.jitsusama.local-sites.feeds.node.kinds;
          LOCAL_SITES_SHARED = "${shared}";
          STRIP = "${./home/strip.css}";
          WEB_SKILL = "${../theme/web/skill}";
          PLAYWRIGHT_BROWSERS_PATH = "${pkgs.callPackage ./headless-browsers.nix { }}";
          PYTHONPATH = "${./tests}";
          PYTHONDONTWRITEBYTECODE = "1";
        };
      }
      ''
        HOME=$TMPDIR ${python}/bin/python -m pytest -p no:cacheprovider -p live_page \
          --import-mode=importlib --basetemp="$TMPDIR/pytest" \
          ${../mycelium/forage/tests}/forage-reads-live-test.py
        touch $out
      '';
  # Code, built on the theme's own web kit as a machine builds it, read in
  # headless Chromium through the server beside a stand-in repos daemon and a
  # stand-in node (modules/repos/site/tests/code-reads-live-test.py): live
  # with every repository drawn, one growing once a tree is written, one
  # going, a cell at work drawn and linked to its home on the map, broken
  # while the daemon is away and live once it's back, stale once the server
  # goes, the web-design skill's page checks passing at two widths in both
  # looks, and nothing logged. Chromium builds only on Linux.
  code-site-reads-live =
    let
      page = sites.localSite {
        name = "code";
        src = ../repos/site;
        inherit webKit;
        tint = "success";
      };
    in
    pkgs.runCommandLocal "code-site-reads-live"
      {
        env = {
          LOCAL_SITES = lib.getExe' (pkgs.callPackage ./package.nix { }) "local-sites";
          CODE_SITE = "${page}";
          LOCAL_SITES_SHARED = "${shared}";
          STRIP = "${./home/strip.css}";
          WEB_SKILL = "${../theme/web/skill}";
          PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
          PYTHONPATH = "${./tests}";
          PYTHONDONTWRITEBYTECODE = "1";
        };
      }
      ''
        HOME=$TMPDIR ${python}/bin/python -m pytest -p no:cacheprovider -p live_page \
          --import-mode=importlib --basetemp="$TMPDIR/pytest" ${../repos/site/tests}/code-reads-live-test.py
        touch $out
      '';
  # The network page, built on the theme's own web kit as a machine builds
  # it, read in headless Chromium through the server beside a stand-in net
  # and a stand-in node (modules/net/site/tests/network-reads-live-test.py):
  # live with every device drawn in its tree and on its map, a name from
  # the Wi-Fi drawn as words, one going with its link, a machine mycelium
  # runs on following the node and linked to its place on the map, broken
  # while net is away and live once it's back, stale once the server goes,
  # within its budget, the web-design skill's page checks passing at two
  # widths in both looks, and nothing logged. Both feeds are the ones
  # home.nix declares for a machine in a realm, so the server under test
  # passes the page what a machine's does. Chromium builds only on Linux.
  network-site-reads-live =
    let
      page = sites.localSite {
        name = "network";
        src = ../net/site;
        inherit webKit;
        tint = "machine";
      };
    in
    pkgs.runCommandLocal "network-site-reads-live"
      {
        env = {
          LOCAL_SITES = lib.getExe' (pkgs.callPackage ./package.nix { }) "local-sites";
          NETWORK_SITE = "${page}";
          NET_KINDS = builtins.toJSON inRealm.jitsusama.local-sites.feeds.net.kinds;
          NODE_KINDS = builtins.toJSON inRealm.jitsusama.local-sites.feeds.node.kinds;
          LOCAL_SITES_SHARED = "${shared}";
          STRIP = "${./home/strip.css}";
          WEB_SKILL = "${../theme/web/skill}";
          PLAYWRIGHT_BROWSERS_PATH = "${pkgs.callPackage ./headless-browsers.nix { }}";
          PYTHONPATH = "${./tests}";
          PYTHONDONTWRITEBYTECODE = "1";
        };
      }
      ''
        HOME=$TMPDIR ${python}/bin/python -m pytest -p no:cacheprovider -p live_page \
          --import-mode=importlib --basetemp="$TMPDIR/pytest" \
          ${../net/site/tests}/network-reads-live-test.py
        touch $out
      '';
  # mycelium-map-check says the last reading's answer, and when, then that
  # it's looking again, before a stand-in for its browser test opens; it
  # keeps each answer whole for the next run, says nothing on the way as
  # JSON, and hands pytest a test in a folder of its own, never the Nix
  # store, which pytest would list whole first
  # (tests/map-check-answers-soon.sh).
  mycelium-map-check-answers-soon =
    let
      # Stands in for pytest and the browser: says what was beside the test
      # and what had been said on the way before it was opened, then answers.
      slowly = pkgs.writeShellScript "slow-map-test" ''
        test=''${*: -1}
        ls -A "''${test%/*}" > "$MAP_CHECK_BESIDE"
        cat "$MAP_CHECK_ERR" > "$MAP_CHECK_BEFORE"
        printf '%s' '{"at":"2026-01-02T14:09:54.139Z","cells":3,"count":0,"failures":[]}' \
          > "$MAP_CHECK_SAID"
      '';
      check = import ./map-check.nix {
        inherit lib pkgs;
        python = "${slowly}";
        test = "${./tests/live_page.py}";
        livePage = "${./tests}";
      };
    in
    pkgs.runCommandLocal "mycelium-map-check-answers-soon" { nativeBuildInputs = [ pkgs.jq ]; } ''
      ${lib.getExe pkgs.bash} ${./tests/map-check-answers-soon.sh} ${lib.getExe check}
      touch $out
    '';
  # clippy finds nothing in the server, its tests included.
  local-sites-passes-clippy = passesClippy "local-sites-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );

  # The system page, laid out as a folder that stands alone, loads nothing
  # from outside it and opens with no strip; a site that still loads
  # something from outside fails its build, saying which file and address;
  # and a link to another site stays
  # (tests/a-live-site-stands-alone-as-a-folder.sh). Its web kit is the
  # Linux workstation's, so it builds only here too.
  a-live-site-stands-alone-as-a-folder =
    let
      # Built on the real web kit, as a folder that leaves would be, so the
      # one it carries in is the theme's own.
      site = sites.localSite {
        name = "system";
        src = ../system/site;
        inherit webKit;
        tint = "action";
      };
    in
    pkgs.runCommandLocal "a-live-site-stands-alone-as-a-folder" { } ''
      ${lib.getExe pkgs.bash} ${./tests/a-live-site-stands-alone-as-a-folder.sh} ${
        lib.escapeShellArgs [
          "${sites.standalone {
            name = "system";
            inherit site;
          }}"
          ./standalone.sh
          "${site}"
          "${sites.kit}/kit"
          "${site.webKit}"
        ]
      }
      touch $out
    '';
}
