# The checks of local-sites, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
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
  # skip link, saying only that it's an app; and a page that carries its
  # own strip fails its build, saying so
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
            "${./kit}"
            "${../system/site}"
            "system"
            "${pkgs.emptyDirectory}"
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
      inRealm = account (bareNixos [
        self.nixosModules.workstation
        {
          home-manager.users.someone = {
            imports = [ self.homeModules.local-sites ];
            jitsusama.mycelium.realm = "harbour";
          };
        }
      ]);
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
  # puts each row Nix declares in its recorded row's place, strips each page
  # that asks, names documents with scripts of their own, draws its mark in
  # each look's tint or the other's where one is missing, leaves out and says
  # a tint or ground that isn't a colour, and changes nothing on a second run
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

  # The code site's writer keeps its last homes while repos fails, and
  # replaces them whole once repos answers, each at Code's address for it
  # (tests/code-site-keeps-its-last-homes.sh).
  code-site-keeps-its-last-homes = pkgs.runCommandLocal "code-site-keeps-its-last-homes" { } ''
    ${lib.getExe pkgs.bash} ${./tests/code-site-keeps-its-last-homes.sh} ${
      lib.escapeShellArgs [
        ./code-site.sh
        (lib.makeBinPath [
          pkgs.coreutils
          pkgs.jq
        ])
      ]
    }
    touch $out
  '';
}
# The server is a Linux user service, and builds only there.
// onLinux {
  # Its tests run as it builds, the server started as its unit starts it
  # beside a stand-in daemon (tests/serve.rs): the documents and a mount
  # served, a stranger's name and a stranger's page refused, two pages
  # sharing one watch on the daemon and each told every change trimmed to
  # what the feed shows, an act refused before it reaches the daemon, and a
  # daemon that isn't there said broken with why.
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
      python = pkgs.python3.withPackages (python: [
        python.playwright
        python.pytest
      ]);
      page = sites.localSite {
        name = "system";
        src = ../system/site;
        webKit = (account workstation).xdg.dataFile."jitsusama/web".source;
        tint = "action";
      };
    in
    pkgs.runCommandLocal "system-site-reads-live"
      {
        env = {
          LOCAL_SITES = lib.getExe' (pkgs.callPackage ./package.nix { }) "local-sites";
          SYSTEM_SITE = "${page}";
          LOCAL_SITES_KIT = "${sites.kit}";
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
    let
      python = pkgs.python3.withPackages (python: [
        python.playwright
        python.pytest
      ]);
    in
    pkgs.runCommandLocal "home-answers-its-keys"
      {
        env = {
          LOCAL_SITES = lib.getExe' (pkgs.callPackage ./package.nix { }) "local-sites";
          LOCAL_SITES_KIT = "${sites.kit}";
          STRIP = "${./home/strip.css}";
          WEB_SKILL = "${../theme/web/skill}";
          WEB_KIT = "${(account workstation).xdg.dataFile."jitsusama/web".source}";
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
  # the web-design skill's page checks and the map's own (site-checks.js)
  # passing at two widths in both looks, nothing logged but what a test
  # causes, and every cell reached (reach.ts) there and on a twin of 10,000.
  # Chromium builds only on Linux.
  # The node's feed is the one home.nix declares for a machine in a realm, so
  # the server under test passes the page exactly what a machine's does.
  mycelium-site-reads-live =
    let
      inRealm = account (bareNixos [
        self.nixosModules.workstation
        {
          home-manager.users.someone = {
            imports = [ self.homeModules.local-sites ];
            jitsusama.mycelium.realm = "harbour";
          };
        }
      ]);
      python = pkgs.python3.withPackages (python: [
        python.playwright
        python.pytest
      ]);
      webKit = (account workstation).xdg.dataFile."jitsusama/web".source;
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
          LOCAL_SITES_KIT = "${sites.kit}";
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
  # head shows stepping the cursor, and the web-design skill's page checks
  # passing at two widths in both looks with nothing logged. Chromium builds
  # only on Linux. The node's feed is the one home.nix declares for a machine
  # in a realm, so the server under test passes the page what a machine's does.
  murmurs-site-reads-live =
    let
      inRealm = account (bareNixos [
        self.nixosModules.workstation
        {
          home-manager.users.someone = {
            imports = [ self.homeModules.local-sites ];
            jitsusama.mycelium.realm = "harbour";
          };
        }
      ]);
      python = pkgs.python3.withPackages (python: [
        python.playwright
        python.pytest
      ]);
      murmurs = sites.localSite {
        name = "web";
        src = ../mycelium/murmurs;
        webKit = (account workstation).xdg.dataFile."jitsusama/web".source;
        tint = "program";
      };
    in
    pkgs.runCommandLocal "murmurs-site-reads-live"
      {
        env = {
          LOCAL_SITES = lib.getExe' (pkgs.callPackage ./package.nix { }) "local-sites";
          MURMURS_SITE = "${murmurs}";
          NODE_KINDS = builtins.toJSON inRealm.jitsusama.local-sites.feeds.node.kinds;
          LOCAL_SITES_KIT = "${sites.kit}";
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
      python = pkgs.python3.withPackages (python: [
        python.playwright
        python.pytest
      ]);
      page = sites.localSite {
        name = "code";
        src = ../repos/site;
        webKit = (account workstation).xdg.dataFile."jitsusama/web".source;
        tint = "success";
      };
    in
    pkgs.runCommandLocal "code-site-reads-live"
      {
        env = {
          LOCAL_SITES = lib.getExe' (pkgs.callPackage ./package.nix { }) "local-sites";
          CODE_SITE = "${page}";
          LOCAL_SITES_KIT = "${sites.kit}";
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
}
