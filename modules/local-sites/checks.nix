# The checks of local-sites, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  lib,
  onLinux,
  passesClippy,
  pkgs,
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
  # strips each page that asks, and changes nothing on a second run
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
  # The map, built on the theme's own web kit as a machine builds it, read in
  # headless Chromium through the server beside a stand-in node
  # (modules/mycelium/site/tests/map-reads-live-test.py): live with the realm
  # drawn, a cell moving and another going, broken while the node is away
  # and live once it's back, stale once the server goes, the web-design
  # skill's page checks and the map's own (site-checks.js) passing at two
  # widths in both looks, and nothing logged. Chromium builds only on Linux.
  mycelium-site-reads-live =
    let
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
          --import-mode=importlib --basetemp="$TMPDIR/pytest" ${../mycelium/site/tests}/map-reads-live-test.py
        touch $out
      '';
  # clippy finds nothing in the server, its tests included.
  local-sites-passes-clippy = passesClippy "local-sites-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
