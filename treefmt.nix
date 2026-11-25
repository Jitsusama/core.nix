# How `nix fmt` formats and lints this repository, and what the formatting
# check holds it to: every linter core shares (treefmtModules.treefmt, from
# modules/treefmt/treefmt.nix), at its strictest, test code's included, and
# what only core has besides. ruff.toml and mypy.ini say how for Python,
# .shellcheckrc for shell, .luacheckrc for Lua, biome.jsonc for JavaScript,
# TypeScript and JSON, and .rumdl.toml for Markdown.
{ pkgs, ... }:
let
  # TOML and JSON laid out by hand before taplo and biome read them, each in
  # the source of a package a machine runs, so laying one out builds that
  # package and every machine and VM test holding it again. Each is linted as
  # it stands, and is laid out with the next change made to it, which builds
  # them again anyway: taking it off its list is part of that change.
  tomlLaidOutByHand = [
    "modules/eero/Cargo.toml"
    "modules/reader/Cargo.toml"
    "modules/theme/flexoki-light.toml"
  ];
  jsonLaidOutByHand = [
    "modules/eero/contract/device.02:00:00:00:00:01.json"
    "modules/eero/contract/eero.2001.connections.json"
    "modules/eero/contract/eero.2001.json"
    "modules/eero/contract/eero.2002.connections.json"
    "modules/eero/contract/eero.2002.json"
    "modules/eero/contract/eero.2003.connections.json"
    "modules/eero/contract/eero.2003.json"
    "modules/eero/contract/net.devices.json"
    "modules/eero/contract/net.eeros.json"
    "modules/eero/contract/net.profiles.json"
    "modules/eero/contract/network.json"
    "modules/eero/home-network.json"
    # Cut from the one words.json, each thing's lines as they were laid out.
    "modules/quickshell/launcher/words/*.json"
    "modules/spend-watch/feeds/claude/tests/logins/expired.json"
    "modules/spend-watch/feeds/claude/tests/logins/no-profile-scope.json"
    "modules/spend-watch/feeds/claude/tests/logins/pro.json"
    "modules/spend-watch/feeds/claude/tests/logins/ultra.json"
    "modules/spotify/spotify-recent/fixtures/token.json"
    # Written by merge-net as serde lays JSON out, a subtest a line, so a
    # quarantine it commits never leaves main unformatted.
    "tools/merge-net/quarantine.json"
  ];
