# The checks of local-sites, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  lib,
  onLinux,
  passesClippy,
  pkgs,
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

  # The code site's writer keeps its last record while repos fails, and
  # replaces it whole once repos answers
  # (tests/code-site-keeps-its-last-record.sh).
  code-site-keeps-its-last-record = pkgs.runCommandLocal "code-site-keeps-its-last-record" { } ''
    ${lib.getExe pkgs.bash} ${./tests/code-site-keeps-its-last-record.sh} ${
      lib.escapeShellArgs [
        ./code-site.sh
        "${pkgs.coreutils}/bin"
      ]
    }
    touch $out
  '';

  # The mycelium site's writer keeps its last map while no node answers, and
  # writes the live sessions alone, without a message's words, once one does
  # (tests/mycelium-map-keeps-its-last-map.sh).
  mycelium-map-keeps-its-last-map = pkgs.runCommandLocal "mycelium-map-keeps-its-last-map" { } ''
    ${lib.getExe pkgs.bash} ${./tests/mycelium-map-keeps-its-last-map.sh} ${
      lib.escapeShellArgs [
        ./mycelium-map.sh
        ./mycelium-map.jq
        (lib.getExe pkgs.jq)
        "${pkgs.coreutils}/bin"
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
  # clippy finds nothing in the server, its tests included.
  local-sites-passes-clippy = passesClippy "local-sites-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