in
{
  # A file no formatter reads stops `nix fmt` and the formatting check, so a
  # new kind of file can't slip past every linter unseen. What no linter
  # reads yet is named below by kind, each with its reason; giving a kind
  # its linter takes its line off.
  settings.on-unmatched = "fatal";

  settings.excludes = [
    # The shell's QML is read by quickshell-passes-qmllint as the account
    # installs it; the QML cases in tests/ run in Qt's own engine, unlinted.
    "*.qml"
    # JSON Lines, a record a line, which biome doesn't read, and a login
    # file broken on purpose for the test that reads it.
    "*.jsonl"
    # Binary property lists, as an Apple TV answers net, which no linter
    # reads; net's tests read the recorded one.
    "*.plist"
    "modules/spend-watch/feeds/claude/tests/logins/garbled.json"
    # What programs and devices answered, recorded for the tests to compare
    # byte for byte, and the stand-in keys and signatures they sign with.
    "modules/eero/src/views/golden/*"
    "modules/firmware-errors/tests/*.crashlog"
    "modules/net/contract/switch/*.htm"
    "modules/guide/fixtures/*"
    "modules/inkscape/tests/inkscape-recent.xbel"
    "modules/signing/tests/*.sig"
    "*.pem"
    "tests/vm/admin/update-to-main-signer"
    # A command's stand-in state, whose times commands-agree writes in as
    # it renders the view, so it's JSON only once they're there.
    "*.json.in"
    "*.jsonl.in"
    # C and C++ built by the kernel's, QMK's and CMake's own builds, which
    # stop on an error but don't lint.
    "*.c"
    "*.cpp"
    "*.h"
    "*/Kbuild"
    "*/CMakeLists.txt"
    "*/rules.mk"
    # Settings in a program's own format, read by the program they set.
    "*.conf"
    "*/kernel/*.config"
    "*.ini"
    "*.kdl"
    "*.quirk"
    "*.script"
    "*.svg"
    "*.xml"
    ".luacheckrc"
    ".shellcheckrc"
    "modules/ghostty/config"
    "modules/gimp/gimprc"
    "modules/gimp/shortcutsrc"
    "modules/ripgrep/ignore"
    # polkit's rules are JavaScript and udev's their own lines; the nft,
    # bpftrace and jq programs are read only by their own tools as they run.
    "*.rules"
    "*.nft"
    "*.bt"
    "*.jq"
    # The local sites' home page, a shell its builder writes the list into,
    # which home-builds-from-its-rows reads and the web skill's probes walk
    # in a browser, as every site's page is.
    "modules/local-sites/home/index.html"
    # The mycelium map's drawing, its page and its styles, served as they
    # were written by hand before the map was fed live: they come under
    # biome as the drawing is rewritten in TypeScript on the kit, as the
    # system page was, and their lines go then.
    "modules/mycelium/site/view.js"
    "modules/mycelium/site/index.html"
    "modules/mycelium/site/site.css"
    # Word lists, one per line: a dictionary, glyphs, paths and check names.
    "*.txt"
    "*.add"
    # What was said in a meeting, a line each with its times and speaker,
    # that call-transcripts' score holds what it hears to.
    "*.ref"
    # The licences that came with code taken from another project, kept
    # beside it word for word.
    "*/LICENSE.*"
    # The launcher's matcher, being redesigned, comes under biome as it's
    # rewritten (as qt-javascript-lint says).
    "modules/quickshell/Fuzzy.js"
  ];

  # A stand-in program for a test, named as the program it stands in for,
  # so its kind isn't in its name.
  settings.formatter.shellcheck.includes = [ "modules/quickshell/in-a-terminal/tests/qs" ];

  # The libraries core's own Python imports.
  jitsusama.mypy.packages =
    python:
    [
      python.dbus-fast
      python.fido2
      # The desktop test's caller of the portal.
      python.jeepney
      # The noise-cancelled microphone's export, and the stand-in it loads.
      python.numpy
      # The desktop test reads its shots with Pillow.
      python.pillow
      # The local sites' page tests drive Chromium with it.
      python.playwright
      # The live pages' heads gate parses each page as a browser does with it.
      python.selectolax
      # The desktop test's window named as Tuple's.
      python.xlib
    ]
    # The desktop test's touchscreen, which only Linux builds, so a Mac's
    # `nix fmt` still evaluates (mypy.ini says how it's read without it).
    ++ pkgs.lib.optional pkgs.stdenv.hostPlatform.isLinux python.evdev;

  # What the live sites' page tests share, which each page's test imports
  # by name, as their checks put the folder on Python's path. niri's
  # stand-in client too, which the windows daemon's prompt check imports
  # beside niri's own checks. And the nested niri's driver, which the
  # shell's niri cases import, and the tests folder's glossary reader and
  # its replaced names, which the shell's cases import as their checks copy
  # them in beside them. And what the sites' VM tests share, which each
  # imports as its driver's plugin, and what the desktop test's areas share,
  # with how they wait on what's reported, which those in its launcher and
  # looks folders import from the folder above.
  # Each is named alone, so an edit to a file beside them rebuilds no mypy.
  jitsusama.mypy.paths = [
    ./modules/local-sites/tests
    ./modules/mycelium/surfaces
    ./modules/niri/tests
    ./modules/quickshell/tests/launcher_on_niri.py
    ./tests/desktop/desktop_session.py
    ./tests/desktop/heard.py
    ./tests/repository/glossary.py
    ./tests/repository/names.py
    ./tests/vm/sites/sites_session.py
  ];

  # The JavaScript QML imports is read as core's linters read every
  # repository's (modules/treefmt/treefmt.nix), but for these.
  settings.formatter.qt-javascript-lint = {
    # The launcher is being redesigned, its matcher with it, which comes
    # under biome as it's rewritten. Chrome's extension isn't QML's.
    excludes = [
      "modules/quickshell/Fuzzy.js"
      "modules/chrome/open-tabs/extension/*.js"
    ];
  };

  # Chrome's open-tabs extension, read by biome as the browser runs it.
  settings.formatter.browser-javascript-lint = {
    command = pkgs.lib.getExe pkgs.biome;
    options = [
      "lint"
      "--error-on-warnings"
      "--max-diagnostics=none"
      "--config-path=biome.jsonc"
    ];
    includes = [ "modules/chrome/open-tabs/extension/*.js" ];
  };

  # Each local site's page, its markup and the script in its head, read by
  # biome as a browser reads them (modules/local-sites/site.nix builds them),
  # and the page call-under-load's test call runs in.
  settings.formatter.site-html-lint = {
    command = pkgs.lib.getExe pkgs.biome;
    options = [
      "lint"
      "--error-on-warnings"
      "--max-diagnostics=none"
      "--config-path=biome.jsonc"
    ];
    includes = [
      "modules/*/site/*.html"
      "modules/mycelium/forage/*.html"
      "modules/mycelium/murmurs/*.html"
      "modules/call-under-load/page.html"
    ];
  };

  # The web kit's CSS, read by biome as a browser reads it.
  settings.formatter.css-lint = {
    command = pkgs.lib.getExe pkgs.biome;
    options = [
      "lint"
      "--error-on-warnings"
      "--max-diagnostics=none"
      "--config-path=biome.jsonc"
    ];
    includes = [ "*.css" ];
  };

  # taplo lints every TOML file, and lays out all but those laid out by hand.
  programs.taplo.excludes = tomlLaidOutByHand;

  # biome lints the JSON laid out by hand as it stands.
  settings.formatter.json-lint = {
    command = pkgs.lib.getExe pkgs.biome;
    options = [
      "lint"
      "--error-on-warnings"
      "--max-diagnostics=none"
      "--config-path=biome.jsonc"
    ];
    includes = jsonLaidOutByHand;
  };

  # biome formats and lints Node's JavaScript, the .mjs tests, pi's
  # TypeScript extensions and the rest of the JSON, as biome.jsonc's overrides
  # for them say.
  settings.formatter.biome = {
    command = pkgs.lib.getExe pkgs.biome;
    options = [
      "check"
      "--write"
      "--error-on-warnings"
      "--max-diagnostics=none"
      "--config-path=biome.jsonc"
    ];
    includes = [
      "*.json"
      "*.jsonc"
      "*.mjs"
      "*.ts"
    ];
    excludes = jsonLaidOutByHand;
  };
}
